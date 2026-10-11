#!/usr/bin/env python3
"""Measure audited compact entries at unchanged full CLBG workloads."""
import argparse, copy, gzip, hashlib, importlib.util, json, math, os, platform
import random, re, shutil, signal, statistics, subprocess, tempfile, threading, time
from datetime import datetime, timezone
from pathlib import Path

HERE=Path(__file__).resolve().parent; CLBG=HERE.parent; ROOT=CLBG.parents[1]
NODE='/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node'
RUNTIMES={'python':shutil.which('python3'),'ruby':'/opt/homebrew/opt/ruby/bin/ruby',
          'perl':'/opt/homebrew/opt/perl/bin/perl','lua':shutil.which('lua'),'javascript':NODE,
          'luajit':'/tmp/mech-clbg-luajit/src/luajit','php':'/opt/homebrew/opt/php/bin/php'}
WORK={'nbody':(1000,50000000),'mandelbrot':(200,16000),'knucleotide':(1000,25000000),
      'spectralnorm':(100,5500),'revcomp':(1000,25000000),'fannkuchredux':(7,12),
      'fasta':(1000,25000000),'binarytrees':(10,21)}
ANCHORS={'nbody':'rust-3','mandelbrot':'mandelbrot-rust-4','knucleotide':'knucleotide-rust-7',
         'spectralnorm':'spectralnorm-rust-4-panicabort','revcomp':'revcomp-gcc-9-llvm',
         'fannkuchredux':'fannkuchredux-rust6-neon-lto','binarytrees':'binarytrees-rust-5'}
ENV={**os.environ,'RAYON_NUM_THREADS':'8','JULIA_NUM_THREADS':'8','OMP_NUM_THREADS':'8',
     'OMP_DYNAMIC':'FALSE','CHPL_RT_NUM_THREADS_PER_LOCALE':'8','GOMAXPROCS':'8',
     'OPENBLAS_NUM_THREADS':'1','PYTHONHASHSEED':'0'}
SMALL_INPUT=CLBG/'knucleotide/inputs/knucleotide-input-1000.txt'
FULL_INPUT=CLBG/'knucleotide/.build/knucleotide-input25000000.txt'
HASHES={
 'mandelbrot':[(5011,'97610473750700638fc63d13cfa49d339b67c18e7f26b3f9c9acb61e746472d5'),(32000015,'609262469ee6a0262ccd03932e557f745c9e7b997ad17835a02a0232a64807be')],
 'knucleotide':[(242,'baa63c46addb89c63e68ea0055952847fbccf450e89997b2b62234c90e66f500'),(259,'f8c042fe770dda8f870b967605ea40ca2e353f2bdf92e89a327209dd31d445c7')],
 'revcomp':[(10245,'549dcdcc7df2685145e6cbdfd682469bd9c12ff1beec93c270f8ba447acd4855'),(254166745,'e5730d4fd55c464f276e5c6062c80f46e0a735281de15d8c957e2731a9baf70e')],
 'fasta':[(10245,'62d1e8d0df7938d2aefda9a37887e0389231ea72c099c29a51afb6edca1bdc73'),(254166745,'3fcf4f78104c8a65ef210fe1d469f4e473456c791225f2f1f9114f4986aa09fa')],
 'binarytrees':[(223,'b7f92c56b5d8aeb0a4d698842d1d87a57b4909865c3c84e5e10313e16663c3cb'),(455,'341de11a51feab3d8122b4b5d6a68b038a2d14434aa9bc2372f39300bf5f48e1')],
}
spec=importlib.util.spec_from_file_location('common',CLBG/'nbody/run.py')
common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)

def sha(p):
    with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

class Deadline(Exception): pass

COMPILERS={'rustc','clang','clang++','cc1','cc1plus','lto1','gcc','g++','swift-frontend'}

def compiler_load():
    """Inspect executable names only; never collect other processes' arguments."""
    listing=subprocess.check_output(['ps','-eo','pid,pcpu,comm'],text=True)
    active=[]
    for line in listing.splitlines()[1:]:
        fields=line.split(None,2)
        if len(fields)!=3:continue
        pid,cpu,command=fields;name=Path(command).name
        if (name in COMPILERS or re.fullmatch(r'(?:clang|gcc|g\+\+)-\d+',name)) and float(cpu)>=5:
            active.append(dict(pid=int(pid),cpu_percent=float(cpu),executable=name))
    return active

