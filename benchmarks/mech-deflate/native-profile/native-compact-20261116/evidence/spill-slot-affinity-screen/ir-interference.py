"""Conservative CLIF phi liveness probe. No machine allocation claims."""
from collections import Counter, defaultdict, deque
from itertools import combinations
from pathlib import Path
import runpy
ctx = runpy.run_path(str(Path(__file__).with_name('clif-census.py')))
incoming, params, defs, blocks, atom = [ctx[k] for k in ('incoming','params','definitions','blocks','atom')]
use = defaultdict(dict)
define = {}
block_head, block_tail = {}, {}
pc = 0
for n, instructions in blocks.items():
    block_head[n] = pc
    for p in params[n]:
        define[p] = (n, pc)
    for i, op in enumerate(instructions):
        if ' = ' in op:
            left, op = op.split(' = ', 1)
            for v in atom.findall(left):
                define[v] = (n, pc+1)
        for v in atom.findall(op):
            use[v][n] = pc + 1
        pc += 2
    block_tail[n] = pc
preds={n: set(a for a,args in incoming[n]) for n in blocks}
# Full exact live intervals per CLIF use. Ranges are half-open.
lives = {}
for v,(n,start) in define.items():
    ends = use[v]
    if not ends:
        continue
    ends = ends.copy()
    queue = deque(k for k in ends if k != n)
    while queue:
        k = queue.pop()
        for p in preds[k]:
            if ends.get(p) != block_tail[p]:
                if p not in ends and p != n:
                    queue.append(p)
                ends[p] = block_tail[p]
    lives[v] = {k:(start if k==n else block_head[k],e) for k,e in ends.items()}
    assert min(b for b,e in lives[v].values())<max(e for b,e in lives[v].values()), (v,lives[v])

def intersects(x,y):
    a,b=lives[x],lives[y]
    if len(a)>len(b): a,b=b,a
    return any(c<e and d>b[k][0] for k,(c,d) in a.items() if k in b for e in [b[k][1]])

hull_ranges={v:(min(a for a,b in i.values()),max(b for a,b in i.values())) for v,i in lives.items()}
stat=Counter()
for n,p in params.items():
    if not p or len(incoming[n])<2: continue
    for a, args in incoming[n]:
        for q,v in zip(p,args):
            if q not in lives or v not in lives or q==v: continue
            m,M=hull_ranges[q]
            s,S=hull_ranges[v]
            full = intersects(q,v)
            hull = m<S and s<M
            stat[(full,hull)] += 1
print('live raw phi edges by actual/hull interference',stat)

parent = {}
subranges={v:i.copy() for v,i in lives.items()}
def root(v):
    while v in parent:
        v=parent[v]
    return v

def colliding(a,b):
    if len(a)>len(b): a,b=b,a
    return any(c<e and d>b[k][0] for k,(c,d) in a.items() if k in b for e in [b[k][1]])

pairs={}
failed=[]
for n,pp in params.items():
    if not pp or len(incoming[n])<2: continue
    for a,args in incoming[n]:
        for q,v in zip(pp,args):
            if q not in lives or v not in lives or q==v: continue
            l,r=root(q),root(v)
            if l==r: continue
            if colliding(subranges[l],subranges[r]):
                failed.append((q,v))
                continue
            parent[r]=l
            for k,(s,e) in subranges[r].items():
                if k in subranges[l]:
                    s,e=min(s,subranges[l][k][0]),max(e,subranges[l][k][1])
                subranges[l][k] = (s,e)
            del subranges[r]
print('raw failed future phi-bundle coalescing',len(failed), 'eventual congruence classes',len(subranges))
print('individual raw phi transfer overlap despite bundle conflict',sum(intersects(q,v) for q,v in failed))
