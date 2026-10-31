from pathlib import Path
import gzip,hashlib,json,shutil
r=Path(__file__).resolve().parent
w=Path('/private/tmp/mech-deflate-linear-stack')
d=w/'benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-11'
assert not d.exists();d.mkdir(parents=True)
excluded=[]
def copy_tree(root,dest):
 for p in sorted(root.rglob('*')):
  if not p.is_file():continue
  rel=p.relative_to(root)
  if p.name=='REPORT.md' or p.name=='environment.json':continue
  out=dest/rel
  if p.name=='complete-disassembly.txt.gz':
   excluded.append({'path':str(p),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'reason':'Full listing remains private; checked-in excerpts, source/audit recipe and object/listing hashes suffice.'});continue
  if p.suffix in ['.json','.py']:
   out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,out)
  elif p.suffix in ['.stdout','.stderr','.log','.rs','.mec','.mbc','.txt']:
   out=Path(str(out)+'.gz');out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
copy_tree(r,d/'observations')
copy_tree(Path('/private/tmp/mech-readonly-numeric-round10-20261028'),d/'pre-implementation')
v=Path('/private/tmp/mech-linear-restack-20261026/validation')
for p in sorted(v.glob('round11-*.json')):
 out=d/'qualification'/p.name;out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,out)
 log=p.with_suffix('.log')
 if log.exists():(out.parent/(log.name+'.gz')).write_bytes(gzip.compress(log.read_bytes(),mtime=0))
env=json.loads((r/'environment.json').read_text())
env['metadata']=[x for x in env['metadata'] if x['command']!=['uname','-a']]
(d/'environment.json').write_text(json.dumps(env,indent=2)+'\n')
(d/'excluded-large-artifacts.json').write_text(json.dumps(excluded,indent=2)+'\n')
shutil.copy2(r/'REPORT.md',d/'REPORT.md')
rows=[{'path':str(p.relative_to(d)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(d.rglob('*')) if p.is_file()]
(d/'manifest.json').write_text(json.dumps({'files':rows},indent=2)+'\n')
for row in rows:
 p=d/row['path'];assert p.stat().st_size==row['bytes'] and hashlib.sha256(p.read_bytes()).hexdigest()==row['sha256']
print(json.dumps({'packet':str(d),'files':len(rows),'bytes':sum(row['bytes'] for row in rows)}))