def wait_for_idle():
    checks=0
    while active:=compiler_load():
        if checks%3==0:print('WAIT competing compiler load',json.dumps(active),flush=True)
        checks+=1;time.sleep(10)

def power_profile():
    """Record power source and active Low Power Mode, excluding battery IDs/charge."""
    state=subprocess.check_output(['pmset','-g','batt'],text=True).splitlines()[0]
    source='AC Power' if "'AC Power'" in state else 'Battery Power' if "'Battery Power'" in state else None
    if source is None:raise RuntimeError('Unsupported power-source state')
    settings=subprocess.check_output(['pmset','-g','custom'],text=True)
    section=None;low_power=None
    for line in settings.splitlines():
        if not line.startswith(' '):section=line.rstrip(':')
        fields=line.split()
        if section==source and len(fields)==2 and fields[0]=='lowpowermode':low_power=int(fields[1])
    if low_power is None:raise RuntimeError('Active Low Power Mode setting not found')
    return dict(source=source,low_power_mode=low_power)

def wait_for_environment(required_power=None):
    checks=0
    while True:
        wait_for_idle()
        if required_power is None or power_profile()==required_power:return
        if checks%3==0:print('WAIT required power profile',json.dumps(required_power),flush=True)
        checks+=1;time.sleep(10)

class CompilerMonitor:
    def __init__(self,expected_power):
        self.stop=threading.Event();self.observations=[];self.errors=[]
        self.expected_power=expected_power;self.power_changes=[]
        self.thread=threading.Thread(target=self.poll,daemon=True)
    def check(self):
        try:
            active=compiler_load()
            if active:self.observations.append(dict(at=datetime.now(timezone.utc).isoformat(),processes=active))
            current=power_profile()
            if current!=self.expected_power:self.power_changes.append(dict(at=datetime.now(timezone.utc).isoformat(),expected=self.expected_power,observed=current))
        except Exception as error:self.errors.append(str(error))
    def poll(self):
        while not self.stop.wait(10):self.check()
    def start(self):
        self.check();self.thread.start()
    def finish(self):
        self.stop.set();self.thread.join();self.check()
        if self.errors:raise RuntimeError('Compiler-load inspection failed: '+repr(self.errors))

def run_process(argv,stdin,out,err,timeout,env):
    def expired(signum,frame):raise Deadline()
    old=signal.signal(signal.SIGALRM,expired)
    try:
        with subprocess.Popen(argv,stdin=stdin,stdout=out,stderr=err,cwd=ROOT,env=env,start_new_session=True) as proc:
            signal.setitimer(signal.ITIMER_REAL,timeout)
            try: status=proc.wait()
            except Deadline:
                os.killpg(proc.pid,signal.SIGKILL);proc.wait()
                raise subprocess.TimeoutExpired(argv,timeout) from None
            finally:signal.setitimer(signal.ITIMER_REAL,0)
            if status:raise subprocess.CalledProcessError(status,argv)
    finally:signal.signal(signal.SIGALRM,old)

