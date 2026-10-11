#!/usr/bin/env python3
"""Build pinned reverse-complement sources, validate, then time fresh processes."""
import argparse,gzip,hashlib,importlib.util,io,json,os,platform,random,re,shutil,statistics,subprocess,tarfile,tempfile,time
from datetime import datetime,timezone
from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];BUILD=HERE/'.build'
spec=importlib.util.spec_from_file_location('common',HERE.parent/'nbody/run.py');common=importlib.util.module_from_spec(spec);spec.loader.exec_module(common)
spec=importlib.util.spec_from_file_location('clbg_process',HERE.parent/'process.py');process=importlib.util.module_from_spec(spec);spec.loader.exec_module(process)
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def command(argv,**kw):return common.command([str(x) for x in argv],**kw)
def metrics(path,language):
 source=(HERE/path).read_text();clean=common.strip_comments(source,language);normalized=re.sub(r'\s+',' ',clean).strip().encode()
 return dict(paths=[path],sha256=sha(HERE/path),physical_lines=len(source.splitlines()),nonblank_noncomment_lines=sum(bool(x.strip()) for x in clean.splitlines()),normalized_bytes=len(normalized),gzip1_bytes=len(gzip.compress(normalized,compresslevel=1,mtime=0)))
def execute(argv,env,input_path,timeout):
 with Path(input_path).open('rb') as stdin,tempfile.TemporaryFile() as stdout,tempfile.TemporaryFile() as stderr:
  start=time.perf_counter_ns();process.run_checked(argv,env=env,cwd=ROOT,stdin=stdin,stdout=stdout,stderr=stderr,timeout=timeout);elapsed=(time.perf_counter_ns()-start)/1e9
  stdout.seek(0);digest=hashlib.file_digest(stdout,'sha256').hexdigest();size=os.fstat(stdout.fileno()).st_size;stderr.seek(0)
  return elapsed,digest,size,stderr.read().decode(errors='replace')
def references():
 small=HERE/'inputs/revcomp-input-1000.txt';full=HERE.parent/'knucleotide/.build/knucleotide-input25000000.txt'
 pins=json.loads((HERE/'input-pins.json').read_text())
 for path,pin in pins.items():
  if sha(HERE/path)!=pin['sha256']:raise ValueError('input pin changed: '+path)
 original=json.loads((HERE.parent/'knucleotide/input25000000.json').read_text())
 if sha(full)!=original['sha256']:raise ValueError('full input pin differs')
 ref=BUILD/'reference25000000.txt'
 if not ref.exists():
  alphabet=b'ACGTUMRWSYKVHDBN';complements=dict(zip(alphabet,b'TGCAAKYWSRMBDHVN'));complements.update({k+32:v for k,v in list(complements.items())})
  lookup=bytes(complements.get(i,i) for i in range(256))
  def write(writer,header,body):
   writer.write(header+b'\n');data=body.translate(lookup)[::-1]
   for i in range(0,len(data),60):writer.write(data[i:i+60]+b'\n')
  with full.open('rb') as reader,ref.open('wb') as writer:
   header=None;body=bytearray()
   for line in reader:
    line=line.rstrip(b'\r\n')
    if line.startswith(b'>'):
     if header is not None:write(writer,header,body)
     header=line;body=bytearray()
    else:body.extend(line)
   if header is not None:write(writer,header,body)
 return small,full,sha(HERE/'inputs/revcomp-output-1000.txt'),sha(ref),original

