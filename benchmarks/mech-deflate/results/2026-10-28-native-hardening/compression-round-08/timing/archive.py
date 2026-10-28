from pathlib import Path
import gzip,hashlib,json,shutil
root=Path(__file__).resolve().parent
w=Path('/private/tmp/mech-deflate-linear-stack')
dest=w/'benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-08'
assert not dest.exists();(dest/'timing').mkdir(parents=True);(dest/'qualification').mkdir()
sc=json.loads((root/'scalar-results.json').read_text())
shutil.copy2('/private/tmp/mech-live-control-target/release/native_frame_transport',root/'frame-after')
assert hashlib.sha256((root/'frame-after').read_bytes()).hexdigest()==sc['after_sha256']
(root/'frame-after.json').write_text(json.dumps({'sha256':sc['after_sha256'],'compiler':'4af4662097dd570f0821fc105a1c4d9de4eb5d89'},indent=2)+'\n')
(root/'error-parent/provenance-correction.json').write_text(json.dumps({'first_test_attempt':'Earlier Cargo library paths were reused by the 7692a445 build; this is not clean-parent evidence. Keep the original receipt and treat it as candidate failure evidence.','replacement':'retained-reproduction.json uses hash-checked retained before/after executables and identical source-derived bytecode.','before_executable_compiler':'51eeb3f5fc5390b7b1b5213870b60ed3c577d182; production identical to published 8c98f465','artifact_producer_compiler':'4af4662097dd570f0821fc105a1c4d9de4eb5d89; frontend/codec and canonical artifact producers unchanged since 8c98f465','primary_timing_claim':False},indent=2)+'\n')
for p in sorted(root.rglob('*')):
 if not p.is_file():continue
 rel=p.relative_to(root)
 if p.name=='REPORT.md':continue
 if p.suffix in ['.json','.py']:
  out=dest/'timing'/rel;out.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,out)
 elif p.suffix in ['.stdout','.stderr','.log','.rs','.mec','.mbc'] or p.name.endswith('.sample.txt'):
  out=dest/'timing'/str(rel.parent)/(rel.name+'.gz');out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(gzip.compress(p.read_bytes(),mtime=0))
v=Path('/private/tmp/mech-linear-restack-20261026/validation')
for p in sorted(v.glob('round08-*.json')):
 shutil.copy2(p,dest/'qualification'/p.name)
 log=p.with_suffix('.log')
 if log.exists():(dest/'qualification'/(log.name+'.gz')).write_bytes(gzip.compress(log.read_bytes(),mtime=0))
shutil.copy2(root/'REPORT.md',dest/'REPORT.md')
rows=[{'path':str(p.relative_to(dest)),'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(dest.rglob('*')) if p.is_file()]
(dest/'manifest.json').write_text(json.dumps({'files':rows},indent=2)+'\n')
print(json.dumps({'packet':str(dest),'files':len(rows),'bytes':sum(r['bytes'] for r in rows)}))
