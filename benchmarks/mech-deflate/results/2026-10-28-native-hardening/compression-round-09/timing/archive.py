from pathlib import Path
import gzip,hashlib,json,shutil,subprocess
root=Path(__file__).resolve().parent
w=Path('/private/tmp/mech-deflate-linear-stack')
dest=w/'benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-09'
assert not dest.exists();(dest/'timing').mkdir(parents=True);(dest/'qualification').mkdir()
for p in sorted(root.rglob('*')):
 if not p.is_file() or p.name=='REPORT.md':continue
 rel=p.relative_to(root)
 if p.suffix in ['.json','.py']:
  out=dest/'timing'/rel;out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,out)
 elif p.suffix in ['.stdout','.stderr','.log','.rs','.mec','.mbc'] or p.name.endswith('.sample.txt'):
  out=dest/'timing'/str(rel.parent)/(rel.name+'.gz');out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
v=Path('/private/tmp/mech-linear-restack-20261026/validation')
for p in sorted(v.glob('round09-*.json')):
 shutil.copy2(p,dest/'qualification'/p.name)
 log=p.with_suffix('.log')
 if log.exists():(dest/'qualification'/(log.name+'.gz')).write_bytes(gzip.compress(log.read_bytes(),mtime=0))
shutil.copy2(root/'REPORT.md',dest/'REPORT.md')
rows=[{'path':str(p.relative_to(dest)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(dest.rglob('*')) if p.is_file()]
(dest/'manifest.json').write_text(json.dumps({'files':rows},indent=2)+'\n')
print(json.dumps({'packet':str(dest),'files':len(rows),'bytes':sum(r['bytes'] for r in rows)}))
