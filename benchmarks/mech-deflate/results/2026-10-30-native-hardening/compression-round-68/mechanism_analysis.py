"""Reconcile disjoint observed descriptor construction families from both full traces."""
from pathlib import Path
from collections import Counter
import json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'schema':'mech.shared-descriptor-semantics-mechanism.v1','variants':{}}
def has(site,token):return any(token in f for f in site['frames'])
def clone(site):return any('ResolvedValueDescriptor' in f and 'Clone5clone' in f for f in site['frames'])
def construction(site):return any('ResolvedValueDescriptor' in f and ('3new' in f or 'from_schema' in f) for f in site['frames'])
rules=[('descriptor_clone',clone),('current_shape',lambda s:has(s,'with_current_shape')),('identified_descriptor_construction',construction),('other_binding',lambda s:has(s,'bind_current')),('other_turn_planning',lambda s:has(s,'derive_turn_memory')),('remaining',lambda s:True)]
for label in ['before','after']:
 d=json.loads((r/('sites-'+label)/'census.json').read_text());groups={name:{'requests':0,'requested_bytes':0,'reallocations':0,'stack_groups':0} for name,_ in rules}
 for site in d['sites']:
  name=next(name for name,test in rules if test(site));g=groups[name];g['requests']+=site['count'];g['requested_bytes']+=site['requested_bytes'];g['reallocations']+=site['count'] if site['kind']=='realloc' else 0;g['stack_groups']+=1
 assert sum(g['requests'] for g in groups.values())==d['requests']==d['counter']['allocation_requests']
 assert sum(g['requested_bytes'] for g in groups.values())==d['counter']['requested_bytes']
 result['variants'][label]={'groups':groups,'requests':d['requests'],'requested_bytes':d['counter']['requested_bytes'],'reallocations':d['counter']['reallocations']}
b=result['variants']['before'];a=result['variants']['after'];result['delta']={name:{key:a['groups'][name][key]-b['groups'][name][key] for key in ['requests','requested_bytes','reallocations']} for name,_ in rules};result['whole_call_delta']={key:a[key]-b[key] for key in ['requests','requested_bytes','reallocations']}
for key in result['whole_call_delta']:assert sum(g[key] for g in result['delta'].values())==result['whole_call_delta'][key]
result['scope']='Ordered first-match categories are mutually exclusive and sum exactly to each full trace. Descriptor clone and current-shape ancestry expose the removed deep semantic construction; identified constructor ancestry is only the observed symbol family, not a complete count of semantic definitions. Other binding/planning are residual after earlier rules, not their overlapping subtree totals. New immutable owners can add requests elsewhere; the net complete-call delta remains authoritative. No per-function retired-instruction counts are inferred.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'delta':result['delta'],'whole_call_delta':result['whole_call_delta']},indent=2))
