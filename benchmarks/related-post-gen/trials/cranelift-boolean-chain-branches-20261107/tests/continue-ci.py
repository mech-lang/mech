from pathlib import Path
import json,subprocess,time
P=Path(__file__).parent
listed=json.loads((P/'ci-list.json').read_text())
names=[x.removesuffix(': test')for x in listed['stdout'].splitlines()if x.endswith(': test')]
prior=json.loads(Path('/private/tmp/mech-related-llvm-native-image-tests-20261107-v4/ci-selected-list.json').read_text())
original={x.removesuffix(': test')for x in prior['stdout'].splitlines()if x.endswith(': test')}
assert len(names)==401 and original<=set(names)
extra=set(names)-original
assert len(extra)==2 and all('cfg_cleanup::boolean_chains::tests::'in x for x in extra)
cmd=listed['command'][:-1]+['--test-threads=1'];t=time.monotonic();r=subprocess.run(cmd,capture_output=True,text=True)
(P/'ci-execute.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n')
assert r.returncode==0,r.stdout[-6000:]+r.stderr[-6000:]
passed=[x.removeprefix('test ').removesuffix(' ... ok')for x in r.stdout.splitlines()if x.startswith('test ')and x.endswith(' ... ok')]
assert sorted(names)==sorted(passed)
(P/'summary.json').write_text(json.dumps(dict(new_branch_tests_passed=True,cfg_cleanup_passed=True,actual_CI_names_exact=True,actual_CI_passed=len(passed),inherited399_all_preserved=True,added_test_names=sorted(extra),first_harness_assertion='Expected399 names before accounting for two new maintained tests; actual401 names all executed.'),indent=2)+'\n')
print('actual CI401 passed; prior399 all preserved; exact name equality',flush=True)
