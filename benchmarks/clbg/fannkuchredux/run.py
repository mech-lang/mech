#!/usr/bin/env python3
"""Pinned, byte-validated, sequential fresh-process fannkuch-redux measurements."""
import argparse,copy,gzip,hashlib,importlib.util,io,json,os,platform,random,re,shutil,statistics,subprocess,tarfile,tempfile,time
from pathlib import Path
from datetime import datetime,timezone
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];BUILD=HERE/'.build'
def module(name,path):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
common=module('common',HERE.parent/'nbody/run.py');process=module('clbg_process',HERE.parent/'process.py')
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def expected(n):return {7:b'228\nPfannkuchen(7) = 16\n',12:b'3968050\nPfannkuchen(12) = 65\n'}[n]
def metrics(path,language):
 raw=(HERE/path).read_text();clean=common.strip_comments(raw,language);normalized=re.sub(r'\s+',' ',clean).strip().encode()
 return dict(paths=[path],sha256=sha(HERE/path),physical_lines=len(raw.splitlines()),nonblank_noncomment_lines=sum(bool(s.strip()) for s in clean.splitlines()),normalized_bytes=len(normalized),gzip1_bytes=len(gzip.compress(normalized,compresslevel=1,mtime=0)))
def execute(argv,env,n,timeout):
 with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
  start=time.perf_counter_ns();process.run_checked([v.replace("{n}",str(n)) for v in argv] if any("{n}" in v for v in argv) else [*argv,str(n)],env=env,cwd=ROOT,stdout=out,stderr=err,stdin=subprocess.DEVNULL,timeout=timeout);elapsed=(time.perf_counter_ns()-start)/1e9
  out.seek(0);data=out.read();err.seek(0);error=err.read().decode(errors='replace')
  if data!=expected(n):raise ValueError(f'output differs for N={n}: {data!r}; {error[:500]}')
  return elapsed

def snapshot(output):
 paths=[*list((ROOT/'hosts/gpu/src/control').glob('*')),*list((ROOT/'src/engine/src/native_sequence').glob('*.rs'))]
 paths += [ROOT/p for p in ('Cargo.toml','Cargo.lock','hosts/gpu/Cargo.toml','hosts/gpu/src/native_build.rs','hosts/gpu/src/lib.rs','src/stdlib/src/native_sequence.rs','src/stdlib/src/lib.rs','scripts/unsafe-boundaries.allowlist','benchmarks/clbg/process.py')]
 paths += [ROOT/p for p in common.command(['git','ls-files','--modified','--others','--exclude-standard','--','src','hosts/gpu']).stdout.splitlines()]
 paths += [ROOT/p for p in ('benchmarks/clbg/nbody/run.py','benchmarks/clbg/nbody/.build/gcc-sdk-compat.h','benchmarks/clbg/mandelbrot/rust-deps/Cargo.toml','benchmarks/clbg/mandelbrot/rust-deps/Cargo.lock','benchmarks/clbg/mandelbrot/rust-deps/src/main.rs')]
 paths += [p for p in HERE.rglob('*') if p.is_file() and not any(v in ('.build','results','__pycache__') for v in p.relative_to(HERE).parts)]
 buf=io.BytesIO();hashes={}
 with tarfile.open(fileobj=buf,mode='w') as tar:
  for p in sorted(set(paths)):
   if not p.is_file():continue
   name=str(p.relative_to(ROOT));raw=p.read_bytes();hashes[name]=sha(p);item=tarfile.TarInfo(name);item.mode=0o644;item.size=len(raw);tar.addfile(item,io.BytesIO(raw))
 archive=output.with_suffix('.compiler.tar.gz');archive.write_bytes(gzip.compress(buf.getvalue(),mtime=0))
 return dict(path=archive.name,sha256=sha(archive),base_commit=common.command(['git','rev-parse','HEAD']).stdout.strip(),source_sha256=hashes)

