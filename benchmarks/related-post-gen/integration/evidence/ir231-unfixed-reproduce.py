from pathlib import Path
import hashlib,json,os,subprocess,time
T=Path(__file__).parent
B=Path('/private/tmp/mech-related-deflate-affine-read-integration-opt-20261106-v1/build.json')
S=Path('/private/tmp/mech-related-native-reduction-terminal-transition-compiler-20261106')
build=json.loads(B.read_text());stage=next(x for x in build['stages']if x['crate']=='mech_gpu')
command=[arg.replace(build['source'],str(S)) for arg in stage['command']]
command[command.index('--crate-type=rlib')]='--test';command[-1]=str(T/'mech_gpu-tests')
for i,arg in enumerate(command):
 if arg.startswith('metadata='):command[i]=arg+'_unfixed_terminal_reproducer'
environment={k:v.replace(build['source'],str(S))for k,v in stage['environment'].items()}
def run(command,name):
 start=time.monotonic()
 with (T/(name+'.stdout.log')).open('w')as out,(T/(name+'.stderr.log')).open('w')as err:
  result=subprocess.run(command,env={**os.environ,**environment},stdout=out,stderr=err)
 receipt=dict(command=command,environment=environment,exit=result.returncode,seconds=time.monotonic()-start)
 (T/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n')
 print(name,result.returncode,flush=True)
 return result.returncode
assert run(command,'build')==0
assert run([command[-1],'numeric_ordered_reduction_preserves_exhausted_payload_updates','--test-threads=1','--nocapture'],'reproducer')==101
text=(T/'reproducer.stderr.log').read_text()+(T/'reproducer.stdout.log').read_text()
assert 'test result: FAILED' in text and 'case=0, start=0, stop=1, seed=9' in text,text
(T/'confirmation.json').write_text(json.dumps(dict(inherited_miscompile_reproduced=True,production_compiler='a7b1c9b269',reproducer_test='numeric_ordered_reduction_preserves_exhausted_payload_updates',binary_sha256=hashlib.sha256((T/'mech_gpu-tests').read_bytes()).hexdigest()),indent=2)+'\n')
