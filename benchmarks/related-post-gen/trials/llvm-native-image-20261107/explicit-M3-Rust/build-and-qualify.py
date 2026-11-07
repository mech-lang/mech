from pathlib import Path
import copy,hashlib,json,os,shutil,struct,subprocess,time
P=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
RUST=Path('/Users/cim310/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin')
base=json.loads(Path('/private/tmp/mech-related-ordered-rust-controls-cohort-20261107-v1/latency-config.json').read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def run(cmd,cwd,path,env=None):
 t=time.monotonic();cmd=list(map(str,cmd));r=subprocess.run(cmd,cwd=cwd,capture_output=True,text=True,env={**os.environ,**(env or {})});d=dict(command=cmd,cwd=str(cwd),environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);path.write_text(json.dumps(d,indent=2)+'\n');assert r.returncode==0,r.stdout[-5000:]+r.stderr[-5000:];return d
variants=[]
for old in base['implementations']:
 if old['kind']!='upstream':continue
 name=old['name']+'-apple-m3';p=P/name;p.mkdir(exist_ok=True)
 source=Path(old['source']);assert sha(source)==old['source_sha256']
 root=source.parent.parent
 package=root if(root/'Cargo.toml').exists()else Path('/private/tmp/mech-related-resource-adapters-20261104/rust')
 for n in ['Cargo.toml','Cargo.lock']:shutil.copyfile(package/n,p/n)
 shutil.copytree(root/'src',p/'src',dirs_exist_ok=True)
 env={'RUSTC':str(RUST/'rustc'),'RUSTFLAGS':'-C target-cpu=apple-m3','CARGO_TARGET_DIR':str(P/'target')}
 run([RUST/'cargo','build','--release','--locked','--offline'],p,p/'build.json',env)
 binary=Path(old['executable']).name;exe=p/binary;shutil.copy2(P/'target/release'/binary,exe)
 assert struct.unpack_from('<II',exe.read_bytes())==(0xfeedfacf,0x100000c)
 test=run([RUST/'cargo','test','--release','--locked','--offline','--bin',binary,'--','--test-threads=1'],p,p/'tests.json',env)
 v=copy.deepcopy(old);v.update(name=name,executable=str(exe),executable_sha256=sha(exe),source=str(p/'src/main.rs'),source_sha256=sha(p/'src/main.rs'),receipt=str(p/'build.json'),receipt_sha256=sha(p/'build.json'),executable_file_bytes=exe.stat().st_size,cpu='apple-m3',source_unchanged_from=old['source'],source_unchanged_sha256=old['source_sha256']);variants.append(v)
 P.joinpath('variants.json').write_text(json.dumps(variants,indent=2)+'\n');print(name,'built and tests passed',flush=True)
rows=json.loads(Path('/private/tmp/mech-related-forward-read-windows-runners-20261107-v8/reuse-4/edges/edge-fixtures.json').read_text());assert len(rows)==31
cases=[]
for x in rows:
 source=Path(x.get('source_posts',x['command'][2]));data=json.loads(source.read_text())
 if 'expected'in x:want=json.loads(Path(x['expected']).read_text())
 else:
  idx=json.loads((R/'fixtures'/('expected-'+x['fixture']+'.json')).read_text())['related_indices'];want=[{'_id':post['_id'],'tags':post['tags'],'related':[data[i]for i in selected]}for post,selected in zip(data,idx)]
 cases.append((x['fixture'],source,want))
cases.append(('full5000',Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json'),json.loads(Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json').read_text())))
exact=0
for v in variants:
 for case,source,want in cases:
  p=P/'qualification'/v['name']/case;(p/'run').mkdir(parents=True,exist_ok=True);shutil.copyfile(source,p/'posts.json');run([v['executable']],p/'run',p/'run.json',v.get('env'));output=p/v['output'];assert json.loads(output.read_text())==want,(v['name'],case);exact+=1
 print(v['name'],'32 exact outputs',flush=True)
P.joinpath('qualification-summary.json').write_text(json.dumps(dict(variants=len(variants),cases_each=len(cases),exact_outputs=exact,source_unchanged=True,cpu='apple-m3',all_setup_boundaries_unchanged=True,performance_measured=False),indent=2)+'\n')
