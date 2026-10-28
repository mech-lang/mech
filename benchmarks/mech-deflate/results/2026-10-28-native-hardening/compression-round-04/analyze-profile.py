from pathlib import Path
from collections import Counter
import json,re,statistics
root=Path(__file__).resolve().parent
text=(root/'after.sample.txt').read_text();graph=text.split('Call graph:\n',1)[1].split('\nTotal number in stack',1)[0]
nodes=[];stack=[];roots=[]
for line in graph.splitlines():
 m=re.match(r'^([ +!|:]*)(\d+) (.*)$',line)
 if not m:continue
 depth=len(m[1]);n={'depth':depth,'count':int(m[2]),'symbol':m[3],'children':[],'ancestors':[]}
 while stack and nodes[stack[-1]]['depth']>=depth:stack.pop()
 n['ancestors']=stack[:]
 if stack:nodes[stack[-1]]['children'].append(len(nodes))
 else:roots.append(len(nodes))
 stack.append(len(nodes));nodes.append(n)
exclusive=Counter();target=0;precharge=0;total=0
for n in nodes:
 leaf=n['count']-sum(nodes[c]['count'] for c in n['children']);assert leaf>=0,n
 total+=leaf;symbol=n['symbol'].split('  (in ',1)[0];exclusive[symbol]+=leaf
 ancestry=[nodes[a]['symbol'] for a in n['ancestors']]+[n['symbol']]
 if any('28clone_allocation_bound_bytes' in a for a in ancestry):target+=leaf
 if any('43clone_allocation_bound_bytes_with_precharge' in a for a in ancestry):precharge+=leaf
assert total==sum(nodes[i]['count'] for i in roots)
summary={'leaf_samples':total,'roots':[(nodes[i]['symbol'],nodes[i]['count']) for i in roots], 'no_callback_bound_inclusive_samples':target,'ordered_precharge_inclusive_samples':precharge,'top_exclusive':exclusive.most_common(25),'note':'Named-symbol attribution only; no unsupported resolution of deduplicated or JIT addresses. Sampling is diagnostic, not a timing fraction.'}
(root/'after-profile-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps({k:summary[k] for k in ['leaf_samples','no_callback_bound_inclusive_samples','ordered_precharge_inclusive_samples']},indent=2));print(exclusive.most_common(8))
a=json.loads((root/'word-update-results.json').read_text())['observations'];b=json.loads(Path('/private/tmp/mech-schema-bound-round03-20261028/word-update-results.json').read_text())['observations'];rows=[]
for kind,words in [('u16',262144),('u32',1500513)]:
 row={'kind':kind,'words':words,'stores':8}
 for label,data in [('before',b),('after',a)]:
  selected=[r for r in data if r['kind']==kind and r['words']==words and r['stores']==8 and r['rep']>0];assert len(selected)==2
  row[label]={'median_seconds':statistics.median(r['seconds'] for r in selected),'allocations':selected[0]['allocations'],'requested_bytes':selected[0]['requested_bytes'],'work':selected[0]['work']}
 assert row['after']['allocations'] < row['before']['allocations']
 assert row['after']['requested_bytes'] < row['before']['requested_bytes']
 assert row['after']['work'] == row['before']['work']
 rows.append(row)
(root/'word-comparison.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps(rows,indent=2))