def snapshot(output):
 paths=list((ROOT/'hosts/gpu/src/sequence').glob('*.rs'))+[ROOT/'hosts/gpu/src/sequence/application.rs.in']+list((ROOT/'src/engine/src/native_sequence').glob('*.rs'))
 paths += [ROOT/p for p in ('Cargo.toml','Cargo.lock','hosts/gpu/Cargo.toml','hosts/gpu/src/native_build.rs','src/cli/commands/build.rs','src/stdlib/src/native_sequence.rs','src/stdlib/Cargo.toml','src/engine/Cargo.toml','src/engine/src/numeric_optimizations.rs','scripts/unsafe-boundaries.allowlist')]
 paths += [p for p in HERE.rglob('*') if p.is_file() and not any(x in ('.build','results','__pycache__') for x in p.relative_to(HERE).parts)]
 paths += [HERE.parent/'nbody/run.py',HERE.parent/'process.py',HERE.parent/'nbody/.build/gcc-sdk-compat.h']
 buf=io.BytesIO();hashes={}
 with tarfile.open(fileobj=buf,mode='w') as tar:
  for p in sorted(set(paths)):
   if not p.is_file():continue
   name=str(p.relative_to(ROOT));raw=p.read_bytes();hashes[name]=sha(p);item=tarfile.TarInfo(name);item.mode=0o644;item.size=len(raw);tar.addfile(item,io.BytesIO(raw))
 archive=output.with_suffix('.compiler.tar.gz');archive.write_bytes(gzip.compress(buf.getvalue(),mtime=0))
 return dict(path=archive.name,sha256=sha(archive),base_commit=command(['git','rev-parse','HEAD']).stdout.strip(),source_sha256=hashes)
