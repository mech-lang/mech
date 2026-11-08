from pathlib import Path
import os,json,subprocess,time,hashlib,sys
p=Path(__file__).parent;b=json.loads(Path(sys.argv[1]).read_text());s=next(x for x in b['stages']if x['crate']=='mech_gpu');c=s['command'].copy();c[c.index('--crate-type=rlib')]='--test';c[-1]=str(p/'gpu-storage-tests');env=s['environment']
for i,a in enumerate(c):
 if a.startswith('metadata='):c[i]='metadata=related_generic_dictionary_storage_v1'
def run(c,n):
 t=time.monotonic();x=subprocess.run(c,cwd=b['source'],env={**os.environ,**env},capture_output=True,text=True);(p/(n+'.json')).write_text(json.dumps(dict(command=c,environment=env,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr,scope='Storage/API contracts only; native selection disabled; no timing claim'),indent=2)+'\n');print(n,x.returncode,flush=True);assert x.returncode==0,x.stderr[-5000:]+x.stdout[-6000:]
check=c.copy();check[-1]=str(p/'gpu-storage.rmeta');check+=['--emit=metadata'];run(check,'storage-check');run(c,'storage-build');run([str(p/'gpu-storage-tests'),'frozen_generic_dictionary','--test-threads=1','--nocapture'],'frozen-generic-storage');run([str(p/'gpu-storage-tests'),'generic_dictionary','--test-threads=1'],'storage-and-use-proofs')
