from pathlib import Path
import hashlib,json,os,subprocess,time,sys
r=Path(__file__).resolve().parent;o=r/(sys.argv[1] if len(sys.argv)>1 else 'core-units');o.mkdir();w=Path('/private/tmp/mech-deflate-linear-stack')
log=Path('/private/tmp/mech-linear-restack-20261026/validation/round22-checkpoint-fixed-release.log')
rows=[json.loads(l) for l in log.read_text().splitlines() if l.startswith('{')];assert rows[-1]=={'reason':'build-finished','success':True}
m=json.loads(Path('/private/tmp/mech-scalar-paths-round22-20261029/default-metadata.json').read_text());package=next(p for p in m['packages'] if p['name']=='mech-core');node=next(n for n in m['resolve']['nodes'] if n['id']==package['id'])
libs={}
for dep in node['deps']:
 a=[x for x in rows if x.get('reason')=='compiler-artifact' and x['package_id']==dep['pkg'] and 'custom-build' not in x['target']['kind']]
 assert len(a)==1,(dep['name'],len(a));paths=[Path(p) for p in a[0]['filenames'] if p.endswith(('.rlib','.dylib'))];assert len(paths)==1
 libs[dep['name']]=paths[0]
features=next(x['features'] for x in rows if x.get('reason')=='compiler-artifact' and x['target']['name']=='mech_core')
cmd=['/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc','--edition=2024','--crate-name','mech_core','--test','-C','opt-level=0','-D','warnings','-L','dependency=/private/tmp/mech-live-control-target/release/deps']
for feature in features:cmd+=['--cfg',f'feature="{feature}"']
for name,path in libs.items():cmd+=['--extern',name+'='+str(path)]
cmd+=[str(w/'src/core/src/lib.rs'),'-o',str(o/'core-unit-tests')]
env={**os.environ,'CARGO_MANIFEST_DIR':str(w/'src/core'),'CARGO_PKG_VERSION':'0.3.5','CARGO_PKG_NAME':'mech-core','CARGO_PKG_DESCRIPTION':package['description']}
t=time.monotonic();x=subprocess.run(cmd,capture_output=True,text=True,env=env);(o/'build.log').write_text(x.stdout+x.stderr)
(o/'build.json').write_text(json.dumps({'command':cmd,'exit':x.returncode,'seconds':time.monotonic()-t,'features':features,'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=w,text=True).strip(),'dirty':subprocess.check_output(['git','status','--porcelain'],cwd=w,text=True),'budget_sha256':hashlib.sha256((w/'src/core/src/memory_plan/budget.rs').read_bytes()).hexdigest(),'libraries':{n:{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for n,p in libs.items()}},indent=2)+'\n');print(x.stderr[-5000:],flush=True);x.check_returncode()
cmd=[str(o/'core-unit-tests'),'memory_plan::budget::tests::','--nocapture','--test-threads=1'];t=time.monotonic();x=subprocess.run(cmd,capture_output=True,text=True);(o/'run.log').write_text(x.stdout+x.stderr);(o/'run.json').write_text(json.dumps({'command':cmd,'exit':x.returncode,'seconds':time.monotonic()-t,'binary_sha256':hashlib.sha256((o/'core-unit-tests').read_bytes()).hexdigest()},indent=2)+'\n');print(x.stdout+x.stderr,flush=True);x.check_returncode()
