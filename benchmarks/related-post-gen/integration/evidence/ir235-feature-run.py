from pathlib import Path
import json,subprocess,os,time
Q=Path(__file__).parent
for name in ['scalar-compiler']:
 d=json.loads((Q/(name+'.spec.json')).read_text());t=time.monotonic();r=subprocess.run(d['command'],capture_output=True,text=True,env={**os.environ,**d['environment']});d.update(exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);(Q/(name+'.json')).write_text(json.dumps(d,indent=2)+'\n');assert r.returncode==0,r.stderr;print(name,0,flush=True)
