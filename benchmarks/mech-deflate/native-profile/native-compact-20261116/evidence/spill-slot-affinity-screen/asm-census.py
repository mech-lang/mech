from pathlib import Path
from collections import Counter
import re
lines = Path('current.s').read_text().split('_mech_compact_closed_checked:\n', 1)[1].split('\n_', 1)[0].splitlines()
pc_re = re.compile(r'^([0-9a-f]+)\s+(\w+)\s*(.*)$')
stack_re = re.compile(r'([xwq]\d+|[xw]zr), \[sp(?:, #(-?(?:0x[0-9a-f]+|\d+)))?\]$')
insns = [(int(m[1], 16), m[2], m[3]) for line in lines if (m:=pc_re.match(line))]
ops = Counter(op for _,op,args in insns)
stack = Counter()
stack_reg = Counter()
pairs = []
for i,(pc,op,args) in enumerate(insns):
    if m:=stack_re.fullmatch(args):
        stack[op] += 1
        stack_reg[op,m[1]] += 1
        if i+1 == len(insns) or op not in ('ldr','ldur'):
            continue
        pc2,op2,args2 = insns[i+1]
        m2 = stack_re.fullmatch(args2)
        if m2 and op2 in ('str','stur') and m[1]==m2[1]:
            a,b = int(m[2] or '0',0),int(m2[2] or '0',0)
            pairs.append((i,a,b,m[1]))
pc_targets = set()
for pc,op,args in insns:
    if op.startswith('b') or op.startswith('tb') or op.startswith('cb'):
        if (match:=re.search(r'(?: |^)0x([0-9a-f]+)$', args)):
            pc_targets.add(int(match[1],16))
fours = []
eligible = []
for first,second in zip(pairs,pairs[1:]):
    i,a,b,r=first
    j,c,d,s=second
    if j!=i+2 or any(insns[k][0] in pc_targets for k in (i+1,j,j+1)):
        continue
    fours.append((first,second))
    if abs(a-c)==8 and c-a==d-b and b!=c:
        eligible.append((first,second))
print('instructions',len(insns),'bytes',len(insns)*4)
print('top',ops.most_common(27))
print('stack',stack)
print('top copy registers', Counter(r for i,a,b,r in pairs))
print('root consecutive stack word copies',len(pairs),'range',pairs[:12])
print('adjacent pairs of stack-word copies',len(fours),'paired offset candidates',len(eligible))
print('immediate GPR fits',sum(max(min(a,c),min(b,d))<=504 for (i,a,b,r),(j,c,d,s) in eligible))
print('immediate SIMD Q fits',sum(min(a,c)%16==min(b,d)%16==0 for (i,a,b,r),(j,c,d,s) in eligible))
print('register scratch fits',sum(r in ('x16','x17') and s in ('x16','x17') for (i,a,b,r),(j,c,d,s) in eligible))
print('pair examples', eligible[:15])
