#!/usr/bin/env python3
"""Permutation source variants and admission rejection checks."""
import json,os,subprocess
import run
from pathlib import Path
checks=0

def reference(n):
 def permutations(a):
  if len(a)==1:yield a;return
  for _ in a:
   for p in permutations(a[:-1]):yield p+a[-1:]
   a=a[1:]+a[:1]
 total=maximum=0
 for i,p in enumerate(permutations(list(range(n)))):
  flips=0
  while p[0]:p=p[:p[0]+1][::-1]+p[p[0]+1:];flips+=1
  total+=flips if i%2==0 else -flips;maximum=max(maximum,flips)
 return f'{total}\nPfannkuchen({n}) = {maximum}\n'.encode()
for policy in ('simd','scalar','coarse','paired','cached','simd-stack-index','paired-stack-index','cached-stack-index','default','even','aligned'):
 binary=run.BUILD/('mech-fannkuch-'+policy)
 for n in range(1,9):
  result=subprocess.run([str(binary),str(n)],env=os.environ|{'RAYON_NUM_THREADS':'3'},capture_output=True,check=True);assert result.stdout==reference(n),(policy,n,result.stdout);checks+=1
 print('independent permutations passed',policy,flush=True)
fixtures={
 'callback-io':'''Range(start<u64>, end<u64>, n<u64>) = (out<u64>) :=
  out := io/argument-u64(1).
result := parallel/reduce-ranges(0, 1, 1, Range, 7, "sum")
io/write(text/format("{}\\n", result))
''',
 'recursion':'''Recurse(n<u64>) = (out<u64>) :=
  out := Recurse(n).
io/write(text/format("{}\\n", Recurse(1)))
''',
 'capture':'''n := 17
Captured(i<u64>) = (out<u64>) :=
  out := i + n.
io/write(text/format("{}\\n", Captured(1)))
''',
 'wrong-loop-state':'''More(n<u64>) = (more<bool>) :=
  more := n < 5.
Step(n<u64>) = (out<bool>) :=
  out := n > 3.
out := control/while(1, More, Step)
io/write(text/format("{}\\n", out))
''',
 'array-range-argument':'''Range(start<u64>, end<u64>, p<[u8]:16,1>) = (out<u64>) :=
  out := sequence/get(p, 1).
result := parallel/reduce-ranges(0, 1, 1, Range, sequence/iota-bytes(16), "sum")
io/write(text/format("{}\\n", result))
'''}
for name,source in fixtures.items():
 path=run.BUILD/'conformance'/(name+'.mec');path.write_text(source)
 result=subprocess.run([str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),'build',str(path),'--backend','cranelift','--offline','--no-config','-o',str(path.with_suffix(''))],env=os.environ|{'RUSTFLAGS':'-D warnings','CARGO_TARGET_DIR':str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target')},capture_output=True)
 assert result.returncode!=0,name;checks+=1
 print('rejection passed',name,flush=True)
(run.HERE/'results/source-conformance.json').write_text(json.dumps(dict(all_passed=True,checked_results=checks,source_variants=['simd','scalar','coarse','paired','cached','simd-stack-index','paired-stack-index','cached-stack-index','default','even','aligned'],independent_n=list(range(1,9)),rejections=list(fixtures)),indent=2)+'\n')
