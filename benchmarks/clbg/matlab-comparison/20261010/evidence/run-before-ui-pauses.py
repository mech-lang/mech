"""Guarded, resumable, sequential MATLAB full-suite measurements."""
import argparse,hashlib,importlib.util,json,random,statistics,subprocess
from datetime import datetime,timezone
from pathlib import Path
D=Path(__file__).resolve().parent;C=D.parents[1]
spec=importlib.util.spec_from_file_location('array_runner',C/'array-comparison/all-20261009/run.py')
arrays=importlib.util.module_from_spec(spec);spec.loader.exec_module(arrays)
compact=arrays.compact;WORK=arrays.WORK;REQUIRED=arrays.REQUIRED
BENCHES=['spectralnorm','mandelbrot','regexredux','revcomp','fasta','nbody','pidigits','fannkuchredux','knucleotide','binarytrees']
def sha(p):
    with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def execute(row,full,timeout):
    if row['benchmark']=='nbody':return compact.execute(row,full,timeout,watch_compilers=True,expected_power=REQUIRED)
    return arrays.execute(row,full,timeout)
def save(path,data):
    tmp=path.with_suffix('.json.tmp');tmp.write_text(json.dumps(data,indent=2)+'\n');tmp.replace(path)
def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--phase',choices=['small','measure'],default='measure');ap.add_argument('--benchmarks',default=','.join(BENCHES));ap.add_argument('--timeout',type=int,default=28800);args=ap.parse_args()
    built=json.loads((D/'results/builds.json').read_text());selected=args.benchmarks.split(',');assert set(selected)<=set(BENCHES)
    def pins():
        for r in built['runtimes'].values():assert sha(r['path'])==r['sha256'],r['path']
        for r in built['programs']:assert sha(r['source'])==r['source_sha256'],r['source']
        for r in built['controls']:assert sha(r['source'])==r['source_sha256'] and sha(r['binary'])==r['binary_sha256']
    pins()
    smallpath=D/'results/small-checks.json'
    small=json.loads(smallpath.read_text()) if smallpath.exists() else dict(status='running',programs={},power=REQUIRED)
    if args.phase=='small':
        for row in built['programs']:
            if row['benchmark'] not in selected:continue
            previous=small['programs'].get(row['key'],{})
            if previous.get('source_sha256')==row['source_sha256'] and previous.get('status')=='passed':continue
            compact.wait_for_environment(REQUIRED);print('SMALL',row['id'],flush=True)
            try:
                result=execute(row,False,args.timeout)
                assert not result['competing_compilers'] and not result['power_changes']
                small['programs'][row['key']]=dict(status='passed',source_sha256=row['source_sha256'],**result)
                print('PASS',row['id'],f'{result["seconds"]:.6f}s',flush=True)
            except Exception as error:
                small['programs'][row['key']]=dict(status='failed',source_sha256=row['source_sha256'],error=str(error));print('FAIL',row['id'],str(error)[:400],flush=True)
            save(smallpath,small)
        small['status']='passed' if all(small['programs'].get(r['key'],{}).get('status')=='passed' and small['programs'][r['key']]['source_sha256']==r['source_sha256'] for r in built['programs']) else 'partial'
        save(smallpath,small);return
    audit=json.loads((D/'source-audit.json').read_text());assert audit['status']=='reviewed' and audit['builds_sha256']==sha(D/'results/builds.json');assert small['status']=='passed'
    rows={}
    for r in built['programs']:
        row=r.copy();a=r['loc_audit'];row.update(loc=a['normalized_loc'],physical_loc=a['physical_loc'],loc_expansions=a['expansions'],algorithm=audit['requirements'][r['benchmark']],small_validation=small['programs'][r['key']]['validation'],samples_seconds=[],sample_validation=[],eligible_for_chart=True)
        rows[row['key']]=row
    for r in built['controls']:
        row=r.copy();row.update(samples_seconds=[],sample_validation=[],eligible_for_chart=True);rows[row['key']]=row
    path=D/'results/timings.json'
    report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),sample_count=3,programs=rows,completed_benchmarks=[],orders={},discarded=[],machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),os=subprocess.check_output(['sw_vers'],text=True).strip(),power=compact.power_profile(),workloads=WORK,runtimes=built['runtimes'],runtime_policy=built['runtime_policy'],timing_boundary=built['timing_boundary'],source_pins={f:sha(D/f) for f in ['prepare.py','run.py','loc.py','audit.py','source-audit.json','results/builds.json','results/small-checks.json']},protocol='Three shuffled sequential fresh-process full-workload samples per compact/optimized MATLAB source and unchanged Mech AOT control. Every complete output is validated after timing; the first full sample also establishes full validation. Startup, licensing checks, JIT, input, computation, full output and teardown are included. Base MATLAB standard operations, no native benchmark executors. AC power, Low Power Mode off; compiler and power changes invalidate samples. Sources, runtimes, work audit and normalized LOC are pinned.')
    assert report['sample_count']==3
    def allpins():
        pins()
        for f,h in report['source_pins'].items():assert sha(D/f)==h,f
        for f,h in audit['evidence'].items():assert sha(D/f)==h,f
    save(path,report);allpins()
    for bench in selected:
        candidates=[r for r in report['programs'].values() if r['benchmark']==bench]
        rng=random.Random(20261010+sum(map(ord,bench)))
        for round_no in range(3):
            order=candidates.copy();rng.shuffle(order);report['orders'].setdefault(bench,[])
            if len(report['orders'][bench])<=round_no:report['orders'][bench].append([r['key'] for r in order])
            else:assert report['orders'][bench][round_no]==[r['key'] for r in order]
            for row in order:
                if len(row['samples_seconds'])>round_no:continue
                while True:
                    compact.wait_for_environment(REQUIRED);allpins();print('RUN',round_no+1,row['id'],flush=True)
                    result=execute(row,True,args.timeout)
                    if result['competing_compilers'] or result['power_changes']:report['discarded'].append(dict(key=row['key'],round=round_no+1,result=result));save(path,report);continue
                    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation']);row.setdefault('full_validation',result['validation']);row['median_seconds']=statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds'])>1:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
                    print('SAMPLE',round_no+1,row['id'],f'{result["seconds"]:.6f}s',flush=True);save(path,report);break
        if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
        save(path,report)
    report['status']='complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial';report['finished_at']=datetime.now(timezone.utc).isoformat();allpins();save(path,report);print('REPORT',path,report['status'])
if __name__=='__main__':main()
