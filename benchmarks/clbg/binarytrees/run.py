#!/usr/bin/env python3
"""Pinned, byte-validated, sequential fresh-process binary-trees measurements."""
import argparse,copy,gzip,hashlib,importlib.util,io,json,os,platform,random,re,shutil,statistics,subprocess,tarfile,tempfile,time
from pathlib import Path
from datetime import datetime,timezone
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];BUILD=HERE/'.build'
def module(name,path):
 spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
common=module('common',HERE.parent/'nbody/run.py');process=module('clbg_process',HERE.parent/'process.py')
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def metrics(path,language):
 raw=(HERE/path).read_text();clean=common.strip_comments(raw,language);normalized=re.sub(r'\s+',' ',clean).strip().encode()
 return dict(paths=[path],sha256=sha(HERE/path),physical_lines=len(raw.splitlines()),nonblank_noncomment_lines=sum(bool(s.strip()) for s in clean.splitlines()),normalized_bytes=len(normalized),gzip1_bytes=len(gzip.compress(normalized,compresslevel=1,mtime=0)))
def execute(argv,env,n,timeout):
 with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
  start=time.perf_counter_ns();process.run_checked([v.replace("{n}",str(n)) for v in argv] if any("{n}" in v for v in argv) else [*argv,str(n)],env=env,cwd=ROOT,stdout=out,stderr=err,stdin=subprocess.DEVNULL,timeout=timeout);elapsed=(time.perf_counter_ns()-start)/1e9
  out.seek(0,os.SEEK_END);size=out.tell();out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest();err.seek(0);error=err.read().decode(errors='replace')
  pin=json.loads((HERE/'reference-pins.json').read_text())['small' if n==10 else 'full']
  if size!=pin['bytes'] or digest!=pin['sha256']:raise ValueError(f'output differs for N={n}: bytes={size}, SHA256={digest}; {error[:500]}')
  return dict(seconds=elapsed,validation=dict(exact_reference_match=True,bytes=size,sha256=digest),stderr=error)
