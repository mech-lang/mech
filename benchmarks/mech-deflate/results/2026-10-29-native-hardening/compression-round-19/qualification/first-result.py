from pathlib import Path
import hashlib,json,shutil,subprocess,time
root=Path(__file__).resolve().parent
validation=Path('/private/tmp/mech-linear-restack-20261026/validation')
start=time.monotonic()
for name in ['round19-scalar-record-release','round19-static-architecture']:
 p=validation/(name+'.json')
 while not p.exists():
  if time.monotonic()-start>1200:raise RuntimeError('Existing build/check did not finish: '+name)
  time.sleep(2)
 d=json.loads(p.read_text());assert d['exit']==0,d
 print('READY',name,d['seconds'],flush=True)
for script in ['primary/build.py','components/qualify.py']:
 print('START',script,flush=True);subprocess.run(['python3',str(root/script)],check=True,cwd='/private/tmp/mech-deflate-linear-stack')
p=root/'primary/frame-after';assert not p.exists();shutil.copy2('/private/tmp/mech-live-control-target/release/native_frame_transport',p)
(root/'primary/frame-after.json').write_text(json.dumps({'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'production':'7f96b935684f15d3a09bbd9a4b949b29d6a625e0'},indent=2)+'\n')
print('START balanced primary',flush=True)
subprocess.run(['python3',str(root/'primary/run.py')],check=True,cwd='/private/tmp/mech-deflate-linear-stack')
