"""Matched n-body timings of pinned published sources; no source rewrites."""
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
WORK = HERE.parents[3]
SOURCES = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-png/iversonian-sources')
PREVIOUS = WORK / 'benchmarks/clbg/function-application-audit/20261009T171437Z/application-v3/builds/nbody'
ENV = dict(os.environ, GOMAXPROCS='8', AMBER_THREADS='8', OMP_NUM_THREADS='8', OPENBLAS_NUM_THREADS='8')
FULL_N = 50_000_000
SAMPLES = 5
(HERE/'binaries').mkdir(exist_ok=True)
(HERE/'sources').mkdir(exist_ok=True)
(HERE/'outputs').mkdir(exist_ok=True)

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def checked(cmd, **kw):
    return subprocess.check_output(cmd, text=True, **kw).strip()

def conditions():
    batt = checked(['pmset','-g','batt'])
    power = checked(['pmset','-g','custom'])
    ac = power.split('AC Power:', 1)[1]
    lpm = int(next(line.split()[-1] for line in ac.splitlines() if 'lowpowermode' in line))
    processes = checked(['ps','-axo','pid,comm'])
    compiler_names = {'rustc','clang','clang-22','gcc','cc1','cc1plus','ld','ld64','lto1','collect2','compile','link'}
    active = []
    for line in processes.splitlines()[1:]:
        pieces = line.strip().split(maxsplit=1)
        if len(pieces)==2 and Path(pieces[1]).name in compiler_names:
            active.append(line.strip())
    assert "'AC Power'" in batt and lpm==0, (batt,lpm)
    assert not active, active
    return dict(ac_power=True,low_power_mode=lpm,active_compilers=active)

specs = [
    ('mech','Mech (AOT, compact)',32, PREVIOUS/'compact-baseline-before', WORK/'benchmarks/clbg/nbody/nbody.mec', None),
    ('mech-optimized','Mech (AOT, optimized)',37, PREVIOUS/'pairwise-mass-tails-before', WORK/'benchmarks/clbg/nbody/nbody-pairwise-mass.mec', None),
    ('amber','K (Amber)',43, Path('/tmp/mech-array-runtimes/amber/amber'), SOURCES/'nbody.growler.k', '/tmp/mech-array-runtimes/amber'),
    ('goal','Goal',45, Path('/tmp/mech-array-runtimes/goal-full'), SOURCES/'nbody.goal', '/tmp/mech-array-runtimes/goal-current'),
    ('bqn','BQN (CBQN)',63, Path('/tmp/mech-array-runtimes/CBQN/BQN'), SOURCES/'nbody.bqn', '/tmp/mech-array-runtimes/CBQN'),
]
programs = {}
audit = json.loads((SOURCES/'normalized-loc-audit.json').read_text())
for key,label,loc,exe,source,repo in specs:
    binary = HERE/'binaries'/key
    snapshot = HERE/'sources'/source.name
    shutil.copy2(exe,binary)
    shutil.copy2(source,snapshot)
    if key != 'mech-optimized':
        lang={'mech':'mech','amber':'growler.k','goal':'goal','bqn':'bqn'}[key]
        pin=next(r for r in audit['rows'] if r['benchmark']=='nbody' and r['language']==lang)
        assert sha(snapshot)==pin['sha256'] and loc==pin['normalized_loc']
    cmd=[str(binary)] if key.startswith('mech') else [str(binary),str(snapshot)]
    programs[key] = dict(label=label,normalized_loc=loc,command=cmd,source=str(snapshot),source_sha256=sha(snapshot),binary_sha256=sha(binary),runtime_commit=checked(['git','rev-parse','HEAD'],cwd=repo) if repo else None,samples=[],warmup=None)

