from pathlib import Path
from collections import defaultdict
import json,statistics
P=Path(__file__).parent/'profile'
read=lambda p:json.loads(Path(p).read_text())
def write(p,d):Path(p).write_text(json.dumps(d,indent=2)+'\n')
names=['preparation_interning_and_eligibility','triple_index_and_coverage','quartet_index_and_masks','queries_cleanup_and_materialization'];groups=defaultdict(list);runs=[];constructor=defaultdict(list);root_counts=defaultdict(list)
def delta(a,b,sa='start',sb='end'):
 d=dict(elapsed_ns=b[sb+'_ns']-a[sa+'_ns'])
 for key,label in [('counters','thread'),('process_counters','process')]:
  if a.get(sa+'_'+key)is not None and b.get(sb+'_'+key)is not None:
   for f in ['instructions','cycles']:d[label+'_'+f]=b[sb+'_'+key][f]-a[sa+'_'+key][f]
 for f in ['thread_requests','thread_requested_bytes','thread_freed_bytes','global_requests','global_requested_bytes','global_freed_bytes']:d[f]=b[sb+'_allocations'][f]-a[sa+'_allocations'][f]
 return d
for row in read(P/'profiles.json'):
 if not row['diagnostic']:continue
 trace=read(row['trace']);es=trace['capture']['events'];main=trace['main_thread'];root=next(e for e in es if e['stage']=='post_JSON');points=sorted([e for e in es if e['stage']=='coarse_checkpoint'],key=lambda e:e['operation']);ps=[(root,'start'),*[(e,'start')for e in points],(root,'end')];parts={name:delta(a,b,sa,sb)for name,((a,sa),(b,sb))in zip(names,zip(ps,ps[1:]))};total=delta(root,root)
 assert all(sum(x[k]for x in parts.values())==total[k]for k in total)
 assert all(v>=0 for part in parts.values()for v in part.values())
 workers={t:[e for e in es if e['thread']==t]for t in {e['thread']for e in es}-{main}};worker_deltas=[]
 for t,events in workers.items():
  a=min(events,key=lambda e:e['start_ns']);b=max(events,key=lambda e:e['end_ns']);assert points[2]['start_ns']<=a['start_ns']<=b['end_ns']<=root['end_ns'];worker_deltas.append(delta(a,b))
 residual=total['process_instructions']-total['thread_instructions']-sum(d['thread_instructions']for d in worker_deltas)
 data=dict(pair=row['pair'],variant=row['variant'],phases=parts,total=total,recorded_worker_thread_instructions=sum(d['thread_instructions']for d in worker_deltas),process_less_main_and_recorded_worker_instructions=residual,all_phases_telescope_to_continuous_interval=True,worker_activity_only_in_query_phase=True);runs.append(data);groups[row['variant']].append(data);root_counts[row['variant']].append(row['native_body_events'])
 if row['variant']=='canonical-control-detail':
  d=read(P/'hosts/canonical-control/build/descriptor.json')['descriptor'];events=[e for e in es if e['thread']==main];callbacks=[]
  for e in events:
   if e['stage']!='helper_ffi' or e['site']>=len(d['calls']):continue
   c=d['calls'][e['site']]
   if not c:continue
   key=c['template']['selected_key'];label='/'.join([*key['module_path'],key['operation_name']]);children=[x for x in events if x['stage']=='leaf_binding'and e['start_ns']<=x['start_ns']<=x['end_ns']<=e['end_ns']]
   if label in ['map/new','map/keys','core/composite-pack']and children:callbacks.append((e,label))
  seen=set()
  for e,label in sorted(callbacks,key=lambda a:a[0]['start_ns']):
   if label in seen:continue
   seen.add(label);inside=[x for x in events if x['stage'].startswith('leaf_')and e['start_ns']<=x['start_ns']<=x['end_ns']<=e['end_ns']];stages={s:{k:sum(delta(v,v)[k]for v in inside if v['stage']==s)for k in ['elapsed_ns','thread_instructions','thread_cycles','thread_requests','thread_requested_bytes','thread_freed_bytes']}for s in {v['stage']for v in inside}};full=delta(e,e);binding=stages.get('leaf_binding',{});invoke=stages.get('leaf_invocation',{});outside={k:full[k]-binding.get(k,0)-invoke.get(k,0)for k in ['elapsed_ns','thread_instructions','thread_cycles','thread_requests','thread_requested_bytes','thread_freed_bytes']};constructor[label].append(dict(pair=row['pair'],site=e['site'],callback=full,nested_stages=stages,outside_binding_and_invocation=outside))
assert len(runs)==40 and len(groups)==5 and all(len(v)==8 for v in groups.values())
summary={}
for variant,rs in groups.items():
 summary[variant]=dict(captures=len(rs),phases={name:{k:statistics.median(r['phases'][name][k]for r in rs)for k in rs[0]['phases'][name]}for name in names},total={k:statistics.median(r['total'][k]for r in rs)for k in rs[0]['total']},native_body_events=sorted(set(root_counts[variant])),worker_recorded_instructions=statistics.median(r['recorded_worker_thread_instructions']for r in rs),process_less_recorded_instructions=statistics.median(r['process_less_main_and_recorded_worker_instructions']for r in rs))
constructors={}
for name,cs in constructor.items():
 assert len(cs)==8
 constructors[name]=dict(captures=8,site=cs[0]['site'],callback={k:statistics.median(c['callback'][k]for c in cs)for k in cs[0]['callback']},nested_stages={s:{k:statistics.median(c['nested_stages'][s][k]for c in cs)for k in cs[0]['nested_stages'][s]}for s in cs[0]['nested_stages']},outside_binding_and_invocation={k:statistics.median(c['outside_binding_and_invocation'][k]for c in cs)for k in cs[0]['outside_binding_and_invocation']})
write(P/'aligned-phase-analysis.json',dict(diagnostic_only=True,promotion_timing_eligible=False,integer_phase_totals_exact_per_capture=True,all_worker_process_instructions_included=True,phase_medians_not_additive=True,phases=summary,constructor_details=constructors,limitations=['Counters include diagnostic probes and tracked-allocator observation; coarse and detailed instrumentation must be compared separately.','Mech phase0 includes canonical input admission, session setup and immutable-snapshot work absent in Rust. Necessary work differs, despite aligned semantic boundaries.','No worker elapsed-time sums are used; coarse elapsed time and process counters are continuous main-thread boundary snapshots.','Query phase includes worker create/run/join, index/other internal cleanup, result export/materialization. Rust generate temporary destruction is included even though its old inner query timer excluded part of it.','Detailed callback subscopes are inclusive; outside_binding_and_invocation is a disjoint residual containing wrapper preparation, binding destruction, probes and publication work outside invoke_scoped.'],runs=runs,constructors_raw=constructor))
for variant in ['rust-fastest-coarse','canonical-control-coarse','native-dictionary-coarse']:
 s=summary[variant];print(variant,'post ms',round(s['total']['elapsed_ns']/1e6,6),'M instructions',round(s['total']['process_instructions']/1e6,4),'alloc',s['total']['global_requests'],'bytes',s['total']['global_requested_bytes'],'native roots',s['native_body_events']);print(' phases',[(n,round(v['elapsed_ns']/1e6,6),v['process_instructions'],v['global_requests'])for n,v in s['phases'].items()])
for n,c in constructors.items():print('constructor',n,'callback',c['callback']['thread_instructions'],'nested',[(s,v['thread_instructions'],v['thread_requests'])for s,v in c['nested_stages'].items()],'outside',c['outside_binding_and_invocation']['thread_instructions'])
