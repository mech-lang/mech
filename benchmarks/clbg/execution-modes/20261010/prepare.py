#!/usr/bin/env python3
"""Freeze Mech source endpoints and rebuild matching AOT controls with the shared lowerings."""
from pathlib import Path
from datetime import datetime, timezone
import hashlib, json, os, shutil, subprocess
HERE=Path(__file__).resolve().parent;CLBG=HERE.parents[1];ROOT=CLBG.parents[1]
SITE=Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site/dist/data.json')
EMIT=Path('/tmp/mech-execution-modes-target/release/mech-execution-modes-emit')
BIN=Path('/tmp/mech-execution-modes-target/release/mech-execution-modes-compiled')
TARGET=CLBG/'nbody/mech-driver/target'
CLANG='/opt/homebrew/opt/llvm@22/bin/clang-22'
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,v):
 t=p.with_suffix('.json.tmp');t.write_text(json.dumps(v,indent=2)+'\n');t.replace(p)
data=json.loads(SITE.read_text())
base={r['key']:r['builds']['after'] for r in json.loads((CLBG/'function-application-audit/20261009T171437Z/application-v3/builds.json').read_text())['programs']}
controls={r['key']:r for r in json.loads((CLBG/'function-application-audit/20261009T171437Z/baseline.json').read_text())['programs']}
fixed=json.loads((CLBG/'work-audit/20261009/builds.json').read_text())['programs']
report= json.loads((HERE/'builds.json').read_text()) if (HERE/'builds.json').exists() else dict(created_at=datetime.now(timezone.utc).isoformat(),status='building',runner=str(BIN),runner_sha256=sha(BIN),emitter=str(EMIT),emitter_sha256=sha(EMIT),programs={})
assert report['runner_sha256']==sha(BIN) and report['emitter_sha256']==sha(EMIT)
for b in data['benchmarks']:
 bench=b['key'];g=next(g for g in b['groups'] if g['language']=='mech')
 for key in g['programs']:
  p=data['programs'][key];source=CLBG/data['files'][p['files'][0]]['path'];assert sha(source)==data['files'][p['files'][0]]['sha256']
  before=base[key];preferred=next((r for r in fixed.values() if r['key'].startswith(key+':') and r['variant'] in ('floating','four-functions')),None)
  if preferred:before=preferred
  build_env=os.environ|before['build_environment']|{'C_INCLUDE_PATH':'/opt/homebrew/include','LIBRARY_PATH':'/opt/homebrew/lib'}
  if bench=='fasta':build_env['MECH_CONTROL_EXACT_FLOAT_COMPARE']='0'
  kind='numeric' if bench=='nbody' else 'grid' if bench=='mandelbrot' else 'sequence' if bench in ('knucleotide','spectralnorm','revcomp','regexredux') else 'control'
  flags=(['--fp-contract','--algebraic-powers']+(['--pack-state-tails'] if key==g['fast'] else [])) if kind=='numeric' else ['--fp-contract-loop'] if kind=='grid' and key==g['fast'] else ['--scalar'] if kind=='grid' else []
  directory=HERE/'builds'/key.replace(':','-');directory.mkdir(parents=True,exist_ok=True)
  output=directory/'aot'
  if key in report['programs']:
   old=report['programs'][key];assert sha(source)==old['source_sha256'] and sha(output)==old['binary_sha256'];continue
  env=build_env|{'CARGO_TARGET_DIR':str(TARGET)}
  if kind in ('sequence','control'):
   original=Path(before['binary']+'.project')
   for f in ['Cargo.toml','Cargo.lock','build.rs','src/main.rs']:
    path=original/f
    if path.exists():destination=directory/f;destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,destination)
  commands=[[str(EMIT),kind,str(source),str(directory),*flags]]
  obj=directory/('mech-numeric.o' if kind=='numeric' else 'mech-grid.o' if kind=='grid' else 'mech-'+kind+'.o')
  with (directory/'build.log').open('w') as log:
   print('EMIT',key,flush=True);subprocess.run(commands[0],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
   old_objects=[v for k,v in before['artifacts'].items() if k.endswith('.o')]
   assert len(old_objects)==1,(key,old_objects)
   identical=sha(obj)==old_objects[0]
   if not identical:raise RuntimeError('AOT object changed for unchanged graph: '+key+' '+sha(obj)+' '+old_objects[0])
   print('OBJECT IDENTICAL',key,flush=True)
   if kind in ('numeric','grid'):
    commands.append([CLANG,'-O3','-isysroot','/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk',str(directory/'generated.c'),str(obj),'-lm',*(['-pthread'] if kind=='grid' else []),'-o',str(output)])
    subprocess.run(commands[-1],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
   else:
    commands.append(['cargo','build','--release','--offline','--manifest-path',str(directory/'Cargo.toml')])
    print('LINK',key,flush=True);subprocess.run(commands[-1],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    import tomllib
    name=tomllib.loads((directory/'Cargo.toml').read_text())['package']['name'];shutil.copy2(TARGET/'release'/name,output)
  runtime=controls[key].get('runtime_environment') or {}
  # Restore the build-time numeric policy used by the AOT graph for JIT lowering.
  for name,value in before['build_environment'].items():
   if name.startswith('MECH_NUMERIC_') or name=='MECH_ARRAY_GRAIN':runtime[name]=value
  if bench=='spectralnorm':runtime['MECH_SEQUENCE_PARTITIONS']='8'
  if bench=='fasta':runtime|={name:build_env[name] for name in ('MECH_CONTROL_EXACT_FLOAT_COMPARE','MECH_CONTROL_SCAN_CACHE','MECH_CONTROL_SCAN_PIPELINE')}
  artifacts={str(f.relative_to(directory)):sha(f) for f in directory.rglob('*') if f.is_file() and f.suffix in ('.o','.json','.rs','.mec','.c','.toml')}
  report['programs'][key]=dict(key=key,benchmark=bench,source=str(source),source_sha256=sha(source),loc=p['loc'],physical_loc=p.get('physical_loc',p['loc']),loc_expansions=p.get('loc_expansions',[]),kind=kind,flags=flags,runtime_environment=runtime,binary=str(output),binary_sha256=sha(output),baseline_binary=before['binary'],baseline_binary_sha256=sha(before['binary']),old_native_object_sha256=old_objects[0],native_object_sha256=sha(obj),identical_native_object=identical,commands=commands,build_environment={n:v for n,v in env.items() if n.startswith('MECH_') or n in ('RUSTFLAGS','CARGO_TARGET_DIR','C_INCLUDE_PATH','LIBRARY_PATH')},artifacts=artifacts)
  save(HERE/'builds.json',report)
report['status']='complete';report['finished_at']=datetime.now(timezone.utc).isoformat();save(HERE/'builds.json',report)
print('REBUILT',len(report['programs']),'AOT controls; all native objects identical',flush=True)