def execute(row,full,timeout,watch_compilers=False,expected_power=None):
    bench=row['benchmark']; n=WORK[bench][int(full)]
    argv=[s.replace('{n}',str(n)) for s in row['command']]
    ip=(FULL_INPUT if full else SMALL_INPUT) if bench in ('knucleotide','revcomp') else None
    with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
        stdin=ip.open('rb') if ip else subprocess.DEVNULL
        monitor=CompilerMonitor(expected_power or power_profile()) if watch_compilers else None
        try:
            if monitor:monitor.start()
            start=time.perf_counter_ns()
            run_process(argv,stdin,out,err,timeout,ENV|row.get('runtime_environment',{}))
            elapsed=(time.perf_counter_ns()-start)/1e9
        except Exception as e:
            err.seek(0);raise RuntimeError(str(e)+'; '+err.read(2000).decode(errors='replace')) from e
        finally:
            if ip:stdin.close()
            if monitor:monitor.finish()
        out.seek(0,os.SEEK_END);size=out.tell();out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest()
        err.seek(0);stderr=err.read().decode(errors='replace');validation={'bytes':size,'sha256':digest}
        if bench=='nbody':
            out.seek(0);output=out.read().decode();expected=[-0.169075164,-0.169059907] if full else [-0.169075164,-0.169087605]
            common.validate(output,expected);validation.update(energies=common.energies(output),tolerance=1e-8)
        elif bench=='spectralnorm':
            out.seek(0);output=out.read().decode();expected='1.274224153\n' if full else '1.274219991\n'
            if output!=expected:raise ValueError('spectral output mismatch: '+repr(output))
        elif bench=='fannkuchredux':
            out.seek(0);output=out.read().decode();expected='3968050\nPfannkuchen(12) = 65\n' if full else '228\nPfannkuchen(7) = 16\n'
            if output not in (expected,expected.removesuffix('\n')):raise ValueError('fannkuch output mismatch: '+repr(output))
            validation['exact_reference_match']=output==expected
            if output!=expected:validation['trailing_newline_only']=True
        else:
            refsize,refhash=HASHES[bench][int(full)]
            if size!=refsize or digest!=refhash:raise ValueError(f'{bench} output mismatch: {size} {digest} (expected {refsize} {refhash})')
        validation['reference_match']=True
        return dict(seconds=elapsed,validation=validation,stderr=stderr,
                    competing_compilers=monitor.observations if monitor else [],
                    power_changes=monitor.power_changes if monitor else [])

def discard_batch_samples(rows,report,bench,reason):
    for row in rows:
        for seconds,validation in zip(row['samples_seconds'],row['sample_validation']):
            report.setdefault('discarded_samples',[]).append(dict(id=row['id'],benchmark=bench,
                seconds=seconds,validation=validation,reason=reason,
                power_profile=report.get('benchmark_power_profiles',{}).get(bench)))
        row['samples_seconds']=[];row['sample_validation']=[]
        for key in ('median_seconds','comparison_time_ratio','measurement_batch','normalization_control'):row.pop(key,None)

def measure_batch(bench,rows,control,args,report,save):
    profiles=report.setdefault('benchmark_power_profiles',{})
    while True:
        wait_for_environment(getattr(args,'required_power',None));profile=power_profile()
        if profiles.get(bench)!=profile:
            if any(row['samples_seconds'] for row in [control,*rows]):
                discard_batch_samples([control,*rows],report,bench,'Power profile changed or was not recorded; restart complete timed batch')
            profiles[bench]=profile;save()
        print('POWER PROFILE',bench,json.dumps(profile),flush=True)
        rng=random.Random(args.seed+list(WORK).index(bench));orders=[];restart=False
        for index in range(args.samples):
            order=[control,*rows];rng.shuffle(order);orders.append([r['id'] for r in order])
            for row in order:
                if len(row['samples_seconds'])>index:continue
                while True:
                    wait_for_environment(getattr(args,'required_power',None))
                    if power_profile()!=profile:restart=True;break
                    if getattr(args,'pre_sample',None):args.pre_sample(row)
                    result=execute(row,True,args.timeout,watch_compilers=True,expected_power=profile)
                    if getattr(args,'post_sample',None):args.post_sample(row)
                    if result['power_changes']:
                        report.setdefault('discarded_samples',[]).append(dict(id=row['id'],benchmark=bench,
                            seconds=result['seconds'],validation=result['validation'],reason='Power profile changed during sample',observations=result['power_changes']))
                        restart=True;break
                    if not result['competing_compilers']:break
                    report.setdefault('discarded_samples',[]).append(dict(id=row['id'],benchmark=bench,
                        seconds=result['seconds'],validation=result['validation'],reason='Competing compiler load detected',
                        observations=result['competing_compilers']))
                    print('DISCARDED competing compiler load',row['id'],f"{result['seconds']:.6f}",flush=True);save()
                if restart:break
                row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation'])
                print(f"{bench} {index+1}/{args.samples} {row['id']} {result['seconds']:.6f}",flush=True);save()
            if restart:break
        if not restart:return orders
        print('RESTART power profile changed',bench,flush=True)
        discard_batch_samples([control,*rows],report,bench,'Power profile changed; restart complete timed batch')
        profiles.pop(bench,None);save()