def snapshot(output):
 paths=[*list((ROOT/'hosts/gpu/src/control').glob('*')),*list((ROOT/'src/engine/src/native_sequence').glob('*.rs'))]
 paths += [ROOT/p for p in ('Cargo.toml','Cargo.lock','hosts/gpu/Cargo.toml','hosts/gpu/src/native_build.rs','hosts/gpu/src/lib.rs','src/stdlib/src/native_sequence.rs','src/stdlib/src/lib.rs','scripts/unsafe-boundaries.allowlist','benchmarks/clbg/process.py')]
 paths += [ROOT/p for p in common.command(['git','ls-files','--modified','--others','--exclude-standard','--','src','hosts/gpu']).stdout.splitlines()]
 paths += [ROOT/p for p in ('benchmarks/clbg/README.md','benchmarks/clbg/nbody/run.py','benchmarks/clbg/nbody/.build/gcc-sdk-compat.h','benchmarks/clbg/binarytrees/rust-deps/Cargo.toml','benchmarks/clbg/binarytrees/rust-deps/Cargo.lock','benchmarks/clbg/binarytrees/rust-deps/src/main.rs')]
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
 if platform.machine()!='x86_64' and '#ifdef __SSE2__' not in text and re.search(r'__builtin_ia32|[xeitsp]+mmintrin|x86intrin',text):raise ValueError('unmodified source requires x86 intrinsics; ARM host')
 if lang in ('c','cpp'):
  llvm=p['cc_backend']=='llvm';cc=args.clang if llvm else args.gpp if lang=='cpp' else args.gcc
  ccmd=[cc,'-O3','-mcpu=native' if llvm else '-march=native','-fomit-frame-pointer','-funroll-loops','-flto','-std=c++17' if lang=='cpp' else '-std=gnu11',source,'-o',binary,'-lm','-pthread']+common.macos_gcc_flags(cc)
  if '#pragma omp' in text:
   if llvm:ccmd += ['-Xpreprocessor','-fopenmp','-I/opt/homebrew/opt/libomp/include','-L/opt/homebrew/opt/libomp/lib','-lomp']
   else:ccmd+=['-fopenmp']
  if 'boost/' in text: ccmd+=['-I/opt/homebrew/opt/boost/include']
  if 'apr_pools.h' in text: ccmd+=['-I/opt/homebrew/opt/apr/include/apr-1','-L/opt/homebrew/opt/apr/lib','-lapr-1']
  if '<execution>' in text: ccmd+=['-I/opt/homebrew/opt/tbb/include','-L/opt/homebrew/opt/tbb/lib','-ltbb']
  if '<malloc.h>' in text: ccmd+=['-I'+str(HERE/'compat')]
  if 'sched_getaffinity' in text:
   ccmd+=['-include',str(HERE/'compat/linux-affinity.h')]
   p['platform_compatibility']='macOS online logical CPU enumeration replaces unavailable Linux affinity query; no affinity mask imposed; unchanged tree source'
  if llvm and lang=='cpp':ccmd+=['-lc++']
  argv=[str(binary)];policy='source OpenMP/pthread/std::thread policy'
 elif lang=='rust':
  deps=Path(getattr(args,'rust_deps',HERE/'.build/rust-deps/release/deps'));ccmd=['rustc','--edition=2015' if p['variant'] in (1,2) else '--edition=2021','-O','-Ctarget-cpu=native','-Ccodegen-units=1','-L',f'dependency={deps}']
  for name in sorted(set(re.findall(r'extern crate (\w+)',text)) | ({'num_cpus'} if 'num_cpus::' in text else set())):
   if name in ('core','std'):continue
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
  build_env.update(CARGO_TARGET_DIR=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),RUSTFLAGS='-D warnings -C target-cpu=native')
  if 'recursive_inline' in p:build_env['MECH_CONTROL_RECURSIVE_INLINE']=str(p['recursive_inline'])
  if p.get('byte_index'):build_env['MECH_CONTROL_BYTE_INDEX']=p['byte_index']
  if p.get('scan_unroll'):build_env['MECH_CONTROL_SCAN_UNROLL']=str(p['scan_unroll'])
  if 'narrow_uint' in p:build_env['MECH_CONTROL_NARROW_UINT']=str(int(p['narrow_uint']))
  if 'narrow_arithmetic' in p:build_env['MECH_CONTROL_NARROW_ARITHMETIC']=str(int(p['narrow_arithmetic']))
  if 'exact_float_compare' in p:build_env['MECH_CONTROL_EXACT_FLOAT_COMPARE']=str(int(p['exact_float_compare']))
  if 'scan_cache' in p:build_env['MECH_CONTROL_SCAN_CACHE']=str(int(p['scan_cache']))
  if 'scan_pipeline' in p:build_env['MECH_CONTROL_SCAN_PIPELINE']=p['scan_pipeline'] if isinstance(p['scan_pipeline'],str) else str(int(p['scan_pipeline']))
  project=Path(str(binary)+'.project')
  if project.exists():shutil.rmtree(project)
  ccmd=[args.mech,'build',source,'--backend','cranelift','--offline','--keep-project','--no-config','-o',binary]
  if not p.get('simd',True):ccmd+=['--scalar']
  argv=[str(binary)];policy='Source construction and traversal; core planned arena projection with scoped retirement; Rayon ranges only in parallel variant'
  p['mech_compiler_executable_sha256']=sha(args.mech);p['build_environment']={k:build_env[k] for k in ('CARGO_TARGET_DIR','RUSTFLAGS','MECH_CONTROL_BYTE_INDEX','MECH_CONTROL_SCAN_UNROLL','MECH_CONTROL_NARROW_UINT','MECH_CONTROL_NARROW_ARITHMETIC','MECH_CONTROL_EXACT_FLOAT_COMPARE','MECH_CONTROL_SCAN_CACHE','MECH_CONTROL_SCAN_PIPELINE','MECH_CONTROL_INLINE','MECH_CONTROL_RECURSIVE_INLINE') if k in build_env}
 if ccmd:common.command([str(v) for v in ccmd],env=build_env)
 if lang=='mech':
  p['native_object_sha256']=sha(project/'mech-control.o');p['descriptor_sha256']=sha(project/'control.json')
  p['generated_project_files']={str(f.relative_to(project)):f.read_text() for f in project.rglob('*') if f.is_file() and f.suffix in ('.rs','.toml','.lock','.json')}
 if lang not in ('julia','java'):p['executable_sha256']=sha(binary)
 p.update(source=metrics(p['path'],lang),compile_command=[str(v) for v in ccmd] if ccmd else None,command=argv,worker_policy=policy,samples_seconds=[],runtime_environment={})
 check=execute(argv,env,10,args.timeout);p['correctness_10']=check['validation'];return p

