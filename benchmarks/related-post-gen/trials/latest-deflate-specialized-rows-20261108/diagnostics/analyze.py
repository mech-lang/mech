from pathlib import Path
import collections,json,statistics
P=Path(__file__).parent;rows=json.loads((P/'profiles.json').read_text());MAX=(1<<64)-1
PRIORITY=['input_admission','input_views','result_construction','session_setup','worker_dispatch','runtime_callbacks','numeric_native','other_generated_entry','runtime_outside_entry','other_driver']
def tag(e):
 s=e['stage']
 if s=='post_views':return 'input_views'
 if s in ['input_admission','result_construction','session_setup']:return s
 if s=='helper_ffi'and e['operation']==MAX-20:return 'worker_dispatch'
 if s in ['helper_ffi','admit']:return 'runtime_callbacks'
 if s=='native_body':return 'numeric_native'
 if s=='generated_entry':return 'other_generated_entry'
 if s in ['evaluate','execute_prepared_after_setup','worker']:return 'runtime_outside_entry'
 return None
def span(e,axis):
 if axis=='elapsed_ns':return e['start_ns'],e['end_ns']
 return e['start_counters'][axis],e['end_counters'][axis]
def partition(es,tid,root,axis):
 e=next(e for e in es if e['thread']==tid and e['stage']==root);lo,hi=span(e,axis);edges=collections.defaultdict(collections.Counter)
 for e in es:
  if e['thread']!=tid:continue
  t=tag(e)
  if not t:continue
  a,b=span(e,axis);a=max(a,lo);b=min(b,hi)
  if a<b:edges[a][t]+=1;edges[b][t]-=1
 active=collections.Counter();parts=collections.Counter();previous=lo
 for at in sorted({lo,hi,*edges}):
  category=next((p for p in PRIORITY if active[p]>0),'other_driver');parts[category]+=at-previous
  active.update(edges[at]);assert all(v>=0 for v in active.values());previous=at
 assert sum(parts.values())==hi-lo
 return dict(total=hi-lo,categories=dict(parts))
runs=[];helpers=collections.defaultdict(list);roots=collections.defaultdict(list);stages=collections.defaultdict(list)
for row in rows:
 if row['kind']!='profile':continue
 j=json.loads(Path(row['trace']).read_text());es=j['capture']['events'];m=j['main_thread'];workers=sorted({e['thread']for e in es if e['stage']=='worker'});assert len(workers)==4
 result=dict(variant=row['variant'],pair=row['pair'],main={a:partition(es,m,'post_JSON',a)for a in ['elapsed_ns','instructions','cycles']},workers={str(t):{a:partition(es,t,'worker',a)for a in ['elapsed_ns','instructions','cycles']}for t in workers});runs.append(result)
 if row['variant'].endswith('callbacks'):
  for stage in ['restoration','json_decode','post_JSON','json_output','input_teardown','program_teardown','posts_teardown','driver_total']:
   e=next(e for e in es if e['thread']==m and e['stage']==stage);stages[(row['variant'],stage)].append({a:span(e,a)[1]-span(e,a)[0]for a in ['elapsed_ns','instructions','cycles']})
  h=collections.defaultdict(collections.Counter);nr=collections.defaultdict(collections.Counter)
  for e in es:
   if e['thread']==m and e['stage']=='helper_ffi'and e['operation']!=MAX-20:
    for a in ['elapsed_ns','instructions','cycles']:x,y=span(e,a);h[(e['site'],e['operation'])][a]+=y-x
   if e['thread']==m and e['stage']=='native_body':
    for a in ['elapsed_ns','instructions','cycles']:x,y=span(e,a);nr[(e['site'],e['operation'])][a]+=y-x
  for site,v in h.items():helpers[(row['variant'],*site)].append(v)
  for site,v in nr.items():roots[(row['variant'],*site)].append(v)
summary={}
for name in sorted({r['variant']for r in runs}):
 xs=[r for r in runs if r['variant']==name];main={axis:{c:statistics.median(r['main'][axis]['categories'].get(c,0)for r in xs)for c in PRIORITY}for axis in ['elapsed_ns','instructions','cycles']};workers={axis:{c:statistics.median(sum(w[axis]['categories'].get(c,0)for w in r['workers'].values())for r in xs)for c in PRIORITY}for axis in ['instructions','cycles']};summary[name]=dict(main_category_medians=main,summed_worker_hardware_medians=workers)
phase={name:{stage:{a:statistics.median(v[a]for v in values)for a in ['elapsed_ns','instructions','cycles']}for (variant,stage),values in stages.items()if variant==name}for name in sorted({k[0]for k in stages})}
def aggregate(items):
 return [dict(variant=k[0],site=k[1],operation=k[2],**{a:statistics.median(v[a]for v in values)for a in ['elapsed_ns','instructions','cycles']})for k,values in sorted(items.items(),key=lambda kv:statistics.median(x['instructions']for x in kv[1]),reverse=True)]
helpers=aggregate(helpers);roots=aggregate(roots)
result=dict(diagnostic_only=True,promotion_timing_eligible=False,captures=32,full_process_phases=phase,post_JSON_partitions=summary,main_callbacks=helpers,main_native_roots=roots,partitions_exact_per_capture=True,category_medians_not_additive=True,hardware_additive_across_threads=True,elapsed_not_additive_across_threads=True,runs=runs)
(P/'interval-analysis.json').write_text(json.dumps(result,indent=2)+'\n')
for name,v in summary.items():
 print(name,'MAIN instructions',v['main_category_medians']['instructions']);print('WORKER sum instructions',v['summed_worker_hardware_medians']['instructions'])
print('top callbacks',json.dumps(helpers[:12],indent=2));print('top native roots',json.dumps(roots[:8],indent=2))
