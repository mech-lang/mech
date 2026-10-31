"""Reconcile disjoint call-certificate construction stacks from complete traces."""
from pathlib import Path
import json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'schema':'mech.retained-call-ports-mechanism.v1','variants':{}}
def has(s,token):return any(token in f for f in s['frames'])
def call_clone(s):return any('BoundCall' in f and 'Clone5clone' in f for f in s['frames'])
def descriptor_clone(s):return any('ResolvedValueDescriptor' in f and 'Clone5clone' in f for f in s['frames'])
def call_construction(s):return any('BoundCall' in f and any(t in f for t in ['syntax_directed','from_resolved_call','artifact_operation_for_target','with_current_descriptors']) for f in s['frames'])
rules=[('bound_call_clone_descriptor',lambda s:call_clone(s) and descriptor_clone(s)),('bound_call_clone_other',call_clone),('call_construction',call_construction),('other_descriptor_clone',descriptor_clone),('current_shape',lambda s:has(s,'with_current_shape')),('other_binding',lambda s:has(s,'bind_current')),('other_turn_planning',lambda s:has(s,'derive_turn_memory')),('remaining',lambda s:True)]
for label in ['before','after']:
 d=json.loads((r/('sites-'+label)/'census.json').read_text());groups={name:{'requests':0,'requested_bytes':0,'reallocations':0,'stack_groups':0} for name,_ in rules}
 for site in d['sites']:
  name=next(name for name,test in rules if test(site));g=groups[name];g['requests']+=site['count'];g['requested_bytes']+=site['requested_bytes'];g['reallocations']+=site['count'] if site['kind']=='realloc' else 0;g['stack_groups']+=1
 assert sum(g['requests'] for g in groups.values())==d['requests']==d['counter']['allocation_requests']
 assert sum(g['requested_bytes'] for g in groups.values())==d['counter']['requested_bytes']
 result['variants'][label]={'groups':groups,'requests':d['requests'],'requested_bytes':d['counter']['requested_bytes'],'reallocations':d['counter']['reallocations']}
b=result['variants']['before'];a=result['variants']['after'];result['delta']={name:{key:a['groups'][name][key]-b['groups'][name][key] for key in ['requests','requested_bytes','reallocations']} for name,_ in rules};result['whole_call_delta']={key:a[key]-b[key] for key in ['requests','requested_bytes','reallocations']}
for key in result['whole_call_delta']:assert sum(g[key] for g in result['delta'].values())==result['whole_call_delta'][key]
result['scope']='Ordered first-match stack categories are mutually exclusive and sum exactly to each complete trace. Descriptor work nested in BoundCall cloning is the preselected target. Remaining BoundCall clone requests include operation/implementation metadata. Current call-constructor ancestry includes any newly allocated shared-array ownership plus other construction, where symbols remain visible; inlined sites may appear under binding or remaining. No prediction assumes new owners are free. Net complete-call census and independently reconciled heap cohorts are authoritative. Static symbol ancestry is not a dynamic object/call count or retired-instruction partition.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'delta':result['delta'],'whole_call_delta':result['whole_call_delta']},indent=2))