def classify(row):
    bench=row['benchmark']; identifier=row['id'];strict=True;reason='Unmodified listed source; prescribed computation and output.'
    if bench=='fasta' and identifier in ('fasta-python3-8','fasta-ruby-8','fasta-node-8','fasta-php-8'):
        strict=False;reason='Listed compact historical source uses repeated probability subtraction rather than a cumulative probability search.'
    if bench=='binarytrees':
        source=(HERE/row['path']).read_text()
        if identifier in ('binarytrees-perl-1','binarytrees-python3-2'):
            strict=False;reason='Listed compact source uses scalar leaves (Perl) or a shared constant leaf tuple (Python); fewer allocated objects than ordinary-node leaves. Complete output is validated; allocation work differs.'
        if identifier=='binarytrees-php-3':
            strict=False;reason='Listed compact source returns a constant array literal at leaves; PHP may share copy-on-write constant arrays. Allocation parity with ordinary-node leaves is not assumed.'
    return dict(strict=strict,reason=reason)

def prepare(args):
    pins=json.loads((HERE/'manifest.json').read_text());studies=json.loads((CLBG/'overview/normalized-results.json').read_text())
    selected=set(pins['selected_ids']);programs=[];excluded=[];reuse=[]
    versions={}
    for lang,path in RUNTIMES.items():
        flags=['-v'] if lang in ('perl','lua','luajit') else ['--version']
        result=subprocess.run([path,*flags],capture_output=True,text=True,check=True)
        versions[lang]=dict(path=path,sha256=sha(path),version=(result.stdout+result.stderr).strip())
    for p in pins['programs']:
        if p['id'] not in selected:continue
        if sha(HERE/p['path'])!=p['sha256']:raise ValueError('Source pin changed: '+p['id'])
        if p['language']=='julia':reuse.append(p);continue
        row=copy.deepcopy(p);lang=row['language'];path=str(HERE/p['path'])
        flags=['--yjit','-W0'] if lang=='ruby' else ['-n','-d','memory_limit=-1','-d','opcache.enable_cli=1','-d','opcache.jit_buffer_size=64M','-d','opcache.jit=tracing'] if lang=='php' else []
        row.update(command=[RUNTIMES[lang],*flags,path]+([] if row['benchmark'] in ('knucleotide','revcomp') else ['{n}']),
                   execution_mode='Ruby YJIT (upstream mode)' if lang=='ruby' else 'Lua bytecode interpreter' if lang=='lua' else 'Node.js V8 JIT' if lang=='javascript' else 'CPython interpreter (default JIT disabled)' if lang=='python' else 'PHP tracing JIT, 64 MiB code buffer' if lang=='php' else 'Perl interpreter',
                   samples_seconds=[],sample_validation=[],endpoint_scope='compact-only',algorithm=classify(row),runtime_environment={})
        row['source'].update(paths=[row['path']],sha256=row['sha256'])
        try:
            result=execute(row,False,60);row['correctness_small']=result['validation']
            row['small_validation_seconds']=result['seconds'];programs.append(row)
            print('small output passed',row['id'],f"{result['seconds']:.3f}s",flush=True)
        except Exception as e:
            excluded.append(dict(id=row['id'],benchmark=row['benchmark'],reason=str(e),stage='small validation',source=row['source']))
            print('EXCLUDED',row['id'],str(e),flush=True)
    controls=[]
    for study in studies:
        bench=study['key']; report=json.loads((CLBG/study['report']).read_text()); raw=report['programs']
        if bench=='nbody':raw=json.loads((CLBG/'nbody/results/apple-m3-chapel-tricks-50m.json').read_text())['programs']
        wanted=ANCHORS.get(bench)
        candidates=[r for r in raw if r['id']==wanted] if wanted else [r for r in raw if r['language']=='c' and 'gcc-2' in r['id'] and 'llvm' in r['id'] and any(s['id']==r['id'] for s in study['rows'])]
        original=candidates[0];display=next(r for r in study['rows'] if r['id']==original['id'])
        row=copy.deepcopy(original);argv=row['command'][:];n=str(WORK[bench][1]);argv=[s.replace(n,'{n}') if s==n or s=='--n='+n else s for s in argv]
        if bench not in ('knucleotide','revcomp') and not any('{n}' in s for s in argv):argv+=['{n}']
        row.update(benchmark=bench,command=argv,anchor=True,old_comparison_score=display['score'],samples_seconds=[],sample_validation=[])
        row['executable_sha256_current']=sha(argv[0]);controls.append(row)
        print('control',bench,row['id'],argv,flush=True)
    report=dict(schema_version=1,created_at=datetime.now(timezone.utc).isoformat(),status='prepared',
        hardware=dict(cpu=subprocess.run(['sysctl','-n','machdep.cpu.brand_string'],capture_output=True,text=True,check=True).stdout.strip(),machine=platform.machine(),logical_cpus=os.cpu_count()),
        source_manifest_sha256=sha(HERE/'manifest.json'),toolchains=versions,
        protocol=dict(workloads=WORK,samples=args.samples,seed=args.seed,timeout_seconds=args.timeout,
             timed_region='Sequential fresh process through exit, including interpreter/JIT startup and complete output to regular file. Builds, tests, output hashing and comparison excluded.',
             selection=pins['selection'],source_metric=pins['source_metric'],endpoint_scope='New languages compact-only: no inference about fastest entries or zero-length tradeoff lines.',
             environment={k:ENV[k] for k in ('RAYON_NUM_THREADS','JULIA_NUM_THREADS','OMP_NUM_THREADS','OMP_DYNAMIC','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','OPENBLAS_NUM_THREADS','PYTHONHASHSEED')},
             normalization='Each benchmark reruns an unchanged existing control; new elapsed times / new control median * preserved control comparison score. Original measurements remain unchanged.'),
        programs=programs,controls=controls,excluded=excluded,julia_existing_audit=reuse,completed_benchmarks=[])
    compatible=[];incompatible=[]
    for p in pins['programs']:
        if p['language']!='lua':continue
        row=copy.deepcopy(p);row.update(id=p['id']+'-luajit',upstream_id=p['id'],language='luajit',
                command=[RUNTIMES['luajit'],str(HERE/p['path'])]+([] if p['benchmark'] in ('knucleotide','revcomp') else ['{n}']),
                execution_mode='LuaJIT 2.1 JIT; runtime built with LUAJIT_ENABLE_LUA52COMPAT; unchanged Lua source',
                samples_seconds=[],sample_validation=[],endpoint_scope='compact-only',runtime_environment={},algorithm=classify(p))
        row['source'].update(paths=[row['path']],sha256=row['sha256'])
        try:
            result=execute(row,False,60);row['correctness_small']=result['validation'];row['small_validation_seconds']=result['seconds'];compatible.append(row)
        except Exception as e:incompatible.append(dict(id=row['id'],benchmark=row['benchmark'],stage='LuaJIT compatibility',reason=str(e),source=row['source']))
    for bench in WORK:
        candidates=[r for r in compatible if r['benchmark']==bench]
        if not candidates:continue
        pub=min(r['published_gz'] for r in candidates);local=min(r['source']['gzip1_bytes'] for r in candidates);loc=min(r['source']['nonblank_noncomment_lines'] for r in candidates)
        for row in candidates:
            reasons=[]
            if row['published_gz']==pub:reasons.append('minimum listed gz among unchanged LuaJIT-compatible entries')
            if row['source']['gzip1_bytes']==local:reasons.append('minimum local gzip among unchanged LuaJIT-compatible entries')
            if row['source']['nonblank_noncomment_lines']==loc:reasons.append('minimum LOC among unchanged LuaJIT-compatible entries')
            if reasons:row['selection_reasons']=reasons;programs.append(row);print('LuaJIT compatible selection',row['id'],flush=True)
    report['excluded'].extend(incompatible)
    report['luajit_compatibility_audit']=dict(compatible_ids=[r['id'] for r in compatible],excluded_ids=[r['id'] for r in incompatible],
              source_commit=subprocess.run(['git','rev-parse','HEAD'],cwd='/tmp/mech-clbg-luajit',capture_output=True,text=True,check=True).stdout.strip(),
              build_command=['env','MACOSX_DEPLOYMENT_TARGET=14.0','make','-j4','XCFLAGS=-DLUAJIT_ENABLE_LUA52COMPAT'],
              note='Same unmodified listed Lua sources. Compact endpoints selected within programs compatible with the documented runtime library mode. No LuaJIT-specific host kernels or source adapters.')
    return report

