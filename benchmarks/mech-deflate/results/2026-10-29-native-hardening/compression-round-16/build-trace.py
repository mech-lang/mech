from pathlib import Path
import json,subprocess,time,hashlib,os
r=Path(__file__).resolve().parent
w=Path('/private/tmp/mech-deflate-linear-stack')
rows=[json.loads(l) for l in Path('/private/tmp/mech-linear-restack-20261026/validation/round15-predicate-release.log').read_text().splitlines() if l.startswith('{')]
meta=next(json.loads(l) for l in Path('/private/tmp/mech-linear-restack-20261026/validation/round12-unit-routing-final-metadata.log').read_text().splitlines() if l.startswith('{"packages"'))
pkg=next(p for p in meta['packages'] if p['name']=='mech-gpu')
node=next(n for n in meta['resolve']['nodes'] if n['id']==pkg['id'])
libs={}
for dep in node['deps']:
 candidates=[row for row in rows if row.get('reason')=='compiler-artifact' and row['package_id']==dep['pkg'] and 'custom-build' not in row['target']['kind']]
 if candidates:
  paths=[Path(p) for p in candidates[-1]['filenames'] if p.endswith(('.rlib','.dylib'))]
  if paths:libs[dep['name']]=paths[0]
build=json.loads(Path('/private/tmp/mech-predicate-versions-round15-20261029/primary/current-build.json').read_text())
for name in ['mech_stdlib','mech_syntax']:libs[name]=Path(build['libraries'][name]['path'])
b=json.loads(Path('/private/tmp/mech-predicate-versions-round15-20261029/focused-v2/component-v2-build.json').read_text())
libs['tempfile']=Path(next(x.split('=',1)[1] for x in b['command'] if x.startswith('tempfile=')))
features=next(row['features'] for row in rows if row.get('reason')=='compiler-artifact' and row['target']['name']=='mech_gpu')
c=['/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc','--edition=2024','--crate-name','mech_gpu','--test','-C','opt-level=1','-D','warnings','-L','dependency=/private/tmp/mech-live-control-target/release/deps']
for f in features:c+=['--cfg',f'feature="{f}"']
for name,path in libs.items():c+=['--extern',name+'='+str(path)]
c+=[str(w/'hosts/gpu/src/lib.rs'),'-o',str(r/'gpu-trace-tests')]
env={**os.environ,'CARGO_MANIFEST_DIR':str(w/'hosts/gpu'),'CARGO_PKG_NAME':'mech-gpu','CARGO_PKG_VERSION':'0.3.5','CARGO_PKG_VERSION_MAJOR':'0','CARGO_PKG_VERSION_MINOR':'3','CARGO_PKG_VERSION_PATCH':'5','CARGO_PKG_DESCRIPTION':pkg['description']}
start=time.monotonic()
p=subprocess.run(c,capture_output=True,text=True,env=env)
(r/'gpu-trace-build.log').write_text(p.stdout+p.stderr)
receipt={'command':c,'exit':p.returncode,'seconds':time.monotonic()-start,'features':features,'libraries':{n:{'path':str(v),'sha256':hashlib.sha256(v.read_bytes()).hexdigest()} for n,v in libs.items()}}
(r/'gpu-trace-build.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(p.stderr[-6000:],flush=True)
p.check_returncode()
c=[str(r/'gpu-trace-tests'),'goal_trace_one_unchanged_complete_eight_byte_evaluation','--nocapture','--test-threads=1']
start=time.monotonic();p=subprocess.run(c,capture_output=True,text=True,timeout=180)
(r/'gpu-trace-run.log').write_text(p.stdout+p.stderr)
(r/'gpu-trace-run.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-start},indent=2)+'\n')
print(p.stdout[-2000:]+p.stderr[-1000:]);p.check_returncode()
