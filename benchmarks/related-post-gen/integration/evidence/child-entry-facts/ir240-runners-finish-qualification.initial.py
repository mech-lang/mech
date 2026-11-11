from pathlib import Path
import json,subprocess,time
U=Path(__file__).parent
for name in ['qualify-loaders.py','qualify-new-source-loaders.py','qualify-other.py','validate-base-qualification.py']:
 c=['python3',str(U/name)];t=time.monotonic();r=subprocess.run(c,capture_output=True,text=True);(U/(name.removesuffix('.py')+'-driver-run.json')).write_text(json.dumps(dict(command=c,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');print(name,r.returncode,flush=True);assert r.returncode==0,r.stdout+r.stderr
