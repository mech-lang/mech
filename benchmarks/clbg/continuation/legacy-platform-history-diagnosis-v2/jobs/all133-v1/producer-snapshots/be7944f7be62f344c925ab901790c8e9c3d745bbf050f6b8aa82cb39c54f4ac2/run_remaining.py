#!/usr/bin/env python3
"""Build, validate and measure source-visible regex-redux and pidigits programs."""
import argparse, gzip, hashlib, importlib.util, io, json, os, platform, random, re, shutil, tarfile
import statistics, subprocess, sys, tempfile, time
from datetime import datetime, timezone
from pathlib import Path
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment

CLBG=Path(__file__).resolve().parent; ROOT=CLBG.parents[1]
spec=importlib.util.spec_from_file_location('guard',CLBG/'compact/run.py')
guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
POWER={'source':'AC Power','low_power_mode':0}
SDK=subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip()
ENV=guard.ENV|{'C_INCLUDE_PATH':'/opt/homebrew/include','LIBRARY_PATH':'/opt/homebrew/lib',
 'SDKROOT':SDK,'CGO_CFLAGS':'-I/opt/homebrew/include -isysroot '+SDK,'CGO_LDFLAGS':'-L/opt/homebrew/lib -isysroot '+SDK,
 'RUSTFLAGS':'-C target-cpu=native -L native=/opt/homebrew/lib',
 'CARGO_TARGET_DIR':str(CLBG/'nbody/mech-driver/target')}
MECH=CLBG/'nbody/mech-driver/target/release/mech'
JULIA='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia'
RUNTIME={'python':sys.executable,'ruby':'/opt/homebrew/opt/ruby/bin/ruby',
 'perl':'/opt/homebrew/opt/perl/bin/perl','lua':shutil.which('lua'),
 'luajit':'/tmp/mech-clbg-luajit/src/luajit','javascript':guard.NODE,'php':shutil.which('php')}

def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def snapshot(output):
    """Keep the exact general compiler/runtime inputs, including untracked work."""
    names=set(command(['git','diff','--name-only','HEAD']).stdout.splitlines())
    names.update(command(['git','ls-files','--others','--exclude-standard']).stdout.splitlines())
    names={name for name in names if (ROOT/name).is_file() and
           ((name.startswith(('src/','hosts/','machines/')) and
             (Path(name).suffix in ('.rs','.c') or Path(name).name in ('Cargo.toml','Cargo.lock')))
            or name in ('Cargo.toml','Cargo.lock','scripts/unsafe-boundaries.allowlist'))}
    names.update(str(p.relative_to(ROOT)) for p in (Path(__file__),CLBG/'remaining_conformance.py',CLBG/'native-features/Cargo.toml',CLBG/'native-features/src/lib.rs'))
    for benchmark in ('regexredux','pidigits'):
        names.update(str(p.relative_to(ROOT)) for p in (CLBG/benchmark).glob('*.mec'))
    hashes={};buffer=io.BytesIO()
    with tarfile.open(fileobj=buffer,mode='w') as archive:
        for name in sorted(names):
            data=(ROOT/name).read_bytes();hashes[name]=hashlib.sha256(data).hexdigest()
            info=tarfile.TarInfo(name);info.size=len(data);info.mode=0o644
            archive.addfile(info,io.BytesIO(data))
    path=output.with_suffix('.compiler.tar.gz')
    path.write_bytes(gzip.compress(buffer.getvalue(),compresslevel=6,mtime=0))
    return dict(path=path.name,sha256=sha(path),files=hashes,
                base_commit=command(['git','rev-parse','HEAD']).stdout.strip(),
                compiler_sha256=sha(MECH),note='Exact changed and untracked compiler/runtime inputs relative to the base commit. This archive is a reproducibility artifact, not a source-size metric.')

def toolchains():
    commands={'rust':['rustc','--version'],'gcc':['/opt/homebrew/bin/gcc-16','--version'],
      'clang':['/opt/homebrew/opt/llvm@22/bin/clang++','--version'],
      'chapel':['chpl','--version'],'julia':[JULIA,'--version'],'go':['go','version'],
      'java':['/opt/homebrew/opt/openjdk/bin/java','--version'],
      'python':[sys.executable,'--version'],'ruby':[RUNTIME['ruby'],'--version'],
      'perl':[RUNTIME['perl'],'-e','print "$^V\\n"'],'lua':[RUNTIME['lua'],'-v'],
      'luajit':[RUNTIME['luajit'],'-v'],'node':[RUNTIME['javascript'],'--version'],
      'php':[RUNTIME['php'],'--version'],'libraries':['brew','list','--versions','gmp','pcre','pcre2','re2','boost']}
    result={}
    for name,argv in commands.items():
        try:
            p=command(argv);result[name]=(p.stdout+p.stderr).strip()
        except Exception as error:result[name]=str(error)[-500:]
    return result
