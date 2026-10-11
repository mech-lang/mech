#!/usr/bin/env python3
"""Build unchanged pinned programs, validate all output, and time identical stdin."""
from __future__ import annotations
import argparse,gzip,hashlib,importlib.util,io,json,os,platform,random,re,shutil,statistics,subprocess,tarfile,tempfile,time
from datetime import datetime,timezone
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
spec=importlib.util.spec_from_file_location('nbody_common',HERE.parent/'nbody/run.py')
common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
BUILD=HERE/'.build'
SMALL=HERE/'inputs/knucleotide-input-1000.txt'
SMALL_OUTPUT=(HERE/'inputs/knucleotide-output-1000.txt').read_bytes()
FULL=BUILD/'knucleotide-input25000000.txt'

def command(argv,**kw):return common.command([str(x) for x in argv],**kw)
def sha(path):return hashlib.file_digest(Path(path).open('rb'),'sha256').hexdigest()
def metrics(path):
 source=(HERE/path).read_text();language='mech' if path.endswith('.mec') else 'julia' if path.endswith('.jl') else 'c'
 clean=common.strip_comments(source.lstrip('\ufeff'),language);normalized=re.sub(r'\s+',' ',clean).strip().encode()
 return {'paths':[path],'sha256':sha(HERE/path),'physical_lines':len(source.splitlines()),'nonblank_noncomment_lines':sum(bool(x.strip()) for x in clean.splitlines()),'normalized_bytes':len(normalized),'gzip1_bytes':len(gzip.compress(normalized,compresslevel=1,mtime=0))}
def execute(argv,env,input_path,timeout):
 with input_path.open('rb') as stdin,tempfile.TemporaryFile() as output:
  start=time.perf_counter_ns();result=subprocess.run(argv,cwd=ROOT,env=env,stdin=stdin,stdout=output,stderr=subprocess.PIPE,timeout=timeout,check=True);elapsed=(time.perf_counter_ns()-start)/1e9
  output.seek(0);data=output.read()
 return elapsed,data,result.stderr.decode(errors='replace')
