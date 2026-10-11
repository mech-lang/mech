"""Sequential, resumable full-workload measurements of BQN and K programs."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
spec = importlib.util.spec_from_file_location('compact', CLBG / 'compact/run.py')
compact = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compact)
REQUIRED = dict(source='AC Power', low_power_mode=0)
RUNTIMES = {l: CLBG / 'array-comparison/nbody-20261009/binaries' / e
            for l, e in [('bqn', 'bqn'), ('k', 'growler')]}

def sha(path):
    with Path(path).open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--benchmarks', default='spectralnorm,knucleotide')
    ap.add_argument('--samples', type=int, default=3)
    ap.add_argument('--timeout', type=int, default=14400)
    args = ap.parse_args()
    assert args.samples >= 3
    path = HERE / 'results/timings.json'
    report = json.loads(path.read_text()) if path.exists() else dict(
        status='running', created_at=datetime.now(timezone.utc).isoformat(),
        machine=subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
        protocol='Sequential fresh processes; randomized rounds; complete stdout to regular files; median and sample SD; startup included; builds, hashing, and comparisons excluded; AC power with Low Power Mode off; compiler and power guards.',
        sample_count=args.samples, programs={}, orders={}, completed_benchmarks=[], exclusions=[])
    assert report['sample_count'] == args.samples
    def save():
        tmp=path.with_suffix('.json.tmp')
        tmp.write_text(json.dumps(report, indent=2)+'\n');tmp.replace(path)
    audit=json.loads((HERE/'source-audit.json').read_text())
    report['admission_policy']='Correct output is necessary but insufficient. A source must pass the required-work audit before new measurements or chart admission. Faster-than-leading results require a documented investigation.'
    for record in audit['rows']:
        source=HERE/'sources'/record['file']
        key=record['benchmark']+':'+record['language']+'-'+source.stem
        algorithm=record.get('algorithm',dict(strict=False,status='unreviewed',reason='Required-work audit is pending.'))
        if key in report['programs']:
            report['programs'][key]['algorithm']=algorithm
            report['programs'][key]['eligible_for_chart']=algorithm.get('strict') is True
            if algorithm.get('strict') is not True:
                report['programs'][key]['measurement_status']='excluded-'+algorithm.get('status','unreviewed')
        if algorithm.get('strict') is not True and not any(e.get('key')==key for e in report['exclusions']):
            report['exclusions'].append(dict(key=key,algorithm=algorithm))
    save()
    builds=json.loads((CLBG/'function-application-audit/20261009T171437Z/application-v3/builds.json').read_text())
    corrections=json.loads((CLBG/'work-audit/20261009/builds.json').read_text())
    for bench in args.benchmarks.split(','):
        records=[r for r in audit['rows'] if r['benchmark']==bench and r.get('algorithm',{}).get('strict') is True]
        if not records:
            print('SKIP no sources passed required-work audit',bench,flush=True)
            continue
        rows=[]
        for record in records:
            source=HERE/'sources'/record['file']
            assert sha(source)==record['sha256']
            key=bench+':'+record['language']+'-'+source.stem
            p=report['programs'].setdefault(key, dict(
                id=key, key=key, benchmark=bench, language=record['language'],
                command=[str(RUNTIMES[record['language']]), str(source), '{n}'],
                source=str(source), source_sha256=sha(source),
                loc=record['normalized_loc'], physical_loc=record['physical_loc'],
                loc_expansions=record.get('expansions', []), source_url=record.get('upstream'),
                algorithm=record['algorithm'], eligible_for_chart=True,
                runtime_sha256=sha(RUNTIMES[record['language']]),
                samples_seconds=[], sample_validation=[]))
            p['command']=[str(RUNTIMES[record['language']]),str(source)]+([] if bench in ('knucleotide','revcomp','regexredux') else ['{n}'])
            assert p['source_sha256']==record['sha256'], 'Use a new source filename or report for a changed source.'
            rows.append(p)
        build=next(p for p in builds['programs'] if p['benchmark']==bench)['builds']['after']
        faithful=[p for p in corrections['programs'].values() if p['benchmark']==bench and p['variant'] in ('four-functions','floating')]
        if faithful:
            build=max(faithful,key=lambda p:p['loc'])  # The measured strict optimized control.
        control_key=bench+':mech-control-'+build['binary_sha256'][:12]
        control=report['programs'].setdefault(control_key, dict(
            id=control_key,key=control_key,benchmark=bench,language='mech',
            command=[build['binary'], '{n}'], source=build['source'],
            source_sha256=build['source_sha256'], binary_sha256=build['binary_sha256'],
            samples_seconds=[],sample_validation=[]))
        for row in [control,*rows]:
            compact.wait_for_environment(REQUIRED)
            if not row.get('small_validation'):
                row['small_validation']=compact.execute(row,False,args.timeout)['validation']
                print('SMALL PASS',row['id'],flush=True);save()
            if not row.get('full_validation'):
                full=compact.execute(row,True,args.timeout)
                row['full_validation']=full['validation'];row['validation_seconds']=full['seconds']
                print('FULL PASS',row['id'],f'{full["seconds"]:.6f}s',flush=True);save()
        rng=random.Random(20261009+sum(map(ord,bench)))
        order_key=bench+':'+hashlib.sha256('|'.join(r['key'] for r in [control,*rows]).encode()).hexdigest()[:12]
        for round_no in range(args.samples):
            order=[control,*rows];rng.shuffle(order)
            report['orders'].setdefault(order_key,[])
            if len(report['orders'][order_key])<=round_no:report['orders'][order_key].append([r['key'] for r in order])
            for row in order:
                if len(row['samples_seconds'])>round_no:continue
                while True:
                    compact.wait_for_environment(REQUIRED)
                    assert sha(row['source'])==row['source_sha256']
                    result=compact.execute(row,True,args.timeout,watch_compilers=True,expected_power=REQUIRED)
                    if result['competing_compilers'] or result['power_changes']:
                        report.setdefault('discarded',[]).append(dict(key=row['key'],round=round_no+1,result=result))
                        save();continue
                    row['samples_seconds'].append(result['seconds'])
                    row['sample_validation'].append(result['validation'])
                    print(f'SAMPLE {round_no+1}/{args.samples}',row['id'],f'{result["seconds"]:.6f}s',flush=True)
                    save();break
        for row in [control,*rows]:
            row['median_seconds']=statistics.median(row['samples_seconds'])
            row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
        report['completed_benchmarks'].append(bench);save()
    report['status']='complete_for_prepared_sources'
    report['finished_at']=datetime.now(timezone.utc).isoformat();save()

if __name__=='__main__':main()
