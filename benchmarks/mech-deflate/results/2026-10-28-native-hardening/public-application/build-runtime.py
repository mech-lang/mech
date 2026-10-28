import os,subprocess,shutil,json,time
from pathlib import Path
w=Path('/private/tmp/mech-deflate-linear-stack')
root=Path('/private/tmp/mech-public-finite-20261028')
receipts=[]
env=dict(os.environ,RUSTUP_TOOLCHAIN='nightly-2026-03-03',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_BUILD_JOBS='1',CARGO_NET_OFFLINE='true')
for form in ['original','decoded']:
 app=root/form
 shutil.copy2(w/'Cargo.lock',app/'Cargo.lock')
 commands=[['cargo','generate-lockfile','--offline','--manifest-path',str(app/'Cargo.toml')],['cargo','build','--locked','--offline','--frozen','-j1','--manifest-path',str(app/'Cargo.toml'),'--target-dir','/private/tmp/mech-live-control-target','--message-format=json']]
 for index,cmd in enumerate(commands):
  start=time.time()
  with (root/f'{form}-build-{index}.stdout').open('w') as out,(root/f'{form}-build-{index}.stderr').open('w') as err:
   r=subprocess.run(cmd,env=env,cwd=w,stdout=out,stderr=err)
  receipts.append({'form':form,'command':cmd,'exit':r.returncode,'seconds':time.time()-start})
  (root/'builds.json').write_text(json.dumps(receipts,indent=2)+'\n')
  print(receipts[-1],flush=True)
  r.check_returncode()
 source=Path('/private/tmp/mech-live-control-target/debug/compression-application')
 shutil.copy2(source,app/'compression-application')
