#!/usr/bin/env python3
"""Matched n-body code-generation and timing probe, with general compiler flags."""
import argparse, hashlib, importlib.util, json, os, random, statistics, struct, subprocess, sys, tempfile, time
from datetime import datetime, timezone
from pathlib import Path

HERE=Path(__file__).resolve().parent
NBODY=HERE.parents[1]
ROOT=NBODY.parents[2]
sys.path.insert(0,str(NBODY))
from run import command, energies, validate
from chapel_tricks import loop_codegen
spec=importlib.util.spec_from_file_location('compact_probe',NBODY.parent/'compact/run.py')
checks=importlib.util.module_from_spec(spec);spec.loader.exec_module(checks)
POWER=dict(source='AC Power',low_power_mode=0)

def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def state(binary,steps):
    result=command([str(binary),str(steps)],env=checks.ENV|{'MECH_DUMP_STATE':'1'})
    return json.loads(next(line[6:] for line in result.stderr.splitlines() if line.startswith('STATE ')))

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--steps',type=int,default=5_000_000)
    ap.add_argument('--samples',type=int,default=3);ap.add_argument('--cases',default='')
    ap.add_argument('--output',type=Path,required=True);args=ap.parse_args()
    assert not args.output.exists()
    driver=NBODY/'mech-driver/target/release/mech-clbg-nbody-compiled'
    clang='/opt/homebrew/opt/llvm@22/bin/clang-22'
    sdk=command(['xcrun','--show-sdk-path']).stdout.strip()
    flags=['--fp-contract','--algebraic-powers']
    rows=[]
    cases=[('compact-baseline','nbody.mec',flags),('pairwise-baseline','nbody-pairwise.mec',flags),
           ('compact-local','nbody.mec',flags+['--simd-local']),('pairwise-local','nbody-pairwise.mec',flags+['--simd-local']),
           ('compact-local-scheduled','nbody.mec',flags+['--simd-local','--schedule']),
           ('pairwise-local-scheduled','nbody-pairwise.mec',flags+['--simd-local','--schedule']),
           ('compact-tails','nbody.mec',flags+['--pack-state-tails']),
           ('pairwise-tails','nbody-pairwise.mec',flags+['--pack-state-tails']),
           ('compact-local-tails','nbody.mec',flags+['--simd-local','--pack-state-tails']),
           ('pairwise-local-tails','nbody-pairwise.mec',flags+['--simd-local','--pack-state-tails'])]
    cases += [('compact-cost','nbody.mec',flags+['--simd-local']),
              ('pairwise-cost','nbody-pairwise.mec',flags+['--simd-local']),
              ('compact-cost-tails','nbody.mec',flags+['--simd-local','--pack-state-tails']),
              ('pairwise-cost-tails','nbody-pairwise.mec',flags+['--simd-local','--pack-state-tails'])]
    mass_source = 'experiments/20261009-local-simd/nbody-pairwise-mass.mec'
    cases += [('pairwise-mass-baseline',mass_source,flags),
              ('pairwise-mass-tails',mass_source,flags+['--pack-state-tails'])]
    cases += [('compact-llvm-diagnostic','nbody.mec',flags),
              ('pairwise-llvm-diagnostic','nbody-pairwise.mec',flags)]
    cases += [('compact-reassociate','nbody.mec',flags+['--reassociate-constants']),
              ('pairwise-reassociate','nbody-pairwise.mec',flags+['--reassociate-constants']),
              ('compact-reassociate-tails','nbody.mec',flags+['--reassociate-constants','--pack-state-tails']),
              ('pairwise-reassociate-tails','nbody-pairwise.mec',flags+['--reassociate-constants','--pack-state-tails'])]
    if args.cases: cases=[c for c in cases if c[0] in args.cases.split(',')]
    for identifier,source,options in cases:
        c=HERE/f'{identifier}.c';obj=HERE/f'{identifier}.o';binary=HERE/identifier
        llvm = identifier.endswith('llvm-diagnostic')
        generation=[str(driver),'0',str(NBODY/source),*(['emit-c',str(c)] if llvm else ['emit-cranelift-simd',str(c),str(obj)]),'--output','energy',*options]
        command(generation)
        build=[clang,'-O3',*(['-march=native','-ffp-contract=off','-fno-math-errno'] if llvm else []),'-isysroot',sdk,str(c),*([] if llvm else [str(obj)]),'-lm','-o',str(binary)]
        command(build)
        clean='\n'.join(line for line in (NBODY/source).read_text().splitlines() if line.strip() and not line.lstrip().startswith('--'))
        try: codegen=loop_codegen(binary,HERE/f'{identifier}.disassembly.txt')
        except ValueError as error: codegen={'audit_error':str(error)}
        row=dict(id=identifier,language='mech',source=source,loc=len(clean.splitlines()),source_sha256=sha(NBODY/source),
                 generation_command=generation,compile_command=build,object_sha256=None if llvm else sha(obj),binary_sha256=sha(binary),
                 backend='LLVM diagnostic only' if llvm else 'Cranelift AOT',
                 command=[str(binary),'{n}'],codegen=codegen,samples_seconds=[],sample_stdout=[])
        rows.append(row);print('CODEGEN',identifier,json.dumps(codegen),flush=True)
    original=json.loads((NBODY.parent/'retime/results/apple-m3-ac-normal-20261008.json').read_text())
    for identifier in ('chapel-3','rust-3','nbody-gpp-6-llvm-native','nbody-gcc-8-llvm-fastmath'):
        p=next(p for p in original['programs'] if p['benchmark']=='nbody' and p['id']==identifier)
        rows.append(dict(id=identifier,language=p['display_metadata']['lang'],loc=p['source']['nonblank_noncomment_lines'],
                         command=p['command'],binary_sha256=sha(p['command'][0]),samples_seconds=[],sample_stdout=[]))
    states=[]
    for row in rows:
        validate(command([arg.replace('{n}','1000') for arg in row['command']],env=checks.ENV).stdout,[-.169075164,-.169087605])
        if row['language']!='mech' or row['id'].endswith('baseline'):continue
        baseline=HERE/next(p['id'] for p in rows if p.get('source')==row['source'] and p['id'].endswith('baseline'))
        for steps in (0,1,1000,10000):
            actual=state(row['command'][0],steps);expected=state(baseline,steps)
            assert len(actual)==len(expected)
            exact=all(struct.pack('>d',a)==struct.pack('>d',b) for a,b in zip(actual,expected))
            error=max(abs(a-b)/max(1.0,abs(b)) for a,b in zip(actual,expected))
            if '--reassociate-constants' in row['generation_command']: assert error<=1e-8,(row['id'],steps,error)
            else: assert exact,(row['id'],steps)
            states.append(dict(id=row['id'],steps=steps,bit_identical=exact,max_scaled_error=error))
    reference=next(p for p in rows if p['id']=='rust-3')
    expected=energies(command([arg.replace('{n}',str(args.steps)) for arg in reference['command']],env=checks.ENV).stdout)
    if args.steps==50_000_000: validate('\n'.join(map(str,expected)),[-.169075164,-.169059907])
    for row in rows:
        validate(command([arg.replace('{n}',str(args.steps)) for arg in row['command']],env=checks.ENV).stdout,expected)
    report=dict(created_at=datetime.now(timezone.utc).isoformat(),steps=args.steps,samples=args.samples,status='running',
                required_power=POWER,programs=rows,state_checks=states,expected_energy=expected,discarded_samples=[],sample_order=[],
                protocol='Fresh sequential processes; one discarded full-workload warmup; seeded shuffle; AC/LPM and competing compiler checks at boundaries. LOC only. AOT builds and validation excluded from time.')
    rng=random.Random(20261009)
    for sample in range(args.samples):
        order=rows.copy();rng.shuffle(order);report['sample_order'].append([p['id'] for p in order])
        for row in order:
            while True:
                checks.wait_for_environment(POWER)
                monitor=checks.CompilerMonitor(POWER);monitor.start()
                with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
                    start=time.perf_counter_ns()
                    checks.run_process([arg.replace('{n}',str(args.steps)) for arg in row['command']],subprocess.DEVNULL,out,err,300,checks.ENV)
                    elapsed=(time.perf_counter_ns()-start)/1e9
                    monitor.finish();out.seek(0);stdout=out.read().decode()
                    validate(stdout,expected)
                if monitor.observations or monitor.power_changes:
                    report['discarded_samples'].append(dict(id=row['id'],seconds=elapsed,compiler_activity=monitor.observations,power_changes=monitor.power_changes))
                    continue
                row['samples_seconds'].append(elapsed);row['sample_stdout'].append(stdout)
                print('SAMPLE',sample+1,row['id'],f'{elapsed:.9f}',flush=True)
                break
            args.output.write_text(json.dumps(report,indent=2)+'\n')
    for row in rows:
        row['median_seconds']=statistics.median(row['samples_seconds'])
        row['stdev_seconds']=statistics.stdev(row['samples_seconds'])
    report['status']='complete';args.output.write_text(json.dumps(report,indent=2)+'\n')
    for row in sorted(rows,key=lambda p:p['median_seconds']):
        print('RESULT',row['id'],f'{row["median_seconds"]:.9f}',row['loc'],'LOC',flush=True)

if __name__=='__main__':main()
