from pathlib import Path
import json,os,re,subprocess,time
P=Path(__file__).parent
S=Path('/private/tmp/mech-related-matrix-length-lowering-compiler-20261107')
O=Path('/private/tmp/mech-related-packed-row-payload-opt-20261107-v2')
def run(cmd,name,env=None):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,cwd=S,capture_output=True,text=True,env={**os.environ,**(env or {})})
 d=dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr)
 (P/(name+'.json')).write_text(json.dumps(d,indent=2)+'\n');print(name,r.returncode,flush=True)
 assert r.returncode==0,r.stdout[-6000:]+r.stderr[-6000:]
 return r
build=json.loads((O/'build.json').read_text());gpu=next(s for s in build['stages']if s['crate']=='mech_gpu');assert gpu['exit']==0
cmd=gpu['command'][:];cmd[cmd.index('--crate-type=rlib')]='--test';cmd[-1]=str(P/'gpu-tests')
for i,a in enumerate(cmd):
 if a.startswith('metadata='):cmd[i]=a+'_matrix_length_tests'
import importlib.util,hashlib
spec=importlib.util.spec_from_file_location('h','/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/build-compiler.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h);files=h.source_inventory(S);(P/'test-source-inventory.json').write_text(json.dumps(dict(root=str(S),sha256=hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest(),files=files),indent=2)+'\n')
run(cmd,'test-build',gpu['environment'])
focus=run([P/'gpu-tests','packed_row_payload','native_row_blocks_preserve','matrix_length_lowering_tests','call_routes::tests','--test-threads=1','--nocapture'],'packed-row-and-route-tests')
assert '8 passed' in focus.stdout
if os.environ.get('FOCUS_ONLY')=='1':raise SystemExit(0)
workflow=(S/'.github/workflows/ci-full.yml').read_text().split('- name: Inventory and execute native composition contracts',1)[1].split('- name:',1)[0]
selection=workflow.split('--lib --',1)[1].split('--list',1)[0].replace('\\',' ').split()
listed=run([P/'gpu-tests',*selection,'--list'],'ci-list');names=[x.removesuffix(': test')for x in listed.stdout.splitlines()if x.endswith(': test')]
prior=json.loads(Path('/private/tmp/mech-related-matrix-length-lowering-tests-20261107-v2/ci-list.json').read_text());old=[x.removesuffix(': test')for x in prior['stdout'].splitlines()if x.endswith(': test')]
assert set(old)<=set(names);assert len(old)==402 and len(names)==405
assert all('packed_row_payload' in x for x in set(names)-set(old))
executed=run([P/'gpu-tests',*selection,'--test-threads=1'],'ci-execute');passed=[x.removeprefix('test ').removesuffix(' ... ok')for x in executed.stdout.splitlines()if x.startswith('test ')and x.endswith(' ... ok')];assert sorted(names)==sorted(passed)
(P/'ci-inventory-audit.json').write_text(json.dumps(dict(expected=405,listed=len(names),executed_passed=len(passed),all_names_match=True,old_names_retained=True,new_names=sorted(set(names)-set(old))),indent=2)+'\n')
run([P/'gpu-tests','program::image::','caller_geometry::tests','--test-threads=1'],'image-and-caller-geometry-tests')
run([P/'gpu-tests','llvm_','--ignored','--test-threads=1','--nocapture'],'optional-llvm-regressions',{'MECH_LLVM_TEST_BIN':'/opt/homebrew/opt/llvm/bin','MECH_LLVM_TEST_CLANG':'/opt/homebrew/opt/llvm/bin/clang','SDKROOT':'/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk'})
(P/'summary.json').write_text(json.dumps(dict(matrix_length_tests_passed=True,canonical_ablation_executed=True,finite_checks_passed=True,actual_CI_passed=len(passed)),indent=2)+'\n')
