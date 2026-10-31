from pathlib import Path
import hashlib,json,subprocess,time
r=Path(__file__).resolve().parent
b=json.loads(Path('/private/tmp/mech-nested-frames-round20-20261029/final-primary/current-build.json').read_text())
c=b['command'][:-3];i=c.index('-C',c.index('-C')+2);del c[i:i+2];c+= [str(r/'audit.rs'),'-o',str(r/'audit')]
t=time.monotonic();p=subprocess.run(c,capture_output=True,text=True)
(r/'audit-build.log').write_text(p.stdout+p.stderr);(r/'audit-build.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-t,'libraries':b['libraries'],'source_sha256':hashlib.sha256((r/'audit.rs').read_bytes()).hexdigest()},indent=2)+'\n');print(p.stderr[-2000:]);p.check_returncode()
c=[str(r/'audit')];t=time.monotonic();p=subprocess.run(c,capture_output=True,text=True,timeout=120)
(r/'audit.json').write_text(p.stdout);(r/'audit.stderr').write_text(p.stderr);(r/'audit-run.json').write_text(json.dumps({'command':c,'exit':p.returncode,'seconds':time.monotonic()-t,'binary_sha256':hashlib.sha256((r/'audit').read_bytes()).hexdigest()},indent=2)+'\n');print(p.stderr[-2000:]);p.check_returncode()
