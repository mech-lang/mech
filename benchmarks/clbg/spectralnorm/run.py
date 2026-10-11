#!/usr/bin/env python3
"""Build unchanged pinned spectral-norm programs; validate and time fresh processes."""
import argparse,gzip,hashlib,importlib.util,io,json,os,platform,random,re,shutil,statistics,subprocess,tarfile,time
from datetime import datetime,timezone
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];BUILD=HERE/'.build'
spec=importlib.util.spec_from_file_location('clbg_common',HERE.parent/'nbody/run.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
def command(args,**kw):return common.command([str(a) for a in args],**kw)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def metrics(path,language):
 source=(HERE/path).read_text();clean=common.strip_comments(source,language);normalized=re.sub(r'\s+',' ',clean).strip().encode()
 return {'paths':[path],'sha256':sha(HERE/path),'physical_lines':len(source.splitlines()),'nonblank_noncomment_lines':sum(bool(l.strip()) for l in clean.splitlines()),'normalized_bytes':len(normalized),'gzip1_bytes':len(gzip.compress(normalized,compresslevel=1,mtime=0))}
def execute(argv,env,n,timeout):
 argv=argv+[f'--n={n}' if 'chapel' in Path(argv[0]).name else str(n)]
 start=time.perf_counter_ns();r=subprocess.run(argv,cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True,timeout=timeout)
 return (time.perf_counter_ns()-start)/1e9,r.stdout,r.stderr.decode(errors='replace')
def snapshot(output):
 paths=list((ROOT/'hosts/gpu/src/sequence').glob('*'))+list((ROOT/'src/engine/src/native_sequence').glob('*.rs'))
 paths += list((ROOT/'hosts/gpu/src/fused_f64').glob('*.rs'))+list((ROOT/'hosts/gpu/src/batched').glob('*.rs'))+list((ROOT/'hosts/gpu/src/bin').glob('*.rs'))
 paths += [ROOT/'src/engine/src/resident/numeric/mod.rs']
 paths += [ROOT/p for p in ('Cargo.toml','Cargo.lock','hosts/gpu/Cargo.toml','hosts/gpu/src/lib.rs','hosts/gpu/src/native_build.rs','src/cli/commands/build.rs','src/engine/Cargo.toml','src/engine/src/lib.rs','src/engine/src/numeric_optimizations.rs','hosts/gpu/src/fused_f64/mod.rs','src/stdlib/Cargo.toml','src/stdlib/src/lib.rs','src/stdlib/src/native_sequence.rs','scripts/unsafe-boundaries.allowlist')]
 paths += [p for p in HERE.rglob('*') if p.is_file() and not any(x in ('.build','results','__pycache__','target') for x in p.relative_to(HERE).parts)]
 paths += [HERE.parent/'nbody/run.py',HERE.parent/'nbody/.build/gcc-sdk-compat.h']
 paths += [HERE.parent/p for p in ('mandelbrot/mandelbrot-compact-program.mec','mandelbrot/mandelbrot-program.mec','mandelbrot/upstream/mandelbrot-200.pbm','knucleotide/knucleotide-compact.mec','knucleotide/knucleotide.mec','knucleotide/input25000000.json','knucleotide/inputs/knucleotide-input-1000.txt','knucleotide/inputs/knucleotide-output-1000.txt')]
 buf=io.BytesIO();hashes={}
 with tarfile.open(fileobj=buf,mode='w') as tar:
  for p in sorted(set(paths)):
   if not p.is_file():continue
   name=str(p.relative_to(ROOT));raw=p.read_bytes();hashes[name]=sha(p);item=tarfile.TarInfo(name);item.mode=0o644;item.size=len(raw);tar.addfile(item,io.BytesIO(raw))
 archive=output.with_suffix('.compiler.tar.gz');archive.write_bytes(gzip.compress(buf.getvalue(),mtime=0))
 return {'path':archive.name,'sha256':sha(archive),'base_commit':command(['git','rev-parse','HEAD']).stdout.strip(),'source_sha256':hashes}
def prepare(p,args,env):
 p=p.copy();lang=p['language'];source=HERE/p['path'];binary=BUILD/p['id'];text=source.read_text();ccmd=None;env=env.copy()
 if p.get('sha256') and sha(source)!=p['sha256']:raise ValueError('source pin changed')
 if platform.machine()!='x86_64' and re.search(r'__builtin_ia32|x86intrin|[ei]mmintrin|std::arch::x86',text):raise ValueError('unmodified source requires x86 intrinsics; host is '+platform.machine())
 if platform.system()!='Linux' and 'sched_getaffinity' in text:raise ValueError('unmodified source requires Linux sched_getaffinity')
 if lang=='rust':
  deps=HERE.parent/'mandelbrot/.build/rust-deps/release/deps';ccmd=['rustc','--edition=2018' if p['variant'] in (1,2,3) else '--edition=2021','-Copt-level=3','-Ctarget-cpu=native','-Ccodegen-units=1','-Ldependency='+str(deps)]
  for dep in re.findall(r'extern crate (\w+);',text):ccmd+=['--extern',dep+'='+str(next(deps.glob('lib'+dep+'-*.rlib')))]
  if p.get('panic_abort'):ccmd+=['-Cpanic=abort']
  ccmd += [source,'-o',binary];argv=[str(binary)];policy='Rayon workers' if 'rayon' in text else 'source-defined std::thread workers' if 'thread' in text else 'serial'
 elif lang in ('c','cpp'):
  llvm=p['cc_backend']=='llvm';cc=args.clang if llvm else args.gpp if lang=='cpp' else args.gcc
  ccmd=[cc,'-O3','-mcpu=native' if llvm else '-march=native','-fomit-frame-pointer','-std=c++17' if lang=='cpp' else '-std=gnu11',source,'-o',binary,'-lm','-pthread']+common.macos_gcc_flags(cc)
  if llvm and lang=='cpp':ccmd+=['-lc++']
  if 'omp' in text:
   ccmd+=['-fopenmp']
   if llvm:ccmd+=['-I/opt/homebrew/opt/libomp/include','-L/opt/homebrew/opt/libomp/lib','-Wl,-rpath,/opt/homebrew/opt/libomp/lib']
  ccmd+=['-ffast-math','-funroll-loops','-flto'] if p['cc_tune']=='fastmath' else ['-ffp-contract=off']
  argv=[str(binary)];policy='OpenMP workers' if 'omp' in text else 'serial'
 elif lang=='chapel':
  sdk=command(['xcrun','--show-sdk-path']).stdout.strip();ccmd=['chpl','--fast',source,'-o',binary,f'--ccflags=-isysroot {sdk}',f'--ldflags=-isysroot {sdk}'];argv=[str(binary)];policy='Chapel runtime tasks'
 elif lang=='go':ccmd=['go','build','-o',binary,source];argv=[str(binary)];policy='source goroutines with GOMAXPROCS limit'
 elif lang=='julia':argv=[args.julia,'--startup-file=no','--history-file=no','-O3','--math-mode=ieee','--',str(source)];policy='Julia threads' if 'Threads.' in text else 'serial'
 elif lang=='java':
  binary.mkdir(exist_ok=True);js=binary/'spectralnorm.java';shutil.copyfile(source,js);ccmd=[str(Path(args.java_home)/'bin/javac'),'-d',binary,js]
  argv=[str(Path(args.java_home)/'bin/java'),f'-XX:ActiveProcessorCount={args.threads}','-cp',str(binary),'spectralnorm'];policy='OpenJDK processor limit; source worker policy'
  p['execution_mode']='OpenJDK JIT, fresh process; unchanged upstream GraalVM AOT submission source'
 else:
  env.update(MECH_NUMERIC_UNROLL=str(p['unroll']),MECH_NUMERIC_ROWS=str(p.get('rows',1)),MECH_NUMERIC_INDUCTION=str(p.get('induction',1)),CARGO_TARGET_DIR=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),RUSTFLAGS='-D warnings',MECH_SEQUENCE_PARTITIONS=str(p['partitions']),MECH_ARRAY_GRAIN=str(p.get('grain',16)))
  project=Path(str(binary)+'.project')
  if project.exists():shutil.rmtree(project)
  ccmd=[args.mech,'build',source,'--backend','cranelift','--offline','--keep-project','--no-config','-o',binary]
  if not p['simd']:ccmd+=['--scalar']
  p['public_build']=True;p['mech_compiler_executable_sha256']=sha(args.mech)
  p['execution_mode']='Mech canonical typed source; Cranelift AOT coefficients, matrix traversal, iteration, dot and sqrt; generic array storage and row scheduling runtime'
  argv=[str(binary)];policy=f"{p['partitions']*p.get('grain',16)} target row chunks over Rayon pool; RAYON_NUM_THREADS={args.threads}"
 if ccmd:command(ccmd,env=env)
 if lang=='mech':
  p['native_object_sha256']=sha(project/'mech-sequence.o');p['constants_sha256']=sha(project/'constants.json');p['executable_sha256']=sha(binary)
  p['build_environment']={key:env[key] for key in ('MECH_NUMERIC_UNROLL','MECH_NUMERIC_ROWS','MECH_NUMERIC_INDUCTION','CARGO_TARGET_DIR','RUSTFLAGS','MECH_SEQUENCE_PARTITIONS','MECH_ARRAY_GRAIN')}
  p['library_rustflags']='-D warnings -C target-cpu=native'
  p['generated_project_files']={str(f.relative_to(project)):f.read_text() for f in project.rglob('*') if f.is_file() and f.suffix in ('.toml','.lock','.rs')}

 p.update(source=metrics(p['path'],lang),compile_command=[str(x) for x in ccmd] if ccmd else None,command=argv,worker_policy=policy,samples_seconds=[],_env=env)
 _,data,_=execute(argv,env,100,args.timeout)
 if data!=(HERE/'output100.txt').read_bytes():raise ValueError('official N=100 output differs: '+repr(data))
 p['correctness_100']={'exact_reference_match':True,'sha256':hashlib.sha256(data).hexdigest()}
 return p

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--samples',type=int,default=5);ap.add_argument('--threads',type=int,default=8);ap.add_argument('--timeout',type=int,default=300);ap.add_argument('--size',type=int,default=5500);ap.add_argument('--only');ap.add_argument('--upstream-only',action='store_true');ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--reuse-builds',action='store_true');ap.add_argument('--mech',default=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'));ap.add_argument('--julia',default='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia');ap.add_argument('--java-home',default='/opt/homebrew/opt/openjdk');ap.add_argument('--clang',default='/opt/homebrew/opt/llvm@22/bin/clang-22');ap.add_argument('--gcc',default=shutil.which('gcc-16'));ap.add_argument('--gpp',default=shutil.which('g++-16'));ap.add_argument('--output',type=Path,default=HERE/'results/apple-m3-5500.json');args=ap.parse_args()
 if args.samples<3:ap.error('at least three samples required')
 BUILD.mkdir(exist_ok=True);args.output.parent.mkdir(exist_ok=True)
 for path,pin in json.loads((HERE/'input-pins.json').read_text()).items():
  if sha(HERE/path)!=pin['sha256']:raise ValueError('official output pin changed: '+path)

 env=os.environ|{'JULIA_NUM_THREADS':str(args.threads),'OMP_NUM_THREADS':str(args.threads),'OMP_DYNAMIC':'FALSE','CHPL_RT_NUM_THREADS_PER_LOCALE':str(args.threads),'GOMAXPROCS':str(args.threads),'RAYON_NUM_THREADS':str(args.threads),'OPENBLAS_NUM_THREADS':'1'}
 pins=json.loads((HERE/'manifest.json').read_text())['programs'];programs=[]
 for p in pins:
  if p['language'] in ('c','cpp'):
   programs += [{**p,'id':p['id']+'-'+cc+'-'+tune,'cc_backend':cc,'cc_tune':tune} for cc in ('gcc','llvm') for tune in ('native','fastmath')]
  else:programs.append(p)
 programs += [{**p,'id':p['id']+'-panicabort','panic_abort':True} for p in pins if p['language']=='rust' and p['variant'] in (4,5,7)]
 if not args.upstream_only:
  programs += [{'id':f'mech-{form}-simd{int(simd)}-u{unroll}-p{partitions}','language':'mech','path':file,'simd':simd,'unroll':unroll,'partitions':partitions,'upstream_starred':False} for form,file in [('compact','spectralnorm-compact.mec'),('optimized','spectralnorm.mec')] for simd,unroll,partitions in [(False,1,1),(False,1,8),(True,1,8),(True,2,8),(True,4,8),(True,8,8),(True,4,4)]]
 if not args.upstream_only:
  programs += [{'id':f'mech-optimized-simd1-u{unroll}-p8-g{grain}','language':'mech','path':'spectralnorm.mec','simd':True,'unroll':unroll,'partitions':8,'grain':grain,'upstream_starred':False} for unroll in (4,8) for grain in (1,4,64)]
 if not args.upstream_only:
  programs += [{'id':f'mech-{form}-simd1-u{unroll}-r{rows}-p8','language':'mech','path':file,'simd':True,'unroll':unroll,'rows':rows,'partitions':8,'upstream_starred':False} for form,file in [('compact','spectralnorm-compact.mec'),('optimized','spectralnorm.mec')] for unroll in (1,2,4) for rows in (2,4)]
 if not args.upstream_only:
  programs += [{'id':f'mech-integer-simd{int(simd)}-u{unroll}-r{rows}-p{partitions}','language':'mech','path':'spectralnorm-integer.mec','simd':simd,'unroll':unroll,'rows':rows,'partitions':partitions,'induction':0,'upstream_starred':False} for simd,unroll,rows,partitions in [(False,1,1,1),(False,1,1,8)]+[(True,u,1,8) for u in (1,2,4,8)]+[(True,u,r,8) for u in (1,2,4) for r in (2,4)]]
 if not args.upstream_only:
  programs += [{'id':f'mech-induction-simd{int(simd)}-u{unroll}-r{rows}-p{partitions}','language':'mech','path':'spectralnorm-integer.mec','simd':simd,'unroll':unroll,'rows':rows,'partitions':partitions,'induction':1,'upstream_starred':False} for simd,unroll,rows,partitions in [(False,1,1,1),(False,1,1,8)]+[(True,u,1,8) for u in (1,2,4,8)]+[(True,u,r,8) for u in (1,2,4) for r in (2,4)]]
 if not args.upstream_only:
  programs += [{'id':f'mech-{form}-simd1-u{unroll}-r4-p8-g{grain}','language':'mech','path':'spectralnorm-integer.mec','simd':True,'unroll':unroll,'rows':4,'partitions':8,'grain':grain,'induction':induction,'upstream_starred':False} for form,unroll,induction in [('integer',4,0),('induction',2,1)] for grain in (1,2,4,8,64)]
 if args.only:programs=[p for p in programs if p['id'] in args.only.split(',')]
 if not args.reuse_builds:
  rows=[];excluded=[]
  for p in programs:
   print('Prepare '+p['id'],flush=True)
   try:rows.append(prepare(p,args,env))
   except (ValueError,OSError,StopIteration,subprocess.SubprocessError) as e:
    diagnostic=e.stderr or e.stdout if isinstance(e,subprocess.CalledProcessError) else str(e)
    if isinstance(diagnostic,bytes):diagnostic=diagnostic.decode(errors='replace')
    excluded.append({'id':p['id'],'path':p['path'],'reason':str(e),'diagnostic':diagnostic});print('Exclude '+p['id']+': '+str(diagnostic)[-800:],flush=True)
  saved={'programs':[{k:v for k,v in r.items() if k!='_env'} for r in rows],'excluded':excluded}
  if (args.only or args.upstream_only) and (BUILD/'prepared.json').exists():
   old=json.loads((BUILD/'prepared.json').read_text());ids={p['id'] for p in programs}
   saved['programs']=[r for r in old['programs'] if r['id'] not in ids]+saved['programs'];saved['excluded']=[r for r in old['excluded'] if r['id'] not in ids]+saved['excluded']
  (BUILD/'prepared.json').write_text(json.dumps(saved,indent=2)+'\n')
  if args.prepare_only:return
 else:
  saved=json.loads((BUILD/'prepared.json').read_text());ids={p['id'] for p in programs};rows=[r for r in saved['programs'] if r['id'] in ids];excluded=[r for r in saved['excluded'] if r['id'] in ids]
  for r in rows:r['_env']=env.copy();r['samples_seconds']=[]
 for r in rows:
  if r['language']=='mech':r['_env'].update(MECH_SEQUENCE_PARTITIONS=str(r['partitions']),MECH_ARRAY_GRAIN=str(r.get('grain',16)))
  _,out,_=execute(r['command'],r['_env'],args.size,args.timeout)
  expected=b'1.274224153\n' if args.size==5500 else (HERE/'output100.txt').read_bytes() if args.size==100 else None
  if expected is not None and out!=expected:raise ValueError('full output mismatch: '+r['id']+' '+repr(out))
  r['correctness_full']={'output':out.decode(),'sha256':hashlib.sha256(out).hexdigest()}
 orders=[];rng=random.Random(20261007)
 for sample in range(args.samples):
  order=rows.copy();rng.shuffle(order);orders.append([r['id'] for r in order])
  for r in order:
   elapsed,out,err=execute(r['command'],r['_env'],args.size,args.timeout)
   if out.decode()!=r['correctness_full']['output']:raise ValueError('timed output differs: '+r['id'])
   r['samples_seconds'].append(elapsed);print(f"{sample+1}/{args.samples} {r['id']}: {elapsed:.6f}s",flush=True)
 for r in rows:r.pop('_env');r['median_seconds']=statistics.median(r['samples_seconds']);r['min_seconds']=min(r['samples_seconds']);r['max_seconds']=max(r['samples_seconds'])
 versions={}
 for name,argv in [('rust',['rustc','--version','--verbose']),('gcc',[args.gcc,'--version']),('clang',[args.clang,'--version']),('chapel',['chpl','--version']),('go',['go','version']),('julia',[args.julia,'--version']),('java',[str(Path(args.java_home)/'bin/java'),'-version'])]:
  result=command(argv);versions[name]=(result.stdout+result.stderr).strip()
 report={'schema_version':1,'created_at':datetime.now(timezone.utc).isoformat(),'hardware':{'platform':platform.platform(),'machine':platform.machine(),'cpu':command(['sysctl','-n','machdep.cpu.brand_string']).stdout.strip(),'logical_cpus':os.cpu_count()},'toolchain_versions':versions,'compiler_snapshot':snapshot(args.output),'protocol':{'n':args.size,'samples':args.samples,'requested_threads':args.threads,'sample_order':orders,'timed_region':'Sequential fresh process to exit; includes CLI, full prescribed work, output, and JIT startup. Builds and validation excluded.','required_work':f'40 matrix-vector applications; {40*args.size**2} weighted contributions; ten pairs of AtAv applications. No stored matrix, answer shortcut or reduced iteration count.','source_scope':'Complete Mech source with four required functions, matrix formula, iteration and norm; compiler/runtime array primitives excluded.','source_metric':'Nonblank noncomment LOC and whitespace-normalized gzip level1 bytes.','compiler_status':'Experimental v0.4 integration extension exposed through public mech build --backend cranelift.','cohort':'All pinned submissions for seven languages attempted. Architecture/build exclusions recorded. Java uses OpenJDK JIT, not published GraalVM AOT.','environment':{k:env[k] for k in ('JULIA_NUM_THREADS','OMP_NUM_THREADS','OMP_DYNAMIC','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','RAYON_NUM_THREADS','OPENBLAS_NUM_THREADS')}},'programs':rows,'excluded':excluded,'endpoints':common.endpoint_summary(rows),'nonheroic_endpoints':common.endpoint_summary([r for r in rows if not r['upstream_starred']])}
 args.output.write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