reference = HERE/'binaries'/'reference'
shutil.copy2('/tmp/mech-array-runtimes/nbody-reference',reference)
reference_source = WORK/'benchmarks/clbg/nbody/upstream/nbody-gcc-3.c'
shutil.copy2(reference_source,HERE/'sources'/'reference.c')
report=dict(created_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
    machine=checked(['sysctl','-n','machdep.cpu.brand_string']),os=checked(['sw_vers']),
    full_steps=FULL_N,sample_count=SAMPLES,energy_tolerance=1e-8,
    protocol='Sequential fresh processes; randomized complete rounds; full stdout to regular files; runtime builds, warm-ups, validation/hashing excluded from samples. No discarded samples.',
    requested_workers=8,environment={k:ENV[k] for k in ('GOMAXPROCS','AMBER_THREADS','OMP_NUM_THREADS','OPENBLAS_NUM_THREADS')},
    reference=dict(source_sha256=sha(reference_source),binary_sha256=sha(reference),build=['clang','-O3','-ffp-contract=off',str(reference_source),'-lm']),
    array_source_commit=audit['commit'],programs=programs,verification=[],status='running',sample_order=[])

def save():
    temp=HERE/'results.json.tmp'
    temp.write_text(json.dumps(report,indent=2)+'\n')
    temp.replace(HERE/'results.json')

def energies(raw):
    lines=raw.decode().strip().splitlines()
    assert len(lines)==2,raw
    return [float(line) for line in lines]

def compare(raw,expected):
    value=energies(raw)
    assert all(abs(a-b)<=1e-8 for a,b in zip(value,expected)),(value,expected)
    return value

conditions()
for n in (1,2,10,1000,10000,1_000_000,FULL_N):
    ref=subprocess.run([str(reference),str(n)],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=ENV,check=True,timeout=120)
    expected=energies(ref.stdout)
    if n==FULL_N:
        assert all(abs(a-b)<=1e-8 for a,b in zip(expected,[-.169075164,-.169059907]))
        report['expected_full_energy']=expected
        print('Full reference validated: '+repr(expected),flush=True)
        continue
    for key,p in programs.items():
        result=subprocess.run(p['command']+[str(n)],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=ENV,check=True,timeout=60)
        compare(result.stdout,expected)
        report['verification'].append(dict(program=key,steps=n,energy=energies(result.stdout),stdout_sha256=hashlib.sha256(result.stdout).hexdigest()))
    print('Validated every source at '+str(n)+' steps',flush=True)
    save()

# An untimed pilot at one million steps warms executable pages and the CPU.
for key,p in programs.items():
    start=time.perf_counter_ns()
    result=subprocess.run(p['command']+['1000000'],stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=ENV,check=True,timeout=60)
    p['warmup']=dict(steps=1_000_000,seconds=(time.perf_counter_ns()-start)/1e9)

rng=random.Random(20261009)
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
        out=HERE/'outputs'/f'{round_no+1}-{key}.stdout'
        err=HERE/'outputs'/f'{round_no+1}-{key}.stderr'
        try:
            with out.open('wb') as stdout,err.open('wb') as stderr:
                start=time.perf_counter_ns()
                result=subprocess.run(p['command']+[str(FULL_N)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=ENV,timeout=180)
                elapsed=(time.perf_counter_ns()-start)/1e9
        finally:
            done.set();watcher.join()
        after=conditions()
        assert result.returncode==0,(key,result.returncode,err.read_text())
        assert not issues,issues
        energy=compare(out.read_bytes(),report['expected_full_energy'])
        row=dict(round=round_no+1,seconds=elapsed,energy=energy,stdout_sha256=sha(out),before=before,after=after,monitor_checks=len(observations))
        p['samples'].append(row)
        print(f'Round {round_no+1}/5 {p["label"]}: {elapsed:.6f}s, output valid',flush=True)
        save()

for key,p in programs.items():
    samples=[r['seconds'] for r in p['samples']]
    p['median_seconds']=statistics.median(samples)
    p['sample_stdev_seconds']=statistics.stdev(samples)
compact=programs['mech']['median_seconds']
for p in programs.values():p['time_over_mech_compact']=p['median_seconds']/compact
report['status']='complete';report['finished_at']=datetime.datetime.now(datetime.timezone.utc).isoformat()
report['final_conditions']=conditions();save()
print(json.dumps([{k:p[k] for k in ('label','normalized_loc','median_seconds','sample_stdev_seconds','time_over_mech_compact')} for p in programs.values()],indent=2),flush=True)
