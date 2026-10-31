from pathlib import Path
import json,subprocess,os,time,gzip,hashlib
r=Path(__file__).resolve().parent
w=Path('/private/tmp/mech-deflate-linear-stack')
logs=Path('/private/tmp/mech-linear-restack-20261026/validation')
rows=[json.loads(l) for l in (logs/'round11-numeric-unit-routing-release.log').read_text().splitlines() if l.startswith('{')]
metadata=next(json.loads(l) for l in (logs/'round11-unit-routing-metadata.log').read_text().splitlines() if l.startswith('{"packages"'))
gpu=next(p for p in metadata['packages'] if p['name']=='mech-gpu')
node=next(n for n in metadata['resolve']['nodes'] if n['id']==gpu['id'])
libs={}
for dep in node['deps']:
 matches=[row for row in rows if row.get('reason')=='compiler-artifact' and row['package_id']==dep['pkg'] and 'custom-build' not in row['target']['kind']]
 if matches:
  row=matches[-1];paths=[Path(p) for p in row['filenames'] if p.endswith(('.rlib','.dylib'))]
  if paths:libs[dep['name']]=paths[0]
for name in ['mech_build','mech_runtime','mech_stdlib','mech_syntax']:
 row=next(row for row in rows if row.get('reason')=='compiler-artifact' and row['target']['name']==name)
 libs[name]=next(Path(p) for p in row['filenames'] if p.endswith('.rlib'))
old=w/'benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-02/round02-runtime-library-refresh.log.gz'
oldrows=[json.loads(l) for l in gzip.decompress(old.read_bytes()).decode().splitlines() if l.startswith('{')]
row=next(row for row in oldrows if row.get('reason')=='compiler-artifact' and row['target']['name']=='tempfile')
libs['tempfile']=next(Path(p) for p in row['filenames'] if p.endswith('.rlib'))
features=next(row['features'] for row in rows if row.get('reason')=='compiler-artifact' and row['target']['name']=='mech_gpu')
cmd=['/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc','--edition=2024','--crate-name','mech_gpu','--test','-C','opt-level=1','-D','warnings','-L','dependency=/private/tmp/mech-live-control-target/release/deps']
for feature in features:cmd+=['--cfg',f'feature="{feature}"']
for name,path in libs.items():assert path.is_file(),path;cmd+=['--extern',name+'='+str(path)]
cmd+=[str(w/'hosts/gpu/src/lib.rs'),'-o',str(r/'unit-tests5')]
a=time.monotonic();p=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,'CARGO_MANIFEST_DIR':str(w/'hosts/gpu'),'CARGO_PKG_VERSION':'0.3.5','CARGO_PKG_NAME':'mech-gpu','CARGO_PKG_DESCRIPTION':gpu['description']})
(r/'unit-build5.log').write_text(p.stdout+p.stderr);(r/'unit-build5.json').write_text(json.dumps({'command':cmd,'exit':p.returncode,'seconds':time.monotonic()-a,'features':features,'libraries':{n:{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for n,p in libs.items()}},indent=2)+'\n');print(p.stderr[-12000:]);p.check_returncode()
cmd=[str(r/'unit-tests5'),'numeric_fuel_exit_synchronizes','--nocapture','--test-threads=1']
a=time.monotonic();p=subprocess.run(cmd,capture_output=True,text=True)
(r/'unit-run5.log').write_text(p.stdout+p.stderr);(r/'unit-run5.json').write_text(json.dumps({'command':cmd,'exit':p.returncode,'seconds':time.monotonic()-a,'binary_sha256':hashlib.sha256((r/'unit-tests5').read_bytes()).hexdigest()},indent=2)+'\n');print(p.stdout+p.stderr);p.check_returncode()
