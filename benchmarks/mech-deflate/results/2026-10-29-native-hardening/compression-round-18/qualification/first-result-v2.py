from pathlib import Path
import hashlib,json,shutil,subprocess,time
root=Path(__file__).resolve().parent
validation=Path('/private/tmp/mech-linear-restack-20261026/validation')
start=time.monotonic()
for name in ['round18-opaque-siblings-release']:
 receipt=validation/(name+'.json')
 while not receipt.exists():
  if time.monotonic()-start>1200:raise RuntimeError('Existing build/check did not finish within the observation window: '+name)
  time.sleep(2)
 result=json.loads(receipt.read_text());assert result['exit']==0,result
 print('READY',name,result['seconds'],flush=True)
for script in ['primary/build.py','components/qualify.py']:
 print('START',script,flush=True)
 subprocess.run(['python3',str(root/script)],check=True,cwd='/private/tmp/mech-deflate-linear-stack')
p=root/'primary/frame-after';assert not p.exists()
shutil.copy2('/private/tmp/mech-live-control-target/release/native_frame_transport',p)
(root/'primary/frame-after.json').write_text(json.dumps({'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'production_compiler':'1d2ef606f0cb576eee343b4806fd1c75e69a7b7a','source':'/private/tmp/mech-live-control-target/release/native_frame_transport'},indent=2)+'\n')
receipt=validation/'round18-static-architecture-final.json'
while not receipt.exists():
 if time.monotonic()-start>1200:raise RuntimeError('Static checks did not finish')
 time.sleep(2)
result=json.loads(receipt.read_text());assert result['exit']==0,result
print('READY final static checks',result['seconds'],flush=True)
print('START primary/run.py',flush=True)
subprocess.run(['python3',str(root/'primary/run.py')],check=True,cwd='/private/tmp/mech-deflate-linear-stack')
