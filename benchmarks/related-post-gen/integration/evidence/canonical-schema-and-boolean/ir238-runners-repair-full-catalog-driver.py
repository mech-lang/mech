from pathlib import Path
import json,subprocess,time
U=Path(__file__).parent;P=U/'reuse-4-full-catalog';R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
def run(command,path):
 command=list(map(str,command));start=time.monotonic();r=subprocess.run(command,capture_output=True,text=True);path.write_text(json.dumps(dict(command=command,exit=r.returncode,seconds=time.monotonic()-start,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==0,r.stdout+r.stderr;print(path.name,'passed',flush=True)
receipt=json.loads((P/'runner-aot.build.json').read_text());assert receipt['exit']==0 and receipt['compiler_exit']==0 and receipt['executable_validation_error']is None
(P/'retry-host-build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
run(['python3',R/'audit-native-link.py','--object',P/'build/object.o','--executable',P/'runner-aot','--output',P/'native-link-audit.json'],P/'retry-audit-run.json')
run(['python3',R/'experiments/check_subset3.py','--runner',P/'runner-aot','--rust','/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust','--source',P/'build/source.mec','--out',U/'edges-reuse-4-full-catalog'],P/'retry-edge-run.json')
print('Rebuilt full-catalog loader validates native link and all31 exact edges; original corrupt artifact/receipts retained',flush=True)