def info(data):return {'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()}
def workers(p,t):
 l,v=p['language'],p.get('variant')
 if l=='mech' and p.get('partitions',1)>1:return f'Rayon pool requests {t} compute workers; {p["partitions"]} disjoint partitions per histogram plus at most 7 waiting coordinator threads'
 if l=='mech':return 'independent histogram tasks, at most 7 live workers; DAG scheduling' if p.get('schedule')=='dag' else 'independent histogram tasks, at most 7 live workers; compact source waits between tasks'
 if l=='chapel':return f'Chapel runtime requests {t} workers; per-frame parallel map reduction'
 if l=='julia':return 'serial' if v==1 else f'Julia requests {t} threads; histogram tasks'
 if l=='rust':return {1:'7 std::thread histogram jobs',2:'7 std::thread histogram jobs',3:'7 std::thread histogram jobs',4:'futures CpuPool::new_num_cpus()',5:'futures CpuPool::new_num_cpus()',6:'scoped pool from num_cpus',7:'tokio ThreadPool::new()',9:'tokio ThreadPool::new()'}[v]
 if l=='go':return f'GOMAXPROCS={t}; source partitions using NumCPU=8' if v!=8 else f'GOMAXPROCS={t}; source goroutine histogram jobs'
 if l=='java':return f'ActiveProcessorCount={t}; source executor/parallel-stream policy' if v not in (4,8) else ('serial' if v==4 else f'ActiveProcessorCount={t}; parallel streams')
 if l=='c':return f'OpenMP requests {t} workers, 7 sections'
 return {1:'6 std::async histogram jobs plus main-thread histogram',2:'fixed 4 std::thread workers per histogram',3:'8 std::async per-frame jobs for each histogram'}[v]

def build(p,args,environment):
 p=p.copy();identifier,lang=p['id'],p['language'];source,binary=HERE/p['path'],BUILD/identifier;text=source.read_text();env=environment.copy();ccmd=None
 if p.get('sha256') and sha(source)!=p['sha256']:raise ValueError('upstream source hash changed')
 if lang=='rust':
  deps=BUILD/'rust-deps/release/deps';ccmd=['rustc','--edition=2021','-Copt-level=3','-Ctarget-cpu=native','-Ccodegen-units=1','-Ldependency='+str(deps)]
  for dep in re.findall(r'extern crate (\w+);',text):ccmd+=['--extern',dep+'='+str(next(deps.glob(f'lib{dep}-*.rlib')))]
  if p.get('panic_abort'):ccmd += ['-Cpanic=abort']
  ccmd += [source,'-o',binary];argv=[str(binary)]
 elif lang in ('c','cpp'):
  llvm=p['cc_backend']=='llvm';cc=args.clang if llvm else args.gpp if lang=='cpp' else args.gcc

  ccmd=[cc,'-O3','-mcpu=native' if llvm else '-march=native','-fomit-frame-pointer','-std=c++17' if lang=='cpp' else '-std=gnu11',source,'-o',binary,'-lm','-pthread','-I'+str(HERE/'upstream')]+common.macos_gcc_flags(cc)
  if llvm and lang=='cpp' and 'ext/pb_ds/' in text:
   gcc_root=Path(args.gpp).resolve().parents[1];headers=next((gcc_root/'include/c++').iterdir());arch_headers=next(headers.glob('*/bits/c++config.h')).parents[1]
   ccmd+=['-femulated-tls','-nostdinc++','-isystem',str(headers),'-isystem',str(arch_headers),'-isystem',str(headers/'backward'),'-include',str(HERE.parent/'nbody/.build/gcc-sdk-compat.h'),'-Wno-nullability-completeness',str(gcc_root/'lib/gcc/current/libstdc++.6.dylib')]
  if '#pragma omp' in text:
   ccmd+=['-fopenmp']
   if llvm:ccmd+=['-I/opt/homebrew/opt/libomp/include','-L/opt/homebrew/opt/libomp/lib','-Wl,-rpath,/opt/homebrew/opt/libomp/lib']
  if p['cc_tune'].startswith('fastmath'):
   ccmd+=['-ffast-math','-funroll-loops']
   if p['cc_tune']!='fastmath-nolto':ccmd+=['-flto']
  else:ccmd+=['-ffp-contract=off']
  argv=[str(binary)]
 elif lang=='chapel':
  sdk=command(['xcrun','--show-sdk-path']).stdout.strip();ccmd=['chpl','--fast',source,'-o',binary,f'--ccflags=-isysroot {sdk}',f'--ldflags=-isysroot {sdk}'];argv=[str(binary)]
 elif lang=='go':ccmd=['go','build','-o',binary,source];argv=[str(binary)]
 elif lang=='julia':argv=[args.julia,'--startup-file=no','--history-file=no','-O3','--math-mode=ieee','--',str(source)]
 elif lang=='java':
  binary.mkdir(exist_ok=True);js=binary/'knucleotide.java';shutil.copyfile(source,js);classpath=str(binary)+os.pathsep+str(BUILD/'fastutil-8.3.1.jar')
  ccmd=[str(Path(args.java_home)/'bin/javac'),'-cp',classpath,'-d',binary,js]
  argv=[str(Path(args.java_home)/'bin/java'),f'-XX:ActiveProcessorCount={args.threads}','-Xmx4g','-cp',classpath,'knucleotide']
 else:
  env.update(CARGO_TARGET_DIR=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),MECH_SEQUENCE_SOURCE=str(source),MECH_SEQUENCE_BATCH=str(p['batch']),MECH_SEQUENCE_SCHEDULE=p.get('schedule','source'),MECH_SEQUENCE_PACK8=str(int(p.get('pack8',False))),MECH_SEQUENCE_FUSE_ENCODE=str(int(p.get('fuse_encode',False))),MECH_SEQUENCE_SIMD=str(int(p.get('simd',False))),RUSTFLAGS='-D warnings -C target-cpu=native')
  if p.get('standard_map') and not p.get('simd') and p.get('partitions')==4:
   project=Path(str(binary)+'.project')
   if project.exists():shutil.rmtree(project)
   generation=[args.mech,'build',str(source),'--backend','cranelift','--offline','--keep-project','--no-config','-o',str(binary)]
   p['compile_command']=generation
   # The builder adds native CPU tuning once; pin the measured library flags.
   command(generation,env=env|{'RUSTFLAGS':'-D warnings'})
   obj=project/'mech-sequence.o';ccmd=None;argv=[str(binary)]
   p['public_build']=True;p['mech_compiler_executable_sha256']=sha(args.mech)
   p['generated_project_files']={str(f.relative_to(project)):f.read_text() for f in project.rglob('*') if f.is_file() and f.suffix in ('.toml','.lock','.rs','.json')}
  else:
   ccmd=['cargo','build','--release','--locked','--manifest-path',HERE/'mech-driver/Cargo.toml']
   p['compile_command']=[str(x) for x in ccmd];p['build_environment']={k:env[k] for k in ('CARGO_TARGET_DIR','MECH_SEQUENCE_SOURCE','MECH_SEQUENCE_BATCH','MECH_SEQUENCE_SCHEDULE','MECH_SEQUENCE_PACK8','MECH_SEQUENCE_FUSE_ENCODE','MECH_SEQUENCE_SIMD','RUSTFLAGS')}
   command(ccmd,env=env);shutil.copy2(Path(env['CARGO_TARGET_DIR'])/'release/mech-clbg-knucleotide',binary);ccmd=None;argv=[str(binary)]
   out_dirs=list((Path(env['CARGO_TARGET_DIR'])/'release/build').glob('mech-clbg-knucleotide-*/out/mech-sequence.o'));obj=max(out_dirs,key=lambda p:p.stat().st_mtime)
  p.update(native_object_sha256=sha(obj),execution_mode='experimental v0.4 engine typed sequence IR with standard-library catalog; source-defined Mech functions; Cranelift AOT arithmetic/window loops; generic Rust map, sort, text and IO library runtime')
 if ccmd:command(ccmd,env=env)
 p.update(source=metrics(p['path']),compile_command=[str(x) for x in ccmd] if ccmd else p.get('compile_command'),command=argv,worker_policy=workers(p,args.threads),samples_seconds=[],_env=env)
 if lang=='java':p['execution_mode']='OpenJDK JIT; unmodified upstream GraalVM AOT source'
 if args.build_only:return p
 try:
  _,data,_=execute(argv,env,SMALL,args.timeout)
  p['correctness_1000']=info(data)|{'exact_reference_match':data==SMALL_OUTPUT}
  if data!=SMALL_OUTPUT:print('Small-test failure retained for full-input validation: '+identifier,flush=True)
 except subprocess.CalledProcessError as error:
  p['correctness_1000']={'exact_reference_match':False,'exit_code':error.returncode,'diagnostic':error.stderr.decode(errors='replace')}
  print('Small-test failure retained for full-input validation: '+identifier,flush=True)
 return p

