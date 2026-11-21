"""Verify captured construction, artifact, timing and contract evidence."""
from pathlib import Path
from hashlib import sha256
from statistics import median
import gzip,json,math,random
HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(p.read_text())
blob=lambda p:gzip.decompress(p.read_bytes())
compressed=lambda p:json.loads(blob(p))
sha=lambda b:sha256(b).hexdigest()
for name,pin in read(HERE/'files.json').items():assert sha((HERE/name).read_bytes())==pin,name
v=read(HERE/'validation.json');normal=compressed(HERE/'normal/results.json.gz');counts=compressed(HERE/'census/results.json.gz')
for label,r in normal.items():
 assert r['artifacts']==counts[label]['artifacts']
 assert r['compiler_sha256']==v['normal_compiler_sha256']
 assert counts[label]['compiler_sha256']==v['census_compiler_sha256']
 for name in ['program.o','metadata.json']:assert sha(blob(HERE/f'normal/{label}/{name}.gz'))==r['artifacts'][name]
 c=counts[label]['census']
 for key in ['blocks','instructions','values']:
  assert c[key]['resident']==0 and c[key]['created']>=c[key]['peak_resident']>=c[key]['maximum_unit']
a=counts['selected-control']['census'];b=counts['early-codebook']['census']
assert [b[key]['created'] for key in ['values','instructions','blocks']]==[1145776,360196,128800]
for key in ['values','instructions','blocks']:assert math.isclose(v['construction_reduction_percent'][key],100*(1-b[key]['created']/a[key]['created']))
assert b['values']['maximum_unit']==692257 and b['values']['peak_resident']==800278
assert sha(blob(HERE/'program.mecb.gz'))==v['artifact_sha256']['program.mecb']
assert sha((HERE/'implementation.patch.gz').read_bytes())==v['source_patch_gzip_sha256']
assert sha(blob(HERE/'early-worker.gz'))==v['worker_sha256']
metadata=compressed(HERE/'normal/early-codebook/metadata.json.gz');region=metadata['early_source_region']
assert region['operations']==885 and (region['source_entry'],region['normal_exit'])==(4119,5031)
assert (len(region['input_word_slots']),len(region['output_word_slots']))==(42,27)
assert (len(region['constant_zero_input_slots']),len(region['constant_zero_output_slots']))==(27,27)
assert metadata['stack_bytes']==3424 and region['helper']['stack_bytes']==1072
account=v['whole_encoder_byte_accounting'];assert account['early_root']-account['matched_unsplit_root']+account['new_helper']+account['other_and_alignment_change']==account['net_increase_vs_matched_unsplit']==2464
linked=read(HERE/'validation/linked-result.json')
assert linked['matches_success_observations']==v['success_observations']==162
assert len(linked['checked_failure_cases'])==v['checked_failure_cases']==6
assert linked['peak_stack_bytes']['early-codebook']==v['stack_bytes']==5808>v['stack_ceiling_bytes']==5280
assert not v['stack_pass'] and not v['full_runtime_qualification'] and not v['selected']
for role in ['maintained','early-codebook']:
 rows=compressed(HERE/f'validation/{role}/smokes.json.gz');assert len(rows)==162
 assert all(all(r[k] for k in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for r in rows)
r=compressed(HERE/'validation/recoveries/result.json.gz');assert r['cases']==6 and r['exact_recoveries']==12
rows=compressed(HERE/'validation/recoveries/checked-recoveries.json.gz');assert len(rows)==12
assert all(all(r[k] for k in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for r in rows)
for name,n in [('word-boundary-tests',10),('partition-plan-tests',7)]:assert f'{n} passed; 0 failed' in (HERE/f'validation/{name}.log').read_text()
pure=compressed(HERE/'validation/pure-native.json.gz');assert len(pure['cases'])==9 and all(r['original']==r['early'] for r in pure['cases'])
plan=read(HERE/'builds/plan.json');raw=compressed(HERE/'builds/comparison.json.gz');summary=read(HERE/'builds/summary.json')
assert sha((HERE/'builds/plan.json').read_bytes())==raw['plan_sha256']==summary['plan_sha256']
rows=raw['observations'];assert len(rows)==10 and len(plan['actors'])==5
for order,row in zip(plan['orders'],rows):assert list(row)==order and set(row)==set(plan['actors'])
for i in range(5):assert all(sum(order[i]==actor for order in plan['orders'])==2 for actor in plan['actors'])
for actor,values in summary['medians'].items():
 for field,value in values.items():assert math.isclose(value,median(r[actor]['seconds'][field] for r in rows),rel_tol=1e-12)
rng=random.Random(summary['seed'])
for control,c in summary['comparisons'].items():
 costs=[(r['mech-early-codebook']['seconds']['source_through_link']-r[control]['seconds']['source_through_link'])*1000 for r in rows]
 assert costs==c['paired_ms_cost'] and median(costs)==c['paired_median_ms_cost'] and c['faster']==0
 boot=sorted(median(rng.choices(costs,k=10)) for _ in range(20000));assert [boot[999],boot[18999]]==c['cost_90pct_interval_ms']
qualified=compressed(HERE.parent/'encoder-size-gap/preserved-root-registers/full-replay.json.gz')
nearest=read(HERE/'runtime-targets/nearest.json');runs={}
for order in ['forward','reverse']:
 data=blob(HERE/f'runtime-targets/{order}-summary.json.gz')
 receipt=next(r for key,r in qualified['receipts'].items() if key.endswith('-'+order))
 assert sha(data)==receipt['summary']['sha256']
 runs[order]={r['case']:r for r in json.loads(data)}
rank=[]
for case in runs['forward']:
 ratios={order:runs[order][case]['paired_x_libdeflate']['mech-experiment'] for order in runs}
 if max(ratios.values())>=1:rank.append((max(ratios.values()),case))
assert [name for _,name in sorted(rank)]==[r['case'] for r in nearest['cases']]
assert nearest['cases'][0]['case']=='prefix-natural-4096'
print('Verified exact artifacts, all-unit work, 17 focused tests, nine native cases, 162 outputs, six failures, 12 recoveries, ten balanced complete builds and nearest remaining C loss. Experimental stack fails; no runtime-win or selection claim.')
