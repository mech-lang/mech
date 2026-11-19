from pathlib import Path
from collections import deque
import re,json
L=Path(__file__).resolve().parent;source=(L/'current.clif').read_text();params={};incoming={};defs={};block=None
for line in source.splitlines():
 line=line.split(';',1)[0].strip()
 if m:=re.fullmatch(r'(block\d+)(?:\((.*)\))?:',line):
  block=m[1]
  for i,v in enumerate(re.findall(r'(v\d+):',m[2] or '')):params[v]=(block,i)
 elif m:=re.match(r'(v\d+) = (.*)',line):defs[m[1]]=(block,m[2])
 elif line.startswith(('jump ','brif ')):
  for dest,args in re.findall(r'(block\d+)(?:\(([^()]*)\))?',line):incoming.setdefault(dest,[]).append((block,re.findall(r'v\d+',args)))
def constant(v):
 if v not in defs:return None
 m=re.fullmatch(r'iconst\.i64 ([-\w]+)',defs[v][1])
 if not m:return None
 try:return int(m[1].replace('_',''),0)&((1<<64)-1)
 except ValueError:return int(m[1].replace('_',''))&((1<<64)-1)
def offset(v):
 bias=0;seen=set()
 while v not in seen:
  seen.add(v);text=defs.get(v,('',None))[1]
  if text is None:break
  if m:=re.fullmatch(r'iadd_imm\.i64 (v\d+), (-?[\dxabcdef_]+)',text):
   v=m[1];raw=m[2].replace('_','');bias=(bias+int(raw,0))&((1<<64)-1)
  elif m:=re.fullmatch(r'iadd\.i64 (v\d+), (v\d+)',text):
   a,b=m.groups();c=constant(b)
   if c is not None:v=a;bias=(bias+c)&((1<<64)-1)
   else:
    c=constant(a)
    if c is None:break
    v=b;bias=(bias+c)&((1<<64)-1)
  else:break
 return v,bias
def equivalent(value,reference):
 root,origin=offset(reference);q=deque([(value,0)]);seen=set();seeds=0;unknown=[]
 while q:
  if len(seen)>4096:return {'result':'budget','nodes':len(seen)}
  v,bias=q.popleft();v,delta=offset(v);bias=(bias+delta-origin)&((1<<64)-1)
  if (v,bias) in seen:continue
  seen.add((v,bias))
  if v==root and bias==0:seeds+=1;continue
  if v not in params:unknown.append({'value':v,'offset':bias,'definition':defs.get(v)});continue
  b,i=params[v]
  for predecessor,args in incoming.get(b,[]):
   if len(args)<=i:unknown.append({'missing':(b,i,predecessor,args)});continue
   q.append((args[i],bias+origin))
 return {'result':bool(seeds and not unknown),'nodes':len(seen),'seeds':seeds,'unknown':unknown[:20],'visited':sorted(seen)}
result=equivalent('v11961','v11551');(L/'index-phi-analysis.json').write_text(json.dumps({'scope':'Diagnostic structural copy/constant-offset traversal of current source-pinned CLIF, not permission to change code without a source implementation and dominance/boundary tests.','reference':'v11551','candidate':'v11961','analysis':result},indent=2)+'\n');print(json.dumps(result,indent=2))
