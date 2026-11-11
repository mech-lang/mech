from pathlib import Path
import json,gzip,hashlib,os,shutil,time
R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results');A=R/'raw-output-copies-20261105';A.mkdir(exist_ok=True);B=A/'objects';B.mkdir(exist_ok=True)
log=A/'manifest.jsonl';known=set();freed=0;count=0
previous={json.loads(line)['path']for line in log.read_text().splitlines()}
sha=lambda b:hashlib.sha256(b).hexdigest()
# Completed benchmark output copies only: original full result reports, binaries,
# inputs, oracles, receipts and all user sources remain at their existing paths.
phases=[Path('/private/tmp/mech-related-native-fixed-frame-arenas-runners-20261106-v2')/name for name in ['latency','confirmation-latency','resources']]
for phase in phases:
 receipt=phase/'run.json'
 if not receipt.exists():continue
 try:
  run=json.loads(receipt.read_text());cmd=run['command'];report=Path(cmd[cmd.index('--output')+1]);j=json.loads(report.read_text())
  if run['exit']!=0 or not all(x['exit']==0 and x.get('exact_rust_output') for x in j['samples']):continue
 except (ValueError,KeyError,json.JSONDecodeError,FileNotFoundError):continue
 expected={f"{x['variant']}-{x['sample']}.json":x['output_sha256'] for x in j['samples']};hashes=set(expected.values());phase_freed=0;phase_count=0
 candidates=[p for p in phase.glob('*.json') if p.name in expected or p.name.startswith('related_posts_')]
 with log.open('a') as out:
  for p in candidates:
   assert str(p)not in previous
   raw=p.read_bytes();digest=sha(raw)
   if p.name in expected:assert digest==expected[p.name],str(p)
   elif digest not in hashes:continue
   blob=B/(digest+'.json.gz')
   if digest not in known:
    if not blob.exists():
     compressed=gzip.compress(raw,compresslevel=6,mtime=0);blob.write_bytes(compressed)
     with blob.open('rb') as f:os.fsync(f.fileno())
    assert sha(gzip.decompress(blob.read_bytes()))==digest
    known.add(digest)
   row=dict(path=str(p),bytes=len(raw),sha256=digest,blob=str(blob.relative_to(A)),report=str(report),verified_exact_output=True,restore='Decompress the blob to path; verify sha256 before use.')
   out.write(json.dumps(row)+'\n');out.flush();os.fsync(out.fileno())
   p.unlink();freed+=len(raw);count+=1;phase_freed+=len(raw);phase_count+=1
 print(json.dumps(dict(phase=str(phase),preserved_output_copies=phase_count,raw_bytes_compacted=phase_freed,total_raw_bytes_compacted=freed)),flush=True)
print(json.dumps(dict(done=True,copies=count,raw_bytes_compacted=freed,free_bytes=shutil.disk_usage('/private/tmp').free,manifest=str(log))),flush=True)
