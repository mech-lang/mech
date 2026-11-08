from pathlib import Path
import collections,json,statistics
p=Path(__file__).parent/'profile';rows=json.loads((p/'profiles.json').read_text());MAX=(1<<64)-1;allgroups={};results={}
for variant in ['canonical-control-detail','native-dictionary-detail']:
 groups=collections.defaultdict(list);runs=[]
 descriptor=json.loads((p/'hosts'/variant.removesuffix('-detail')/'build/descriptor.json').read_text())['descriptor']
 for row in rows:
  if row['variant']!=variant:continue
  trace=json.loads(Path(row['trace']).read_text());es=[e for e in trace['capture']['events']if e['thread']==trace['main_thread']];start=next(e for e in es if e['stage']=='helper_ffi'and e['site']==38);end=next(e for e in es if e['stage']=='helper_ffi'and e['operation']==MAX-35);es=sorted([e for e in es if e['stage']=='helper_ffi'and start['end_ns']<=e['start_ns']and e['end_ns']<=end['start_ns']],key=lambda e:e['start_ns']);assert all(a['end_ns']<=b['start_ns']for a,b in zip(es,es[1:]));run=collections.defaultdict(collections.Counter)
  for e in es:
   v=run[(e['site'],e['operation'])];v['callbacks']+=1;v['elapsed_ns']+=e['end_ns']-e['start_ns'];v['instructions']+=e['end_counters']['instructions']-e['start_counters']['instructions'];v['requests']+=e['end_allocations']['thread_requests']-e['start_allocations']['thread_requests'];v['requested_bytes']+=e['end_allocations']['thread_requested_bytes']-e['start_allocations']['thread_requested_bytes']
  for k,v in run.items():groups[k].append(dict(v))
  runs.append(dict(pair=row['pair'],callbacks=sum(v['callbacks']for v in run.values()),requests=sum(v['requests']for v in run.values())))
 out=[]
 for (site,op),vs in groups.items():
  assert len(vs)==8
  call=descriptor['calls'][site];key=call['template']['selected_key']if call else None
  out.append(dict(site=site,operation=op,source_operation='/'.join([*key['module_path'],key['operation_name']])if key else None,**{k:statistics.median(v[k]for v in vs)for k in vs[0]}))
 results[variant]=dict(top_callbacks=sorted(out,key=lambda v:v['elapsed_ns'],reverse=True),runs=runs)
(p/'dictionary-prelude-breakdown.json').write_text(json.dumps(dict(diagnostic_only=True,promotion_timing_eligible=False,callbacks_disjoint_per_capture=True,medians_not_additive=True,variants=results),indent=2)+'\n')
for name,d in results.items():
 print(name)
 for r in d['top_callbacks'][:8]:print({**r,'elapsed_us':r['elapsed_ns']/1000})
