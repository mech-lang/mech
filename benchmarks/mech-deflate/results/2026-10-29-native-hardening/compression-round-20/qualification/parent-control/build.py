from pathlib import Path
import hashlib,io,json,os,subprocess,tarfile,time
r=Path(__file__).resolve().parent;workspace=Path('/private/tmp/mech-deflate-linear-stack');revision='96721e45d454f507595c54e445d18266eff05ced';source=r/'source';source.mkdir(exist_ok=True);archive=subprocess.check_output(['git','archive',revision,'src/engine','src/stdlib','tests/fixtures/resident-ekf'],cwd=workspace)
with tarfile.open(fileobj=io.BytesIO(archive))as tar:tar.extractall(source,filter='data')
rows=[json.loads(l)for l in Path('/private/tmp/mech-linear-restack-20261026/validation/round20-nested-frame-release.log').read_text().splitlines()if l.startswith('{')]
metadata=next(json.loads(l)for l in Path('/private/tmp/mech-linear-restack-20261026/validation/round12-unit-routing-final-metadata.log').read_text().splitlines()if l.startswith('{"packages"'))
libsdir=r/'libs';libsdir.mkdir(exist_ok=True);receipts=[];rustc='/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc';deps=Path('/private/tmp/mech-live-control-target/release/deps');digest=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def execute(label,command,env=None):
 started=time.monotonic();p=subprocess.run(command,capture_output=True,text=True,env=env);(r/(label+'.log')).write_text(p.stdout+p.stderr);receipts.append({'label':label,'command':command,'exit':p.returncode,'seconds':time.monotonic()-started});(r/'commands.json').write_text(json.dumps({'source_revision':revision,'source_archive_sha256':hashlib.sha256(archive).hexdigest(),'configuration':'Semantic control only; opt0 libraries from clean pinned source with unchanged selected-operation dependencies. No performance observations.','commands':receipts},indent=2)+'\n');print(label,p.returncode,receipts[-1]['seconds'],p.stderr[-1500:],flush=True);p.check_returncode();return p
for name,folder in [('mech_engine','engine'),('mech_stdlib','stdlib')]:
 package=next(p for p in metadata['packages']if p['name']==name.replace('_','-'));node=next(n for n in metadata['resolve']['nodes']if n['id']==package['id']);features=next(row['features']for row in rows if row.get('reason')=='compiler-artifact'and row['target']['name']==name);libs={}
 for dep in node['deps']:
  matches=[row for row in rows if row.get('reason')=='compiler-artifact'and row['package_id']==dep['pkg']and 'custom-build'not in row['target']['kind']]
  if matches:
   paths=[Path(p)for p in matches[-1]['filenames']if p.endswith(('.rlib','.dylib'))]
   if paths:libs[dep['name']]=paths[0]
 if name=='mech_stdlib':libs['mech_engine']=libsdir/'libmech_engine.rlib'
 command=[rustc,'--edition=2024','--crate-name',name,'--crate-type','rlib','-C','opt-level=0','-D','warnings','-L','dependency='+str(libsdir),'-L','dependency='+str(deps)]
 for feature in features:command+=['--cfg',f'feature="{feature}"']
 for n,p in libs.items():command+=['--extern',n+'='+str(p)]
 command+=[str(source/f'src/{folder}/src/lib.rs'),'-o',str(libsdir/('lib'+name+'.rlib'))]
 env={**os.environ,'CARGO_MANIFEST_DIR':str(source/f'src/{folder}'),'CARGO_PKG_NAME':name.replace('_','-'),'CARGO_PKG_VERSION':'0.3.5','CARGO_PKG_VERSION_MAJOR':'0','CARGO_PKG_VERSION_MINOR':'3','CARGO_PKG_VERSION_PATCH':'5','CARGO_PKG_DESCRIPTION':package['description']}
 execute(name+'-build',command,env)
 (r/(name+'-libraries.json')).write_text(json.dumps({'features':features,'libraries':{n:{'path':str(p),'sha256':digest(p)}for n,p in libs.items()},'output_sha256':digest(libsdir/('lib'+name+'.rlib'))},indent=2)+'\n')
for version in['parent','current']:
 libraries={}
 for n in ['mech_core','mech_syntax','serde_json']:
  libraries[n]=next(Path(p)for row in rows if row.get('reason')=='compiler-artifact'and row['target']['name']==n for p in row['filenames']if p.endswith('.rlib'))
 for n in ['mech_engine','mech_stdlib']:
  libraries[n]=libsdir/('lib'+n+'.rlib')if version=='parent'else next(Path(p)for row in rows if row.get('reason')=='compiler-artifact'and row['target']['name']==n for p in row['filenames']if p.endswith('.rlib'))
 command=[rustc,'--edition=2024','-C','opt-level=0','-D','warnings','-L','dependency='+str(libsdir),'-L','dependency='+str(deps)]
 for n,p in libraries.items():command+=['--extern',n+'='+str(p)]
 command+=[str(r/'oracle.rs'),'-o',str(r/(version+'-oracle'))];execute(version+'-oracle-build',command)
 execute(version+'-oracle-run',[str(r/(version+'-oracle')),str(r/'public-escape.mec'),str(r/(version+'.mbc'))])
assert(r/'parent.mbc').read_bytes()==(r/'current.mbc').read_bytes()
assert(r/'parent-oracle-run.log').read_text()==(r/'current-oracle-run.log').read_text()
print('SOURCE-PINNED PARENT/CURRENT RESIDENT RESULTS AND ARTIFACTS MATCH',flush=True)
