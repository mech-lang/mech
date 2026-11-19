"""Recompute the checked-phase build effects and check their artifact bindings."""
from pathlib import Path
from hashlib import sha256
from statistics import median
import gzip,json,math,random,tarfile
here=Path(__file__).resolve().parent
read=lambda name:json.loads((here/name).read_text())
raw=json.loads(gzip.decompress((here/'build-comparison.json.gz').read_bytes()))
plan=read('build-plan.json');summary=read('build-summary.json');status=read('status.json')
assert sha256((here/'build-plan.json').read_bytes()).hexdigest()==raw['plan_sha256']==summary['plan_sha256']
rows=raw['observations'];assert len(rows)==8
control='mech-control-reference';phase='mech-checked-phase'
assert plan['actors'][control]['flags']|{'MECH_COMPACT_CHECKED_PHASE_WINDOWS':'4069:4954:-'}==plan['actors'][phase]['flags']
assert plan['actors'][control]['compiler_sha256']==plan['actors'][phase]['compiler_sha256']==status['compiler_sha256']
close=lambda a,b:math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-12)
for actor in raw['medians']:
 for field,value in raw['medians'][actor].items():
  assert close(median(r[actor]['seconds'][field] for r in rows),value)
  assert close(summary['medians'][actor][field],value)
 assert close(median(sum(r[actor]['phase_cpu_seconds']['source'].values()) for r in rows),summary['source_cpu_seconds'][actor])
 assert close(median(r[actor]['phase_peak_rss_bytes']['source'] for r in rows),summary['source_rss_bytes'][actor])
 for peer,ratio in raw['paired_ratios'].get(actor,{}).items():
  assert close(median(r[actor]['seconds']['source_through_link']/r[peer]['seconds']['source_through_link'] for r in rows),ratio)
 for row in [raw['warmup'],*rows]:
  if actor in [control,phase]:assert row[actor]['generated_artifacts']==plan['actors'][actor]['artifact_sha256']
deltas=[(r[phase]['seconds']['source_through_link']-r[control]['seconds']['source_through_link'])*1000 for r in rows]
assert close(median(deltas),summary['paired_ms_phase_vs_control'])
assert sum(d<0 for d in deltas)==summary['phase_faster_than_control']==7
rng=random.Random(9951119);resamples=sorted(median(rng.choices(deltas,k=8)) for _ in range(20000))
assert all(close(a,b) for a,b in zip([resamples[1000],resamples[19000]],summary['paired_build_90pct_interval_ms']))
proof=read('linked-result.json');assert proof['matches_success_observations']==162 and len(proof['checked_failure_cases'])==6
stack=proof['peak_stack_bytes'];assert stack=={'maintained':4848,'codebook-core-private-helpers':4688}
assert read('timing-isolation.json')['test_build_resumed']
assert status['status']=='qualified alternative; not selected' and status['selected_control_bytes']==63804
assert status['extra_bytes']==status['experiment_bytes']-63804==640
replay=read('source-replay.json');assert replay['matches_experiment']
assert replay['artifacts']['program.o']==status['object_sha256']
assert replay['artifacts']['metadata.json']==status['metadata_sha256']
full=json.loads(gzip.decompress((here/'full-replay.json.gz').read_bytes()))
gate=read('full-screen.json');runtime_plan=read('full-plan.json');pin=read('candidate.json')
assert sha256((here/'full-plan.json').read_bytes()).hexdigest()==full['plan']['sha256']
assert gate['decision']==full['decision']==status['runtime_decision']=='Retain'
assert gate['rejected']==gate['unresolved']==full['rejected']==full['unresolved']==[]
assert {run['order'] for run in full['runs']}=={'forward','reverse'}
for run in full['runs']:
 assert run['case_count']==81
 for peer,count,unique in [('roc',81,74),('libdeflate',62,56)]:
  data=run['peers'][peer]
  assert data['point_estimate_wins']['mech-experiment']==data['precision_supported_wins']['mech-experiment']==count
  assert data['distinct_contract_point_estimate_wins']['mech-experiment']==data['distinct_contract_precision_supported_wins']['mech-experiment']==unique
  assert data['lost_cases']==data['gained_cases']==[]
 receipts=full['receipts'][run['id']]['observations']
 assert len(receipts)==81
 with tarfile.open(here/('full-'+run['order']+'-observations.tar')) as archive:
  assert len(archive.getmembers())==81
  for case,receipt in receipts.items():
   assert sha256(archive.extractfile(case+'/observations.json.gz').read()).hexdigest()==receipt['sha256']
assert status['runtime_precision']==[run['peers'] for run in full['runs']]
assert full['candidate_control_timing_observations']==status['candidate_control_timing_observations']
assert full['candidate_control_timing_observations']['slowdowns']==[]
assert len(full['candidate_control_timing_observations']['uncertain'])==8
bindings=runtime_plan['bindings']['mech-experiment']
for name,key in [('program.o','object_sha256'),('metadata.json','metadata_sha256')]:
 assert bindings[name]['sha256']==pin['artifact_sha256'][name]==status[key]
assert bindings['mech-worker']['sha256']==pin['worker_sha256']==status['worker_sha256']
source=read('source-rebuild.json')
assert source['verified_exact_calls']==162 and source['full_integrated_source_identity'] and source['object_text_bytes']==64444
assert source['checked_failure_probe']['cases']==6 and source['checked_failure_probe']['exact_recoveries']==12
assert source['compiler_sha256']==pin['compiler_sha256']==status['compiler_sha256']
assert source['source_recipe']['manifest_sha256']==sha256((here/'candidate.json').read_bytes()).hexdigest()
assert source['worker_sha256']==status['source_rebuild']['worker_sha256']
assert source['worker_sha256']!=status['worker_sha256'] and not source['worker_matches_archive']
success=json.loads(gzip.decompress((here/'source-correctness.json.gz').read_bytes()))
recoveries=json.loads(gzip.decompress((here/'source-checked-recoveries.json.gz').read_bytes()))
failures=json.loads(gzip.decompress((here/'source-checked-failures.json.gz').read_bytes()))
assert len(success)==162 and len(recoveries)==len(failures)==12
assert len({row['case'] for row in failures})==6 and all(row['status']!=0 for row in failures)
for row in success+recoveries:
 assert row['exact_reference'] and row['independent_inflation'] and row['borrowed_output_alias_unchanged'] and row['input_alias_unchanged']
 assert row['nonreturned_private_bytes_at_return']==0
tests=read('validation.json');assert tests['passed']==15 and tests['failed']==0
print('Verified qualified 64,444 B alternative: both full 81/62 orders and precision counts, all 162 raw case receipts, 15 JIT tests, source checks, stack and eight matched builds. Selected size recipe remains 63,804 B.')
