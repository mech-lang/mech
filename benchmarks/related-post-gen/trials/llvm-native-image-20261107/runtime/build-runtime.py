from pathlib import Path
import copy,hashlib,json,os,shutil,subprocess,time
P=Path(__file__).parent;O=Path('/private/tmp/mech-related-llvm-native-image-opt-20261107-v4')
build=json.loads((O/'build.json').read_text());old=json.loads(Path('/private/tmp/mech-related-llvm-native-image-runtime-20261107-v3/runtime-build.json').read_text());gpu=next(s for s in build['stages']if s['crate']=='mech_gpu');cmd=old['command'][:];cmd[-1]=str(P/'libmech_gpu.rlib')
for i,a in enumerate(cmd):
 if a.startswith('metadata='):cmd[i]=a+'_v4'
env=old.get('environment',gpu['environment']);t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**env});receipt=dict(command=cmd,environment=env,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);(P/'runtime-build.json').write_text(json.dumps(receipt,indent=2)+'\n');print('runtime-only-build',r.returncode,flush=True);assert r.returncode==0,r.stderr
assert not any('cranelift_'in x or 'libloading='in x for x in cmd)
gpu.update(command=cmd,environment=env,exit=0,seconds=receipt['seconds'],reused=False,library_sha256=hashlib.sha256((P/'libmech_gpu.rlib').read_bytes()).hexdigest());build.update(runtime_only_GPU=True,source_inventory_from=str(O/'source-inventory.json'));(P/'build.json').write_text(json.dumps(build,indent=2)+'\n');shutil.copyfile(O/'source-inventory.json',P/'source-inventory.json')
