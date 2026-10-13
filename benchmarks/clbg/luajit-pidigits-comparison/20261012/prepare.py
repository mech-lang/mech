"""One isolated locked build/qualification; no scored samples or data admission."""
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
import re
import shutil
import subprocess
import tempfile
import sys
from datetime import datetime,timezone
from pathlib import Path

HERE=Path(__file__).resolve().parent
C=HERE.parents[1];W=C.parents[1]
N=C/'r-apl-uiua-comparison/20261011'
LUA_STUDY=C/'lua-pidigits-comparison/20261012'
SOURCES=LUA_STUDY/'sources'
sys.path.insert(0,str(LUA_STUDY))
from prepare_sources import normalize
LUA=Path('/tmp/mech-clbg-luajit/src/luajit')
CLANG=Path('/opt/homebrew/opt/llvm@22/bin/clang-22')
HEADERS=Path('/tmp/mech-clbg-luajit/src')
GMP=Path('/opt/homebrew/opt/gmp')
LOCK=Path('/tmp/mech-clbg-timing.lock')
RUNTIME_SHA='ffe74a185b665faae02da56c194493c82d921afae03f3421b3d0a270886dd4c3'
LUA_STUDY_SHA='aaf6af5fad6bb4d2f9008e8f91be7037025e117f2cb9532dfb14ae928f6f55dd'
MANIFEST=HERE/'full-study/manifest.json'
EXPECTED_UPSTREAM={1:'17afbb71f557f5fa3fdc4d0274b8377d7717e467017d4d322897327d12a0c125',5:'c202b92ca5e8b0c9c6b2e38dec0fcb6f150b9641d2c8620b0de125b5fb983f15',6:'3ea1f580839006463723faa69092df13c41fc76a25d62bccc5e35075edb0b692',7:'e2e6c1d50d2c399d58cc231b02513f7e080cab776b486c7c61a084ffacd34b43'}
def now():return datetime.now(timezone.utc).isoformat()
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(path,value):
 path.parent.mkdir(parents=True,exist_ok=True);tmp=path.with_suffix(path.suffix+'.tmp');tmp.write_text(json.dumps(value,indent=2)+'\n');tmp.replace(path)
def run(command,env=None):return subprocess.run(list(map(str,command)),cwd=HERE,env=env,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=180,check=True)
def output(command,env,expected):
 result=run(command,env);assert result.stderr==b'',(command,result.stderr[:500]);assert result.stdout==expected,(command,len(result.stdout),hashlib.sha256(result.stdout).hexdigest(),len(expected));return dict(bytes=len(result.stdout),sha256=hashlib.sha256(result.stdout).hexdigest(),reference_match=True)
def physical_loc(text):return sum(bool(line.strip()) and not line.lstrip().startswith('--') for line in text.splitlines())
def expansions(text):
 rows=[]
 for line,raw in enumerate(text.splitlines(),1):
  if not raw.strip() or raw.lstrip().startswith('--'):continue
  expanded=normalize(raw+'\n').splitlines()
  if len(expanded)>1:rows.append(dict(line=line,original=raw,expanded=expanded,independent_source_lines=len(expanded)))
 return rows

