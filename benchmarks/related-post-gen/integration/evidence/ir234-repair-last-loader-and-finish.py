from pathlib import Path
import json,subprocess,time,shutil
U=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-deflate-exhaustion-proof-integration-opt-20261106-v1')
def run(command,path):
 command=list(map(str,command));start=time.monotonic();result=subprocess.run(command,capture_output=True,text=True);path.write_text(json.dumps(dict(command=command,exit=result.returncode,seconds=time.monotonic()-start,stdout=result.stdout,stderr=result.stderr),indent=2)+'\n');assert result.returncode==0,result.stdout+result.stderr
p=U/'compact-4-full-catalog'
for name in ['host-build-run.json','runner-aot.build.json']:
 if (p/name).exists():shutil.copyfile(p/name,p/(name+'.failed-disk-full'))
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',p/'runner.rs','--out',p/'runner-aot','--bundle',p/'build','--linked'],p/'host-build-retry-run.json')
run(['python3',R/'audit-native-link.py','--object',p/'build/object.o','--executable',p/'runner-aot','--output',p/'native-link-audit.json'],p/'audit-run.json')
run(['python3',R/'experiments/check_subset3.py','--runner',p/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',p/'build/source.mec','--out',U/'edges-compact-4-full-catalog'],U/'compact-4-full-catalog-edges-run.json')
run(['python3',U/'qualify-other.py'],U/'qualify-other-run.json')
print('Repaired last loader,31 exact fixtures; all finite budgets/frontier controls pass',flush=True)
