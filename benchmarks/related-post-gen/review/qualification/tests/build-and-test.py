from pathlib import Path
import json,os,subprocess,time,hashlib,importlib.util
T=Path(__file__).parent;O=Path('/private/tmp/mech-related-native-fixed-frame-arenas-opt-20261106-v2');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
d=json.loads((O/'build.json').read_text());assert len(d['stages'])==12 and all(s['exit']==0 for s in d['stages'])
spec=importlib.util.spec_from_file_location('compiler_helpers',R/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
i=json.loads((O/'source-inventory.json').read_text());assert h.source_inventory(Path(d['source']))==i['files']
stage=next(x for x in d['stages']if x['crate']=='mech_gpu');cmd=stage['command'][:];cmd[cmd.index('--crate-type=rlib')]='--test';cmd[-1]=str(T/'mech_gpu-tests')
for n,a in enumerate(cmd):
 if a.startswith('metadata='):cmd[n]=a+'_integration_tests'
def run(cmd,name,env=None):
 start=time.monotonic()
 with (T/(name+'.stdout.log')).open('w')as out,(T/(name+'.stderr.log')).open('w')as err:x=subprocess.run(cmd,env={**os.environ,**(env or {})},stdout=out,stderr=err)
 receipt=dict(command=cmd,exit=x.returncode,seconds=time.monotonic()-start);(T/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n');print(name,x.returncode,flush=True);return x.returncode
assert run(cmd,'mech_gpu-build',stage['environment'])==0
assert run([cmd[-1],'typed_transport_admission','--test-threads=1','--nocapture'],'fixed-arenas-run')==0
assert run([cmd[-1],'numeric_tree','--test-threads=1','--nocapture'],'numeric-tree-run')==0
run([cmd[-1],'--test-threads=1'],'test-run')
(T/'source-inventory.json').write_bytes((O/'source-inventory.json').read_bytes())
