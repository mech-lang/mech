from pathlib import Path
from collections import Counter
import json,re
root=Path(__file__).resolve().parent
for label,path in [('pinned',Path('/private/tmp/mech-mixed-scalar-state-round08-20261028/after.sample.txt')),('independent',root/'after.sample.txt')]:
 text=path.read_text();graph=text.split('Call graph:\n',1)[1].split('\nTotal number in stack',1)[0]
 nodes=[];stack=[]
 for line in graph.splitlines():
  m=re.match(r'^([ +!|:]*)(\d+) (.*)$',line)
  if not m:continue
  depth=len(m[1]);n={'depth':depth,'count':int(m[2]),'symbol':m[3].split('  (in ',1)[0],'children':[],'ancestors':[]}
  while stack and nodes[stack[-1]]['depth']>=depth:stack.pop()
  n['ancestors']=stack[:]
  if stack:nodes[stack[-1]]['children'].append(len(nodes))
  stack.append(len(nodes));nodes.append(n)
 inclusive=Counter();total=0
 for n in nodes:
  leaf=n['count']-sum(nodes[c]['count'] for c in n['children']);total+=leaf
  for s in {nodes[a]['symbol'] for a in n['ancestors']}|{n['symbol']}:inclusive[s]+=leaf
 focus=['buffer_frames','frame_shape','admit_buffer','charge_compute_work','charge_control_account','observe_control_work','evaluate_memory_budget','10Invocation5visit','10Invocation6helper','prepared_library_call','word_reads','word_updates','native_project_structural','RuntimeState','Session','turn_plan','scalar']
 out={'label':label,'total_samples':total,'top_inclusive':inclusive.most_common(40),'focus_inclusive':[(s,c) for s,c in inclusive.most_common() if any(x in s for x in focus)][:70], 'interpretation':'Inclusive ancestors overlap; no addition of these counts. Unresolved/deduplicated/JIT names are not attributed.'}
 (root/(label+'-inclusive-profile.json')).write_text(json.dumps(out,indent=2)+'\n')
 print(label,total)
 for s,c in out['focus_inclusive'][:30]:print(c,s)
