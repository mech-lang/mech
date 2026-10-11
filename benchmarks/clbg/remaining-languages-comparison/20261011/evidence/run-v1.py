"""Sequential, resumable full-workload measurements of the frozen language cohort."""
import argparse
import json
import os
import random
import statistics
import subprocess
from datetime import datetime,timezone
from dotnet_fetch import D,sha
from probe import execute,save,compact,REQUIRED,WORK

BENCHES=['pidigits','spectralnorm','nbody','mandelbrot','regexredux','binarytrees','fannkuchredux','fasta','knucleotide','revcomp']

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--report',default='native-timings.json')
    parser.add_argument('--benchmarks',default=','.join(BENCHES))
    parser.add_argument('--languages',default='native')
    parser.add_argument('--samples',type=int,default=3)
    parser.add_argument('--timeout',type=int,default=14400)
    args=parser.parse_args();assert args.samples>=3
    audit_path=D/'source-audit.json';audit=json.loads(audit_path.read_text());assert audit['status']=='reviewed'
    selected=[r for r in audit['programs'] if (r['language']!='micropython' if args.languages=='native'
        else r['language'] in args.languages.split(','))]
    path=D/'results'/args.report;path.parent.mkdir(exist_ok=True)
    rows={r['key']:dict(r,samples_seconds=[],sample_validation=[]) for r in selected+audit['controls']}
    report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),programs=rows,
        sample_count=args.samples,orders={},discarded=[],completed_benchmarks=[],source_audit=str(audit_path),source_audit_sha256=sha(audit_path),
        protocol=audit['timing_boundary'],machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),
        os=subprocess.check_output(['sw_vers'],text=True).strip(),workloads=WORK,power=compact.power_profile(),runtimes=audit['runtimes'])
    assert report['sample_count']==args.samples and report['source_audit_sha256']==sha(audit_path)
    assert set(report['programs'])==set(rows)
    # Verify all bytes once, then avoid rereading huge Pharo/library images before
    # every short sample. Changed metadata causes a full SHA check and failure.
    file_pins=audit['pin_files']|audit['libraries']|{r['path']:r['sha256'] for r in audit['runtimes'].values()}
    for r in rows.values():
        for f in ('source','binary'):file_pins[r[f]]=r[f+'_sha256']
    stats={}
    def pins():
        for filename,digest in file_pins.items():
            stat=os.stat(filename);state=(stat.st_dev,stat.st_ino,stat.st_size,stat.st_mtime_ns,stat.st_ctime_ns)
            if stats.get(filename)!=state:
                assert sha(filename)==digest,filename
                stats[filename]=state
    pins();save(path,report)
    for bench in args.benchmarks.split(','):
        candidates=[r for r in report['programs'].values() if r['benchmark']==bench]
        assert candidates
        rng=random.Random(20261011+sum(map(ord,bench)))
        for sample in range(args.samples):
            order=candidates.copy();rng.shuffle(order)
            orders=report['orders'].setdefault(bench,[])
            if len(orders)<=sample:orders.append([r['key'] for r in order])
            else:assert orders[sample]==[r['key'] for r in order]
            for row in order:
                if len(row['samples_seconds'])>sample or row.get('full_failure'):continue
                while True:
                    compact.wait_for_environment(REQUIRED);pins()
                    report['active']=dict(key=row['key'],sample=sample+1,started_at=datetime.now(timezone.utc).isoformat())
                    save(path,report);print('RUN',sample+1,row['id'],flush=True)
                    try:result=execute(row,True,args.timeout)
                    except Exception as error:
                        # A failing full workload is excluded rather than assigned
                        # a misleading timing. Other candidates continue intact.
                        row['full_failure']=dict(error=str(error),at=datetime.now(timezone.utc).isoformat())
                        report.pop('active',None);save(path,report)
                        print('FULL FAILURE',row['id'],str(error)[:1000],flush=True);break
                    if result['competing_compilers'] or result['power_changes']:
                        report['discarded'].append(dict(key=row['key'],sample=sample+1,result=result));save(path,report);continue
                    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation'])
                    row.setdefault('full_validation',result['validation']);row['median_seconds']=statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds'])>1:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
                    report.pop('active',None);save(path,report)
                    print('SAMPLE',sample+1,row['id'],f"{result['seconds']:.6f}s",flush=True);break
        if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
        save(path,report)
    report['status']='complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial'
    report['finished_at']=datetime.now(timezone.utc).isoformat();pins();save(path,report)
    print('REPORT',report['status'],flush=True)

if __name__=='__main__':main()
