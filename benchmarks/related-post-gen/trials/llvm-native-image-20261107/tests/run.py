import importlib.util,json,os,pathlib,re,subprocess,time
S=pathlib.Path('/private/tmp/mech-related-llvm-native-image-compiler-20261107')
O=pathlib.Path('/private/tmp/mech-related-llvm-native-image-opt-20261107-v4')
Q=pathlib.Path('/private/tmp/mech-related-llvm-native-image-tests-20261107-v4')
def run(cmd,name,env=None,expected=0):
 t=time.monotonic();cmd=list(map(str,cmd));r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})});d=dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);Q.joinpath(name+'.json').write_text(json.dumps(d,indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==expected,r.stdout[-5000:]+r.stderr[-5000:];return d
stage=next(s for s in json.loads(O.joinpath('build.json').read_text())['stages']if s['crate']=='mech_gpu')
cmd=stage['command'][:];cmd[cmd.index('--crate-type=rlib')]='--test';cmd[-1]=str(Q/'gpu-tests')
for i,arg in enumerate(cmd):
 if arg.startswith('metadata='):cmd[i]=arg+'_image_tests'
run(cmd,'test-build',stage['environment'])
run([Q/'gpu-tests','llvm_','--test-threads=1','--nocapture'],'llvm-default-tests')
run([Q/'gpu-tests','llvm_','--ignored','--test-threads=1','--nocapture'],'llvm-native-tests',{'MECH_LLVM_TEST_BIN':'/opt/homebrew/opt/llvm/bin','MECH_LLVM_TEST_CLANG':'/opt/homebrew/opt/llvm/bin/clang','SDKROOT':subprocess.check_output(['/usr/bin/xcrun','--show-sdk-path'],text=True).strip()})
full=run([Q/'gpu-tests','--test-threads=1'],'gpu-full',expected=101)
failed=[x.split(' ... FAILED')[0].removeprefix('test ')for x in full['stdout'].splitlines()if x.startswith('test ')and x.endswith(' ... FAILED')]
assert sorted(failed)==sorted(['batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable','native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map']),failed
workflow=(S/'.github/workflows/ci-full.yml').read_text().split('- name: Inventory and execute native composition contracts',1)[1].split('- name:',1)[0]
selection=workflow.split('--lib --',1)[1].split('--list',1)[0].replace('\\',' ').split()
listed=run([Q/'gpu-tests',*selection,'--list'],'ci-selected-list')
names=[x.removesuffix(': test')for x in listed['stdout'].splitlines()if x.endswith(': test')];assert len(names)==399
executed=run([Q/'gpu-tests',*selection,'--test-threads=1'],'ci-selected-execute')
passed=[x.removeprefix('test ').removesuffix(' ... ok')for x in executed['stdout'].splitlines()if x.startswith('test ')and x.endswith(' ... ok')];assert sorted(names)==sorted(passed)
Q.joinpath('summary.json').write_text(json.dumps(dict(full_GPU=[x for x in full['stdout'].splitlines()if x.startswith('test result:')],inherited_failures=failed,CI399_names_exact=True,CI399_passed=len(passed),LLVM_native_tests_passed=True,performance_measured=False),indent=2)+'\n')
