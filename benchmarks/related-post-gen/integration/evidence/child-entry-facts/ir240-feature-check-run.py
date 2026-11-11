from pathlib import Path
import json,os,subprocess,time
Q=Path(__file__).parent;O=Path('/private/tmp/mech-related-child-entry-facts-integration-opt-20261107-v1')
d=json.loads((O/'build.json').read_text());stage=next(s for s in d['stages']if s['crate']=='mech_gpu');command=[];i=0
while i<len(stage['command']):
 a=stage['command'][i]
 if a=='--cfg' and 'canonical-native-program' in stage['command'][i+1]:i+=2;continue
 command.append(a);i+=1
command[-1]=str(Q/'libmech_gpu_scalar_compiler.rlib')
for i,a in enumerate(command):
 if a.startswith('metadata='):command[i]=a+'_scalar_only_238'
spec=dict(command=command,environment=stage['environment']);(Q/'scalar-compiler.spec.json').write_text(json.dumps(spec,indent=2)+'\n');start=time.monotonic();r=subprocess.run(command,capture_output=True,text=True,env={**os.environ,**stage['environment']});spec.update(exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr);(Q/'scalar-compiler.json').write_text(json.dumps(spec,indent=2)+'\n');assert r.returncode==0,r.stderr;print('scalar-only current dependency closure passed',flush=True)
