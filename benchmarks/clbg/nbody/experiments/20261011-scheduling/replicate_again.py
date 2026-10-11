#!/usr/bin/env python3
"""Matched process timings and complete-state checks of general codegen options."""
import argparse, hashlib, importlib.util, json, random, statistics, struct, subprocess, sys, tempfile, time
from datetime import datetime, timezone
from pathlib import Path

HERE=Path(__file__).resolve().parent
NBODY=HERE.parents[1]
sys.path.insert(0,str(NBODY))
from run import command, energies, validate, compiler_sources
from chapel_tricks import loop_codegen
spec=importlib.util.spec_from_file_location('compact_probe',NBODY.parent/'compact/run.py')
checks=importlib.util.module_from_spec(spec);spec.loader.exec_module(checks)
POWER=dict(source='AC Power',low_power_mode=0)

def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def argv(row,n):return [arg.replace('{n}',str(n)) for arg in row['command']]
def state(row,n):
    run=command(argv(row,n),env=checks.ENV|{'MECH_DUMP_STATE':'1'})
    values=json.loads(next(line[6:] for line in run.stderr.splitlines() if line.startswith('STATE ')))
    if row.get('state_orientation')=='columns':
        assert len(values)==30
        values=[values[offset+body*3+axis] for offset in (0,15) for axis in range(3) for body in range(5)]
    return values

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--programs',type=Path,required=True)
    ap.add_argument('--steps',type=int,default=50000000);ap.add_argument('--samples',type=int,default=7)
    ap.add_argument('--output',type=Path,required=True);args=ap.parse_args()
    assert args.samples>=3 and args.steps>=0 and not args.output.exists()
    rows=json.loads(args.programs.read_text())
    for row in rows:
        assert sha(row['command'][0])==row['binary_sha256']
        row.update(samples_seconds=[],sample_stdout=[])
        validate(command(argv(row,1000),env=checks.ENV).stdout,[-.169075164,-.169087605])
        if row['language']=='mech':
            row['codegen']=loop_codegen(row['command'][0],HERE/(row['id']+'.disassembly.txt'))
            assert not row['codegen']['loop_calls']
    state_checks=[]
    for row in rows:
        if row['language']!='mech' or not row.get('state_control'):continue
        baseline=next(x for x in rows if x['id']==row['state_control'])
        for n in (0,1,1000,10000):
            a=state(row,n);b=state(baseline,n)
            exact=all(struct.pack('>d',x)==struct.pack('>d',y) for x,y in zip(a,b))
            error=max(abs(x-y)/max(1,abs(y)) for x,y in zip(a,b))
            assert len(a)==len(b)==30
            assert exact if row.get('preserves_bits',True) else error<=1e-10
            state_checks.append(dict(id=row['id'],control=baseline['id'],steps=n,bit_identical=exact,max_scaled_error=error))
    reference=next(x for x in rows if x['id']=='rust-3')
    expected=energies(command(argv(reference,args.steps),env=checks.ENV).stdout)
    if args.steps==50000000:validate('\n'.join(map(str,expected)),[-.169075164,-.169059907])
    # A full untimed correctness/warmup run precedes each process-timed sample set.
    for row in rows:validate(command(argv(row,args.steps),env=checks.ENV).stdout,expected)
    report=dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),steps=args.steps,samples=args.samples,
        required_power=POWER,programs=rows,state_checks=state_checks,expected_energy=expected,discarded_samples=[],sample_order=[],
        compiler_sha256=compiler_sources(),probe_sha256=sha(__file__),programs_manifest_sha256=sha(args.programs),
        protocol='Fresh sequential processes through exit; full input, initial/final output and startup counted; builds, full validation and one full-workload warmup excluded. Seeded shuffled rounds; AC/LPM and competing compiler checks. LOC only.')
    rng=random.Random(20261013)
    for sample in range(args.samples):
        order=rows.copy();rng.shuffle(order);report['sample_order'].append([x['id'] for x in order])
        for row in order:
            while True:
                checks.wait_for_environment(POWER)
                assert sha(row['command'][0])==row['binary_sha256']
                monitor=checks.CompilerMonitor(POWER);monitor.start()
                with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
                    start=time.perf_counter_ns()
                    checks.run_process(argv(row,args.steps),subprocess.DEVNULL,out,err,300,checks.ENV)
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
    report.update(status='complete',finished_at=datetime.now(timezone.utc).isoformat())
    args.output.write_text(json.dumps(report,indent=2)+'\n')
    for row in sorted(rows,key=lambda x:x['median_seconds']):
        print('RESULT',row['id'],f"{row['median_seconds']:.9f}",json.dumps(row.get('codegen',{})),flush=True)

if __name__=='__main__':main()
