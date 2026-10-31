from pathlib import Path
import csv,hashlib,json,statistics,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);out.mkdir(parents=True,exist_ok=True)
summary=json.loads((r/'linked-primary/summary.json').read_text())
def rows(label,kind):
 p=r/'counters'/(label+'-'+kind)
 if label=='reference' or kind=='memory':
  data=[json.loads(line.removeprefix('ALLOC_PROBE ')) for line in (p/'run.stderr').read_text().splitlines() if line.startswith('ALLOC_PROBE ')]
  if label!='reference':data=data[1:-1]
 else:data=[x for x in map(json.loads,(p/'run.stdout').read_text().splitlines()) if not x['warmup'] and not x['profile']]
 assert len(data)==(8 if kind=='memory' and label!='reference' else 32)
 return data
labels=['reference','before','after'];cpu_keys=['instructions','cycles','rss_after','physical_after','pageins'];memory_keys=['allocation_requests','requested_bytes','reallocations','relocating_reallocations','relocating_realloc_copy_upper_bound','live_usable_before','live_usable_after','peak_live_usable','incremental_peak_live_usable']
result={'schema':'mech.shared-numeric-shadows.v1','baseline_published':'f81cbfdc67cc591ed54bd861d63d693c005706ad','implementation':json.loads((r/'implementation.json').read_text()),'primary':summary,'cpu':{},'allocation':{},'static':{},'validation':json.loads((r/'validation.json').read_text()),'mechanism':json.loads((r/'shadow-layout.json').read_text()),'confirmation':json.loads((r/'confirmation-primary/summary.json').read_text()),'profiles':json.loads((r/'profile-numbers.json').read_text()),'outcome':{'retained':True,'reason':'Initial elapsed -0.179%; independent longer confirmation -0.604%, seven of eight preparations improve. Memory and code shrink; instructions increase. Preserve both cohorts and all contrary metrics.'}}
for label in labels:
 for kind,keys,section in [('cpu',cpu_keys,'cpu'),('memory',memory_keys,'allocation')]:
  data=rows(label,kind);result[section][label]={'observations':len(data),**{key:{'median':statistics.median(x[key] for x in data),'min':min(x[key] for x in data),'max':max(x[key] for x in data)} for key in keys}}
  if kind=='cpu':assert all(x['instructions']>0 and x['cycles']>0 and x['pageins']==0 for x in data)
  if label!='reference' and kind=='cpu':assert all(x['exact_reference'] and x['independent_inflation'] and x['capacity']==13 and x['work']==60260219686 for x in data)
 if label!='reference':
  binding=json.loads((r/'linked-primary'/label/'binding.json').read_text());meta=json.loads((r/'linked-primary'/label/'metadata.json').read_text());result['static'][label]={'source':binding['compiler_source'],'libraries':binding['libraries'],'object_sha256':binding['pins']['object.o']['sha256'],**{key:meta[key] for key in ['typed_ir_version','abi_version','code_bytes','stack_bytes']}}
  for kind in ['cpu','memory']:
   build=json.loads((r/'counters'/label/(kind+'-build.json')).read_text());assert build['libraries']==binding['libraries'];assert 'link-arg='+str(Path(build['command'][-3]).parent.parent.parent/'linked-primary'/label/'object.o') in build['command']
  assert result['allocation'][label]['allocation_requests']['median']==13484 and result['allocation'][label]['requested_bytes']['median']==(98529723 if label=='before' else 98480739)
assert result['static']['before']['typed_ir_version']==131 and result['static']['after']['typed_ir_version']==132
for name in result['static']['before']['libraries']:
 if name!='mech_gpu':assert result['static']['before']['libraries'][name]==result['static']['after']['libraries'][name]
result['changes_percent']={'primary_seconds':100*(summary['comparisons']['after_over_before']['pooled_ratio']-1)}
for section,keys in [('cpu',cpu_keys),('allocation',memory_keys)]:
 for key in keys:
  before=result[section]['before'][key]['median'];after=result[section]['after'][key]['median'];result['changes_percent'][key]=100*(after/before-1) if before else None
result['scope']='Complete eight-byte level-12 only; source/bytecode/algorithm/buffers/allocator/output reuse/timer unchanged. Four prepared pairs. Hardware, allocator, sampling and exact mechanism census are separate serialized cohorts. Hardware reads include boundary overhead; cycle change from one preparation is not a universal speed claim. Current post-call RSS is not OS peak; tracked heap includes harness/catalog state. No larger-input claim.'
(out/'scorecard.json').write_text(json.dumps(result,indent=2)+'\n')
with (out/'scorecard.csv').open('w',newline='') as file:
 writer=csv.writer(file);writer.writerow(['metric','reference','before','after','change_percent','scope']);writer.writerow(['seconds',summary['reference_seconds'],summary['medians']['before']['seconds'],summary['medians']['after']['seconds'],result['changes_percent']['primary_seconds'],'matched primary'])
 confirmation=result['confirmation'];writer.writerow(['confirmation_seconds',confirmation['reference_seconds'],confirmation['medians']['before']['seconds'],confirmation['medians']['after']['seconds'],100*(confirmation['comparisons']['after_over_before']['pooled_ratio']-1),'independent longer cohort'])
 for section,keys in [('cpu',cpu_keys),('allocation',memory_keys)]:
  for key in keys:writer.writerow([key,*[result[section][label][key]['median'] for label in labels],result['changes_percent'][key],'separate diagnostics'])
print(json.dumps({'changes_percent':result['changes_percent'],'mechanism':result['mechanism']['after']['physical_scratch_words'],'validation':result['validation']['fresh_rust_tests']},indent=2))