def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--prepare',action='store_true')
    ap.add_argument('--output',type=Path,default=HERE/'results/apple-m3-compact.json')
    ap.add_argument('--samples',type=int,default=3);ap.add_argument('--timeout',type=int,default=3600)
    ap.add_argument('--seed',type=int,default=20261008);ap.add_argument('--benchmarks',default='revcomp,knucleotide,fasta,binarytrees,spectralnorm,nbody,mandelbrot,fannkuchredux')
    args=ap.parse_args()
    if args.samples<3:ap.error('At least three samples required')
    if args.prepare:
        report=prepare(args);args.output.write_text(json.dumps(report,indent=2)+'\n');return
    report=json.loads(args.output.read_text())
    if sha(HERE/'manifest.json')!=report['source_manifest_sha256']:raise ValueError('Source manifest changed')
    def save():args.output.write_text(json.dumps(report,indent=2)+'\n')
    report['protocol']['compiler_load_guard']='Before full invocations wait for no compiler with >=5% reported CPU. During timed samples inspect executable names every 10 seconds and at both boundaries; discard and retry samples overlapping detected compiler load. Inspection, output validation and retry bookkeeping are outside the timed region, except the lightweight periodic process snapshot.'
    report['protocol']['power_state_guard']='Record active power source and Low Power Mode per benchmark. Inspect at sample boundaries and every 10 seconds during timing. A detected change discards and restarts the complete timed batch, including its control, under the new profile. Existing completed batches retain their original measurements; power state was not tracked for those.'
    for row in report['programs']:
        if sha(HERE/row['path'])!=row['sha256']:raise ValueError('Source changed: '+row['id'])
    for lang,runtime in report['toolchains'].items():
        if sha(runtime['path'])!=runtime['sha256']:raise ValueError('Runtime changed: '+lang)
    for bench in args.benchmarks.split(','):
        if bench in report['completed_benchmarks']:continue
        rows=[r for r in report['programs'] if r['benchmark']==bench and not r.get('excluded_full')]
        control=next(r for r in report['controls'] if r['benchmark']==bench)
        if sha(control['command'][0])!=control['executable_sha256_current']:raise ValueError('Control executable changed')
        print('BEGIN',bench,len(rows),'compact entries',flush=True);report['status']='running';save()
        for row in [control,*rows]:
            if 'correctness_full' in row and row.get('full_validation_current_batch'):continue
            try:
                wait_for_idle()
                result=execute(row,True,args.timeout);row['correctness_full']=result['validation'];row['full_validation_current_batch']=True
                row['full_validation_seconds']=result['seconds'];print('full output passed',row['id'],f"{result['seconds']:.6f}",flush=True)
            except Exception as e:
                if row is control:raise
                row['excluded_full']=True;report['excluded'].append(dict(id=row['id'],benchmark=bench,stage='full validation',reason=str(e),source=row['source']))
                print('EXCLUDED FULL',row['id'],str(e),flush=True)
            save()
        rows=[r for r in rows if not r.get('excluded_full')]
        orders=measure_batch(bench,rows,control,args,report,save)
        control['median_seconds']=statistics.median(control['samples_seconds'])
        for row in rows:
            row['median_seconds']=statistics.median(row['samples_seconds'])
            row['comparison_time_ratio']=row['median_seconds']/control['median_seconds']*control['old_comparison_score']
            row['measurement_batch']=args.output.name;row['normalization_control']=control['id']
        report.setdefault('sample_order',{})[bench]=orders;report['completed_benchmarks'].append(bench);save()
        print('DONE',bench,flush=True)
    report['status']='complete' if len(report['completed_benchmarks'])==len(WORK) else 'partial';save()

if __name__=='__main__':main()
