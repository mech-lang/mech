"""Run a frozen language cohort sequentially with complete output validation."""
import argparse
import json
import random
import statistics
import subprocess
from datetime import datetime,timezone
from bootstrap import D,sha
from probe import execute,save,compact,REQUIRED,WORK

BENCHES=['pidigits','spectralnorm','nbody','mandelbrot','regexredux','binarytrees','fannkuchredux','fasta','knucleotide','revcomp']

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--audit',required=True)
    parser.add_argument('--report',required=True)
    parser.add_argument('--benchmarks',default=','.join(BENCHES))
    parser.add_argument('--samples',type=int,default=3)
    parser.add_argument('--timeout',type=int,default=14400)
    args=parser.parse_args()
    assert args.samples>=3
    audit=json.loads((D/args.audit).read_text())
    assert audit['status']=='reviewed'
    path=D/'results'/args.report
    rows={r['key']:dict(r,samples_seconds=[],sample_validation=[]) for r in audit['programs']+audit['controls']}
    report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),programs=rows,
        sample_count=args.samples,orders={},discarded=[],completed_benchmarks=[],source_audit=str(D/args.audit),source_audit_sha256=sha(D/args.audit),
        protocol=audit['timing_boundary'],machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),os=subprocess.check_output(['sw_vers'],text=True).strip(),
        workloads=WORK,power=compact.power_profile(),runtimes=audit['runtimes'])
    assert report['sample_count']==args.samples and report['source_audit_sha256']==sha(D/args.audit)
    def pins(row=None):
        for filename,digest in audit['pin_files'].items(): assert sha(filename)==digest,filename
        for filename,digest in audit['libraries'].items(): assert sha(filename)==digest,filename
        for runtime in audit['runtimes'].values(): assert sha(runtime['path'])==runtime['sha256']
        for record in ([row] if row else rows.values()):
            assert sha(record['source'])==record['source_sha256'] and sha(record['binary'])==record['binary_sha256'],record['key']
    pins()
    save(path,report)
    for bench in args.benchmarks.split(','):
        candidates=[r for r in report['programs'].values() if r['benchmark']==bench]
        assert candidates
        rng=random.Random(20261010+sum(map(ord,bench)))
        for sample in range(args.samples):
            order=candidates.copy()
            rng.shuffle(order)
            orders=report['orders'].setdefault(bench,[])
            if len(orders)<=sample: orders.append([r['key'] for r in order])
            else: assert orders[sample]==[r['key'] for r in order]
            for row in order:
                if len(row['samples_seconds'])>sample: continue
                while True:
                    compact.wait_for_environment(REQUIRED)
                    pins(row)
                    print('RUN',sample+1,row['id'],flush=True)
                    result=execute(row,True,args.timeout)
                    if result['competing_compilers'] or result['power_changes']:
                        report['discarded'].append(dict(key=row['key'],sample=sample+1,result=result))
                        save(path,report)
                        continue
                    row['samples_seconds'].append(result['seconds'])
                    row['sample_validation'].append(result['validation'])
                    row.setdefault('full_validation',result['validation'])
                    row['median_seconds']=statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds'])>1: row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
                    print('SAMPLE',sample+1,row['id'],f"{result['seconds']:.6f}s",flush=True)
                    save(path,report)
                    break
        if bench not in report['completed_benchmarks']: report['completed_benchmarks'].append(bench)
        save(path,report)
    report['status']='complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial'
    report['finished_at']=datetime.now(timezone.utc).isoformat()
    pins()
    save(path,report)
    print('REPORT',report['status'],flush=True)

if __name__=='__main__': main()
