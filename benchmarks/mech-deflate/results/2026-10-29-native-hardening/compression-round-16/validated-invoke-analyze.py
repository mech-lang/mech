from pathlib import Path
from collections import Counter
import re,json
root=Path(__file__).resolve().parent
path=Path('/private/tmp/mech-predicate-versions-round15-20261029/primary/after.sample.txt')
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

roots=Counter();paths=Counter()
for n in nodes:
 weight=n['count']-sum(nodes[c]['count'] for c in n['children'])
 syms=[nodes[a]['symbol'] for a in n['ancestors']]+[n['symbol']]
 if any('19buffer_frame_action' in s or '21prepared_library_call' in s for s in syms):continue
 if not any('21invoke_current_cached' in s for s in syms):continue
 indexes=[i for i,s in enumerate(syms) if '24invoke_validated_current' in s]
 if not indexes:continue
 i=indexes[-1]
 chain=syms[i+1:]
 roots[chain[0] if chain else '(invoke exclusive)']+=weight
 if weight:paths[' -> '.join(syms[i:])]+=weight
assert sum(roots.values())==422
report={'total':sum(roots.values()),'subtrees':roots.most_common(),'weighted_paths':paths.most_common()}
(root/'validated-invoke-costs.json').write_text(json.dumps(report,indent=2)+'\n')
for symbol,count in roots.most_common(25):print(count,symbol)
