from pathlib import Path
import csv,json,statistics,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);out.mkdir(exist_ok=True,parents=True)
summary=json.loads((r/'linked-primary/summary.json').read_text())
def rows(label,kind):
 p=r/'counters'/(label+'-'+kind)
 if label=='reference' or kind=='memory':
  values=[json.loads(s.removeprefix('ALLOC_PROBE ')) for s in (p/'run.stderr').read_text().splitlines() if s.startswith('ALLOC_PROBE ')]
  if label!='reference':values=values[1:-1]
 else:values=[json.loads(s) for s in (p/'run.stdout').read_text().splitlines() if s.startswith('{')];values=[v for v in values if not v['warmup'] and not v['profile']]
 assert len(values)==(8 if kind=='memory' and label!='reference' else 32)
 return values
labels=['reference','before','after'];cpu_keys=['instructions','cycles','rss_after','physical_after','pageins'];memory_keys=['allocation_requests','requested_bytes','reallocations','relocating_reallocations','relocating_realloc_copy_upper_bound','live_usable_before','live_usable_after','peak_live_usable','incremental_peak_live_usable']
result={'schema':'mech.borrowed-numeric-matrix-validation.v1','implementation':json.loads((r/'implementation.json').read_text()),'primary':summary,'cpu':{},'allocation':{},'static':{},'validation':json.loads((r/'validation.json').read_text()),'allocation_target':json.loads((r/'allocation-target.json').read_text()),'profiles':json.loads((r/'profile-numbers.json').read_text())}
for label in labels:
 for kind,keys,section in [('cpu',cpu_keys,'cpu'),('memory',memory_keys,'allocation')]:
  values=rows(label,kind);result[section][label]={'observations':len(values),**{key:{'median':statistics.median(v[key] for v in values),'min':min(v[key] for v in values),'max':max(v[key] for v in values)} for key in keys}}
  if kind=='cpu':assert all(v['instructions']>0 and v['cycles']>0 and v['pageins']==0 for v in values)
  if label!='reference' and kind=='cpu':assert all(v['exact_reference'] and v['independent_inflation'] and v['capacity']==13 and v['work']==60260219686 for v in values)
 if label!='reference':
  binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());meta=json.loads((r/'linked-primary'/label/'metadata.json').read_text());result['static'][label]={'source':binding['compiler_source'],'libraries':binding['libraries'],'pins':binding['pins'],**{key:meta[key] for key in ['typed_ir_version','abi_version','code_bytes','stack_bytes']}}
  for kind in ['cpu','memory']:assert json.loads((r/'counters'/label/(kind+'-build.json')).read_text())['libraries']==binding['libraries']
  assert result['allocation'][label]['allocation_requests']['median']==(13484 if label=='before' else 10119)
  assert result['allocation'][label]['requested_bytes']['median']==(98480739 if label=='before' else 98329987)
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
