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
result={'schema':'mech.interned-schema-key-admission.v1','implementation':json.loads((r/'implementation.json').read_text()),'primary':summary,'cpu':{},'allocation':{},'static':{},'validation':json.loads((r/'validation.json').read_text()),'allocation_target':json.loads((r/'allocation-target.json').read_text()),'profiles':json.loads((r/'profile-numbers.json').read_text())}
for label in labels:
 for kind,keys,section in [('cpu',cpu_keys,'cpu'),('memory',memory_keys,'allocation')]:
  values=rows(label,kind);result[section][label]={'observations':len(values),**{key:{'median':statistics.median(v[key] for v in values),'min':min(v[key] for v in values),'max':max(v[key] for v in values)} for key in keys}}
  if kind=='cpu':assert all(v['instructions']>0 and v['cycles']>0 and v['pageins']==0 for v in values)
  if label!='reference' and kind=='cpu':assert all(v['exact_reference'] and v['independent_inflation'] and v['capacity']==13 and v['work']==60260219686 for v in values)
 if label!='reference':
  binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());meta=json.loads((r/'linked-primary'/label/'metadata.json').read_text());result['static'][label]={'source':binding['compiler_source'],'libraries':binding['libraries'],'pins':binding['pins'],**{key:meta[key] for key in ['typed_ir_version','abi_version','code_bytes','stack_bytes']}}
  for kind in ['cpu','memory']:assert json.loads((r/'counters'/label/(kind+'-build.json')).read_text())['libraries']==binding['libraries']
  assert result['allocation'][label]['allocation_requests']['median']==(8773 if label=='before' else 8173)
  assert result['allocation'][label]['requested_bytes']['median']==(98243843 if label=='before' else 98228396)
assert result['static']['before']['pins']==result['static']['after']['pins']
assert result['static']['before']['typed_ir_version']==result['static']['after']['typed_ir_version']==132
for name,before in result['static']['before']['libraries'].items():
 after=result['static']['after']['libraries'][name];assert before['features']==after['features'] and before['profile']==after['profile']
 if not name.startswith('mech_'):assert before['sha256']==after['sha256']
result['changes_percent']={'primary_seconds':100*(summary['comparisons']['after_over_before']['pooled_ratio']-1)}
for section,keys in [('cpu',cpu_keys),('allocation',memory_keys)]:
 for key in keys:
  b=result[section]['before'][key]['median'];a=result[section]['after'][key]['median'];result['changes_percent'][key]=100*(a/b-1) if b else None
result['scope']='Complete eight-byte level-12 only. Same source/bytecode, algorithm, source allocation sites, buffer sizes, allocator and timing boundaries. Coherent before and after graphs; identical common dependencies and feature/profile selection. Four independently prepared executable pairs. Counter, heap, stack-census and sample runs are separate serialized diagnostics. Post-call OS memory is not peak RSS; retained compiler/catalog/image storage is included. No larger-input or minimum standalone memory claim.'
(out/'scorecard.json').write_text(json.dumps(result,indent=2)+'\n')
with (out/'scorecard.csv').open('w',newline='') as f:
 writer=csv.writer(f);writer.writerow(['metric','reference','before','after','change_percent','scope']);writer.writerow(['seconds',summary['reference_seconds'],summary['medians']['before']['seconds'],summary['medians']['after']['seconds'],result['changes_percent']['primary_seconds'],'matched complete-compression primary'])
 for section,keys in [('cpu',cpu_keys),('allocation',memory_keys)]:
  for key in keys:writer.writerow([key,*[result[section][label][key]['median'] for label in labels],result['changes_percent'][key],'separate serialized diagnostics'])
print(json.dumps({'primary':summary['medians'],'changes_percent':result['changes_percent'],'validation':result['validation']},indent=2))
