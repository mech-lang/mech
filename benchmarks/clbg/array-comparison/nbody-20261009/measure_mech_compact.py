"""Confirm 20/25-line pure Mech sources alongside the unchanged K and BQN sources.

All earlier measurement files remain unchanged.
"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import statistics
import subprocess
import threading
import time

HERE = Path(__file__).resolve().parent
ENV = dict(os.environ, GOMAXPROCS='8', AMBER_THREADS='8', OMP_NUM_THREADS='8', OPENBLAS_NUM_THREADS='8')
FULL_N = 50_000_000
SAMPLES = 5
EARLIER = json.loads((HERE/'comparison-compact.json').read_text())
COMPILER = json.loads((HERE.parents[1]/'nbody/experiments/20261009-source-compact/source-compaction.json').read_text())
assert COMPILER['status']=='complete'
OUT = HERE/'mech-compact-outputs'
OUT.mkdir(exist_ok=True)

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def checked(cmd, **kw):
    return subprocess.check_output(cmd, text=True, **kw).strip()

def conditions():
    batt = checked(['pmset','-g','batt'])
    power = checked(['pmset','-g','custom']).split('AC Power:', 1)[1]
    lpm = int(next(line.split()[-1] for line in power.splitlines() if 'lowpowermode' in line))
    compiler_names = {'rustc','clang','clang-22','gcc','cc1','cc1plus','ld','ld64','lto1','collect2','compile','link'}
    active = []
    for line in checked(['ps','-axo','pid,comm']).splitlines()[1:]:
        pieces = line.strip().split(maxsplit=1)
        if len(pieces)==2 and Path(pieces[1]).name in compiler_names:
            active.append(line.strip())
    assert "'AC Power'" in batt and lpm==0, (batt,lpm)
    assert not active, active
    return dict(ac_power=True,low_power_mode=lpm,active_compilers=active)

def energy(raw):
    lines=raw.decode().strip().splitlines()
    assert len(lines)==2, raw
    return [float(line) for line in lines]

def compare(raw, expected):
    values=energy(raw)
    assert all(abs(a-b)<=1e-8 for a,b in zip(values,expected)), (values,expected)
    return values

specs = [
    ('mech','Mech (AOT, compact)',20,'mech-matrix-compact','nbody-matrix-short.mec',None,'v0.4 integration; pure Mech matrix initialization and implicit final result; byte-identical native object to 32-line baseline'),
    ('mech-optimized','Mech (AOT, optimized)',25,'mech-matrix-optimized','nbody-matrix-pairwise.mec',None,'v0.4 integration; pure Mech matrix initialization and implicit final result; byte-identical native object to 37-line baseline'),
    ('k','K (Growler, compact/optimized)',36,'growler','nbody-optimized-short.k','/tmp/mech-array-runtimes/growler','locally simplified initialization/reporting; original advance algorithm'),
    ('bqn','BQN (CBQN, compact)',26,'bqn','nbody-compact.bqn','/tmp/mech-array-runtimes/CBQN','locally simplified initialization/reporting; original full-pair advance algorithm'),
    ('bqn-optimized','BQN (CBQN, optimized)',39,'bqn','nbody-optimized-short.bqn','/tmp/mech-array-runtimes/CBQN','locally optimized pure BQN'),
]
programs={}
for key,label,loc,exe,source,repo,provenance in specs:
    binary=HERE/'binaries'/exe
    snapshot=HERE/'sources'/source
    if key.startswith('mech'):
        pin=COMPILER['programs']['compact' if key=='mech' else 'optimized']
        assert sha(snapshot)==pin['source_sha256'] and sha(binary)==pin['binary_sha256']
        lines=[s for s in snapshot.read_text().splitlines() if s.strip() and not s.lstrip().startswith('--')]
        assert len(lines)==loc and lines[-1]=='energy' and '<+' not in snapshot.read_text()
        assert all(s.count(':=')<=1 for s in lines)
    else:
        pin=EARLIER['programs'][key]
        assert sha(snapshot)==pin['source_sha256'] and sha(binary)==pin['binary_sha256']
    if key=='k':
        assert len([s for s in snapshot.read_text().splitlines() if s.strip() and not s.lstrip().startswith(('/','#!'))])==loc
    if key.startswith('bqn'):
        lines=[s for s in snapshot.read_text().splitlines() if s.strip() and not s.lstrip().startswith('#')]
        assert len(lines)==loc
        assert all('⋄' not in s and ';' not in s for s in lines)
        assert all(s.count('←')+s.count('↩')<=1 for s in lines)
    cmd=[str(binary)] if key.startswith('mech') else [str(binary),str(snapshot)]
    programs[key]=dict(label=label,normalized_loc=loc,command=cmd,source=str(snapshot),
        source_sha256=sha(snapshot),binary_sha256=sha(binary),runtime_commit=checked(['git','rev-parse','HEAD'],cwd=repo) if repo else None,
        source_provenance=provenance,samples=[],warmup=None)

reference=HERE/'binaries/reference'
report=dict(created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
    machine=checked(['sysctl','-n','machdep.cpu.brand_string']),os=checked(['sw_vers']),
    full_steps=FULL_N,sample_count=SAMPLES,energy_tolerance=1e-8,
    protocol='Sequential fresh processes, randomized complete rounds. Blocking wait for process exit, without timeout polling; a separate watchdog kills only a stalled process after 180 seconds. Stdout to regular files. Builds, warmups, validation, conditions checks, and hashing excluded from samples. All samples retained.',
    requested_workers=8,environment={k:ENV[k] for k in ('GOMAXPROCS','AMBER_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS')},
    reference=EARLIER['reference'],array_source_commit=EARLIER['array_source_commit'],
    programs=programs,verification=[],status='running',sample_order=[],selection_pilots=['candidate-pilots.json','candidate-pilots-2.json','candidate-pilots-3.json','compaction-pilots.json'],
    selection='K retains the winning published cached-pair advance algorithm and simplifies setup/reporting. BQN compact retains the published full-pair advance algorithm with matrix data; BQN optimized retains the winning flat-pair advance algorithm with shared state and cached indices.',
    earlier_results=['results.json','comparison.json','comparison-compact.json'],compiler_source_compaction=COMPILER,
    timer_change='Same blocking-wait protocol as comparison-compact.json; all five endpoints remeasured. Earlier samples remain separate.')

def save():
    tmp=HERE/'comparison-mech-compact.json.tmp'
    tmp.write_text(json.dumps(report,indent=2)+'\n')
    tmp.replace(HERE/'comparison-mech-compact.json')

conditions()
for n in (1,2,10,1000,10000,1_000_000,FULL_N):
    expected=energy(subprocess.check_output([str(reference),str(n)],env=ENV))
    if n==FULL_N:
        assert expected==EARLIER['expected_full_energy']
        report['expected_full_energy']=expected
        print('Full reference validated: '+repr(expected),flush=True)
        continue
    for key,p in programs.items():
        raw=subprocess.check_output(p['command']+[str(n)],env=ENV)
        values=compare(raw,expected)
        report['verification'].append(dict(program=key,steps=n,energy=values,stdout_sha256=hashlib.sha256(raw).hexdigest()))
    print('Validated all five endpoints at '+str(n)+' steps',flush=True)
    save()

for key,p in programs.items():
    start=time.perf_counter_ns()
    raw=subprocess.check_output(p['command']+['1000000'],env=ENV)
    p['warmup']=dict(steps=1_000_000,seconds=(time.perf_counter_ns()-start)/1e9)

rng=random.Random(2026100915)
for round_no in range(SAMPLES):
    order=list(programs);rng.shuffle(order);report['sample_order'].append(order)
    for key in order:
        p=programs[key]
        before=conditions()
        issues=[];observations=[];done=threading.Event()
        def monitor():
            while not done.wait(10):
                try:observations.append(conditions())
                except Exception as e:issues.append(repr(e))
        watcher=threading.Thread(target=monitor,daemon=True);watcher.start()
        out=OUT/f'{round_no+1}-{key}.stdout'
        err=OUT/f'{round_no+1}-{key}.stderr'
        killed=threading.Event()
        try:
            with out.open('wb') as stdout,err.open('wb') as stderr:
                start=time.perf_counter_ns()
                process=subprocess.Popen(p['command']+[str(FULL_N)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=ENV)
                def watchdog():
                    if process.poll() is None:
                        killed.set();process.kill()
                timer=threading.Timer(180,watchdog);timer.daemon=True;timer.start()
                code=process.wait()
                elapsed=(time.perf_counter_ns()-start)/1e9
                timer.cancel();timer.join()
        finally:
            done.set();watcher.join()
        after=conditions()
        assert code==0 and not killed.is_set(),(key,code,err.read_text())
        assert not issues,issues
        values=compare(out.read_bytes(),report['expected_full_energy'])
        p['samples'].append(dict(round=round_no+1,seconds=elapsed,energy=values,stdout_sha256=sha(out),before=before,after=after,monitor_checks=len(observations)))
        print(f'Round {round_no+1}/5 {p["label"]}: {elapsed:.6f}s, output valid',flush=True)
        save()

for key,p in programs.items():
    samples=[r['seconds'] for r in p['samples']]
    p['median_seconds']=statistics.median(samples)
    p['sample_stdev_seconds']=statistics.stdev(samples)
for p in programs.values():
    p['time_over_mech_compact']=p['median_seconds']/programs['mech']['median_seconds']
report['status']='complete';report['finished_at']=datetime.datetime.now(datetime.timezone.utc).isoformat()
report['final_conditions']=conditions();save()
print(json.dumps([{k:p[k] for k in ('label','normalized_loc','median_seconds','sample_stdev_seconds','time_over_mech_compact')} for p in programs.values()],indent=2),flush=True)
