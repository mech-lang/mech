from pathlib import Path
import hashlib,json,os,subprocess,time
root=Path(__file__).resolve().parent
workspace=Path('/private/tmp/mech-deflate-linear-stack')
logs=Path('/private/tmp/mech-linear-restack-20261026/validation')
rows=[json.loads(l) for l in (logs/'round16-record-precharge-release.log').read_text().splitlines() if l.startswith('{')]
assert rows[-1]=={'reason':'build-finished','success':True}
metadata=next(json.loads(l) for l in (logs/'round12-unit-routing-final-metadata.log').read_text().splitlines() if l.startswith('{"packages"'))
package=next(p for p in metadata['packages'] if p['name']=='mech-engine')
node=next(n for n in metadata['resolve']['nodes'] if n['id']==package['id'])
libs={}
for dep in node['deps']:
 matches=[row for row in rows if row.get('reason')=='compiler-artifact' and row['package_id']==dep['pkg'] and 'custom-build' not in row['target']['kind']]
 if matches:
  paths=[Path(p) for p in matches[-1]['filenames'] if p.endswith(('.rlib','.dylib'))]
  if paths:libs[dep['name']]=paths[0]
features=next(row['features'] for row in rows if row.get('reason')=='compiler-artifact' and row['target']['name']=='mech_engine')
command=['/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc','--edition=2024','--crate-name','mech_engine','--crate-type','lib','--emit=metadata','-C','opt-level=0','-D','warnings','-L','dependency=/private/tmp/mech-live-control-target/release/deps']
features=[feature for feature in features if feature!='native-artifact']
for feature in features:command+=['--cfg',f'feature="{feature}"']
for name,path in libs.items():assert path.is_file(),path;command+=['--extern',name+'='+str(path)]
command+=[str(workspace/'src/engine/src/lib.rs'),'-o',str(root/'engine-nonnative.rmeta')]
start=time.monotonic();result=subprocess.run(command,capture_output=True,text=True,env={**os.environ,'CARGO_MANIFEST_DIR':str(workspace/'src/engine'),'CARGO_PKG_NAME':'mech-engine','CARGO_PKG_VERSION':'0.3.5','CARGO_PKG_VERSION_MAJOR':'0','CARGO_PKG_VERSION_MINOR':'3','CARGO_PKG_VERSION_PATCH':'5','CARGO_PKG_DESCRIPTION':package['description']})
(root/'nonnative-build.log').write_text(result.stdout+result.stderr)
(root/'nonnative-build.json').write_text(json.dumps({'command':command,'exit':result.returncode,'seconds':time.monotonic()-start,'features':features,'libraries':{n:{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for n,p in libs.items()}},indent=2)+'\n')
print(result.stderr[-5000:]);result.check_returncode()
