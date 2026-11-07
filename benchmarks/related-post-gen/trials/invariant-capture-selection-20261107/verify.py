from pathlib import Path
import gzip,hashlib,json,statistics
P=Path(__file__).resolve().parent
m=json.loads((P/'manifest.json').read_text())
for name,v in m['files'].items():
 b=(P/name).read_bytes();assert len(b)==v['bytes'] and hashlib.sha256(b).hexdigest()==v['sha256'],name
 raw=gzip.decompress(b)if name.endswith('.gz')else b
 assert len(raw)==v['decoded_bytes'] and hashlib.sha256(raw).hexdigest()==v['decoded_sha256'],name
read=lambda name:json.loads(gzip.decompress((P/name).read_bytes())if name.endswith('.gz')else(P/name).read_bytes())
t=read('trial.json');a=read('latency.json.gz');b=read('resources.json.gz')
assert len(a['samples'])==372 and len(b['samples'])==91
assert all(x['exit']==0 and x['exact_rust_output']for x in a['samples']+b['samples'])
rows={(x['sample'],x['variant']):x for x in a['samples']};assert len(rows)==372
for name,v in t['trials'].items():
 passed=True
 for field in ['post_json_seconds','wall_seconds']:
  deltas=[x[field]-rows[(x['sample'],'mech-ir242-reuse-4')][field]for x in a['samples']if x['variant']==name]
  assert len(deltas)==31
  wins=sum(x < 0 for x in deltas);assert wins==v['paired_vs_selected'][field]['wins']
  assert abs(statistics.median(deltas)-v['paired_vs_selected'][field]['median_paired_delta_seconds'])<1e-12
  passed=passed and wins>=21
 assert passed==v['passes_joint_timing_gate']
assert not t['selected_changed'] and not t['candidate_selected']
print(json.dumps(dict(files=len(m['files']),latency_exact=372,resources_exact=91,paired_gates_verified=True,selected_changed=False)))
