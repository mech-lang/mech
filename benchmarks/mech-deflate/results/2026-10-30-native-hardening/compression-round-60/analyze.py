from pathlib import Path
import csv,json,statistics,sys,math,hashlib
r=Path(sys.argv[1]);out=Path(sys.argv[2]);out.mkdir(exist_ok=True,parents=True)
summary=json.loads((r/'linked-primary/summary.json').read_text())
# Recompute the primary summary from all raw observations; no executable runs.
reference_rows=[line.split('\t') for line in (r/'linked-primary/reference.stdout').read_text().splitlines() if line.startswith('rep\troc-deflate\t')]
assert len(reference_rows)==20000
assert statistics.median(int(row[3]) for row in reference_rows)/1e9==summary['reference_seconds']
all_rows={label:[] for label in ['before','after']};all_blocks=[];groups=[]
for group in range(4):
 data=json.loads((r/f'linked-primary/group{group}/results.json').read_text());all_blocks+=data['blocks']
 medians={}
 for label in all_rows:
  raw=data['all_observations'][label];assert len(raw)==498
  assert all(v['exact_reference'] and v['independent_inflation'] and v['input_bytes']==8 and v['raw_bytes']==10 and v['capacity']==13 and v['work']==60260219686 for v in raw)
  values=[v for v in raw if not v['warmup'] and not v['profile']];assert len(values)==496
  all_rows[label]+=values;medians[label]=statistics.median(v['seconds'] for v in values)
  assert len(data['blocks'])==16
  for index,block in enumerate(data['blocks']):
   assert block['block']==index
   assert block['medians'][label]==statistics.median(v['seconds'] for v in values[index*31:(index+1)*31])
   assert block['after_over_before']==block['medians']['after']/block['medians']['before']
  assert (r/f'linked-primary/group{group}/{label}.stderr').read_text().endswith('CLEANUP bytes=0 nodes=0\n')
 groups.append({'group':group,'medians':medians,'after_over_before':medians['after']/medians['before']})
assert groups==summary['groups']
for label,values in all_rows.items():
 median=statistics.median(v['seconds'] for v in values);assert len(values)==1984
 assert median==summary['medians'][label]['seconds'] and median/summary['reference_seconds']==summary['medians'][label]['multiple_of_reference']
ratios=[block['after_over_before'] for block in all_blocks]
assert len(ratios)==64 and sum(v<1 for v in ratios)==summary['comparisons']['after_over_before']['improving_blocks']
assert statistics.median(ratios)==summary['comparisons']['after_over_before']['median_block_ratio']
assert math.exp(statistics.mean(math.log(v) for v in ratios))==summary['comparisons']['after_over_before']['geometric_block_ratio']
assert sum(v['after_over_before']<1 for v in groups)==summary['comparisons']['after_over_before']['improving_groups']

def rows(label,kind):
 p=r/'counters'/(label+'-'+kind)
 if label=='reference' or kind=='memory':
  values=[json.loads(s.removeprefix('ALLOC_PROBE ')) for s in (p/'run.stderr').read_text().splitlines() if s.startswith('ALLOC_PROBE ')]
  if label!='reference':values=values[1:-1]
 else:values=[json.loads(s) for s in (p/'run.stdout').read_text().splitlines() if s.startswith('{')];values=[v for v in values if not v['warmup'] and not v['profile']]
 assert len(values)==(8 if kind=='memory' and label!='reference' else 32)
 return values
labels=['reference','before','after'];cpu_keys=['instructions','cycles','rss_after','physical_after','pageins'];memory_keys=['allocation_requests','requested_bytes','reallocations','relocating_reallocations','relocating_realloc_copy_upper_bound','live_usable_before','live_usable_after','peak_live_usable','incremental_peak_live_usable']
result={'schema':'mech.static-vector-address.v1','implementation':json.loads((r/'implementation.json').read_text()),'primary':summary,'cpu':{},'allocation':{},'static':{},'validation':json.loads((r/'validation.json').read_text()),'allocation_target':json.loads((r/'allocation-target.json').read_text()),'profiles':json.loads((r/'profile-numbers.json').read_text())}
for label in labels:
 for kind,keys,section in [('cpu',cpu_keys,'cpu'),('memory',memory_keys,'allocation')]:
  values=rows(label,kind);result[section][label]={'observations':len(values),**{key:{'median':statistics.median(v[key] for v in values),'min':min(v[key] for v in values),'max':max(v[key] for v in values)} for key in keys}}
  if kind=='cpu':assert all(v['instructions']>0 and v['cycles']>0 and v['pageins']==0 for v in values)
  if label!='reference' and kind=='cpu':assert all(v['exact_reference'] and v['independent_inflation'] and v['capacity']==13 and v['work']==60260219686 for v in values)
 if label!='reference':
  pin_root=r/'baseline' if label=='before' else r/'linked-primary/after';binding=json.loads((pin_root/'binding.json').read_text());meta=json.loads((pin_root/'metadata.json').read_text());result['static'][label]={'source':binding['compiler_source'],'libraries':binding['libraries'],'pins':binding['pins'],'normalized_source_sha256':binding['normalized_source_sha256'],'original_worker_sha256':binding['original_worker_sha256'],**{key:meta[key] for key in ['typed_ir_version','abi_version','code_bytes','stack_bytes']}}
  for kind in ['cpu','memory']:assert json.loads((r/'counters'/label/(kind+'-build.json')).read_text())['libraries']==binding['libraries']
  assert result['allocation'][label]['allocation_requests']['median']==8173
  assert result['allocation'][label]['requested_bytes']['median']==98228396
