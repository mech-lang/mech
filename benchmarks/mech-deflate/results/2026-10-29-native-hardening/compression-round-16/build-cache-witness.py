from pathlib import Path
import json,subprocess,time,hashlib
r=Path(__file__).resolve().parent
b=json.loads(Path('/private/tmp/mech-predicate-versions-round15-20261029/primary/current-build.json').read_text())
c=b['command'][:-3];i=c.index('-C',c.index('-C')+2);del c[i:i+2]
c += [str(r/'cache-witness.rs'),'-o',str(r/'cache-witness')]
a=time.monotonic();p=subprocess.run(c,capture_output=True,text=True);(r/'cache-witness-build.log').write_text(p.stdout+p.stderr);(r/'cache-witness-build.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-a},indent=2)+'\n');print(p.stderr);p.check_returncode()
c=[str(r/'cache-witness')];p=subprocess.run(c,capture_output=True,text=True);(r/'cache-witness.stdout').write_text(p.stdout);(r/'cache-witness.stderr').write_text(p.stderr);print(p.stdout+p.stderr);p.check_returncode()
