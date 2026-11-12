from pathlib import Path
import json,subprocess,time
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-deflate-exhaustion-proof-integration-opt-20261106-v1')
def run(command,receipt):
    command=list(map(str,command));start=time.monotonic()
    result=subprocess.run(command,capture_output=True,text=True)
    receipt.write_text(json.dumps(dict(command=command,exit=result.returncode,seconds=time.monotonic()-start,stdout=result.stdout,stderr=result.stderr),indent=2)+'\n')
    assert result.returncode==0,result.stdout+result.stderr
    print(receipt.name,'passed',flush=True)
for name in ['qualify-budgets.py','qualify-compact-budgets.py']:
    run(['python3',U/name],U/(name.replace('.py','-run.json')))
P=U/'frontier-image-probe';(P/'build').symlink_to(U/'after-4/build',target_is_directory=True)
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'probe.rs','--out',P/'probe','--bundle',P/'build','--linked'],P/'build-run.json')
run(['python3',R/'audit-native-link.py','--object',P/'build/object.o','--executable',P/'probe','--output',P/'native-link-audit.json'],P/'audit-run.json')
run([P/'probe'],P/'run.json')
row=json.loads(json.loads((P/'run.json').read_text())['stdout'])
assert row['genuine_legacy_independent_bound_and_compiled_module_accepted'] and len(row['forged_cases'])==8 and row['native_entry_never_invoked']
print('Eight forged capacity-frontier cases rejected by both independent and Object-bound loaders',flush=True)
