#!/usr/bin/env python3
import os,json,subprocess,time,sys,hashlib
from pathlib import Path
build=Path(sys.argv[1]); p=Path(sys.argv[2]); p.mkdir(parents=True,exist_ok=True)
b=json.loads(build.read_text()); s=next(v for v in b['stages'] if v['crate']=='mech_gpu'); assert s['exit']==0
cmd=s['command'].copy(); cmd[cmd.index('--crate-type=rlib')]='--test'; cmd[-1]=str(p/'gpu-tests')
for i,v in enumerate(cmd):
 if v.startswith('metadata='): cmd[i]='metadata=related_ir303_tests_'+b['source_sha256'][:16]
env={**s['environment']}; source=Path(b['source']);
(p/'gpu-test-recipe.json').write_text(json.dumps(dict(command=cmd,environment=env,build=str(build),build_sha256=hashlib.sha256(build.read_bytes()).hexdigest(),scope='Full production dependent Engine/runtime/stdlib; IR303 runtime qualification'),indent=2)+'\n')
def run(c,n):
 t=time.monotonic()
 with (p/(n+'.stdout.log')).open('w') as out,(p/(n+'.stderr.log')).open('w') as err:
  r=subprocess.run(c,cwd=source,env={**os.environ,**env},stdout=out,stderr=err)
 (p/(n+'.json')).write_text(json.dumps(dict(command=c,environment=env,exit=r.returncode,seconds=time.monotonic()-t,scope='Qualification only; concurrent timers ineligible'),indent=2)+'\n');print(n,r.returncode,flush=True)
 if r.returncode: print((p/(n+'.stderr.log')).read_text()[-10000:],flush=True); print((p/(n+'.stdout.log')).read_text()[-6000:],flush=True);sys.exit(r.returncode)
check=cmd.copy();check[-1]=str(p/'gpu-tests.rmeta');check+=['--emit=metadata'];run(check,'gpu-test-check')
run(cmd,'gpu-test-build')
run([str(p/'gpu-tests'),'borrowed_map_keys','--test-threads=1','--nocapture'],'borrowed-map-key-runtime')
run([str(p/'gpu-tests'),'matrix_map_keys','--test-threads=1','--nocapture'],'matrix-map-key-regressions')
