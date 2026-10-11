#!/usr/bin/env python3
"""Remeasure the entire chart cohort on AC power with Low Power Mode disabled."""
import argparse, copy, importlib.util, json, os, random, statistics, sys
from datetime import datetime, timezone
from pathlib import Path

HERE=Path(__file__).resolve().parent;CLBG=HERE.parent;ROOT=CLBG.parents[1]
spec=importlib.util.spec_from_file_location('compact_run',CLBG/'compact/run.py')
run=importlib.util.module_from_spec(spec);spec.loader.exec_module(run)
REQUIRED_POWER=dict(source='AC Power',low_power_mode=0)
DEFAULT_OUTPUT=HERE/'results/apple-m3-ac-normal-20261008.json'

def command_assets(command):
    assets=set()
    for arg in command:
        if '{n}' in arg:continue
        p=Path(arg)
        if not p.is_absolute():p=ROOT/p
        if p.is_file():assets.add(p.resolve())
        elif p.is_dir():assets.update(f.resolve() for f in p.rglob('*.class'))
    return assets

def prepare(args):
    if args.output.exists():raise ValueError('Result already exists; choose a new --output path instead of overwriting measurements')
    studies=json.loads((CLBG/'overview/normalized-results.json').read_text())
    compact=json.loads((CLBG/'compact/results/apple-m3-compact.json').read_text())
    programs=[];archives={};assets=set()
    for study in studies:
        bench=study['key'];reports=[study['report']]
        if bench=='nbody':reports.append('nbody/results/apple-m3-chapel-tricks-50m.json')
        originals={}
        for filename in reports:
            p=CLBG/filename;report=json.loads(p.read_text())
            archives[filename]=dict(sha256=run.sha(p),status='Preserved original measurements; superseded in the new chart by fresh timings')
            for row in report['programs']:originals[row['id']]=(row,filename)
        for display in study['rows']:
            if display.get('endpoint_scope')=='compact-only':continue
            original,filename=originals[display['id']]
            row=copy.deepcopy(original)
            row.update(benchmark=bench,original_report=filename,original_median_seconds=original['median_seconds'],
                original_samples_seconds=original['samples_seconds'],display_metadata=display,
                full_validation_current_batch=False,samples_seconds=[],sample_validation=[])
            row.pop('median_seconds',None);row.pop('comparison_time_ratio',None)
            programs.append(row)
    for original in compact['programs']:
        row=copy.deepcopy(original)
        row.update(original_report='compact/results/apple-m3-compact.json',
            original_samples_seconds=original['samples_seconds'],full_validation_current_batch=False,
            samples_seconds=[],sample_validation=[])
        for key in ('median_seconds','comparison_time_ratio','excluded_full'):row.pop(key,None)
        row['algorithm']=run.classify(row);programs.append(row)
    for row in programs:
        n=str(run.WORK[row['benchmark']][1])
        argv=[arg.replace(n,'{n}') if arg==n or arg=='--n='+n else arg for arg in row['command']]
        if row['benchmark'] not in ('knucleotide','revcomp') and not any('{n}' in arg for arg in argv):argv.append('{n}')
        if row['language']=='chapel':argv=['--n={n}' if arg=='{n}' else arg for arg in argv]
        row['command']=argv
        if not Path(argv[0]).is_file():raise ValueError('Missing executable: '+repr(argv))
        row['runtime_environment']=row.get('runtime_environment',{})
        assets.update(command_assets(argv))
    asset_hashes={str(p):run.sha(p) for p in sorted(assets)}
    for row in programs:row['command_assets']=[str(p) for p in sorted(command_assets(row['command']))]
    report=dict(schema_version=1,created_at=datetime.now(timezone.utc).isoformat(),status='prepared',
        hardware=compact['hardware'],toolchains=compact['toolchains'],
        protocol=dict(workloads=run.WORK,samples=args.samples,timeout_seconds=args.timeout,seed=args.seed,
            required_power=REQUIRED_POWER,
            timed_region='Sequential fresh process through exit, including interpreter/JIT startup and complete stdout to regular file. Builds, tests, hashing and comparisons excluded.',
            normalization='Fresh raw medians from one AC normal-power cohort; no old low-power timings or matched-control rescaling used for new chart scores.',
            compiler_load_guard='Check executable names before samples, at both boundaries and every 10 seconds; discard samples with detected competing compiler activity.',
            power_state_guard='Require AC power with Low Power Mode off. Record the profile and check at boundaries and every 10 seconds. A detected change discards the entire timed benchmark batch, including its repeated control.',
            environment={k:run.ENV[k] for k in ('RAYON_NUM_THREADS','JULIA_NUM_THREADS','OMP_NUM_THREADS','OMP_DYNAMIC','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','OPENBLAS_NUM_THREADS','PYTHONHASHSEED')},
            scope='All 495 existing chart builds plus 80 audited compact-runtime entries across eight benchmarks. Compact-only new runtimes do not establish fastest-language endpoints.'),
        original_reports=archives,compact_manifest_sha256=run.sha(CLBG/'compact/manifest.json'),
        asset_hashes=asset_hashes,programs=programs,excluded=[],completed_benchmarks=[])
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    print('Prepared',len(programs),'builds;',len(asset_hashes),'executable/source/classfile asset pins',flush=True)

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--prepare',action='store_true')
    ap.add_argument('--output',type=Path,default=DEFAULT_OUTPUT);ap.add_argument('--samples',type=int,default=3)
    ap.add_argument('--timeout',type=int,default=14400);ap.add_argument('--seed',type=int,default=20261008)
    ap.add_argument('--benchmarks',default='revcomp,knucleotide,fasta,binarytrees,spectralnorm,nbody,mandelbrot,fannkuchredux')
    args=ap.parse_args();args.required_power=REQUIRED_POWER
    if args.samples<3:ap.error('At least three samples required')
    if args.prepare:prepare(args);return
    report=json.loads(args.output.read_text())
    if args.samples!=report['protocol']['samples']:ap.error('Resume with the prepared sample count')
    if run.sha(CLBG/'compact/manifest.json')!=report['compact_manifest_sha256']:raise ValueError('Compact source manifest changed')
    def save():
        temporary=args.output.with_suffix('.json.tmp');temporary.write_text(json.dumps(report,indent=2)+'\n');temporary.replace(args.output)
    def verify_assets(row):
        for path in row['command_assets']:
            if run.sha(path)!=report['asset_hashes'][path]:raise ValueError('Pinned command asset changed: '+path)
    args.pre_sample=verify_assets;args.post_sample=verify_assets
    for bench in args.benchmarks.split(','):
        if bench in report['completed_benchmarks']:continue
        rows=[r for r in report['programs'] if r['benchmark']==bench and not r.get('excluded_full')]
        for path in sorted({path for row in rows for path in row['command_assets']}):
            if run.sha(path)!=report['asset_hashes'][path]:raise ValueError('Pinned command asset changed: '+path)
        control_id=run.ANCHORS.get(bench,'fasta-gcc-2-llvm')
        control=next(r for r in rows if r['id']==control_id)
        report['status']='running';save();print('BEGIN',bench,len(rows),'full-cohort entries',flush=True)
        for row in rows:
            if row.get('full_validation_current_batch'):continue
            run.wait_for_environment(REQUIRED_POWER)
            try:
                result=run.execute(row,True,args.timeout)
                row['correctness_full']=result['validation'];row['full_validation_seconds']=result['seconds'];row['full_validation_current_batch']=True
                print('full output passed',row['id'],f"{result['seconds']:.6f}",flush=True)
            except Exception as error:
                if row is control:raise
                row['excluded_full']=True;report['excluded'].append(dict(id=row['id'],benchmark=bench,stage='full validation',reason=str(error)))
                print('EXCLUDED FULL',row['id'],str(error),flush=True)
            save()
        timed=[r for r in rows if r is not control and not r.get('excluded_full')]
        orders=run.measure_batch(bench,timed,control,args,report,save)
        for row in [control,*timed]:
            row['median_seconds']=statistics.median(row['samples_seconds'])
            row['min_seconds']=min(row['samples_seconds']);row['max_seconds']=max(row['samples_seconds'])
            row['measurement_batch']=args.output.name
        for row in [control,*timed]:
            row['comparison_time_ratio']=row['median_seconds']/control['median_seconds']
            row['normalization_control']=control['id']
        report.setdefault('sample_order',{})[bench]=orders;report['completed_benchmarks'].append(bench);save()
        print('DONE',bench,flush=True)
    report['status']='complete' if len(report['completed_benchmarks'])==len(run.WORK) else 'partial'
    report['updated_at']=datetime.now(timezone.utc).isoformat();save()

if __name__=='__main__':main()