def snapshot(output):
 paths=[p for p in (ROOT/'hosts/gpu/src/sequence').glob('*.rs')]+[ROOT/'hosts/gpu/Cargo.toml',ROOT/'hosts/gpu/src/lib.rs',ROOT/'scripts/unsafe-boundaries.allowlist']
 paths += [p for p in HERE.rglob('*') if p.is_file() and not any(x in ('.build','results','__pycache__','target') for x in p.relative_to(HERE).parts)]
 paths += [ROOT/p for p in ('Cargo.toml','Cargo.lock','hosts/gpu/src/native_build.rs','hosts/gpu/src/sequence/application.rs.in','src/cli/commands/build.rs','src/engine/Cargo.toml','src/engine/src/lib.rs','src/stdlib/Cargo.toml','src/stdlib/src/lib.rs','src/stdlib/src/native_sequence.rs')]
 paths += list((ROOT/'src/engine/src/native_sequence').glob('*.rs'))
 paths += [HERE.parent/'nbody/run.py',HERE.parent/'nbody/.build/gcc-sdk-compat.h']
 data=io.BytesIO();hashes={}
 with tarfile.open(fileobj=data,mode='w') as tar:
  for p in sorted(set(paths)):
   name=str(p.relative_to(ROOT));raw=p.read_bytes();hashes[name]=hashlib.sha256(raw).hexdigest();item=tarfile.TarInfo(name);item.size=len(raw);item.mode=0o644;tar.addfile(item,io.BytesIO(raw))
 archive=output.with_suffix('.compiler.tar.gz');archive.write_bytes(gzip.compress(data.getvalue(),mtime=0))
 return {'path':archive.name,'sha256':sha(archive),'base_commit':command(['git','rev-parse','HEAD']).stdout.strip(),'source_sha256':hashes}

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--samples',type=int,default=3);parser.add_argument('--threads',type=int,default=8);parser.add_argument('--timeout',type=int,default=300);parser.add_argument('--seed',type=int,default=20261007);parser.add_argument('--mech',default=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'));parser.add_argument('--only');parser.add_argument('--build-only',action='store_true');parser.add_argument('--prepare-only',action='store_true');parser.add_argument('--reuse-builds',action='store_true');parser.add_argument('--julia',default='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia');parser.add_argument('--java-home',default='/opt/homebrew/opt/openjdk');parser.add_argument('--clang',default='/opt/homebrew/opt/llvm@22/bin/clang-22');parser.add_argument('--gcc',default=shutil.which('gcc-16'));parser.add_argument('--gpp',default=shutil.which('g++-16'));parser.add_argument('--output',type=Path,default=HERE/'results/apple-m3-25000000.json');args=parser.parse_args()
 BUILD.mkdir(exist_ok=True);args.output.parent.mkdir(exist_ok=True)
 if args.samples<3:parser.error('at least three samples required')
 overrides={'JULIA_NUM_THREADS':str(args.threads),'OMP_NUM_THREADS':str(args.threads),'OMP_DYNAMIC':'FALSE','CHPL_RT_NUM_THREADS_PER_LOCALE':str(args.threads),'GOMAXPROCS':str(args.threads),'RAYON_NUM_THREADS':str(args.threads),'OPENBLAS_NUM_THREADS':'1'};environment=os.environ|overrides
 pins=json.loads((HERE/'input-pins.json').read_text())
 for path,pin in pins.items():
  if not (HERE/path).exists():
   import urllib.request
   (HERE/path).write_bytes(urllib.request.urlopen(pin['url']).read())
  if sha(HERE/path)!=pin['sha256']:raise ValueError('pinned input/dependency changed: '+path)
 programs=json.loads((HERE/'manifest.json').read_text())['programs']
 programs=[c for p in programs for c in ([{**p,'upstream_id':p['id'],'id':f"{p['id']}-{cc}-{tune}",'cc_backend':cc,'cc_tune':tune} for cc in ('gcc','llvm') for tune in ('native','fastmath')] if p['language'] in ('c','cpp') else [p])]
 programs += [{**p,'upstream_id':p['id'],'id':p['id']+'-llvm-fastmath-nolto','cc_backend':'llvm','cc_tune':'fastmath-nolto'} for p in json.loads((HERE/'manifest.json').read_text())['programs'] if p['language']=='cpp' and p['variant'] in (1,3)]
 programs += [{'id':f'mech-{name}-batch{batch}','language':'mech','path':path,'batch':batch,'upstream_starred':False} for name,path,batch in [('compact','knucleotide-compact.mec',4096),('parallel','knucleotide.mec',4096),('parallel','knucleotide.mec',1),('parallel','knucleotide.mec',256)]]
 programs += [{**p,'id':p['id']+'-panicabort','panic_abort':True} for p in json.loads((HERE/'manifest.json').read_text())['programs'] if p['language']=='rust' and p['variant']==6]
 programs += [{'id':f'mech-{name}-{schedule}-pack{int(pack8)}-batch{batch}','language':'mech','path':path,'batch':batch,'schedule':schedule,'pack8':pack8,'upstream_starred':False} for name,path in [('compact','knucleotide-compact.mec'),('parallel','knucleotide.mec')] for schedule,pack8,batch in [('dag',False,256),('dag',True,256),('dag',True,4096),('source',True,256)]]
 programs += [{'id':f'mech-{name}-runtime-{tag}-batch{batch}','language':'mech','path':path,'batch':batch,'schedule':'dag','pack8':True,'fuse_encode':fused,'narrow':narrow,'partitions':partitions,'upstream_starred':False} for name,path in [('compact','knucleotide-compact.mec'),('parallel','knucleotide.mec')] for tag,narrow,partitions,fused,batch in [('control',False,1,False,4096),('narrow',True,1,False,4096),('partition8',False,8,False,4096),('narrow-partition8',True,8,False,4096),('narrow-fused',True,1,True,4096),('narrow-partition2-fused',True,2,True,4096),('narrow-partition4-fused',True,4,True,4096),('narrow-partition8-fused',True,8,True,4096),('narrow-partition2-fused',True,2,True,256)]]
 programs += [{'id':f'mech-{name}-final-std{int(standard)}-simd{int(simd)}-p{partitions}','language':'mech','path':path,'batch':4096,'schedule':'dag','pack8':True,'fuse_encode':True,'narrow':True,'partitions':partitions,'simd':simd,'standard_map':standard,'upstream_starred':False} for name,path in [('compact','knucleotide-compact.mec'),('parallel','knucleotide.mec')] for standard in (False,True) for simd in (False,True) for partitions in (1,4)]
 if args.only:programs=[p for p in programs if p['id'] in set(args.only.split(','))]
 rows=[];excluded=[]
 if not args.reuse_builds:
  if any(p['language']=='rust' for p in programs):command(['cargo','build','--release','--locked','--manifest-path',HERE/'rust-deps/Cargo.toml'],env=environment|{'CARGO_TARGET_DIR':str(BUILD/'rust-deps')})
  for p in programs:
   print('Prepare '+p['id'],flush=True)
   try:rows.append(build(p,args,environment))
   except (ValueError,OSError,StopIteration,subprocess.SubprocessError) as e:
    detail=e.stderr or e.stdout if isinstance(e,subprocess.CalledProcessError) else str(e)
    if isinstance(detail,bytes):detail=detail.decode(errors='replace')
    excluded.append({'id':p['id'],'path':p['path'],'reason':str(e),'diagnostic':detail});print('Exclude '+p['id']+': '+str(detail)[-500:],flush=True)
  prepared={'programs':[{k:v for k,v in r.items() if k!='_env'} for r in rows],'excluded':excluded}
  if args.only and (BUILD/'prepared.json').exists():
   old=json.loads((BUILD/'prepared.json').read_text());selected={p['id'] for p in programs}
   prepared['programs']=[r for r in old['programs'] if r['id'] not in selected]+prepared['programs']
   prepared['excluded']=[r for r in old['excluded'] if r['id'] not in selected]+prepared['excluded']
  (BUILD/'prepared.json').write_text(json.dumps(prepared,indent=2)+'\n')
  if args.prepare_only or args.build_only:return
 else:
  prepared=json.loads((BUILD/'prepared.json').read_text());rows=prepared['programs'];excluded=prepared['excluded']
  if args.only:rows=[r for r in rows if r['id'] in set(args.only.split(','))]
  for row in rows:
   row['_env']=environment.copy();row['worker_policy']=workers(row,args.threads)
   if row['language']=='mech' and not row.get('public_build'):
    row['compile_command']=['cargo','build','--release','--locked','--manifest-path',str(HERE/'mech-driver/Cargo.toml')]
    row['build_environment']={'CARGO_TARGET_DIR':str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'MECH_SEQUENCE_SOURCE':str(HERE/row['path']),'MECH_SEQUENCE_BATCH':str(row['batch']),'MECH_SEQUENCE_SCHEDULE':row.get('schedule','source'),'MECH_SEQUENCE_PACK8':str(int(row.get('pack8',False))),'MECH_SEQUENCE_FUSE_ENCODE':str(int(row.get('fuse_encode',False))),'MECH_SEQUENCE_SIMD':str(int(row.get('simd',False))),'RUSTFLAGS':'-D warnings -C target-cpu=native'}
 expected_sha=json.loads((HERE/'input25000000.json').read_text())['sha256']
 if sha(FULL)!=expected_sha:raise ValueError('full FASTA input differs from pin')
 for row in rows:
  if row['language']=='mech':
   row['runtime_environment']={'MECH_SEQUENCE_PARTITIONS':str(row.get('partitions',1)),'MECH_SEQUENCE_NARROW':str(int(row.get('narrow',False))),'MECH_SEQUENCE_PROFILE':'0','MECH_SEQUENCE_STD_MAP':str(int(row.get('standard_map',False)))}
   row['_env'].update(row['runtime_environment'])
  if sha(HERE/row['path'])!=row['source']['sha256']:raise ValueError('source changed after preparing: '+row['id'])
  binary=BUILD/row['id']
  row['executable_sha256']=sha(binary) if binary.is_file() else {str(p.relative_to(binary)):sha(p) for p in binary.rglob('*.class')}
 reference_row=next(r for r in rows if r['id']=='knucleotide-rust-7');_,reference,_=execute(reference_row['command'],environment,FULL,args.timeout)
 published=next(p for p in programs if p['id']=='knucleotide-rust-7')['upstream_notes'].split('PROGRAM OUTPUT:\n',1)[1].encode()+b'\n'
 if reference!=published:raise ValueError('full output differs from published reference')
 (HERE/'inputs/knucleotide-output-25000000.txt').write_bytes(reference)
 good=[]
 for row in rows:
  print('Warmup '+row['id'],flush=True)
  try:
   elapsed,data,stderr=execute(row['command'],row['_env'],FULL,args.timeout)
   if data!=reference:raise ValueError('full output differs: '+repr(data[:300]))
   row['warmup_output']=info(data);row['warmup_stderr']=stderr;good.append(row);print(f'Full output matches: {elapsed:.3f}s',flush=True)
  except (ValueError,subprocess.SubprocessError) as e:
   excluded.append({'id':row['id'],'path':row['path'],'reason':str(e),'diagnostic':str(getattr(e,'stderr',''))});print('Exclude '+row['id']+': '+str(e)[-300:],flush=True)
 rows=good;rng=random.Random(args.seed);orders=[]
 for sample in range(args.samples):
  order=rows.copy();rng.shuffle(order);orders.append([r['id'] for r in order])
  for row in order:
   elapsed,data,_=execute(row['command'],row['_env'],FULL,args.timeout)
   if data!=reference:raise ValueError('nondeterministic output: '+row['id'])
   row['samples_seconds'].append(elapsed);print(f"Sample {sample+1}/{args.samples} {row['id']} {elapsed:.3f}s",flush=True)
 for row in rows:
  row.pop('_env');row.update(median_seconds=statistics.median(row['samples_seconds']),min_seconds=min(row['samples_seconds']),max_seconds=max(row['samples_seconds']))
 versions={}
 for name,argv in {'rust':['rustc','-vV'],'chapel':['chpl','--version'],'julia':[args.julia,'--version'],'go':['go','version'],'java':[str(Path(args.java_home)/'bin/java'),'-version'],'gcc':[args.gcc,'--version'],'gpp':[args.gpp,'--version'],'llvm':[args.clang,'--version']}.items():
  r=command(argv);versions[name]=(r.stdout+r.stderr).strip()
 report={'schema_version':1,'created_at':datetime.now(timezone.utc).isoformat(),'hardware':{'platform':platform.platform(),'machine':platform.machine(),'logical_cpus':os.cpu_count(),'cpu':command(['sysctl','-n','machdep.cpu.brand_string']).stdout.strip()},'toolchain_versions':versions,'compiler_snapshot':snapshot(args.output),'protocol':{'fasta_n':25000000,'three_letters':125000000,'total_hash_updates_per_program':874999961,'input':json.loads((HERE/'input25000000.json').read_text()),'samples':args.samples,'seed':args.seed,'sample_order':orders,'requested_threads':args.threads,'environment':overrides,'timed_region':'Fresh process to exit including reading the same regular stdin file, encoding, every hash lookup/update, output and JIT initialization; AOT builds/input generation/validation excluded.','source_scope':'Complete Mech source including source-defined frequency and query functions; reusable sequence, map, sorting, formatting and IO library primitives excluded. Compiler/frontend extension is experimental and not yet wired into the normal v0.4 resident ProgramArtifact/catalog. Competitors: complete unchanged upstream programs.','source_metric':'Comment removal, whitespace collapse, UTF-8 gzip level1 mtime0; nonblank noncomment LOC also reported.','required_work':'All overlapping windows for k=1,2,3,4,6,12,18 counted using growing library hash maps; no periodicity shortcut or query-only counting.','correctness':'Every candidate must match unmodified Rust #7 full output byte-for-byte, checked against published full output. Official N=1000 failures are retained and flagged; strict endpoints additionally require passing the small reference.'},'programs':rows,'excluded':excluded,'endpoints':common.endpoint_summary(rows),'nonheroic_endpoints':common.endpoint_summary([r for r in rows if not r['upstream_starred']]),'strict_endpoints':common.endpoint_summary([r for r in rows if r['correctness_1000'].get('exact_reference_match')])}
 for row in report['programs']:
  if row['language']!='mech':continue
  origins=[]
  for old_path in sorted((HERE/'results').glob('apple-m3-*.json')):
   if old_path.resolve()==args.output.resolve():continue
   prior=json.loads(old_path.read_text())
   if 'compiler_snapshot' not in prior:continue
   if any(r.get('executable_sha256')==row['executable_sha256'] for r in prior.get('programs',[]) if r['language']=='mech'):
    origins.append((prior['created_at'],old_path,prior['compiler_snapshot']))
  if origins:
   _,origin,compiler=min(origins,key=lambda x:x[0]);row['build_provenance']={'report':origin.name,'compiler_snapshot':compiler}
  else:row['build_provenance']={'report':args.output.name,'compiler_snapshot':report['compiler_snapshot']}
 args.output.write_text(json.dumps(report,indent=2)+'\n');print(f'Saved {args.output}: {len(rows)} configurations, {len(excluded)} exclusions',flush=True)
if __name__=='__main__':main()
