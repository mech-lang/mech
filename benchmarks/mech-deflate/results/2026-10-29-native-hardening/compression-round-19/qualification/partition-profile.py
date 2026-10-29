from pathlib import Path
from collections import Counter
import hashlib,json,re
root=Path(__file__).resolve().parent
observations={}
for label,path in [
 ('round13',Path('/private/tmp/mech-numeric-results-round13-20261029/primary/after.sample.txt')),
 ('round14',Path('/private/tmp/mech-frame-admission-round14-20261029/final-primary/after.sample.txt')),
 ('round15',Path('/private/tmp/mech-predicate-versions-round15-20261029/primary/after.sample.txt')),
 ('round16',Path('/private/tmp/mech-record-precharge-round16-20261029/primary/after.sample.txt')),
 ('round17',Path('/private/tmp/mech-binding-descriptors-round17-20261029/primary/after.sample.txt')),
 ('round18',Path('/private/tmp/mech-record-boundaries-round18-20261029/primary/after.sample.txt')),
 ('round19',root/'primary/after.sample.txt'),
]:
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
 categories=Counter();admission=Counter();leaves=Counter()
 for n in nodes:
  weight=n['count']-sum(nodes[c]['count'] for c in n['children']);assert weight>=0
  symbols=[nodes[a]['symbol'] for a in n['ancestors']]+[n['symbol']]
  has=lambda s:any(s in name for name in symbols)
  if has('19buffer_frame_action'):category='buffer_frame_actions'
  elif has('21prepared_library_call'):category='selected_library_call'
  elif has('10Invocation6helper'):category='other_invocation_helper'
  elif has('10Invocation5visit'):category='ordinary_control_visit'
  else:category='remaining'
  categories[category]+=weight;leaves[n['symbol']]+=weight
  if any(name.endswith('25admit_buffer_frame_inputs') for name in symbols):
   if has('measure_native_scalar_frame_footprint'):part='native_scalar_footprint'
   elif has('frame_shape') and has('validate'):part='frame_shape_validation'
   elif any(name.endswith('32measure_borrowed_value_footprint') for name in symbols):part='managed_value_footprint'
   else:part='other_admission'
   admission[part]+=weight
 total=sum(categories.values())
 observations[label]={'path':str(path),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'total_samples':total,'disjoint_categories':dict(categories),'admission_partition':dict(admission),'admission_total':sum(admission.values()),'exclusive_top_leaves':leaves.most_common(25)}
assert observations['round13']['disjoint_categories']=={'buffer_frame_actions':472,'other_invocation_helper':377,'ordinary_control_visit':213,'selected_library_call':196,'remaining':89},observations['round13']
assert observations['round13']['admission_total']==329
(root/'final-partitioned-profile.json').write_text(json.dumps({'method':'Reconstruct call tree; exclusive leaf=count-sum(child counts), partition by ancestry with frame > selected call > helper > visit > remaining precedence. Categories are disjoint. Sampling shares are not elapsed-time guarantees.','observations':observations},indent=2)+'\n')
for label,d in observations.items():print(label,d['total_samples'],d['disjoint_categories'],d['admission_partition'])
