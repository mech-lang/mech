from pathlib import Path
import json,sys
r=Path(sys.argv[1]);output=Path(sys.argv[2]);rows=[]
for repetition in range(2):
 data=json.loads((r/('small-allocations/resolved'+str(repetition)+'.json')).read_text());groups={}
 for site in data['sites']:
  assert sum(p>0 for p in site['phases'])==1, 'A mixed-phase site needs per-phase bytes'
  project=[(i,s) for i,s in enumerate(site['symbols']) if 'mech_' in s and 'once_box' not in s]
  assert project,site['symbols']
  at,symbol=project[0];key=symbol.split(' (in ',1)[0]
  if any('once_box' in s for s in site['symbols'][:at]):key='lazy mutex → '+key
  group=groups.setdefault(key,{'constructor':key,'count':0,'bytes':0,'freed':0,'survivors':0,'min':site['min'],'max':site['max'],'max_age_requests':0,'total_age_requests':0,'phases':[0,0,0],'free_phases':[0,0,0],'examples':[]})
  for field in ['count','bytes','freed','survivors','total_age_requests']:group[field]+=site[field]
  group['min']=min(group['min'],site['min']);group['max']=max(group['max'],site['max']);group['max_age_requests']=max(group['max_age_requests'],site['max_age_requests'])
  for field in ['phases','free_phases']:group[field]=[a+b for a,b in zip(group[field],site[field])]
  if len(group['examples'])<2:group['examples'].append(site['symbols'][:12])
 ordered=sorted(groups.values(),key=lambda x:(-x['count'],x['constructor']))
 for group in ordered:group['mean_lifetime_in_requests']=group['total_age_requests']/group['freed'] if group['freed'] else None
 result={k:data[k] for k in ['sample','requests','bytes','large_requests','large_bytes','phase_requests','phase_bytes']};result.update({'groups':ordered,'top_five':ordered[:5],'small_requests':sum(s['count'] for s in ordered),'small_bytes':sum(s['bytes'] for s in ordered),'small_phase_requests':[sum(s['phases'][i] for s in ordered) for i in range(3)],'small_phase_bytes':[sum(s['bytes'] for s in data['sites'] if s['phases'][i]) for i in range(3)],'raw_stack_sites':len(data['sites'])});rows.append(result)
assert [(g['constructor'],g['count'],g['bytes'],g['phases'],g['survivors']) for g in rows[0]['groups']]==[(g['constructor'],g['count'],g['bytes'],g['phases'],g['survivors']) for g in rows[1]['groups']]
output.write_text(json.dumps({'scope':'Two fresh executions, steady-state sample 1; all traces outside primary timing. Lifetimes are request ages through next reallocation/free, not undisturbed nanoseconds. Phase 1 before linked entry, 2 inside generated entry, 3 after its return.','repetitions':rows},indent=2)+'\n')
