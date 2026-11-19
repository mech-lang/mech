"""Recompute the checked-phase build effects and check their artifact bindings."""
from pathlib import Path
from hashlib import sha256
from statistics import median
import gzip,json,math,random
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
assert status['status']=='experimental; not selected' and status['selected_control_bytes']==63804
assert status['extra_bytes']==status['experiment_bytes']-63804==640
replay=read('source-replay.json');assert replay['matches_experiment']
assert replay['artifacts']['program.o']==status['object_sha256']
assert replay['artifacts']['metadata.json']==status['metadata_sha256']
print('Verified eight balanced builds, paired saving/interval, exact artifacts, 162 outputs, six failures and measured stack. Runtime wins remain unmeasured.')
