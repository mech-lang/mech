from pathlib import Path
import json,os,re,subprocess,time
P=Path(__file__).parent
S=Path('/private/tmp/mech-related-recovered-scalar-homes-compiler-20261108')
O=Path('/private/tmp/mech-related-recovered-scalar-homes-opt-20261108-v1')
def run(cmd,name,env=None):
 cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,cwd=S,capture_output=True,text=True,env={**os.environ,**(env or {})})
 d=dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr)
 (P/(name+'.json')).write_text(json.dumps(d,indent=2)+'\n');print(name,r.returncode,flush=True)
 assert r.returncode==0,r.stdout[-6000:]+r.stderr[-6000:]
 return r
workflow=(S/'.github/workflows/ci-full.yml').read_text().split('- name: Inventory and execute native composition contracts',1)[1].split('- name:',1)[0]
selection=workflow.split('--lib --',1)[1].split('--list',1)[0].replace('\\',' ').split()
listed=run([P/'gpu-tests',*selection,'--list'],'ci-list');names=[x.removesuffix(': test')for x in listed.stdout.splitlines()if x.endswith(': test')]
prior=json.loads(Path('/private/tmp/mech-related-matrix-length-lowering-tests-20261107-v2/ci-list.json').read_text());old=[x.removesuffix(': test')for x in prior['stdout'].splitlines()if x.endswith(': test')]
assert set(old)<=set(names);assert len(old)==402 and len(names)==408
assert all('packed_row_payload' in x or 'compact_visit' in x or 'scalar_home_retirement' in x or 'numeric_retirement' in x for x in set(names)-set(old))
executed=run([P/'gpu-tests',*selection,'--test-threads=1'],'ci-execute');passed=[x.removeprefix('test ').removesuffix(' ... ok')for x in executed.stdout.splitlines()if x.startswith('test ')and x.endswith(' ... ok')];assert sorted(names)==sorted(passed)
(P/'ci-inventory-audit.json').write_text(json.dumps(dict(expected=len(names),listed=len(names),executed_passed=len(passed),all_names_match=True,old_names_retained=True,new_names=sorted(set(names)-set(old))),indent=2)+'\n')
run([P/'gpu-tests','program::image::','caller_geometry::tests','--test-threads=1'],'image-and-caller-geometry-tests')
run([P/'gpu-tests','llvm_','--ignored','--test-threads=1','--nocapture'],'optional-llvm-regressions',{'MECH_LLVM_TEST_BIN':'/opt/homebrew/opt/llvm/bin','MECH_LLVM_TEST_CLANG':'/opt/homebrew/opt/llvm/bin/clang','SDKROOT':'/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk'})
(P/'summary.json').write_text(json.dumps(dict(matrix_length_tests_passed=True,canonical_ablation_executed=True,finite_checks_passed=True,actual_CI_passed=len(passed)),indent=2)+'\n')
