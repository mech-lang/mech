"""Recompute same-compiler closure comparisons and check exact artifact bindings."""
from pathlib import Path
from hashlib import sha256
from statistics import median
import gzip,json,math,random
HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(p.read_text())
close=lambda a,b:math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-12)
CONTROL='mech-control-reference';ROLE='mech-sparse-rounds'
for name in ['eager-seed','lazy-index']:
 folder=HERE/name
 assert folder.exists()
 plan=read(folder/'build-plan.json');summary=read(folder/'build-summary.json')
 raw=json.loads(gzip.decompress((folder/'build-comparison.json.gz').read_bytes()))
 assert sha256((folder/'build-plan.json').read_bytes()).hexdigest()==summary['plan_sha256']==raw['plan_sha256']
 actors=plan['actors'];rows=raw['observations'];assert len(rows)==8
 for order,row in zip(plan['orders'],rows):assert list(row)==order and set(row)==set(actors)
 for position in range(len(actors)):
  assert all(sum(order[position]==actor for order in plan['orders'])==2 for actor in actors)
 assert actors[CONTROL]['compiler_sha256']==actors[ROLE]['compiler_sha256']==summary['compiler_sha256']
 assert actors[CONTROL]['flags']['MECH_GUARD_SPARSE_ROUNDS']=='0' and actors[ROLE]['flags']['MECH_GUARD_SPARSE_ROUNDS']=='1'
 assert {k:v for k,v in actors[CONTROL]['flags'].items() if k!='MECH_GUARD_SPARSE_ROUNDS'}=={k:v for k,v in actors[ROLE]['flags'].items() if k!='MECH_GUARD_SPARSE_ROUNDS'}
 assert actors[CONTROL]['artifact_sha256']==actors[ROLE]['artifact_sha256']==summary['artifact_sha256']
 assert summary['artifact_sha256']=={'encoder':'c5aab01c69ff4ed325b5149d2e1402a59477e5858f52052939d51f7eff699ef8','metadata':'c187c6f40d81d45995eaf3313c3282d41aadbd066413d7f44c0775b1c22225fa','bytecode':'c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3'}
 for actor,values in summary['medians'].items():
  for field,value in values.items():assert close(median(row[actor]['seconds'][field] for row in rows),value)
  assert close(summary['source_cpu_seconds'][actor],median(sum(row[actor]['phase_cpu_seconds']['source'].values()) for row in rows))
  assert summary['source_rss_bytes'][actor]==median(row[actor]['phase_peak_rss_bytes']['source'] for row in rows)
  for row in [raw['warmup'],*rows]:
   assert row[actor]['stripped_executable']==summary['stripped_executables'][actor]
   if actor in [CONTROL,ROLE]:assert row[actor]['generated_artifacts']==summary['artifact_sha256']
 for actor,values in summary['paired_ratios'].items():
  for peer,value in values.items():assert close(value,median(row[actor]['seconds']['source_through_link']/row[peer]['seconds']['source_through_link'] for row in rows))
 for actor,values in summary['phases'].items():
  for phase,value in values.items():
   assert close(value,median(next(p['seconds'] for p in row[actor]['compiler_phases'] if p['phase']==phase) for row in rows))
 costs=[(row[ROLE]['seconds']['source_through_link']-row[CONTROL]['seconds']['source_through_link'])*1000 for row in rows]
 assert costs==summary['paired_ms_cost'] and median(costs)==summary['paired_median_ms_cost']
 assert sum(cost<0 for cost in costs)==summary['faster']
 rng=random.Random(summary['seed']);resamples=sorted(median(rng.choices(costs,k=8)) for _ in range(summary['bootstrap_resamples']))
 assert [resamples[999],resamples[18999]]==summary['cost_90pct_interval_ms']
 source=read(folder/'source-rebuild.json')
 assert source['object_sha256']==summary['artifact_sha256']['encoder'] and source['metadata_sha256']==summary['artifact_sha256']['metadata']
 assert source['compiler_sha256']==summary['compiler_sha256'] and source['object_text_bytes']==63804
 assert source['verified_exact_calls']==162 and source['checked_failure_probe']['cases']==6 and source['checked_failure_probe']['exact_recoveries']==12
 for file,count in [('source-correctness.json.gz',162),('source-checked-recoveries.json.gz',12)]:
  observations=json.loads(gzip.decompress((folder/file).read_bytes()));assert len(observations)==count
  assert all(row['exact_reference'] and row['independent_inflation'] and row['borrowed_output_alias_unchanged'] and row['input_alias_unchanged'] and row['nonreturned_private_bytes_at_return']==0 for row in observations)
 tests=read(folder/'test-result.json');assert tests['passed']==29 and tests['failed']==0
 print('%s: verified exact 63,804 B artifacts, source checks, 29 tests and eight balanced builds; paired cost %.6f ms.'%(name,median(costs)))

status=read(HERE/'status.json');pin=read(HERE/'candidate.json');source=read(HERE/'source-rebuild.json')
assert status['recipe']==source['variant']=='m1-sparse-guard-rounds'
assert source['source_recipe']['manifest_sha256']==sha256((HERE/'candidate.json').read_bytes()).hexdigest()
assert source['compiler_sha256']==pin['compiler_sha256']==status['compiler_sha256']
assert source['full_integrated_source_identity'] and source['object_text_bytes']==63804 and source['verified_exact_calls']==162
assert source['checked_failure_probe']['cases']==6 and source['checked_failure_probe']['exact_recoveries']==12
identity=read(HERE/'lazy-index/worker-identity.json')
assert identity['reproduces_qualified_fixed_support_worker']
assert identity['worker_sha256']==pin['worker_sha256']==status['worker_sha256']=='5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0'
assert source['worker_sha256']==status['named_source_checks']['worker_sha256']!=identity['worker_sha256']
old=read(HERE.parent/'preserved-root-registers/candidate.json')
assert pin['artifact_sha256']==old['artifact_sha256']
assert pin['override_flags']==old['override_flags']|{'MECH_GUARD_SPARSE_ROUNDS':'1'}
full=json.loads(gzip.decompress((HERE.parent/'preserved-root-registers/full-replay.json.gz').read_bytes()))
assert full['decision']=='Retain' and {run['order'] for run in full['runs']}=={'forward','reverse'}
for run in full['runs']:
 for peer,count in [('roc',81),('libdeflate',62)]:
  assert run['peers'][peer]['point_estimate_wins']['mech-experiment']==run['peers'][peer]['precision_supported_wins']['mech-experiment']==count
print('Verified named source recipe, fresh host-support checks, and exact identity with the fully qualified 81/62 fixed-support worker.')
