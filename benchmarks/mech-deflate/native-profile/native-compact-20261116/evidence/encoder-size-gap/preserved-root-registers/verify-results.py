"""Recompute matched-build statistics and verify the complete runtime receipts."""
from pathlib import Path
from hashlib import sha256
from statistics import median
import gzip,json,random
here=Path(__file__).resolve().parent
read=lambda p:json.loads(p.read_text())
s=read(here/'deployment.json');raw=json.loads(gzip.decompress((here/'build-comparison.json.gz').read_bytes()));plan=read(here/'build-plan.json')
assert sha256((here/'build-plan.json').read_bytes()).hexdigest()==raw['plan_sha256']==s['plan_sha256']
rows=raw['observations'];assert len(rows)==s['trials']==8
for order,row in zip(plan['orders'],rows):assert list(row)==order and set(row)==set(plan['actors'])
for position in range(4):
 assert all(sum(order[position]==actor for order in plan['orders'])==2 for actor in plan['actors'])
for actor,values in s['medians'].items():
 assert values=={name:median(row[actor]['seconds'][name] for row in rows) for name in values}
 assert s['source_cpu_seconds'][actor]==median(sum(row[actor]['phase_cpu_seconds']['source'].values()) for row in rows)
 assert s['source_rss_bytes'][actor]==median(row[actor]['phase_peak_rss_bytes']['source'] for row in rows)
 assert all(row[actor]['stripped_executable']==s['stripped_executables'][actor] for row in rows)
for actor,values in s['paired_ratios'].items():
 for peer,value in values.items():assert value==median(row[actor]['seconds']['source_through_link']/row[peer]['seconds']['source_through_link'] for row in rows)
a='mech-preserved';b='mech-control-reference'
cost=[(row[a]['seconds']['source_through_link']-row[b]['seconds']['source_through_link'])*1000 for row in rows]
assert cost==s['paired_ms_cost_vs_control'] and median(cost)==s['paired_ms_preserved_vs_control']
assert sum(value<0 for value in cost)==s['preserved_faster_than_control']==3
rng=random.Random(9951118);boot=sorted(median(rng.choices(cost,k=8)) for _ in range(20000))
assert [boot[999],boot[18999]]==s['paired_median_ms_cost_90_percent_interval']
phase=lambda row:{p['phase']:p['seconds'] for p in row['compiler_phases']}
for actor,phases in s['phases'].items():
 assert phases=={name:median(phase(row[actor])[name] for row in rows) for name in phases}
 assert s['explicit_phase_residual_seconds'][actor]==median(row[actor]['seconds']['source']-sum(phase(row[actor]).values()) for row in rows)
assert s['paired_source_cpu_ms_cost']==median((sum(row[a]['phase_cpu_seconds']['source'].values())-sum(row[b]['phase_cpu_seconds']['source'].values()))*1000 for row in rows)
assert s['paired_native_ms_cost']==median((phase(row[a])['native']-phase(row[b])['native'])*1000 for row in rows)
full=json.loads(gzip.decompress((here/'full-replay.json.gz').read_bytes()));gate=read(here/'full-screen.json');result=read(here/'result.json');pin=read(here/'candidate.json')
assert gate['decision']==full['decision']==result['decision']=='Retain'
assert gate['rejected']==gate['unresolved']==full['rejected']==full['unresolved']==[]
assert {run['order'] for run in full['runs']}=={'forward','reverse'}
for run in full['runs']:
 assert run['case_count']==81
 for peer,count,unique in [('roc',81,74),('libdeflate',62,56)]:
  data=run['peers'][peer]
  assert data['point_estimate_wins']['mech-experiment']==data['precision_supported_wins']['mech-experiment']==count
  assert data['distinct_contract_point_estimate_wins']['mech-experiment']==data['distinct_contract_precision_supported_wins']['mech-experiment']==unique
  assert data['lost_cases']==[]
assert full['candidate_control_timing_observations']['slowdowns']==[]
assert len(full['candidate_control_timing_observations']['uncertain'])==7
bindings=read(here/'full-plan.json')['bindings']['mech-experiment']
assert bindings['program.o']['sha256']==pin['artifact_sha256']['program.o']
assert bindings['metadata.json']['sha256']==pin['artifact_sha256']['metadata.json']
assert bindings['mech-worker']['sha256']==pin['worker_sha256']==result['worker_sha256']
source=read(here/'source-rebuild.json');assert source['verified_exact_calls']==162 and source['full_integrated_source_identity'] and source['object_text_bytes']==63804
assert source['checked_failure_probe']['cases']==6 and source['checked_failure_probe']['exact_recoveries']==12
assert source['compiler_sha256']==s['compiler_sha256']==pin['compiler_sha256']
assert source['source_recipe']['manifest_sha256']==sha256((here/'candidate.json').read_bytes()).hexdigest()
print('Verified 63,804 B recipe, both full 81/62 orders and precision counts, source checks, and eight matched builds; paired cost %.6f ms.'%median(cost))