def finish_report(rows,excluded,orders,args,env):
 for r in rows:r.update(median_seconds=statistics.median(r['samples_seconds']),min_seconds=min(r['samples_seconds']),max_seconds=max(r['samples_seconds']))
 report=dict(schema_version=1,created_at=datetime.now(timezone.utc).isoformat(),hardware=dict(platform=platform.platform(),machine=platform.machine(),cpu=common.command(['sysctl','-n','machdep.cpu.brand_string']).stdout.strip(),logical_cpus=os.cpu_count()),compiler_snapshot=snapshot(args.output),protocol=dict(n=21,samples=args.samples,requested_threads=args.threads,sample_order=orders,environment={k:env[k] for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')},selection_skip_regex=args.skip,timing_wait='Blocking waitpid with POSIX deadline; no timeout poll quantization.',timed_region='Sequential fresh process to exit, complete reference output to regular file. Runtime/CLI and JIT startup included; building, tests and output hashing excluded.',required_work='Fully allocate stretch tree depth N+1; keep long-lived tree depth N while constructing/checking/deallocating 2^(N-depth+4) perfect trees for every even depth 4..N; every leaf allocated as an ordinary node; check long-lived tree last.',source_scope='Complete Mech node construction, recursive traversal and orchestration; source selects general memory scopes for reclamation; generic compiler/runtime/library support excluded.',cohorts='All accepted individual/default-GC and library-pool implementations; allocator policy is labeled per source. No custom node pools or omitted tree work.'),programs=rows,excluded=excluded,endpoints=common.endpoint_summary(rows),strict_endpoints=common.endpoint_summary([r for r in rows if r['algorithm']['strict']]),nonheroic_endpoints=common.endpoint_summary([r for r in rows if not r['upstream_starred']]))
 report['allocator_endpoints']={policy:common.endpoint_summary(cohort) for policy in ('individual','default-gc','library-pool') if (cohort:=[r for r in rows if r['algorithm']['allocation']==policy])}
 version_commands={'rust':['rustc','--version'],'go':['go','version'],'chapel':['chpl','--version'],'julia':[args.julia,'--version'],'java':[str(Path(args.java_home)/'bin/java'),'-version'],'gcc':[args.gcc,'--version'],'llvm':[args.clang,'--version'],'libraries':['brew','list','--versions','apr','boost','tbb']}
 report['toolchains']={}
 for name,cmd in version_commands.items():
  result=common.command(cmd);report['toolchains'][name]=(result.stdout+result.stderr).strip().splitlines()[:3]
 report['conformance']=json.loads((HERE/'results/conformance.json').read_text())
 report['regressions']=json.loads((HERE/'results/regressions.json').read_text())
 report['records_conformance']=json.loads((HERE/'results/records-conformance.json').read_text())
 report['forms_conformance']=json.loads((HERE/'results/forms-conformance.json').read_text())
 report['protocol']['selection_only']=args.only
 if args.only:report['protocol']['cohorts']='Selected builds from the pinned cohort, as listed in selection_only; allocator policies remain labeled. This is not a complete-cohort ranking.'
 for name in ('conformance','regressions','records_conformance','forms_conformance'):
  if report[name]['compiler_sha256']!=sha(args.mech):raise ValueError('validation compiler changed: '+name)
 args.output.write_text(json.dumps(report,indent=2)+'\n')

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--samples',type=int,default=3);parser.add_argument('--threads',type=int,default=8);parser.add_argument('--timeout',type=int,default=300);parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--reuse-builds',action='store_true');parser.add_argument('--only');parser.add_argument('--skip');parser.add_argument('--output',type=Path,default=HERE/'results/apple-m3-21.json')
 parser.add_argument('--rust-deps',type=Path,default=HERE/'.build/rust-deps-bitcode/release/deps');parser.add_argument('--mech',default=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'));parser.add_argument('--julia',default='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia');parser.add_argument('--java-home',default='/opt/homebrew/opt/openjdk');parser.add_argument('--gcc',default='/opt/homebrew/bin/gcc-16');parser.add_argument('--gpp',default='/opt/homebrew/bin/g++-16');parser.add_argument('--clang',default='/opt/homebrew/opt/llvm@22/bin/clang-22');args=parser.parse_args();BUILD.mkdir(exist_ok=True);args.output.parent.mkdir(exist_ok=True)
 if args.samples<3:parser.error('at least three samples required')
 env=os.environ|{k:str(args.threads) for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')}
 if args.reuse_builds:
  d=json.loads((BUILD/'prepared.json').read_text());rows=d['programs'];excluded=d['excluded']
 else:
  programs=json.loads((HERE/'manifest.json').read_text())['programs'];audit=json.loads((HERE/'algorithm-audit.json').read_text())['programs']
  programs=[dict(p,algorithm=audit[p['id']]) for p in programs]
  programs=[candidate for p in programs for candidate in ([dict(p,id=p['id']+'-'+cc,cc_backend=cc) for cc in ('gcc','llvm')] if p['language'] in ('c','cpp') else [p,dict(p,id=p['id']+'-thin',lto='thin')] if p['language']=='rust' else [p])]
  programs += [dict(id='mech-binarytrees-'+policy,language='mech',path=source,simd=True,upstream_starred=False,algorithm=dict(strict=True,allocation='library-pool',reason='Source constructs and traverses every node including leaves; general core planned arena storage is retired by scope.')) for policy,source in [('compact','binarytrees.mec'),('parallel','binarytrees-parallel.mec')]]
  rows=[];excluded=[]
  for p in programs:
   if args.only and p['id'] not in args.only.split(','):continue
   if args.skip and re.search(args.skip,p['id']):continue
   try:rows.append(prepare(p,args,env));print('prepared',p['id'],flush=True)
   except Exception as error:excluded.append(dict(id=p['id'],language=p['language'],reason=(str(error)+'\n'+str(getattr(error,'stderr','') or getattr(error,'stdout','')))[-3500:]));print('excluded',p['id'],str(error)[-200:],flush=True)
  (BUILD/'prepared.json').write_text(json.dumps(dict(programs=rows,excluded=excluded),indent=2)+'\n')
 if args.prepare_only:return
 rows=[r.copy() for r in rows if (not args.only or r['id'] in args.only.split(',')) and (not args.skip or not re.search(args.skip,r['id']))]
 for r in rows:
  if sha(HERE/r['path'])!=r['source']['sha256']:raise ValueError('source changed: '+r['id'])
  if r.get('executable_sha256') and sha(r['command'][0])!=r['executable_sha256']:raise ValueError('executable changed: '+r['id'])
  if r['language']=='mech':
   if r['mech_compiler_executable_sha256']!=sha(args.mech):raise ValueError('Mech build compiler changed: '+r['id'])
   project=Path(r['command'][0]+'.project')
   if sha(project/'mech-control.o')!=r['native_object_sha256'] or sha(project/'control.json')!=r['descriptor_sha256']:raise ValueError('native object/descriptor changed: '+r['id'])
 validated=[]
 for r in rows:
  try:
   check=execute(r['command'],env|r['runtime_environment'],21,args.timeout);r['correctness_21']=check['validation'];r['stderr_full']=check['stderr'];r['validation_elapsed_seconds']=check['seconds'];r['samples_seconds']=[];validated.append(r);print('full output passed',r['id'],f"{check['seconds']:.6f}",flush=True)
  except Exception as error:excluded.append(dict(id=r['id'],language=r['language'],reason=str(error)));print('full exclusion',r['id'],str(error)[-200:],flush=True)
 rows=validated;rng=random.Random(20261008);orders=[]
 (BUILD/'sampling-progress.json').write_text(json.dumps(dict(programs=rows,excluded=excluded,sample_order=orders),indent=2)+'\n')
 for i in range(args.samples):
  order=rows.copy();rng.shuffle(order);orders.append([r['id'] for r in order])
  for r in order:
   check=execute(r['command'],env|r['runtime_environment'],21,args.timeout);elapsed=check['seconds'];r['samples_seconds'].append(elapsed);print(f"{i+1}/{args.samples} {r['id']} {elapsed:.6f}",flush=True)
   (BUILD/'sampling-progress.json').write_text(json.dumps(dict(programs=rows,excluded=excluded,sample_order=orders),indent=2)+'\n')
 finish_report(rows,excluded,orders,args,env)
if __name__=='__main__':main()
