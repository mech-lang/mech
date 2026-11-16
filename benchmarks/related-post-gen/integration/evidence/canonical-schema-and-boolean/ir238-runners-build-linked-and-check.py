from pathlib import Path
import json,hashlib,shutil,subprocess,time
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-canonical-schema-sinks-opt-20261107-v1');B=Path('/private/tmp/mech-related-deflate-reduction-integration-runners-20261106-v1')
def run(command,receipt):
 command=list(map(str,command));start=time.monotonic();result=subprocess.run(command,capture_output=True,text=True)
 receipt.write_text(json.dumps(dict(command=command,exit=result.returncode,seconds=time.monotonic()-start,stdout=result.stdout,stderr=result.stderr),indent=2)+'\n')
 assert result.returncode==0,result.stdout+result.stderr
for name in ['after-4','after-8','compact-4','compact-8']:
 p=U/name;shutil.copyfile(B/name/'runner.rs',p/'runner.rs')
 run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
 run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
 run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',p/'edges'],p/'edge-run.json')
 print(name,'linked Object audited,31 exact outputs',flush=True)
for task in ['qualify-loaders.py','qualify-other.py']:
 run(['python3',U/task],U/(task.removesuffix('.py')+'-run.json'))
 print(task,'passed',flush=True)
