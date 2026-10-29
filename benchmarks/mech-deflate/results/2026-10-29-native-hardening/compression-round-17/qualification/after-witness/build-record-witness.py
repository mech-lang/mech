from pathlib import Path
import json,subprocess,time,hashlib
r=Path(__file__).resolve().parent
b=json.loads(Path('/private/tmp/mech-binding-descriptors-round17-20261029/primary/current-build.json').read_text())
c=b['command'][:-3];i=c.index('-C',c.index('-C')+2);del c[i:i+2]
c += [str(r/'record-witness.rs'),'-o',str(r/'record-witness')]
a=time.monotonic();p=subprocess.run(c,capture_output=True,text=True);(r/'record-witness-build.log').write_text(p.stdout+p.stderr);(r/'record-witness-build.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-a},indent=2)+'\n');print(p.stderr);p.check_returncode()
c=[str(r/'record-witness')];p=subprocess.run(c,capture_output=True,text=True);(r/'record-witness.stdout').write_text(p.stdout);(r/'record-witness.stderr').write_text(p.stderr);print(p.stdout[-800:]+p.stderr);p.check_returncode()
