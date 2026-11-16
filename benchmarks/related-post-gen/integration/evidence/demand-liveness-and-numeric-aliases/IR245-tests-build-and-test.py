from pathlib import Path
import json,os,subprocess,time,hashlib,importlib.util
T=Path(__file__).parent;O=Path('/private/tmp/mech-related-native-numeric-aliases-opt-20261107-v1');R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
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
for case in ['numeric_array_alias','numeric_first_change','numeric_shadow','cfg_cleanup','unsigned_guard_facts']:
 assert run([cmd[-1],case,'--test-threads=1','--nocapture'],'focus-'+case)==0
full_exit=run([cmd[-1],'--test-threads=1'],'test-run')
text=(T/'test-run.stdout.log').read_text()
expected=['batched::axis_tests::legacy_matrix_multiply_artifacts_remain_gpu_lowerable','native_artifact::program::runtime::collections::text_join::compiled_map::owned_tests::complete_unchanged_knucleotide_body_uses_the_owned_emitted_map']
failed=[line.split(' ... FAILED')[0].removeprefix('test ') for line in text.splitlines() if line.startswith('test ') and line.endswith(' ... FAILED')]
receipt=json.loads((T/'test-run.json').read_text());receipt.update(summary=[line for line in text.splitlines() if line.startswith('test result:')],expected_failures_exact=sorted(failed)==sorted(expected),failed_tests=failed)
(T/'test-run.json').write_text(json.dumps(receipt,indent=2)+'\n')
receipt['new_failures']=[name for name in failed if name not in expected]
(T/'test-run.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(dict(summary=receipt['summary'],new_failures=receipt['new_failures'])),flush=True)

(T/'source-inventory.json').write_bytes((O/'source-inventory.json').read_bytes())

assert full_exit == 101 and receipt["expected_failures_exact"], receipt
assert receipt["summary"], "test process ended before its summary"

import re
workflow=(Path(d["source"])/".github/workflows/ci-full.yml").read_text()
section=workflow.split("- name: Inventory and execute native composition contracts",1)[1].split("- name:",1)[0]
selection=section.split("--lib -- ",1)[1].split("--list",1)[0].replace("\\", " ").split()
assert selection and selection[0] == "composition_tests", selection
assert run([cmd[-1],*selection,"--list"],"ci-selected-list")==0
names=[line.removesuffix(": test") for line in (T/"ci-selected-list.stdout.log").read_text().splitlines() if line.endswith(": test")]
expected_count=int(re.search(r"-eq (\d+)",section).group(1))
assert len(names)==len(set(names))==expected_count,(len(names),expected_count)
assert run([cmd[-1],*selection,"--test-threads=1"],"ci-selected-execute")==0
output=(T/"ci-selected-execute.stdout.log").read_text()
passed=[line[len("test "):].split(" ... ")[0] for line in output.splitlines() if line.startswith("test ") and line.endswith(" ... ok")]
assert sorted(passed)==sorted(names),(len(passed),len(names))
(T/"ci-selected-inventory-audit.json").write_text(json.dumps(dict(expected=expected_count,listed=len(names),executed_passed=len(passed),all_names_match=True,names=names),indent=2)+"\n")
print("CI actual inventory and execution",len(passed),flush=True)
