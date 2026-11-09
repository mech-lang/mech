from pathlib import Path
import json,subprocess,os,time,hashlib,importlib.util
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-fixed-key-native-loops-compiler-20261108');T=P/'engine-tests-v1';T.mkdir(exist_ok=False)
read=lambda p:json.loads(Path(p).read_text())
b=read('/private/tmp/mech-related-fixed-key-native-loops-core-engine-20261108-v1/build.json');libs={v['crate']:v['command'][-1]for v in b['stages']if v['exit']==0}
base=read('/private/tmp/mech-related-borrowed-dictionary-values-20261108/engine-tests-v2/test-build.json');old='/private/tmp/mech-related-borrowed-dictionary-values-compiler-20261108';c=[a.replace(old,str(S))for a in base['command']];env={k:v.replace(old,str(S))for k,v in base['environment'].items()};c[-1]=str(T/'engine-tests')
for i,a in enumerate(c):
 if a=='--extern':
  name=c[i+1].split('=',1)[0]
  if name in libs:c[i+1]=name+'='+libs[name]
 if a.startswith('metadata='):c[i]='metadata=related_fixed_key_native_loop_engine_tests_v1'
for root in sorted({str(Path(v).parent)for v in libs.values()}):c[1:1]=['-L','dependency='+root]
spec=importlib.util.spec_from_file_location('h',P/'build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(S);component={k:v for k,v in files.items()if k.startswith(('src/core/','src/engine/','src/syntax/','src/bytecode/'))};(T/'source-inventory.json').write_text(json.dumps(dict(root=str(S),files=component),indent=2)+'\n')
def run(c,name):
 t=time.monotonic();r=subprocess.run(c,cwd=S,capture_output=True,text=True,env={**os.environ,**env});d=dict(command=c,environment=env,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr,timing_eligible=False);(T/(name+'.json')).write_text(json.dumps(d,indent=2)+'\n');print(name,r.returncode,flush=True)
 if r.returncode:print(r.stdout[-7000:]+r.stderr[-7000:])
 assert r.returncode==0
 return d
check=c.copy();check[-1]=str(T/'engine-check.rmeta');check+=['--emit=metadata'];run(check,'test-check');run(c,'test-build')
B=Path('/private/tmp/mech-related-borrowed-dictionary-values-20261108');prior=read(B/'Engine-component-qualification.json');names=sorted(set(prior['native_collection_names'])|set(prior['projection_frame_names']));result=run([str(T/'engine-tests'),*names,'--test-threads=1'],'all-native-and-projection-contracts');passed={l.removeprefix('test ').removesuffix(' ... ok')for l in result['stdout'].splitlines()if l.startswith('test ')and l.endswith(' ... ok')};assert set(names)<=passed
assert component=={k:v for k,v in h.source_inventory(S).items()if k.startswith(('src/core/','src/engine/','src/syntax/','src/bytecode/'))}
(T/'qualification.json').write_text(json.dumps(dict(passed=len(passed),requested_names=names,source_unchanged=True,component_sha256=hashlib.sha256(json.dumps(component,sort_keys=True).encode()).hexdigest()),indent=2)+'\n');print('Engine qualified',len(passed),flush=True)