def main():
 print('WAIT common timing mutex for separate LuaJIT GMP qualification',flush=True)
 with LOCK.open('a+') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX);print('LOCK',now(),flush=True)
  assert not MANIFEST.exists(),'Sealed study inputs are immutable; no overwrite'
  assert sha(LUA)==RUNTIME_SHA,'Recorded LuaJIT runtime changed'
  assert sha(LUA_STUDY/'full-study/manifest.json')==LUA_STUDY_SHA,'The separate reviewed interpreter preparation changed'
  assert all((SOURCES/f'pidigits-lua-{v}-normalized.lua').read_text()==normalize((C/'pidigits/upstream'/f'pidigits-lua-{v}.lua').read_text()) for v in EXPECTED_UPSTREAM)
  upstream=json.loads((C/'pidigits/manifest.json').read_text())
  old=json.loads((C/'pidigits/results/apple-m3-ac-normal-20261009.json').read_text())
  current=json.loads((C/'overview/explorer/dist/data.json').read_text())
  old_paths=[C/'pidigits/.build/lua/bn.so',C/'pidigits/.build/luajit/bn.so',C/'pidigits/.build/lua-library-pins.json',C/'pidigits/results/apple-m3-ac-normal-20261009.json',C/'pidigits/manifest.json']
  preserved={str(p):sha(p) for p in old_paths}
  for v,digest in EXPECTED_UPSTREAM.items():
   path=C/'pidigits/upstream'/f'pidigits-lua-{v}.lua';assert sha(path)==digest
   recorded=next(p for p in upstream['programs'] if p['id']==f'pidigits-lua-{v}');assert recorded['sha256']==digest
  build=HERE/'build';build.mkdir(exist_ok=True)
  compiler_version=run([CLANG,'--version']).stdout.decode().splitlines()[0]
  version_result=run([LUA,'-v'])
  runtime_version=(version_result.stdout+version_result.stderr).decode().strip()
  command=[CLANG,'-O3','-Wall','-Wextra','-Werror','-bundle','-undefined','dynamic_lookup','-isysroot','/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk',f'-I{HEADERS}',f'-I{GMP}/include',HERE/'support/gmp-lua51.c',f'-L{GMP}/lib','-lgmp','-o',build/'general-gmp.so']
  result=run(command);assert not result.stderr,result.stderr
  for name in ('c-gmp.so','bn.so'):shutil.copy2(build/'general-gmp.so',build/name)
  runtime_environment={'LUA_CPATH':str(build/'?.so')+';;'}
  env={k:os.environ[k] for k in ('PATH','HOME','TMPDIR','LANG','LC_ALL') if k in os.environ}|runtime_environment
  tests=run([LUA,HERE/'support/test-gmp.lua'],env)
  assert tests.stdout==b'general mpz tests passed\n' and not tests.stderr
  reference=C/'pidigits/inputs/output-full.txt';full=reference.read_bytes()
  chunks=[line.split(b'\t:')[0].strip() for line in full.splitlines()];digits=b''.join(chunks)
  assert len(digits)==10000 and digits.isdigit()
  def expected(n):
   n=max(n,0);result=[]
   for offset in range(0,n,10):
    count=min(n,offset+10);chunk=digits[offset:count];result.append(chunk+b' '*(10-len(chunk))+b'\t:'+str(count).encode()+b'\n')
   return b''.join(result)
  small_cases=list(range(0,22))+[27,30,97,100,1000,-1]
  validations=[];division_checks=[];profiles=[]
  for variant in EXPECTED_UPSTREAM:
   original=C/'pidigits/upstream'/f'pidigits-lua-{variant}.lua'
   source=SOURCES/f'pidigits-lua-{variant}-normalized.lua'
   for input_value in small_cases:
    reference_bytes=expected(input_value)
    for kind,path in (('original',original),('normalized',source)):
     checked=output([LUA,path,str(input_value)],env,reference_bytes)
     validations.append(dict(variant=variant,kind=kind,n=input_value,command=[str(LUA),str(path),str(input_value)],**checked))
   for kind,path in (('original',original),('normalized',source)):
    for argv in ([LUA,path],[LUA,path,'invalid']):
     checked=output(argv,env,expected(27));validations.append(dict(variant=variant,kind=kind,n=27,cli_case='omitted' if len(argv)==2 else 'invalid-default',command=list(map(str,argv)),**checked))
    checked=output([LUA,path,'10000'],env,full);validations.append(dict(variant=variant,kind=kind,n=10000,full_output_validation=True,command=[str(LUA),str(path),'10000'],**checked))
   trace_path=HERE/'results'/f'divisions-{variant}-n97.tsv'
   output([LUA,LUA_STUDY/'support/trace-divisions.lua',source,'97',trace_path],env,expected(97))
   rows=[line.split('\t') for line in trace_path.read_text().splitlines()]
   assert rows and len(rows)%2==0
   for method,num,den,quotient in rows:
    num,den,quotient=int(num),int(den),int(quotient);assert den>0 and num>=0 and quotient==num//den
   pairs=list(zip(rows[::2],rows[1::2]));assert all(first[2]==second[2] and int(first[1])!=int(second[1]) for first,second in pairs)
   assert sum(first[3]==second[3] for first,second in pairs)==97
   division_checks.append(dict(variant=variant,path=str(trace_path),sha256=sha(trace_path),n=97,actual_divisions=len(rows),tested_candidates=len(pairs),accepted_digits=97,both_exact_divisions_per_candidate=True,quotients_independently_verified=True))
   original_text=original.read_text();source_text=source.read_text();recorded=next(p for p in upstream['programs'] if p['id']==f'pidigits-lua-{variant}')
   loc=physical_loc(source_text)
   row=dict(id=f'luajit-listed-{variant}',key=f'pidigits:luajit-gmp-listed-{variant}',benchmark='pidigits',language='luajit',role='faithful listed source using general GMP compatibility library',
    source=str(source),source_sha256=sha(source),upstream_source=str(original),upstream_sha256=sha(original),upstream_id=recorded['id'],source_url=recorded['url'],
    command=[str(LUA),str(source),'{n}'],cwd=str(W),runtime_environment=runtime_environment,runtime=dict(path=str(LUA),sha256=RUNTIME_SHA,version=runtime_version),
    loc=loc,physical_loc=loc,original_physical_loc=physical_loc(original_text),loc_expansions=expansions(original_text),loc_metric='statement-normalized',
    loc_rule='Nonblank/noncomment Lua application source lines. Independent bindings, multiple assignments, semicolon packing and inline if bodies are physically expanded; standalone Lua end lines use existing Lua counting treatment. General arbitrary-precision library binding excluded uniformly.',
    adaptations=['Formatting only: physically expand independent source statements and bindings, preserving their original evaluation order.','Supply ordinary GMP mpz arithmetic through a new private compatibility library; no host benchmark algorithm.'],
    small_validation=next(v for v in validations if v['variant']==variant and v['kind']=='normalized' and v.get('n')==30 and not v.get('cli_case')),
    full_validation=next(v for v in validations if v['variant']==variant and v['kind']=='normalized' and v.get('full_output_validation')),
    algorithm=dict(strict=True,native_benchmark_kernel=False,reason='Complete published sequential LFT spigot; both actual exact endpoint divisions for every tested candidate. #6 changes to equivalent endpoint-numerator basis; #1 retains the permitted impossible-candidate guard. No digit cache or alternative pi algorithm.',division_check=division_checks[-1]),
    maximum_workers=1)
   profiles.append(row)
  builds_path=C/'function-application-audit/20261009T171437Z/application-v3/builds.json';builds=json.loads(builds_path.read_text())
  group=next(g for b in current['benchmarks'] if b['key']=='pidigits' for g in b['groups'] if g['language']=='mech')
  for role,loc in (('fast',38),('small',35)):
   key=group[role];endpoint=current['programs'][key];built=next(r['builds']['after'] for r in builds['programs'] if r['key']==key)
   assert endpoint['loc']==built['loc']==loc and built['source_sha256'] in endpoint['files']
   assert sha(built['source'])==built['source_sha256'] and sha(built['binary'])==built['binary_sha256']
   row=dict(id='mech-'+role+'-control',key='pidigits:lua-study:mech-'+role+'-control',benchmark='pidigits',language='mech',role='unchanged current Mech '+role+' AOT endpoint, freshly measured control',source=built['source'],source_sha256=built['source_sha256'],binary=built['binary'],binary_sha256=built['binary_sha256'],command=[built['binary'],'{n}'],cwd=str(W),runtime_environment={},loc=loc,published_chart_key=key,maximum_workers=1,algorithm=dict(strict=True,native_benchmark_kernel=False))
   for n in (0,1,9,10,11,27,30,97):output([built['binary'],str(n)],env,expected(n))
   row['full_validation']=output([built['binary'],'10000'],env,full);profiles.append(row)
  validation=dict(status='passed',created_at=now(),scored_samples=0,application_output_checks=len(validations),mech_output_checks=18,generic_primitive_tests=True,output_validations=validations,division_checks=division_checks,preserved_historical_files=preserved)
  save(HERE/'results/validation.json',validation)
  rejected=[row for row in old.get('excluded',[]) if row.get('language')=='luajit']
  save(HERE/'results/historical-exclusions.json',dict(source_report=str(old_paths[3]),source_report_sha256=preserved[str(old_paths[3])],excluded_rows=rejected,preserve_original_report=True))
  support_files=[HERE/'support/gmp-lua51.c',HERE/'support/test-gmp.lua',LUA_STUDY/'support/trace-divisions.lua',build/'general-gmp.so',build/'c-gmp.so',build/'bn.so',GMP/'lib/libgmp.10.dylib',GMP/'include/gmp.h']
  support_files += [HERE/'support/lua51_compat.h',LUA_STUDY/'support/gmp.c',LUA_STUDY/'full-study/manifest.json']
  support_files += [HEADERS/name for name in ('lua.h','luaconf.h','lauxlib.h','lualib.h')]
  pins={str(path):sha(path) for path in support_files+[LUA,CLANG,Path(__file__).resolve(),LUA_STUDY/'prepare_sources.py',HERE/'full_study.py',HERE/'PLAN.md',HERE/'results/validation.json',HERE/'results/historical-exclusions.json',reference,C/'pidigits/inputs/pidigits-output-small.txt',C/'pidigits/manifest.json',builds_path,N/'run.py',N/'prepare.py',C/'compact/run.py',C/'nbody/run.py',C/'array-comparison/all-20261009/run.py',N/'uiua-mandel-parallel-v4/full_study.py']}
  for row in profiles:
   pins[row['source']]=row['source_sha256']
   if row.get('binary'):pins[row['binary']]=row['binary_sha256']
   if row.get('upstream_source'):pins[row['upstream_source']]=row['upstream_sha256']
  pins.update({str(p):sha(p) for p in (HERE/'results').glob('divisions-*-n97.tsv')})
  assert preserved=={path:sha(path) for path in preserved},'Historical source or module changed'
  profile_ids=[row['id'] for row in profiles]
  manifest=dict(status='prepared-awaiting-root-review',created_at=now(),sample_count=3,samples=3,size=10000,profiles=profiles,orders=[dict(round=i+1,profiles=profile_ids[i:]+profile_ids[:i]) for i in range(3)],pins=pins,
   shared_timing_lock=str(LOCK),required_power_profile=dict(source='AC Power',low_power_mode=0),maximum_rss_mib=2048,timeout_seconds=300,environment_retry_limit=3,maximum_compute_workers=1,
   validation=str(HERE/'results/validation.json'),validation_sha256=sha(HERE/'results/validation.json'),full_output_reference=dict(bytes=len(full),sha256=sha(reference)),
   library=dict(source=str(HERE/'support/gmp-lua51.c'),source_sha256=sha(HERE/'support/gmp-lua51.c'),shared_implementation=str(LUA_STUDY/'support/gmp.c'),shared_implementation_sha256=sha(LUA_STUDY/'support/gmp.c'),abi_compatibility_header=str(HERE/'support/lua51_compat.h'),abi_compatibility_sha256=sha(HERE/'support/lua51_compat.h'),numeric_policy='Lua 5.1 doubles; finite integral in-range scalar arguments; arbitrary constructors retain GMP set_d semantics; numeric results preserve ordinary GMP truncation and overflow to infinities.',build_command=list(map(str,command)),compiler_version=compiler_version,modules={str(p):sha(p) for p in support_files if p.suffix in ('.so','.dylib')},scope='Ordinary standard arbitrary-precision arithmetic only; all spigot/extraction/control/output work remains Lua application source.',counted_in_application_loc=False),
   timing_boundary='Fresh LuaJIT/Mech process startup, full sequential N10000 spigot work, both true quotient extracts per tested candidate, complete stdout regular-file write, process shutdown and common supervisor notification included. Compilation, prequalification, validation and hashing excluded. No previous samples reused.',
   ownership='Frozen common execute, separate sampler and native child retain inherited common mutex descriptor through complete process-group cleanup and result persistence. One owner lock; no duplicate launch or automatic resume.',
   retry_policy='Only bounded environment-only retries allowed. Output, pins, timeout, resources, signals, cleanup and sampler errors are recorded and stop.',
   chart_policy='No chart admission or scored full launch until root reviews this exact manifest. All six profiles require three actual unchanged canonical full records. Primary speed multiplier uses current fresh Mech fast38 control; fewest-LOC ratio uses current Mech35 source; the additional Mech35 speed control is explicit.',
   preparation_full_output_checks=10,full_performance_runs=0)
  assert all(sha(path)==digest for path,digest in pins.items())
  save(MANIFEST,manifest)
  print('SEALED',json.dumps(dict(manifest=str(MANIFEST),sha256=sha(MANIFEST),profiles=[dict(id=row['id'],loc=row['loc']) for row in profiles],output_checks=len(validations)+18,full_output_checks=10)),flush=True)
  print('LOCK RELEASED',now(),flush=True)
if __name__=='__main__':main()
