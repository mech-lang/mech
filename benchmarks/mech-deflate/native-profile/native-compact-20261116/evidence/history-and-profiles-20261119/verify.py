"""Check capture coverage, identities, sample accounting and the unchanged qualified reference."""
from pathlib import Path
from hashlib import sha256
import gzip,json,math,runpy
HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(p.read_text())
blob=lambda p:gzip.decompress(p.read_bytes())
sha=lambda b:sha256(b).hexdigest()
for name,pin in read(HERE/'files.json').items():assert sha((HERE/name).read_bytes())==pin,name
plan=read(HERE/'plan.json');summary=read(HERE/'summary.json');history=read(HERE/'history.json');complete=read(HERE/'complete.json')
assert len(history['entries'])==19
assert (complete['profiles'],complete['named_cases'],complete['distinct_contracts'])==(36,19,18)
assert len(summary['ranking_by_absolute_gap'])==19 and len(summary['profiles'])==18
assert not summary['selected_worker_changed'] and (summary['roc_floor'],summary['libdeflate_floor'])==(81,62)
actors=plan['actors'];assert actors['mech']['sha256']=='5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0'
assert sha(blob(HERE/'actors/mech-worker.gz'))==actors['mech']['sha256']
assert sha(blob(HERE/'actors/libdeflate-worker.gz'))==actors['libdeflate']['sha256']
covered=set();minimum=10**9
for group in plan['distinct_contracts']:
 covered.update(group['aliases']);case=group['case']
 for actor in ['mech','libdeflate']:
  folder=HERE/'profiles'/case/actor;r=read(folder/'result.json');stats=summary['profiles'][case][actor]
  assert r['sample_exit_code']==0 and r['worker_sha256']==actors[actor]['sha256']
  assert sha(blob(folder/'sample.txt.gz'))==r['sample_sha256']==stats['sample_sha256']
  calls=r['verified_calls'];assert len(calls)==2 and all(c['case']==case for c in calls)
  assert all(all(c[k] for k in ['independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for c in calls)
  if actor=='mech':assert all(c['exact_reference'] for c in calls)
  else:assert all(c['level']==12 and c['version']=='1.26' for c in calls)
  assert sum(b['samples'] for b in stats['buckets'].values())==stats['encoder_stack_samples']
  minimum=min(minimum,stats['encoder_stack_samples'])
assert len(covered)==19 and covered=={r['case'] for r in summary['ranking_by_absolute_gap']}
assert minimum>=1800
for row in summary['ranking_by_absolute_gap']:
 assert row['profile_contract'] in summary['profiles']
 for order in ['forward','reverse']:
  times=row['median_seconds'][order];assert math.isclose(row['excess_seconds'][order],times['mech-experiment']-times['libdeflate'],abs_tol=1e-15)
counts=json.loads(blob(HERE/'matcher-counts/counts.json.gz'))
assert counts['object_sha256']=='c5aab01c69ff4ed325b5149d2e1402a59477e5858f52052939d51f7eff699ef8'
assert counts['symbol']=='_mech_compact_numeric_475_vector'
r=counts['cases']['prefix-random-1048576'];assert r['calls']==35
assert (r['counts']['instructions'],r['counts']['branches'],r['counts']['stack_loads'],r['counts']['stack_stores'])==(245026411,45024643,25947639,9698800)
assert len(read(HERE/'idle-attempt/prefix-random-8192/libdeflate/result.json')['verified_calls'])==2
packet=HERE.parents[1]
for entry in history['entries']:assert sha((packet/'evidence'/entry['evidence']).read_bytes())==entry['evidence_sha256']
print('Verified 19 reviewed mechanisms, all 19 losing names / 18 contracts, 36 useful Mech/C captures, pinned workers, exact matcher counts and unchanged 81/62 reference. No changed-worker performance claim.')
