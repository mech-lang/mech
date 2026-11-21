from pathlib import Path
from collections import Counter,defaultdict
import json,argparse
parser=argparse.ArgumentParser(description='Rank checked-region candidates; the compiler must independently prove each boundary.')
parser.add_argument('inventory',type=Path)
parser.add_argument('--output',type=Path,default=Path('structural-candidates.json'))
args=parser.parse_args()
rows={}
for line in args.inventory.read_text().splitlines():
 s=line.split('\t'); rows[int(s[0])]={'cold':bool(int(s[1])),'status':s[2],'origin':s[3],'next':[int(x) for x in s[4].split(',') if x],'params':s[5],'ops':s[6].split(',')[:-1]}

def immediate_dominators(edges,start):
 pred=defaultdict(list)
 for b,ns in edges.items():
  for n in ns:pred[n].append(b)
 seen={start}; post=[]; todo=[(start,iter(edges[start]))]
 while todo:
  b,ns=todo[-1]
  try:n=next(ns)
  except StopIteration:post.append(b);todo.pop();continue
  if n not in seen:seen.add(n);todo.append((n,iter(edges[n])))
 order={b:i for i,b in enumerate(reversed(post))};dom={start:start}
 changed=True
 while changed:
  changed=False
  for b in reversed(post[:-1]):
   ps=[p for p in pred[b] if p in dom]
   if not ps:continue
   d=ps[0]
   for p in ps[1:]:
    while d!=p:
     if order[d]>order[p]:d=dom[d]
     else:p=dom[p]
   if dom.get(b)!=d:dom[b]=d;changed=True
 return dom,pred,order
edges={b:r['next'] for b,r in rows.items()}
dom,pred,order=immediate_dominators(edges,0)
children=defaultdict(list)
for b,p in dom.items():
 if b!=p:children[p].append(b)
# Success-only postdom ignores terminal checked-error edges. The actual region
# admission must still preserve every error return and shared return in Rust.
filtered={b:[n for n in ns if not (rows[n]['cold'] and not edges[n])] for b,ns in edges.items() if not (rows[b]['cold'] and not ns)}
filtered[-1]=[]
for b,ns in filtered.items():
 if b!=-1 and not ns:ns.append(-1)
rev={b:[] for b in filtered}
for b,ns in filtered.items():
 for n in ns:rev[n].append(b)
pdom,_,_=immediate_dominators(rev,-1)
rank=[]
for start,exit in pdom.items():
 if start<1 or exit<0:continue
 blocks=set();todo=[start]
 while todo:
  b=todo.pop()
  if b==exit or b in blocks:continue
  blocks.add(b);todo.extend(edges[b])
 if len(blocks)<20 or len(blocks)>7000:continue
 bad=[b for b in blocks if b!=start and any(p not in blocks for p in pred[b]) and not(rows[b]['cold'] and not edges[b])]
 if bad:continue
 ops=Counter(op.split(':')[0] for b in blocks for op in rows[b]['ops'])
 # A structural entry in a loop must not put a call into its inner iteration.
 # A backedge entering the entry from the proposed region means the loop is
 # wholly in the region; outside backedges are reported for review below.
 outside=[p for p in pred[start] if p not in blocks]
 rank.append({'entry':start,'exit':exit,'blocks':len(blocks),'instructions':sum(ops.values()),'calls':sum(n for op,n in ops.items() if op.startswith('call')),'outside_predecessors':outside,'exit_params':rows[exit]['params'],'first_ops':rows[start]['ops'][:12],'errors':sum(not edges[b] and rows[b]['cold'] for b in blocks)})
rank.sort(key=lambda x:-x['instructions'])
args.output.write_text(json.dumps(rank,indent=2)+'\n')
print('Candidates',len(rank),'graph',len(rows),'reachable',len(dom))
for r in rank[:18]:print(json.dumps(r))
