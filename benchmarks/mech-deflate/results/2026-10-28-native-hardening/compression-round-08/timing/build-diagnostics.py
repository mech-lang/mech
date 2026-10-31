from pathlib import Path
import hashlib,json,subprocess,time
root=Path(__file__).resolve().parent
build=json.loads((root/'current-build.json').read_text())
base=build['command'][:]
i=base.index('-C',base.index('-C')+2)
assert base[i+1].startswith('link-arg=');del base[i:i+2]
base=base[:-3]
rows=[]
for name in ['produce','inventory']:
 command=base+[str(root/(name+'.rs')),'-o',str(root/name)]
 start=time.monotonic();r=subprocess.run(command,capture_output=True,text=True)
 (root/(name+'-build.log')).write_text(r.stdout+r.stderr)
 rows.append({'command':command,'exit':r.returncode,'seconds':time.monotonic()-start,'libraries':build['libraries'],'binary_sha256':hashlib.sha256((root/name).read_bytes()).hexdigest() if r.returncode==0 else None})
 (root/'diagnostic-builds.json').write_text(json.dumps(rows,indent=2)+'\n');r.check_returncode()
 print(name,'built',flush=True)
