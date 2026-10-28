import subprocess,json,hashlib
from pathlib import Path
root=Path('/private/tmp/mech-compression-round-20261028')
deps=Path('/private/tmp/mech-live-control-target/release/deps')
def resolve_hash(name,want):
 for d in (deps.parent/'.fingerprint').iterdir():
  f=d/('lib-'+name)
  if f.is_file() and int.from_bytes(bytes.fromhex(f.read_text().strip()),'little')==want:
   suffix=d.name.rsplit('-',1)[1];p=deps/('lib'+name+'-'+suffix+'.rlib')
   if p.is_file(): return p
 raise ValueError((name,want))
versions={'current':{'mech_gpu':'3c73528bc7ca0ab2','mech_engine':'4aee2ad1a00f13dd','mech_stdlib':'bf12058d26333a71','mech_core':'3f09556dc3716cd3','mech_syntax':'9be4337841a5564e'},'retained':{'mech_gpu':'7fa2845c539b987a','mech_engine':'0d214f77bb5acf15','mech_stdlib':'0ce0772bdec500ff','mech_core':'4ff4856784490a86','mech_syntax':'99fe397930ae7109'}}
rustc='/Users/coreymontella/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin/rustc'
mi=Path('/private/tmp/roc-deflate-repro-20261024/roc-deflate/benchmark/echo-clock/targets/arm64mac/mimalloc.o')
assert hashlib.sha256(mi.read_bytes()).hexdigest()=='38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874'
for label,libs in versions.items():
 fp=json.loads((deps.parent/'.fingerprint'/('mech-gpu-'+libs['mech_gpu'])/'lib-mech_gpu.json').read_text())
 for _,name,_,value in fp['deps']:
  if name in ['serde_json','serde','miniz_oxide']: libs[name]=resolve_hash(name,value)
 cmd=[rustc,'--edition=2024','-C','opt-level=3','-D','warnings','-L','dependency='+str(deps),'-C','link-arg='+str(mi)]
 resolved={}
 for name,path in libs.items():
  if not isinstance(path,Path): path=deps/('lib'+name+'-'+path+'.rlib')
  resolved[name]={'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()};cmd+=['--extern',name+'='+str(path)]
 cmd+=['/private/tmp/mech-timing-round.rs','-o',str(root/label)]
 print(label,flush=True);r=subprocess.run(cmd,capture_output=True,text=True)
 (root/(label+'-build.log')).write_text(r.stdout+r.stderr)
 (root/(label+'-build.json')).write_text(json.dumps({'command':cmd,'exit':r.returncode,'rlibs':resolved,'harness_sha256':hashlib.sha256(Path('/private/tmp/mech-timing-round.rs').read_bytes()).hexdigest()},indent=2)+'\n')
 if r.returncode:print(r.stderr);raise SystemExit(r.returncode)
 print(hashlib.sha256((root/label).read_bytes()).hexdigest())
