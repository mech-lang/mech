from pathlib import Path
from collections import Counter
import hashlib,json,re
root=Path(__file__).resolve().parent
observations={}
for label,path in [('before',Path('/private/tmp/mech-scalar-paths-round22-20261029/fixed-primary/after.sample.txt')),('after',root/'final-primary/after.sample.txt')]:
 graph=path.read_text().split('Call graph:\n',1)[1].split('\nTotal number in stack',1)[0]
 nodes=[];stack=[]
 for line in graph.splitlines():
  match=re.match(r'^([ +!|:]*)(\d+) (.*)$',line)
  if not match:continue
  depth=len(match[1])
  while stack and nodes[stack[-1]]['depth']>=depth:stack.pop()
  node={'depth':depth,'count':int(match[2]),'symbol':match[3].split('  (in ',1)[0],'children':[],'ancestors':stack[:]}
  if stack:nodes[stack[-1]]['children'].append(len(nodes))
  stack.append(len(nodes));nodes.append(node)
 categories=Counter();leaves=Counter();budget=0;admission=0
 for n in nodes:
  weight=n['count']-sum(nodes[c]['count'] for c in n['children']);assert weight>=0
  symbols=[nodes[a]['symbol'] for a in n['ancestors']]+[n['symbol']]
  has=lambda s:any(s in name for name in symbols)
  if has('buffer_frame_action'):category='buffer_frame_actions'
  elif has('prepared_library_call'):category='selected_library_call'
  elif has('10Invocation6helper'):category='other_invocation_helper'
  elif has('10Invocation5visit'):category='ordinary_control_visit'
  else:category='remaining'
  categories[category]+=weight;leaves[n['symbol']]+=weight
  if 'evaluate_memory_budget' in n['symbol']:budget+=weight
  if has('admit_buffer_frame_inputs'):admission+=weight
 observations[label]={'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'total_samples':sum(categories.values()),'disjoint_categories':dict(categories),'exclusive_complete_budget_evaluator':budget,'frame_input_admission_inclusive':admission,'exclusive_top_leaves':leaves.most_common(30)}
assert observations['before']['total_samples']==1315
assert observations['before']['exclusive_complete_budget_evaluator']==58
(root/'partitioned-profile.json').write_text(json.dumps({'method':'Exclusive leaf=count-sum(children), partition by frame > selected call > other helper > ordinary visit > remaining ancestry; disjoint weights, not elapsed-time guarantees. Before is the prior recorded sample, not a paired profile.','observations':observations},indent=2)+'\n')
for k,v in observations.items():print(k,v['total_samples'],v['disjoint_categories'],'budget exclusive',v['exclusive_complete_budget_evaluator'])
