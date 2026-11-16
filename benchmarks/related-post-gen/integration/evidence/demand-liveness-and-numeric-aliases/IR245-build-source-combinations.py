from pathlib import Path
import json,subprocess,time,shutil
U=Path(__file__).parent
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O=Path('/private/tmp/mech-related-native-numeric-aliases-opt-20261107-v1')
def run(c,p):
 c=list(map(str,c));t=time.monotonic();x=subprocess.run(c,capture_output=True,text=True);p.write_text(json.dumps(dict(command=c,exit=x.returncode,seconds=time.monotonic()-t,stdout=x.stdout,stderr=x.stderr),indent=2)+'\n');assert x.returncode==0,x.stdout+x.stderr
for w in [4,8]:
 p=U/f'compact-reuse-{w}'
 run([O/'compiler',p/'source.mec',p/'build'],p/'compile-run.json')
 assert (p/'source.mec').read_bytes()==(p/'build/source.mec').read_bytes()
 shutil.copyfile(U/'reuse-4/runner.rs',p/'runner.rs')
 run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-run.json')
 run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
 run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',p/'edges'],p/'edge-run.json')
 metadata=json.loads((p/'build/metadata.json').read_text());assert (metadata['typed_ir_version'],metadata['abi_version'],metadata['work_policy_version'])==(245,41,115)
 print('compact-reuse',w,'native audit and31 fixtures exact','code',metadata['code_bytes'],flush=True)
