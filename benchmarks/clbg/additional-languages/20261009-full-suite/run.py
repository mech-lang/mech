"""Resume guarded, sequential, full-workload Mojo/Futhark CPU measurements."""
import argparse,hashlib,importlib.util,json,random,statistics,subprocess
from datetime import datetime,timezone
from pathlib import Path
D=Path(__file__).resolve().parent;CLBG=D.parents[1]
spec=importlib.util.spec_from_file_location('arrays',CLBG/'array-comparison/all-20261009/run.py')
arrays=importlib.util.module_from_spec(spec);spec.loader.exec_module(arrays)
compact=arrays.compact;compact.COMPILERS|={'mojo','futhark'}
WORK=arrays.WORK;REQUIRED=arrays.REQUIRED
BENCHES=['spectralnorm','regexredux','pidigits','nbody','mandelbrot','revcomp','fasta','binarytrees','knucleotide','fannkuchredux']
def sha(p):
    with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def execute(row,full,timeout):
    return compact.execute(row,full,timeout,watch_compilers=True,expected_power=REQUIRED) if row['benchmark']=='nbody' else arrays.execute(row,full,timeout)
def controls():
    directory=D/'results/controls'
    records=[json.loads(p.read_text()) for p in sorted(directory.glob('*.json'))]
    fixed=json.loads((CLBG/'work-audit/20261009/builds.json').read_text())['programs']
    baseline={r['key']:r for r in json.loads((CLBG/'function-application-audit/20261009T171437Z/baseline.json').read_text())['programs']}
    for row in records:
        preferred=[r for r in fixed.values() if r['key'].startswith(row['chart_key']+':') and r['variant'] in ('floating','four-functions')]
        if preferred:
            r=preferred[0];row|={k:r[k] for k in ('source','source_sha256','binary','binary_sha256')}
            row['audited_variant']=r['variant']
        original=baseline[row['chart_key']]
        row|=dict(key=row['benchmark']+':mech-control',language='mech',command=[row['binary']]+original['command'][1:],runtime_environment=original.get('runtime_environment') or {},samples_seconds=[],sample_validation=[])
    return records
def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--samples',type=int,default=3);ap.add_argument('--timeout',type=int,default=14400)
    ap.add_argument('--benchmarks',default=','.join(BENCHES))
    ap.add_argument('--validate-only',action='store_true');args=ap.parse_args();assert args.samples>=3
    audit=json.loads((D/'source-audit.json').read_text())
    path=D/'results/timings.json'
    report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),sample_count=args.samples,source_audit_sha256=sha(D/'source-audit.json'),programs={},completed_benchmarks=[],orders={},protocol='CPU only, <=8 workers; sequential fresh processes in randomized rounds; elapsed time includes startup, input parsing, language/library execution, full output to a regular file and process teardown. Compilation and full output hashing excluded. AC power, Low Power Mode off; concurrent compiler/power guards. Three full samples per candidate and current Mech fastest control.',machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),workloads=WORK,runtimes=audit['runtimes'],library_pins=audit['libraries'],review_pins={f:sha(D/f) for f in ['profiles.json','results/builds.json','results/stdlib-tests.json','results/tree-work-audit.json','source-audit.json']})
    assert report['source_audit_sha256']==sha(D/'source-audit.json')
    assert report['sample_count']==args.samples
    report.setdefault('external_library_pins',audit.get('external_library_pins',{}))
    def save():
        temp=path.with_suffix('.json.tmp');temp.write_text(json.dumps(report,indent=2)+'\n');temp.replace(path)
    def pins():
        for file,digest in report['review_pins'].items():assert sha(D/file)==digest,file
        for file,digest in report['library_pins'].items():assert sha(D/file)==digest,file
        for file,digest in report['external_library_pins'].items():assert sha(file)==digest,file
        for runtime in report['runtimes'].values():assert sha(runtime['path'])==runtime['sha256']
    for r in audit['rows']:
        row=dict(key=r['id'],benchmark=r['benchmark'],language=r['language'],source=r['source'],source_sha256=r['sha256'],binary=r['binary'],binary_sha256=r['binary_sha256'],loc=r['normalized_loc'],physical_loc=r['physical_loc'],loc_expansions=r['expansions'],algorithm=r['algorithm'],stdlib_files=r['stdlib_files'],source_provenance=r['source_provenance'],candidate_role=r['candidate_role'],eligible_for_chart=True,command=[r['binary']]+([] if r['benchmark'] in ('knucleotide','revcomp','regexredux') else ['{n}']),samples_seconds=[],sample_validation=[])
        report['programs'].setdefault(row['key'],row)
    for row in controls():report['programs'].setdefault(row['key'],row)
    save();pins()
    selected=args.benchmarks.split(',');assert set(selected)<=set(BENCHES)
    for bench in selected:
        rows=[p for p in report['programs'].values() if p['benchmark']==bench]
        for row in rows:
            assert sha(row['source'])==row['source_sha256'],row['key']
            assert sha(row['binary'])==row['binary_sha256'],row['key']
            for full,label in [(False,'small'),(True,'full')]:
                if row.get(label+'_validation'):continue
                compact.wait_for_environment(REQUIRED)
                print('VALIDATE',label,row['key'],flush=True)
                result=execute(row,full,args.timeout)
                row[label+'_validation']=result['validation']
                row[label+'_validation_seconds']=result['seconds']
                print('PASS',label,row['key'],f'{result["seconds"]:.6f}s',flush=True);save()
        if args.validate_only:continue
        rng=random.Random(20261009+sum(map(ord,bench)))
        for round_no in range(args.samples):
            order=rows.copy();rng.shuffle(order)
            report['orders'].setdefault(bench,[])
            if len(report['orders'][bench])<=round_no:report['orders'][bench].append([r['key'] for r in order])
            elif report['orders'][bench][round_no]!=[r['key'] for r in order]:
                pending=[r['key'] for r in order if len(r['samples_seconds'])<=round_no]
                if pending:report.setdefault('supplemental_orders',{}).setdefault(bench,[]).append(dict(round=round_no+1,programs=pending))
            for row in order:
                if len(row['samples_seconds'])>round_no:continue
                while True:
                    compact.wait_for_environment(REQUIRED);pins()
                    print('RUN',round_no+1,row['key'],flush=True)
                    result=execute(row,True,args.timeout)
                    if result['competing_compilers'] or result['power_changes']:
                        report.setdefault('discarded',[]).append(dict(key=row['key'],round=round_no+1,result=result));save();continue
                    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation'])
                    row['median_seconds']=statistics.median(row['samples_seconds'])
                    if len(row['samples_seconds'])>1:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
                    print('SAMPLE',round_no+1,row['key'],f'{result["seconds"]:.6f}s',flush=True);save();break
        if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
        save()
    report['status']='validated' if args.validate_only else ('complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial')
    report['finished_at']=datetime.now(timezone.utc).isoformat();pins();save()
    print('REPORT',path,report['status'],flush=True)
if __name__=='__main__':main()
