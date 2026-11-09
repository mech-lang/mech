from pathlib import Path
import json,os,subprocess,sys,time,importlib.util
P=Path(__file__).parent;T=P/sys.argv[2];T.mkdir(exist_ok=False);S=Path('/private/tmp/mech-related-fixed-key-native-loops-compiler-20261108');b=json.loads(Path(sys.argv[1]).read_text());st=next(x for x in b['stages']if x['crate']=='mech_gpu');assert st['exit']==0;c=[a.replace(b['source'],str(S))for a in st['command']];c[c.index('--crate-type=rlib')]='--test';c[-1]=str(T/'gpu-tests');env={k:v.replace(b['source'],str(S))for k,v in st['environment'].items()}
for i,a in enumerate(c):
 if a.startswith('metadata='):c[i]='metadata=related_fixed_key_native_loop_applicability_v1'
spec=importlib.util.spec_from_file_location('h',P/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(S);(T/'source-inventory.json').write_text(json.dumps(dict(root=str(S),files=files,scope='borrowed current-value bank storage and full-use applicability; no native route or timing yet'),indent=2)+'\n')
def run(cmd,name,extra=None):
 t=time.monotonic();r=subprocess.run(cmd,cwd=S,capture_output=True,text=True,env={**os.environ,**env,**(extra or{})});(T/(name+'.json')).write_text(json.dumps(dict(command=cmd,environment={**env,**(extra or{})},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,timing_eligible=False),indent=2)+'\n');print(name,r.returncode,flush=True)
 if r.returncode:print(r.stdout[-7000:]+r.stderr[-8000:],flush=True)
 assert r.returncode==0
 if name=='contracts':assert '6 passed; 0 failed' in r.stdout,'expected six actual named contract executions'
 if name=='actual-source':assert '1 passed; 0 failed' in r.stdout,'expected pinned actual source test execution'
check=c.copy();check[-1]=str(T/'gpu-check.rmeta');check+=['--emit=metadata'];run(check,'test-check');run(c,'test-build');run([str(T/'gpu-tests'),'fixed_key_native_loops','--test-threads=1','--nocapture'],'contracts');run([str(T/'gpu-tests'),'fixed_key_native_loops_actual','--ignored','--test-threads=1','--nocapture'],'actual-source',{'MECH_BORROWED_DICTIONARY_SOURCE':'/private/tmp/mech-related-native-dictionary-seed-frames-runners-20261108-v1/histogram-reuse/build/source.mec'});assert files==h.source_inventory(S)
