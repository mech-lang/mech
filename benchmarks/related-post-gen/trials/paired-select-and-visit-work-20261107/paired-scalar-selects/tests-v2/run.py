from pathlib import Path
import json,os,subprocess,time
P=Path(__file__).parent;S=Path('/private/tmp/mech-related-paired-scalar-selects-compiler-20261107');O=Path('/private/tmp/mech-related-paired-scalar-selects-opt-20261107-v3')
def run(cmd,name,env=None):
    cmd=list(map(str,cmd));t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True,env={**os.environ,**(env or {})})
    (P/(name+'.json')).write_text(json.dumps(dict(command=cmd,environment=env or {},exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-5000:]+r.stderr[-5000:];return r
gpu=next(s for s in json.loads((O/'build.json').read_text())['stages']if s['crate']=='mech_gpu');cmd=gpu['command'][:];cmd[cmd.index('--crate-type=rlib')]='--test';cmd[-1]=str(P/'gpu-tests')
for i,a in enumerate(cmd):
    if a.startswith('metadata='):cmd[i]=a+'_paired_select_tests'
run(cmd,'test-build',gpu['environment'])
run([P/'gpu-tests','paired_selects','--test-threads=1'],'new-paired-selects-tests')
run([P/'gpu-tests','scalar_selects','--test-threads=1'],'scalar-select-tests')
workflow=(S/'.github/workflows/ci-full.yml').read_text().split('- name: Inventory and execute native composition contracts',1)[1].split('- name:',1)[0]
selection=workflow.split('--lib --',1)[1].split('--list',1)[0].replace('\\',' ').split();listed=run([P/'gpu-tests',*selection,'--list'],'ci-list')
names=[x.removesuffix(': test')for x in listed.stdout.splitlines()if x.endswith(': test')];prior=json.loads(Path('/private/tmp/mech-related-positional-metadata-tests-20261107-v1/ci-list.json').read_text())
old_names=[x.removesuffix(': test')for x in prior['stdout'].splitlines()if x.endswith(': test')]
assert set(old_names)<=set(names)
assert all('paired_select_source'in x for x in set(names)-set(old_names))
assert len(names)==401
executed=run([P/'gpu-tests',*selection,'--test-threads=1'],'ci-execute');passed=[x.removeprefix('test ').removesuffix(' ... ok')for x in executed.stdout.splitlines()if x.startswith('test ')and x.endswith(' ... ok')];assert sorted(names)==sorted(passed)
(P/'summary.json').write_text(json.dumps(dict(paired_selects2_passed=True,scalar_select_tests_passed=True,actual_CI_names_exact=True,actual_CI_passed=len(passed)),indent=2)+'\n')
