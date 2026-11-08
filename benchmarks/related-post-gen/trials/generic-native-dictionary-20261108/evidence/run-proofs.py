from pathlib import Path
import json,subprocess,os,time
p=Path(__file__).parent;r=json.loads((p/'gpu-proof-recipe.json').read_text());env=r['environment'];c=r['command']
def run(c,n,extra=None):
 t=time.monotonic();x=subprocess.run(c,env={**os.environ,**env,**(extra or{})},capture_output=True,text=True);(p/(n+'.json')).write_text(json.dumps(dict(command=c,environment={**env,**(extra or{})},exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr,scope=r['scope']),indent=2)+'\n');print(n,x.returncode,flush=True);assert x.returncode==0,x.stdout[-7000:]+x.stderr[-7000:]
check=c.copy();check[-1]=str(p/'gpu-proof.rmeta');check+=['--emit=metadata'];run(check,'proof-check');run(c,'proof-build');run([str(p/'gpu-proof-tests'),'generic_dictionary','--test-threads=1','--nocapture'],'generic-dictionary-proofs');run([str(p/'gpu-proof-tests'),'generic_dictionary_actual','--ignored','--test-threads=1','--nocapture'],'actual-dictionary-proof',{'MECH_MATRIX_KEY_BENCH_SOURCE':'/private/tmp/mech-related-native-string-classification-reuse-runners-20261108-v2/histogram-reuse/build/source.mec'})
