from pathlib import Path
import json,subprocess,time
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen');O=Path('/private/tmp/mech-related-restore-instruction-phases-opt-20261107-v3')
def run(cmd,name):
 t=time.monotonic();cmd=list(map(str,cmd));r=subprocess.run(cmd,capture_output=True,text=True);(P/(name+'.json')).write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout[-4000:]+r.stderr[-4000:]
run(['python3',R/'build-host.py','--compiler-build',O/'build.json','--source',P/'runner.rs','--out',P/'runner-phases','--bundle',P/'build','--linked'],'probe-build')
run(['python3',R/'audit-native-link.py','--object',P/'build/object.o','--executable',P/'runner-phases','--output',P/'native-link-proof.json'],'native-link-audit')