def command(argv, cwd=ROOT, env=ENV, timeout=600):
    p=subprocess.run(list(map(str,argv)),cwd=cwd,env=env,capture_output=True,text=True,timeout=timeout)
    if p.returncode: raise RuntimeError((p.stdout+'\n'+p.stderr)[-5000:])
    return p
def source_metrics(path,language):
    source=path.read_text()
    if language=='mech': clean=guard.common.strip_comments(source,'mech')
    elif language=='julia': clean=guard.common.strip_comments(source,'julia')
    else:
        clean=''.join('\n'*value.count('\n') if token in Comment and token not in Comment.Preproc else value
                      for token,value in lex(source,get_lexer_by_name('lua' if language=='luajit' else language,stripnl=False,ensurenl=False)))
    return dict(sha256=sha(path),physical_lines=len(source.splitlines()),
                nonblank_noncomment_lines=sum(bool(line.strip()) for line in clean.splitlines()))

def prepare(here,p):
    p=p.copy(); path=here/p['path']; language=p['language']; text=path.read_text()
    if p.get('sha256') and sha(path)!=p['sha256']: raise RuntimeError('pinned source changed')
    build=here/'.build';build.mkdir(exist_ok=True);binary=build/p['id'];argv=None;compile_command=None
    if language=='mech':
        project=Path(str(binary)+'.project')
        if project.exists():shutil.rmtree(project)
        compile_command=[MECH,'build',path,'--backend','cranelift','--keep-project','--no-config','-o',binary]
        command(compile_command);argv=[str(binary)]
        project=Path(str(binary)+'.project')
        object_path=next(project/name for name in ('mech-sequence.o','mech-control.o','grid.o') if (project/name).exists())
        p['object_sha256']=sha(object_path)
        p['compiler_sha256']=sha(MECH)
    elif language in ('c','cpp'):
        # Homebrew's C++ libraries use libc++; use its matching LLVM toolchain.
        compiler='/opt/homebrew/bin/gcc-16' if language=='c' else '/opt/homebrew/opt/llvm@22/bin/clang++'
        compile_command=[compiler,'-O3','-march=native',*guard.common.macos_gcc_flags(compiler),'-I/opt/homebrew/include','-L/opt/homebrew/lib']
        if language=='cpp':compile_command+=['-std=c++17']
        if '#pragma omp' in text or '<omp.h>' in text:
            compile_command+=['-fopenmp']
            if language=='cpp':compile_command+=['-I/opt/homebrew/opt/libomp/include','-L/opt/homebrew/opt/libomp/lib']
        compile_command += [path,'-o',binary]
        if here.name=='pidigits':compile_command+=['-lgmp']
        elif 'pcre2.h' in text:compile_command+=['-lpcre2-8']
        elif 'pcre.h' in text:compile_command+=['-lpcre']
        elif 're2' in text:compile_command+=['-I/opt/homebrew/include/re2','-lre2']
        elif 'boost/regex.hpp' in text:compile_command+=['-lboost_regex']
        command(compile_command);argv=[str(binary)]
    elif language=='rust':
        project=build/(p['id']+'.cargo');project.mkdir(exist_ok=True)
        dependencies='libc="=0.2.189"\nrayon="=1.12.0"\nregex="=1.13.1"\ncrossbeam-utils="=0.8.21"\npcre2-sys="=0.2.10"\nrug={version="=1.28.1",default-features=false,features=["integer"]}\ngmp-mpfr-sys={version="=1.6.8",default-features=false,features=["use-system-libs"]}\n'
        manifest='[package]\nname="'+p['id']+'"\nversion="0.0.0"\nedition="2021"\n[[bin]]\nname="'+p['id']+'"\npath='+json.dumps(str(path))+'\n[dependencies]\n'+dependencies+'[workspace]\n'
        (project/'Cargo.toml').write_text(manifest)
        compile_command=['cargo','build','--release','--manifest-path',project/'Cargo.toml']
        command(compile_command);shutil.copy2(Path(ENV['CARGO_TARGET_DIR'])/'release'/p['id'],binary);argv=[str(binary)]
        p['dependency_lock_sha256']=sha(project/'Cargo.lock')
    elif language=='chapel':
        compile_command=['chpl','--fast',path,'-o',binary,'--ccflags=-isysroot '+SDK,'--ldflags=-isysroot '+SDK];command(compile_command);argv=[str(binary)]
    elif language=='julia':argv=[JULIA,'--threads=8,0','--startup-file=no','--history-file=no','-O3',str(path)]
    elif language=='go':
        project=build/(p['id']+'.go');project.mkdir(exist_ok=True);shutil.copy2(path,project/'main.go')
        command(['go','mod','init',p['id']],cwd=project) if not (project/'go.mod').exists() else None
        if 'github.com/GRbit/go-pcre' in text:
            command(['go','get','github.com/GRbit/go-pcre@latest'],cwd=project)
        compile_command=['go','build','-o',binary,'.'];command(compile_command,cwd=project);argv=[str(binary)]
        p['dependency_manifest_sha256']=sha(project/'go.mod')
        if (project/'go.sum').exists():p['dependency_lock_sha256']=sha(project/'go.sum')
    elif language=='java':
        project=build/(p['id']+'.java');project.mkdir(exist_ok=True)
        name=re.search(r'(?m)^\s*(?:public\s+)?(?:final\s+)?class\s+(\w+)',text).group(1)
        java=project/(name+'.java');shutil.copy2(path,java)
        compile_command=['/opt/homebrew/opt/openjdk/bin/javac','-d',project,java]
        command(compile_command);argv=['/opt/homebrew/opt/openjdk/bin/java','-XX:ActiveProcessorCount=8','-cp',str(project),name]
        p['execution_mode']='OpenJDK JIT; pinned entry published as GraalVM native AOT'
    elif language in RUNTIME:
        if not RUNTIME[language]:raise RuntimeError('runtime unavailable')
        argv=[RUNTIME[language],str(path)]
        if language=='ruby':argv.insert(1,'--yjit')
        if language=='php':argv=[RUNTIME[language],'-d','memory_limit=-1','-d','opcache.enable_cli=1','-d','opcache.jit_buffer_size=64M','-d','opcache.jit=tracing',str(path)]
        if language in ('lua','luajit'):
            p['runtime_environment']={'LUA_CPATH':str(here/'.build'/language/'?.so')+';;'}
            p['library_pins_sha256']=sha(here/'.build/lua-library-pins.json')
    else:raise RuntimeError('runtime not configured')
    if here.name=='pidigits':argv+=['--n={n}' if language=='chapel' else '{n}']
    p.update(command=argv,compile_command=list(map(str,compile_command)) if compile_command else None,
             source=source_metrics(path,language),runtime_environment=p.get('runtime_environment',{}),samples_seconds=[])
    if language not in RUNTIME and language not in ('julia','java'):p['executable_sha256']=sha(binary)
    return p

