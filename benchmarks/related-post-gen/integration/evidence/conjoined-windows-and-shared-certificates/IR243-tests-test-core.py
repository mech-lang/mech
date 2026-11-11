from pathlib import Path
import hashlib,json,os,subprocess,time
T=Path(__file__).parent;O=Path('/private/tmp/mech-related-shared-call-certificates-opt-20261107-v1');d=json.loads((O/'build.json').read_text())
def run(cmd,name,env=None):
 start=time.monotonic()
 with (T/(name+'.stdout.log')).open('w')as out,(T/(name+'.stderr.log')).open('w')as err:r=subprocess.run(cmd,env={**os.environ,**(env or {})},stdout=out,stderr=err)
 p=T/(name+'.json');p.write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-start),indent=2)+'\n');assert r.returncode==0,(T/(name+'.stderr.log')).read_text();print(name,'passed',flush=True)
for crate in ['mech_core']:
 stage=next(s for s in d['stages']if s['crate']==crate);assert stage['exit']==0;cmd=stage['command'][:];cmd[cmd.index('--crate-type=rlib')]='--test';cmd[-1]=str(T/(crate+'-tests'))
 for i,a in enumerate(cmd):
  if a.startswith('metadata='):cmd[i]=a+'_source_layer_tests'
  if a.startswith('opt-level='):cmd[i]='opt-level=0'
 run(cmd,crate+'-build',stage['environment'])
 filters=['']if crate=='mech_core'else['native_artifact::','memory_planner::','resident::general::native_collection']
 for n,f in enumerate(filters):run([cmd[-1],f,'--test-threads=1'],crate+'-run-'+str(n))
print('Source layers complete; test glue opt0, all production dependencies opt3',flush=True)