result['binary_audit']=json.loads((r/'binary-audit.json').read_text())
result['decision']=json.loads((r/'decision.json').read_text())
result['preflight_attempts']=[json.loads((r/name).read_text()) for name in ['primary-preflight.json','primary-preflight-second.json']]
assert all(row['exit']!=0 and row['observations']==0 for row in result['preflight_attempts'])
result['artifact_pin_equality']={name:result['static']['before']['pins'][name]==result['static']['after']['pins'][name] for name in result['static']['before']['pins']}
for label in ['before','after']:
 for name,pin in result['static'][label]['pins'].items():assert result['binary_audit'][label][name]['sha256']==pin['sha256']
 for key in ['code_bytes','stack_bytes']:assert result['binary_audit'][key][label]==result['static'][label][key]
 assert result['binary_audit']['static_instructions'][label]*4==result['static'][label]['code_bytes']
for key in ['code_bytes','stack_bytes','static_instructions','UDIV','MSUB']:
 audit=result['binary_audit'][key];assert audit['delta']==audit['after']-audit['before']
# Object/image changes are allowed: each variant is independently bound and pinned.
assert result['static']['before']['libraries'].keys()==result['static']['after']['libraries'].keys()
assert result['static']['before']['typed_ir_version']==result['static']['after']['typed_ir_version']==132
assert result['static']['before']['normalized_source_sha256']==result['static']['after']['normalized_source_sha256']
assert result['static']['before']['original_worker_sha256']==result['static']['after']['original_worker_sha256']
assert result['static']['before']['source']==result['implementation']['baseline_build_source']
assert result['static']['after']['source']==result['implementation']['candidate_build_source']
for name,before in result['static']['before']['libraries'].items():
 after=result['static']['after']['libraries'][name];assert before['features']==after['features'] and before['profile']==after['profile']
 if name!='mech_gpu':assert before['sha256']==after['sha256']
census=json.loads((r/'sites-after/census.json').read_text())
assert census['requests']==census['counter']['allocation_requests']==8173
assert sum(site['count'] for site in census['sites'])==8173
assert sum(site['requested_bytes'] for site in census['sites'])==98228396==census['counter']['requested_bytes']
assert sum(site['count'] for site in census['sites'] if site['kind']=='realloc')==162==census['counter']['reallocations']
assert all(site['requested_bytes']==site['bytes_each']*site['count'] and len(site['frames'])==len(site['pcs']) for site in census['sites'])
result['census']={'requests':census['requests'],'requested_bytes':census['counter']['requested_bytes'],'reallocations':census['counter']['reallocations'],'unique_stacks_with_size_and_kind':census['unique_stacks_with_size_and_kind'],'unique_pcs':census['unique_pcs'],'reconciled_to_heap_counters':True,'scope':'Full candidate allocation trace; stack counts are exact requests, not CPU samples. No allocation saving claimed for vector addressing.'}
result['changes_percent']={'primary_seconds':100*(summary['comparisons']['after_over_before']['pooled_ratio']-1)}
for section,keys in [('cpu',cpu_keys),('allocation',memory_keys)]:
 for key in keys:
  b=result[section]['before'][key]['median'];a=result[section]['after'][key]['median'];result['changes_percent'][key]=100*(a/b-1) if b else None
result['reference_multiples']={section:{key:{label:result[section][label][key]['median']/result[section]['reference'][key]['median'] for label in ['before','after']} for key in keys if result[section]['reference'][key]['median']!=0} for section,keys in [('cpu',cpu_keys),('allocation',memory_keys)]}
result['contrary_primary_groups']=[group for group in groups if group['after_over_before']>=1]
result['contrary_counter_directions']={key:value for key,value in result['changes_percent'].items() if value is not None and value>0}
result['source_pins']={'source_sha256':'331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9','bytecode_sha256':'a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7','allocator_sha256':'38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874','logical_work':60260219686,'comparison_work':47612,'input_bytes':8,'expected_output':[75,76,74,78,73,77,75,207,0,0],'output_capacity':13}
result['scope']='Complete eight-byte level-12 only. Same source/bytecode, algorithm, source allocation sites, buffer sizes, allocator and timing boundaries. Baseline R59 graph and candidate GPU-only rebuild; all non-GPU libraries, features and profile selection identical. Four independently prepared executable pairs. Counter, heap, stack-census and sample runs are separate serialized diagnostics. Post-call OS memory is not peak RSS; retained compiler/catalog/image storage is included. No larger-input or minimum standalone memory claim.'
(out/'scorecard.json').write_text(json.dumps(result,indent=2)+'\n')
with (out/'scorecard.csv').open('w',newline='') as f:
 writer=csv.writer(f);writer.writerow(['metric','reference','before','after','change_percent','scope']);writer.writerow(['seconds',summary['reference_seconds'],summary['medians']['before']['seconds'],summary['medians']['after']['seconds'],result['changes_percent']['primary_seconds'],'matched complete-compression primary'])
 for section,keys in [('cpu',cpu_keys),('allocation',memory_keys)]:
  for key in keys:writer.writerow([key,*[result[section][label][key]['median'] for label in labels],result['changes_percent'][key],'separate serialized diagnostics'])
print(json.dumps({'primary':summary['medians'],'changes_percent':result['changes_percent'],'validation':result['validation']},indent=2))