def execute(here,row,full,watch=False):
    n=10000 if full else 30
    argv=[arg.replace('{n}',str(n)) for arg in row['command']]
    input_path=here/('.build/input5000000.txt' if full else 'inputs/regexredux-input-small.txt') if here.name=='regexredux' else None
    monitor=guard.CompilerMonitor(POWER) if watch else None
    with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
        stdin=input_path.open('rb') if input_path else subprocess.DEVNULL
        try:
            if monitor:monitor.start()
            start=time.perf_counter_ns();guard.run_process(argv,stdin,out,err,600,ENV|row['runtime_environment'])
            elapsed=(time.perf_counter_ns()-start)/1e9
        except Exception as error:
            err.seek(0);raise RuntimeError(str(error)+' '+err.read().decode(errors='replace')) from error
        finally:
            if input_path:stdin.close()
            if monitor:monitor.finish()
        out.seek(0);data=out.read()
    if monitor and (monitor.observations or monitor.power_changes): return None
    expected=here/('inputs/output-full.txt' if full else 'inputs/'+here.name+'-output-small.txt')
    if data!=expected.read_bytes():raise RuntimeError('complete output mismatch: '+row['id'])
    return elapsed

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('benchmark',choices=['regexredux','pidigits'])
    parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--reuse-builds',action='store_true')
    parser.add_argument('--samples',type=int,default=3);parser.add_argument('--only');parser.add_argument('--all',action='store_true')
    args=parser.parse_args();here=CLBG/args.benchmark;build=here/'.build';build.mkdir(exist_ok=True)
    prepared=build/'prepared.json';excluded=[]
    if args.reuse_builds:
        d=json.loads(prepared.read_text());rows=d['programs'];excluded=d['excluded']
    else:
        sources=json.loads((here/'manifest.json').read_text())['programs']
        if not args.all:
            selected=set()
            for lang in sorted({p['language'] for p in sources}):
                cohort=[p for p in sources if p['language']==lang]
                if lang in ('c','cpp','rust','chapel','julia','go','java'):
                    selected.update(p['id'] for p in cohort)
                    continue
                for p in cohort:p['loc']=source_metrics(here/p['path'],lang)['nonblank_noncomment_lines']
                smallest=min(p['loc'] for p in cohort)
                fastest=min((p['published_secs'] for p in cohort if p['published_secs'] is not None),default=None)
                selected.update(p['id'] for p in cohort if p['loc']==smallest or p['published_secs']==fastest)
            sources=[p for p in sources if p['id'] in selected]
        mech=[dict(id='mech-'+args.benchmark,language='mech',path=args.benchmark+'.mec')]
        if args.benchmark=='regexredux':
            mech.append(dict(id='mech-regexredux-async',language='mech',path='regexredux.mec',runtime_environment={'MECH_SEQUENCE_ASYNC':'1'}))
        else:mech.append(dict(id='mech-pidigits-gated',language='mech',path='pidigits-gated.mec'))
        sources=mech+sources
        sources += [dict(p,id=p['id']+'-luajit',language='luajit',upstream_id=p['id']) for p in sources.copy() if p['language']=='lua']
        rows=[]
        for p in sources:
            if args.only and p['id'] not in args.only.split(','):continue
            try:row=prepare(here,p);execute(here,row,False);rows.append(row);print('prepared',row['id'],flush=True)
            except Exception as error:excluded.append(dict(id=p['id'],language=p['language'],reason=str(error)));print('excluded',p['id'],str(error)[-220:],flush=True)
        prepared.write_text(json.dumps(dict(programs=rows,excluded=excluded),indent=2)+'\n')
    if args.prepare_only:return
    if args.samples<3:parser.error('at least three samples')
    rows=[r for r in rows if not args.only or r['id'] in args.only.split(',')]
    valid=[]
    for row in rows:
        if sha(here/row['path'])!=row['source']['sha256']:raise RuntimeError('source changed')
        if row.get('executable_sha256') and sha(row['command'][0])!=row['executable_sha256']:raise RuntimeError('binary changed')
        try:execute(here,row,True)
        except Exception as error:
            excluded.append(dict(id=row['id'],language=row['language'],stage='full workload validation',reason=str(error)))
            print('excluded full workload',row['id'],str(error)[-220:],flush=True);continue
        row['samples_seconds']=[];valid.append(row)
    rows=valid
    if not rows:raise RuntimeError('no full-workload-conforming programs')
    orders=[];rng=random.Random(20261009);discarded=[]
    report=dict(schema_version=1,created_at=datetime.now(timezone.utc).isoformat(),benchmark=args.benchmark,
      hardware=dict(platform=platform.platform(),machine=platform.machine()),toolchains=toolchains(),programs=rows,excluded=excluded,
      protocol=dict(samples=args.samples,power=POWER,selection='All listed native-cohort sources; minimum LOC and published fastest for compact runtimes, ties retained' if not args.all else 'All listed configured sources',
        workload='FASTA N=5000000; nine separate counts and five ordered regex replacements' if args.benchmark=='regexredux' else 'N=10000; sequential unbounded spigot with both extract(3) and extract(4)',
        timed_region='Sequential fresh process through exit, full stdout to file; build, warmup, validation outside clock; JIT and startup included',
        source_metric='Nonblank noncomment physical LOC of complete source; generic library/runtime/compiler excluded',
        validation='Complete stdout byte equality on small input, full warmup and every accepted measured sample'),
      sample_order=orders,discarded_samples=discarded)
    (here/'results').mkdir(exist_ok=True);output=here/'results/apple-m3-ac-normal-20261009.json'
    if output.exists():raise RuntimeError('refusing to overwrite measured report: '+str(output))
    report['compiler_snapshot']=snapshot(output)
    for sample in range(args.samples):
        order=rows.copy();rng.shuffle(order);orders.append([r['id'] for r in order])
        for row in order:
            while True:
                guard.wait_for_environment(POWER);seconds=execute(here,row,True,True)
                if seconds is not None:break
                discarded.append(dict(id=row['id'],sample=sample+1))
            row['samples_seconds'].append(seconds)
            row['median_seconds']=statistics.median(row['samples_seconds'])
            if len(row['samples_seconds'])>1:row['stdev_seconds']=statistics.stdev(row['samples_seconds'])
            print(sample+1,row['id'],round(seconds,6),flush=True)
            output.write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
