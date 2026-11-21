"""Check captured all-unit work and typed pilot domains; no emission claim."""
from pathlib import Path
from hashlib import sha256
import gzip,json
HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(p.read_text())
compressed=lambda p:json.loads(gzip.decompress(p.read_bytes()))
expected={'program.o':'c5aab01c69ff4ed325b5149d2e1402a59477e5858f52052939d51f7eff699ef8','metadata.json':'c187c6f40d81d45995eaf3313c3282d41aadbd066413d7f44c0775b1c22225fa','program.mecb':'c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3'}
x=compressed(HERE/'construction-census/results.json.gz')
assert x['selected-63804']['artifacts']==expected
for label,r in x.items():
 assert r['matches_qualified_artifacts']
 assert r['census']==read(HERE/'construction-census'/label/'census.json')
 for arena in ['values','blocks','instructions']:
  counts=r['census'][arena]
  assert counts['resident']==0 and counts['created']>=counts['peak_resident']>=counts['maximum_unit']
 assert r['census']['cfg_builds']>=r['census']['dominator_builds']>=r['census']['full_verifications']
selected=x['selected-63804']['census'];late=x['late-codebook-64444']['census']
assert [selected[k]['created'] for k in ['values','instructions','blocks']]==[1407099,497878,174834]
assert selected['values']['maximum_unit']==late['values']['maximum_unit']==831378
assert selected['values']['peak_resident']==late['values']['peak_resident']==955192
assert late['values']['created']-selected['values']['created']==-122303
assert late['instructions']['created']-selected['instructions']['created']==-105665
assert late['blocks']['created']-selected['blocks']['created']==-35732
p=compressed(HERE/'typed-plan/partition.json.gz');r=p['regions'][0]
receipt=compressed(HERE/'typed-plan/receipt.json.gz')
assert receipt['artifacts']==expected and receipt['native_emission_unchanged']
assert not p['emission_authorized'] and r['all_path_initialization_proved']
assert (r['entry'],r['normal_exit'])==(4119,5031) and len(r['source_sites'])==885
assert r['input_words']==22 and r['output_words']==6
assert len(r['constant_values'])==20
assert r['owned_iterations']==list(range(44,53)) and r['owned_collections']==[15,16]
assert len(r['explicit_failures'])==44
for unit in [p['root'],r]:
 rows=unit['local_values'];ids=[v['global_value'] for v in rows]
 assert ids==sorted(set(ids)) and [v['local_value'] for v in rows]==list(range(len(rows)))
 assert unit['local_words']==sum(v['words'] for v in rows)==len(unit['local_word_slots'])
 assert len(set(unit['local_word_slots']))==len(unit['local_word_slots'])
 for v in rows:assert all(0<=lane<v['words'] for lane in v['owner_lanes'])
for field,total in [('input_values',22),('output_values',6),('constant_values',20)]:
 assert sum(v['words'] for v in r[field])==total
 for value in r[field]:assert r['local_values'][value['local_value']]['global_value']==value['global_value']
assert p['root']['local_words']==13371 and r['local_words']==1578
assert p['root']['local_collection_state_words']==230 and r['local_collection_state_words']==20
assert sum(owner==0 for owner in p['source_owners'])==len(p['root']['source_sites'])==7193
assert sum(owner==1 for owner in p['source_owners'])==885
assert all(p['source_owners'][s]==1 for s in r['source_sites'])
assert set(p['root']['source_sites']).isdisjoint(r['source_sites'])
assert sha256(gzip.decompress((HERE/'typed-plan/source.patch.gz').read_bytes())).hexdigest()==receipt['source_patch_sha256']
print('Verified all-unit census and exact-artifact typed source plan: 885 sites, 22/6 words, local domains and initialization proof. Early emission remains unimplemented.')
validation=read(HERE/'validation.json');source=read(HERE/'default-source-rebuild.json');worker=read(HERE/'default-worker-identity.json')
assert validation['focused_tests']=={'partition_plan':7,'census_lifecycle':1,'census_serialization':1,'failed':0}
assert source['full_integrated_source_identity'] and source['object_text_bytes']==63804 and source['verified_exact_calls']==162
assert source['checked_failure_probe']['cases']==6 and source['checked_failure_probe']['exact_recoveries']==12
assert source['compiler_sha256']==validation['default_compiler_sha256']
assert worker['reproduces_qualified_fixed_support_worker'] and worker['worker_sha256']==validation['fixed_support_worker_sha256']=='5afce59aa83c03df81c693d78e9a65f17df1c88c64a7e04c490c58ecc4fffba0'
for name,count in [('correctness',162),('checked-recoveries',12)]:
 rows=compressed(HERE/('default-'+name+'.json.gz'));assert len(rows)==count
 assert all(r['exact_reference'] and r['independent_inflation'] and r['borrowed_output_alias_unchanged'] and r['input_alias_unchanged'] and r['nonreturned_private_bytes_at_return']==0 for r in rows)
assert sha256(gzip.decompress((HERE/'implementation.patch.gz').read_bytes())).hexdigest()==validation['implementation_patch_sha256']
assert not validation['native_early_emission'] and not validation['new_size_or_build_time_gain_claimed']
print('Verified nine focused tests, final default source checks and the unchanged 81/62-qualified worker. No new optimization result is inferred from diagnostics.')
from statistics import median
import random,math
for folder in ['default-timing-before-gate','default-timing']:
 root=HERE/folder;plan=read(root/'plan.json');raw=compressed(root/'comparison.json.gz');summary=read(root/'summary.json')
 assert sha256((root/'plan.json').read_bytes()).hexdigest()==raw['plan_sha256']==summary['plan_sha256']
 rows=raw['observations'];assert len(rows)==8
 actors=plan['actors'];control='mech-control-reference';current='mech-current-default'
 if folder=='default-timing':assert actors[control]['flags']==actors[current]['flags']
 for order,row in zip(plan['orders'],rows):assert list(row)==order and set(row)==set(actors)
 for position in range(4):assert all(sum(order[position]==actor for order in plan['orders'])==2 for actor in actors)
 for actor,values in summary['medians'].items():
  for field,value in values.items():assert math.isclose(value,median(row[actor]['seconds'][field] for row in rows),rel_tol=1e-12,abs_tol=1e-12)
  for row in [raw['warmup'],*rows]:
   assert row[actor]['stripped_executable']==raw['warmup'][actor]['stripped_executable']
   if actor in [control,current]:assert row[actor]['generated_artifacts']=={'encoder':expected['program.o'],'metadata':expected['metadata.json'],'bytecode':expected['program.mecb']}
 for actor,peers in summary['paired_ratios'].items():
  for peer,value in peers.items():assert math.isclose(value,median(row[actor]['seconds']['source_through_link']/row[peer]['seconds']['source_through_link'] for row in rows),rel_tol=1e-12,abs_tol=1e-12)
 costs=[(row[current]['seconds']['source_through_link']-row[control]['seconds']['source_through_link'])*1000 for row in rows]
 assert costs==summary['paired_ms_cost'] and median(costs)==summary['paired_median_ms_cost']
 assert sum(cost<0 for cost in costs)==summary['faster']
 rng=random.Random(summary['seed']);boot=sorted(median(rng.choices(costs,k=8)) for _ in range(20000))
 assert [boot[999],boot[18999]]==summary['cost_90pct_interval_ms']
 print('%s: verified eight complete builds, exact artifacts, paired cost %.6f ms and interval.'%(folder,median(costs)))