def prepare(p,args,env):
 p=p.copy();lang=p['language'];source=HERE/p['path'];binary=BUILD/p['id'];text=source.read_text();ccmd=None;build_env=env.copy()
 if p.get('sha256') and sha(source)!=p['sha256']:raise ValueError('source pin differs')
 if platform.machine()!='x86_64' and re.search(r'__builtin_ia32|[xeitsp]+mmintrin|x86intrin|std::arch::x86_64',text):raise ValueError('unmodified source requires x86 intrinsics; ARM host')
 if lang in ('c','cpp'):
  llvm=p['cc_backend']=='llvm';cc=args.clang if llvm else args.gpp if lang=='cpp' else args.gcc
  ccmd=[cc,'-O3','-mcpu=native' if llvm else '-march=native','-fomit-frame-pointer','-funroll-loops','-flto','-std=c++17' if lang=='cpp' else '-std=gnu11',source,'-o',binary,'-lm','-pthread']+common.macos_gcc_flags(cc)
  if '#pragma omp' in text:
   if llvm:ccmd += ['-Xpreprocessor','-fopenmp','-I/opt/homebrew/opt/libomp/include','-L/opt/homebrew/opt/libomp/lib','-lomp']
   else:ccmd+=['-fopenmp']
  if 'boost/' in text:
   boost=Path('/opt/homebrew/opt/boost');ccmd+=['-I'+str(boost/'include'),'-L'+str(boost/'lib'),'-lboost_thread','-lboost_system']
  if llvm and lang=='cpp':ccmd+=['-lc++']
  argv=[str(binary)];policy='source OpenMP/pthread/std::thread policy'
 elif lang=='rust':
  deps=Path(getattr(args,'rust_deps',HERE.parent/'mandelbrot/.build/rust-deps/release/deps'));ccmd=['rustc','--edition=2021','-O','-Ctarget-cpu=native','-Ccodegen-units=1','-L',f'dependency={deps}']
  for name in re.findall(r'extern crate (\w+)',text):
   candidates=list(deps.glob('lib'+name+'-*.rlib'))
   if not candidates:raise ValueError('missing cached dependency '+name)
   ccmd+=['--extern',f'{name}={candidates[0]}']
  if p.get('lto'):ccmd+=['-Clto='+p['lto']]
  if p.get('panic_abort'):ccmd+=['-Cpanic=abort']
  ccmd+=[source,'-o',binary];argv=[str(binary)];policy='source Rayon/fixed thread policy'
 elif lang=='chapel':
  sdk=common.command(['xcrun','--show-sdk-path']).stdout.strip();ccmd=['chpl','--fast',source,'-o',binary,f'--ccflags=-isysroot {sdk}',f'--ldflags=-isysroot {sdk}'];argv=[str(binary),"--n={n}"];policy='Chapel task runtime (runtime may cap workers; stderr warning recorded separately)'
 elif lang=='go':ccmd=['go','build','-o',binary,source];argv=[str(binary)];policy='source goroutines/GOMAXPROCS'
 elif lang=='julia':argv=[args.julia,f'--threads={args.threads},0','--startup-file=no','--history-file=no','-O3','--',str(source)];policy='Julia source Threads policy'
 elif lang=='java':
  binary.mkdir(exist_ok=True);name=re.search(r'(?m)^\s*(?:public\s+)?(?:final\s+)?class\s+(\w+)',text).group(1);java=binary/(name+'.java');shutil.copyfile(source,java);ccmd=[str(Path(args.java_home)/'bin/javac'),'-d',binary,java];argv=[str(Path(args.java_home)/'bin/java'),f'-XX:ActiveProcessorCount={args.threads}','-cp',str(binary),name];policy='OpenJDK JIT; source was published as native AOT';p['execution_mode']=policy
 else:
  build_env.update(CARGO_TARGET_DIR=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),RUSTFLAGS='-D warnings')
  if p.get('byte_index'):build_env['MECH_CONTROL_BYTE_INDEX']=p['byte_index']
  project=Path(str(binary)+'.project')
  if project.exists():shutil.rmtree(project)
  ccmd=[args.mech,'build',source,'--backend','cranelift','--offline','--keep-project','--no-config','-o',binary]
  if not p['simd']:ccmd+=['--scalar']
  argv=[str(binary)];policy='general Rayon range scheduler; Mech loop callbacks compiled by Cranelift'
  p['mech_compiler_executable_sha256']=sha(args.mech);p['build_environment']={k:build_env[k] for k in ('CARGO_TARGET_DIR','RUSTFLAGS','MECH_CONTROL_BYTE_INDEX') if k in build_env}
 if ccmd:common.command([str(v) for v in ccmd],env=build_env)
 if lang=='mech':p['native_object_sha256']=sha(project/'mech-control.o');p['descriptor_sha256']=sha(project/'control.json')
 if lang not in ('julia','java'):p['executable_sha256']=sha(binary)
 p.update(source=metrics(p['path'],lang),compile_command=[str(v) for v in ccmd] if ccmd else None,command=argv,worker_policy=policy,samples_seconds=[],runtime_environment={})
 execute(argv,env,7,args.timeout);p['correctness_7']=dict(exact_reference_match=True,output=expected(7).decode());return p

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--samples',type=int,default=3);parser.add_argument('--threads',type=int,default=8);parser.add_argument('--timeout',type=int,default=300);parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--reuse-builds',action='store_true');parser.add_argument('--only');parser.add_argument('--skip');parser.add_argument('--output',type=Path,default=HERE/'results/apple-m3-12.json')
 parser.add_argument('--mech',default=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'));parser.add_argument('--julia',default='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia');parser.add_argument('--java-home',default='/opt/homebrew/opt/openjdk');parser.add_argument('--gcc',default='/opt/homebrew/bin/gcc-16');parser.add_argument('--gpp',default='/opt/homebrew/bin/g++-16');parser.add_argument('--clang',default='/opt/homebrew/opt/llvm@22/bin/clang-22');args=parser.parse_args();BUILD.mkdir(exist_ok=True);args.output.parent.mkdir(exist_ok=True)
 if args.samples<3:parser.error('at least three samples required')
 env=os.environ|{k:str(args.threads) for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')}
 if args.reuse_builds:
  d=json.loads((BUILD/'prepared.json').read_text());rows=d['programs'];excluded=d['excluded']
 else:
  programs=json.loads((HERE/'manifest.json').read_text())['programs'];programs=[candidate for p in programs for candidate in ([dict(p,id=p['id']+'-'+cc,cc_backend=cc) for cc in ('gcc','llvm')] if p['language'] in ('c','cpp') else [p,dict(p,id=p['id']+'-panic-abort',panic_abort=True)] if p['language']=='rust' else [p])]
  programs += [dict(id='mech-fannkuch-'+policy,language='mech',path=source,simd=simd,upstream_starred=False) for policy,source,simd in [('simd','fannkuchredux.mec',True),('scalar','fannkuchredux.mec',False),('coarse','fannkuchredux-coarse.mec',True),('paired','fannkuchredux-paired.mec',True),('cached','fannkuchredux-cached.mec',True)]]
  rows=[];excluded=[]
  for p in programs:
   if args.only and p['id'] not in args.only.split(','):continue
   if args.skip and re.search(args.skip,p['id']):continue
   try:rows.append(prepare(p,args,env));print('prepared',p['id'],flush=True)
   except Exception as error:excluded.append(dict(id=p['id'],language=p['language'],reason=str(error)[-1800:]));print('excluded',p['id'],str(error)[-200:],flush=True)
  (BUILD/'prepared.json').write_text(json.dumps(dict(programs=rows,excluded=excluded),indent=2)+'\n')
 if args.prepare_only:return
 rows=[r.copy() for r in rows if (not args.only or r['id'] in args.only.split(',')) and (not args.skip or not re.search(args.skip,r['id']))]
 for r in rows:
  if sha(HERE/r['path'])!=r['source']['sha256']:raise ValueError('source changed: '+r['id'])
  if r.get('executable_sha256') and sha(r['command'][0])!=r['executable_sha256']:raise ValueError('executable changed: '+r['id'])
  if r['language']=='mech':
   project=Path(r['command'][0]+'.project')
   if sha(project/'mech-control.o')!=r['native_object_sha256'] or sha(project/'control.json')!=r['descriptor_sha256']:raise ValueError('native object/descriptor changed: '+r['id'])
 validated=[]
 for r in rows:
  try:execute(r['command'],env|r['runtime_environment'],12,args.timeout);r['correctness_12']=dict(exact_reference_match=True,output=expected(12).decode());r['samples_seconds']=[];validated.append(r);print('full output passed',r['id'],flush=True)
  except Exception as error:excluded.append(dict(id=r['id'],language=r['language'],reason=str(error)));print('full exclusion',r['id'],str(error)[-200:],flush=True)
 rows=validated;rng=random.Random(20261007);orders=[]
 for i in range(args.samples):
  order=rows.copy();rng.shuffle(order);orders.append([r['id'] for r in order])
  for r in order:
   elapsed=execute(r['command'],env|r['runtime_environment'],12,args.timeout);r['samples_seconds'].append(elapsed);print(f"{i+1}/{args.samples} {r['id']} {elapsed:.6f}",flush=True)
 for r in rows:r.update(median_seconds=statistics.median(r['samples_seconds']),min_seconds=min(r['samples_seconds']),max_seconds=max(r['samples_seconds']))
 report=dict(schema_version=1,created_at=datetime.now(timezone.utc).isoformat(),hardware=dict(platform=platform.platform(),machine=platform.machine(),cpu=common.command(['sysctl','-n','machdep.cpu.brand_string']).stdout.strip(),logical_cpus=os.cpu_count()),compiler_snapshot=snapshot(args.output),protocol=dict(n=12,samples=args.samples,requested_threads=args.threads,sample_order=orders,environment={k:env[k] for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')},selection_skip_regex=args.skip,timing_wait='Blocking waitpid with POSIX deadline; no timeout poll quantization.',timed_region='Sequential fresh process to exit, complete output to regular file. Runtime/CLI and JIT startup included; building, tests and output comparison excluded.',required_work='All 12! permutations in prescribed order, dependent prefix flips, signed checksum and maximum; no precomputed input answers.',source_scope='Complete Mech permutation generation, factoradic initialization, flipping and checksum; generic compiler/runtime/library support excluded.'),programs=rows,excluded=excluded,endpoints=common.endpoint_summary(rows),nonheroic_endpoints=common.endpoint_summary([r for r in rows if not r['upstream_starred']]))
 args.output.write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
