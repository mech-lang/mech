"""Reconcile the owned observed-call copy separately from other planner copies."""
from pathlib import Path
import json,sys
r=Path(sys.argv[1]);out=Path(sys.argv[2]);result={'schema':'mech.owned-observed-call-mechanism.v1','variants':{}}
def has(s,t):return any(t in f for f in s['frames'])
def call_clone(s):return any('CallMemoryPlan' in f and 'Clone5clone' in f for f in s['frames'])
def turn_clone(s):return any('TurnMemoryPlan' in f and 'Clone5clone' in f for f in s['frames'])
rules=[('observed_owned_call_clone',lambda s:has(s,'apply_observed_turn_demand') and call_clone(s)),('initial_call_clone',lambda s:has(s,'derive_turn_memory') and call_clone(s)),('active_turn_plan_clone',turn_clone),('other_observed_refinement',lambda s:has(s,'apply_observed_turn_demand')),('other_initial_turn',lambda s:has(s,'derive_turn_memory')),('remaining',lambda s:True)]
for label in ['before','after']:
 d=json.loads((r/('sites-'+label)/'census.json').read_text());groups={name:{'requests':0,'requested_bytes':0,'reallocations':0,'stack_groups':0} for name,_ in rules}
 for site in d['sites']:
  name=next(name for name,test in rules if test(site));g=groups[name];g['requests']+=site['count'];g['requested_bytes']+=site['requested_bytes'];g['reallocations']+=site['count'] if site['kind']=='realloc' else 0;g['stack_groups']+=1
 assert sum(g['requests'] for g in groups.values())==d['requests']==d['counter']['allocation_requests']
 assert sum(g['requested_bytes'] for g in groups.values())==d['counter']['requested_bytes']
 result['variants'][label]={'groups':groups,'requests':d['requests'],'requested_bytes':d['counter']['requested_bytes'],'reallocations':d['counter']['reallocations']}
b=result['variants']['before'];a=result['variants']['after'];result['delta']={name:{key:a['groups'][name][key]-b['groups'][name][key] for key in ['requests','requested_bytes','reallocations']} for name,_ in rules};result['whole_call_delta']={key:a[key]-b[key] for key in ['requests','requested_bytes','reallocations']}
for key in result['whole_call_delta']:assert sum(g[key] for g in result['delta'].values())==result['whole_call_delta'][key]
result['scope']='Ordered disjoint allocation stack families sum exactly to complete traces and separate heap counters. Owned call copy in observed refinement is the selected target. Initial call/template and active turn-plan copies have different lifetimes and remain separate. Source does not substitute another planner or alter current facts/permissions. Symbol ancestry is not a complete object/call or retired-instruction ledger.'
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'delta':result['delta'],'whole_call_delta':result['whole_call_delta']},indent=2))