def prepare(p,args,env,small,expected):
 p=p.copy();lang=p['language'];source=HERE/p['path'];binary=BUILD/p['id'];text=source.read_text();ccmd=None;env=env.copy()
 if p.get('sha256') and sha(source)!=p['sha256']:raise ValueError('source pin changed')
 if platform.machine()!='x86_64' and re.search(r'__builtin_ia32|x86intrin|[ei]mmintrin',text):raise ValueError('unmodified source requires x86 intrinsics; host '+platform.machine())
 # Rust #1 has a cfg-gated x86 path AND a portable fallback; compile it on ARM.
 if lang=='rust':
  deps=HERE.parent/'mandelbrot/.build/rust-deps/release/deps';ccmd=['rustc','--edition=2021','-O','-Ctarget-cpu=native','-L',f'dependency={deps}']
  for name in re.findall(r'extern crate (\w+)',text):
   candidates=list(deps.glob('lib'+name+'-*.rlib'))
   if not candidates:
    other=ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/deps';candidates=list(other.glob('lib'+name+'-*.rlib'));ccmd+=['-L',f'dependency={other}']
   if not candidates:raise ValueError('missing cached dependency '+name)
   ccmd+=['--extern',f'{name}={candidates[0]}']
  if p.get('panic_abort'):ccmd+=['-Cpanic=abort']
  ccmd += [source,'-o',binary];argv=[str(binary)];policy='Rayon pool' if 'rayon' in text else 'serial'
 elif lang in ('c','cpp'):
  llvm=p['cc_backend']=='llvm';cc=args.clang if llvm else args.gpp if lang=='cpp' else args.gcc
  ccmd=[cc,'-O3','-mcpu=native' if llvm else '-march=native','-fomit-frame-pointer','-funroll-loops','-flto','-std=c++17' if lang=='cpp' else '-std=gnu11',source,'-o',binary,'-lm','-pthread']+common.macos_gcc_flags(cc)
  if llvm and lang=='cpp':ccmd+=['-lc++']
  argv=[str(binary)];policy='source pthread policy' if 'pthread' in text else 'serial'
 elif lang=='chapel':
  sdk=command(['xcrun','--show-sdk-path']).stdout.strip();ccmd=['chpl','--fast',source,'-o',binary,f'--ccflags=-isysroot {sdk}',f'--ldflags=-isysroot {sdk}'];argv=[str(binary)];policy='Chapel task runtime'
 elif lang=='go':ccmd=['go','build','-o',binary,source];argv=[str(binary)];policy='source goroutines; GOMAXPROCS limit'
 elif lang=='julia':argv=[args.julia,'--startup-file=no','--history-file=no','-O3','--',str(source)];policy='Julia source worker policy'
 elif lang=='java':
  binary.mkdir(exist_ok=True);js=binary/'revcomp.java';shutil.copyfile(source,js);ccmd=[str(Path(args.java_home)/'bin/javac'),'-d',binary,js];argv=[str(Path(args.java_home)/'bin/java'),f'-XX:ActiveProcessorCount={args.threads}','-cp',str(binary),'revcomp'];policy='OpenJDK processor limit'
  p['execution_mode']='OpenJDK JIT fresh process; source was published for native AOT'
 else:
  env.update(CARGO_TARGET_DIR=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),RUSTFLAGS='-D warnings',MECH_BYTE_FUSION=str(p.get('fusion',1)),MECH_BYTE_REUSE=str(p.get('reuse',1)))
  project=Path(str(binary)+'.project')
  if project.exists():shutil.rmtree(project)
  ccmd=[args.mech,'build',source,'--backend','cranelift','--offline','--keep-project','--no-config','-o',binary]
  if not p['simd']:ccmd+=['--scalar']
  argv=[str(binary)];policy='disjoint byte-pair ranges over Rayon pool; general record streaming'
  p['mech_compiler_executable_sha256']=sha(args.mech);p['build_environment']={k:env[k] for k in ('RUSTFLAGS','MECH_BYTE_FUSION','MECH_BYTE_REUSE','CARGO_TARGET_DIR')}
  p['execution_mode']='Mech source function, reversal/lookup and mapping traversal compiled by Cranelift; generic library byte storage, line parsing and formatting/IO'
 if ccmd:command(ccmd,env=env)
 if lang=='mech':p['native_object_sha256']=sha(project/'mech-sequence.o');p['constants_sha256']=sha(project/'constants.json')
 if lang not in ('julia','java'):p['executable_sha256']=sha(binary)
 p.update(source=metrics(p['path'],lang),compile_command=[str(x) for x in ccmd] if ccmd else None,command=argv,worker_policy=policy,samples_seconds=[],runtime_environment={'MECH_SEQUENCE_PARTITIONS':str(p.get('partitions',4))} if lang=='mech' else {})
 _,digest,size,_=execute(argv,env|p['runtime_environment'],small,args.timeout)
 if digest!=expected:raise ValueError('official N=1000 output differs: '+digest)
 p['correctness_1000']=dict(exact_reference_match=True,sha256=digest,bytes=size)
 return p

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--samples',type=int,default=5);ap.add_argument('--threads',type=int,default=8);ap.add_argument('--timeout',type=int,default=180);ap.add_argument('--only');ap.add_argument('--skip');ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--reuse-builds',action='store_true');ap.add_argument('--mech',default=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'));ap.add_argument('--julia',default='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia');ap.add_argument('--java-home',default='/opt/homebrew/opt/openjdk');ap.add_argument('--clang',default='/opt/homebrew/opt/llvm@22/bin/clang-22');ap.add_argument('--gcc',default=shutil.which('gcc-16'));ap.add_argument('--gpp',default=shutil.which('g++-16'));ap.add_argument('--output',type=Path,default=HERE/'results/apple-m3-25000000.json');args=ap.parse_args();BUILD.mkdir(exist_ok=True)
 if args.samples<3:ap.error('at least 3 samples')
 small,full,small_sha,full_sha,input_pin=references()
 env=os.environ|{k:str(args.threads) for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')}|{'OPENBLAS_NUM_THREADS':'1','OMP_DYNAMIC':'FALSE'}
 programs=[]
 for p in json.loads((HERE/'manifest.json').read_text())['programs']:
  if p['language'] in ('c','cpp'):
   for backend in ('gcc','llvm'):programs.append(p|dict(id=p['id']+'-'+backend,cc_backend=backend))
  elif p['language']=='rust':
   for abort in (False,True):programs.append(p|dict(id=p['id']+('-panic-abort' if abort else ''),panic_abort=abort))
  else:programs.append(p)
 for source in ('revcomp.mec','revcomp-buffered.mec'):
  for name,simd,fusion,reuse,partitions in [('optimized',True,1,1,4),('scalar',False,1,1,4),('unfused',True,0,1,4),('copy',True,1,0,4),('serial',True,1,1,1)]:
   if source.endswith('buffered.mec') and name!='optimized':continue
   programs.append(dict(id='mech-'+source[:-4]+'-'+name,language='mech',path=source,upstream_starred=False,simd=simd,fusion=fusion,reuse=reuse,partitions=partitions))
 if args.only:programs=[p for p in programs if re.search(args.only,p['id'])]
 if args.skip:programs=[p for p in programs if not re.search(args.skip,p['id'])]
 if not args.reuse_builds:
  rows=[];excluded=[]
  for p in programs:
   print('Prepare '+p['id'],flush=True)
   try:rows.append(prepare(p,args,env,small,small_sha))
   except Exception as error:excluded.append(dict(id=p['id'],language=p['language'],reason=str(error)));print('Exclude '+p['id']+': '+str(error)[-600:],flush=True)
  saved=dict(programs=rows,excluded=excluded)
  if args.only and (BUILD/'prepared.json').exists():
   old=json.loads((BUILD/'prepared.json').read_text());ids={p['id'] for p in programs};saved['programs']=[r for r in old['programs'] if r['id'] not in ids]+rows;saved['excluded']=[r for r in old['excluded'] if r['id'] not in ids]+excluded
  (BUILD/'prepared.json').write_text(json.dumps(saved,indent=2)+'\n')
  if args.prepare_only:return
 else:
  saved=json.loads((BUILD/'prepared.json').read_text());ids={p['id'] for p in programs};rows=[r for r in saved['programs'] if r['id'] in ids];excluded=[r for r in saved['excluded'] if r['id'] in ids]
 eligible=[]
 for r in rows:
  try:
   _,digest,size,_=execute(r['command'],env|r['runtime_environment'],full,args.timeout)
   if digest!=full_sha:raise ValueError('full reference differs: '+digest)
   r['correctness_full']=dict(sha256=digest,bytes=size,independent_reference_match=True);r['samples_seconds']=[];eligible.append(r)
  except Exception as error:
   if r['language']=='mech':raise
   excluded.append(dict(id=r['id'],language=r['language'],reason=str(error)))
 rows=eligible
 orders=[];rng=random.Random(20261007)
 for sample in range(args.samples):
  order=rows.copy();rng.shuffle(order);orders.append([r['id'] for r in order])
  for r in order:
   elapsed,digest,_,_=execute(r['command'],env|r['runtime_environment'],full,args.timeout)
   if digest!=full_sha:raise ValueError('timed output differs: '+r['id'])
   r['samples_seconds'].append(elapsed);print(f"{sample+1}/{args.samples} {r['id']}: {elapsed:.6f}s",flush=True)
 for r in rows:r['median_seconds']=statistics.median(r['samples_seconds']);r['min_seconds']=min(r['samples_seconds']);r['max_seconds']=max(r['samples_seconds'])
 report=dict(schema_version=1,created_at=datetime.now(timezone.utc).isoformat(),hardware=dict(platform=platform.platform(),machine=platform.machine(),cpu=command(['sysctl','-n','machdep.cpu.brand_string']).stdout.strip(),logical_cpus=os.cpu_count()),compiler_snapshot=snapshot(args.output),protocol=dict(fasta_n=25000000,input=input_pin,samples=args.samples,requested_threads=args.threads,sample_order=orders,environment={k:env[k] for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE','GOMAXPROCS','JULIA_NUM_THREADS')},selection_skip_regex=args.skip,timing_wait='Blocking waitpid with POSIX alarm deadline; avoids Python timeout-poll quantization.',timed_region='Sequential fresh process to exit; same regular stdin file; all output to a regular file. Includes IO, runtime startup and JIT startup. Builds and hashing excluded.',required_work='Reverse and complement every base in all three sequences; preserve headers and output 60-column FASTA. Growing buffered input, no file-size preallocation.',source_scope='Complete counted Mech transform function, complement table, record mapping and output; reusable byte, record and IO library services excluded.',source_metric='Nonblank noncomment LOC and whitespace-normalized gzip level1 bytes.',cohort='Pinned submissions built/validated in the preserved full-cohort report; this measured selection records its explicit skip regex. Architecture, build and incorrect-output exclusions recorded. Java measured with OpenJDK JIT, not published native AOT.'),programs=rows,excluded=excluded,endpoints=common.endpoint_summary(rows),nonheroic_endpoints=common.endpoint_summary([r for r in rows if not r['upstream_starred']]))
 args.output.write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
