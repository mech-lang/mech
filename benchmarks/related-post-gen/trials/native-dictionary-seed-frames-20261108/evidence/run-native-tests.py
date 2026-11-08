from pathlib import Path
import os,json,subprocess,time,hashlib,sys
packet=Path(__file__).parent
p=packet/sys.argv[2]
p.mkdir(exist_ok=False)
b=json.loads(Path(sys.argv[1]).read_text())
s=next(x for x in b['stages'] if x['crate']=='mech_gpu')
c=s['command'].copy()
c[c.index('--crate-type=rlib')]='--test'
c[-1]=str(p/'gpu-native-tests')
env=s['environment']
for i,a in enumerate(c):
 if a.startswith('metadata='): c[i]='metadata=related_dictionary_seed_frames_native_v1'
def run(c,n):
 t=time.monotonic()
 x=subprocess.run(c,cwd=b['source'],env={**os.environ,**env},capture_output=True,text=True)
 (p/(n+'.json')).write_text(json.dumps(dict(command=c,environment=env,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr,scope='IR305 native seed-frame, typed import and original-factory fallback contracts; functional/resource qualification only; no timing claim'),indent=2)+'\n')
 print(n,x.returncode,flush=True)
 if x.returncode: print(x.stderr[-10000:]+x.stdout[-10000:],flush=True)
 assert x.returncode==0
check=c.copy()
check[-1]=str(p/'gpu-native.rmeta')
check+=['--emit=metadata']
run(check,'test-check')
run(c,'test-build')
run([str(p/'gpu-native-tests'),'native_dictionary_seed','--test-threads=1','--nocapture'],'generic-dictionary')
