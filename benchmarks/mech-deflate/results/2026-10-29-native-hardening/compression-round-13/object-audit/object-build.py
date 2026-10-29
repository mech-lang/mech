from pathlib import Path
import json,subprocess,hashlib,time
r=Path(__file__).resolve().parent
b=json.loads((r/'primary/current-build.json').read_text());c=b['command'][:-3]
i=c.index('-C',c.index('-C')+2);del c[i:i+2]
c += [str(r/'object-evidence.rs'),'-o',str(r/'object-evidence')]
a=time.monotonic();p=subprocess.run(c,capture_output=True,text=True)
(r/'object-build.log').write_text(p.stdout+p.stderr);(r/'object-build.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-a,'libraries':b['libraries']},indent=2)+'\n');p.check_returncode()
c=[str(r/'object-evidence'),'/private/tmp/mech-linear-restack-20261026/complete-compressor-borrowed-caller.mbc',str(r/'complete-object')]
a=time.monotonic();p=subprocess.run(c,capture_output=True,text=True,timeout=240)
(r/'object-run.stdout').write_text(p.stdout);(r/'object-run.stderr').write_text(p.stderr);(r/'object-run.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-a,'binary_sha256':hashlib.sha256((r/'object-evidence').read_bytes()).hexdigest()},indent=2)+'\n');print(p.stdout+p.stderr);p.check_returncode()
