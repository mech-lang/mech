"""Reconcile the removed owned adapter with both fresh whole-call traces."""
from pathlib import Path
from collections import Counter
import json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);results={};histograms={}
def hist(sites):
 h=Counter()
 for s in sites:h[(s['kind'],s['bytes_each'])]+=s['count']
 return h
def row(sites):return {'requests':sum(s['count'] for s in sites),'requested_bytes':sum(s['requested_bytes'] for s in sites),'stack_groups':len(sites)}
for label,inner in [('before','plan_turn_memory'),('after','derive_turn_memory')]:
 d=json.loads((r/('sites-'+label)/'census.json').read_text());sites=d['sites'];groups={}
 has=lambda s,token:any(token in f for f in s['frames'])
 groups['native_planner']=[s for s in sites if has(s,'plan_current')]
 groups['inner_planner']=[s for s in sites if has(s,inner)]
 groups['outer_owned_wrapper']=[s for s in groups['native_planner'] if not has(s,inner)]
 groups['complete_resolution']=[s for s in sites if has(s,'resolve_current_call_memory')]
 groups['binding']=[s for s in sites if has(s,'bind_current')]
 results[label]={k:row(v) for k,v in groups.items()};histograms[label]={k:hist(v) for k,v in groups.items()};histograms[label]['whole_call']=hist(sites)
 results[label]['nested_groups_overlap']=True
b=results['before'];a=results['after'];assert b['outer_owned_wrapper']['requests']==157 and b['outer_owned_wrapper']['requested_bytes']==19238
assert a['outer_owned_wrapper']['requests']==0
for key in ['inner_planner','complete_resolution','binding']:assert histograms['before'][key]==histograms['after'][key],key
saved=histograms['before']['whole_call'].copy();saved.subtract(histograms['after']['whole_call']);saved=Counter({k:v for k,v in saved.items() if v})
assert saved==histograms['before']['outer_owned_wrapper']
result={'schema':'mech.borrowed-turn-source-mechanism.v1','groups':results,'removed_size_kind_histogram':[{'kind':k,'bytes_each':size,'count':n} for (k,size),n in sorted(saved.items())],'exact_removed_wrapper_histogram_equals_whole_call_delta':True,'inner_planning_binding_and_resolution_histograms_unchanged':True,'scope':'Fresh current traces: the 157-request/19238-byte outer adapter disappears exactly. The remaining 414-request/52969-byte turn planner and 447-request/40431-byte binding remain. These nested family counts overlap; do not sum them. Histogram equality establishes count/size evidence, not unchanged instruction or exact call counts. Current facts and output plans still get freshly derived.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(results,indent=2))
