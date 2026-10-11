#!/usr/bin/env python3
"""Independent checks of general native control and byte permutations."""
import json,os,subprocess,shutil
from pathlib import Path
import run
BUILD=run.BUILD/'conformance';BUILD.mkdir(exist_ok=True)
ENV=os.environ|{'RUSTFLAGS':'-D warnings','CARGO_TARGET_DIR':str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'RAYON_NUM_THREADS':'4'}
COMPILER=str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech')
checks=0

def build(name,source,simd=True):
 path=BUILD/(name+'.mec');path.write_text(source);binary=BUILD/name
 project=Path(str(binary)+'.project')
 if project.exists():shutil.rmtree(project)
 command=[COMPILER,'build',str(path),'--backend','cranelift','--offline','--no-config','--keep-project','-o',str(binary)]
 if not simd:command+=['--scalar']
 run.common.command(command,env=ENV);return binary

def execute(binary,args,expected=None,failed=False,env=ENV):
 global checks
 p=subprocess.run([str(binary),*map(str,args)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=30)
 if failed:assert p.returncode!=0 and not p.stdout,(p.stdout,p.stderr)
 else:assert p.returncode==0 and p.stdout.decode()==expected,(binary,args,p.stdout,p.stderr,expected)
 checks+=1

def checksum(a):
 result=0
 for x in a:result=(result*131+x)&((1<<64)-1)
 return result

def source(n):
 return f'''MakeMore(p<[u8]:{n},1>, i<u64>, seed<u64>) = (more<bool>) :=
  more := i <= {n}.
Make(p<[u8]:{n},1>, i<u64>, seed<u64>) = (q<[u8]:{n},1>, next<u64>, value<u64>) :=
  q := sequence/set(p, i, number/mod(i * 37 + seed, 256))
  next := i + 1
  value := seed.
More(p<[u8]:{n},1>, i<u64>, sum<u64>) = (more<bool>) :=
  more := i <= {n}.
Step(p<[u8]:{n},1>, i<u64>, sum<u64>) = (q<[u8]:{n},1>, next<u64>, checksum<u64>) :=
  q := p
  next := i + 1
  checksum := sum * 131 + sequence/get(p, i).
(p, i, seed) := control/while((sequence/zero-bytes({n}), 1, io/argument-u64(4)), MakeMore, Make)
reverse := sequence/reverse-prefix(p, io/argument-u64(1))
rotate := sequence/rotate-prefix(p, io/argument-u64(1), io/argument-u64(2))
update := sequence/set(p, io/argument-u64(3), number/mod(seed, 256))
(a, ia, original) := control/while((p, 1, 0), More, Step)
(b, ib, reversed) := control/while((reverse, 1, 0), More, Step)
(c, ic, rotated) := control/while((rotate, 1, 0), More, Step)
(d, id, updated) := control/while((update, 1, 0), More, Step)
io/write(text/format("{{}} {{}} {{}} {{}}\\n", original, reversed, rotated, updated))
'''

for simd in (False,True):
 for n in (1,2,15,16,17,32,33,64):
  binary=build(f'bytes-{n}-{int(simd)}',source(n),simd)
  cases=[(length,shift,(length%n)+1,73) for length in range(n+1) for shift in (0,1,n,n+1,(1<<64)-1)]
  if n==16:cases += [(seed%17,seed*7,seed%16+1,seed) for seed in range(256)]
  for length,shift,index,seed in cases:
   a=[(i*37+seed)%256 for i in range(1,n+1)];r=a[:length][::-1]+a[length:];offset=shift%length if length else 0;t=a[offset:length]+a[:offset]+a[length:];u=a.copy();u[index-1]=seed%256
   execute(binary,(length,shift,index,seed),' '.join(str(checksum(x)) for x in (a,r,t,u))+'\n')
  execute(binary,(n+1,0,1,0),failed=True);execute(binary,(n,0,0,0),failed=True);execute(binary,(n,0,n+1,0),failed=True)
  print('byte shape passed',n,'SIMD',simd,flush=True)
 branch='''Narrow(n<u64>, p<[u8]:16,1>) = (q<[u8]:16,1>) :=
  q := sequence/reverse-prefix(p, number/bounded(n, 0, 16)).
Wide(n<u64>, p<[u8]:16,1>) = (q<[u8]:16,1>) :=
  q := sequence/reverse-prefix(p, n).
p := control/if(io/argument-u64(1) == 1, Narrow, Wide, (io/argument-u64(2), sequence/iota-bytes(16)))
io/write(text/format("{}\\n", sequence/get(p, 1)))
'''
 binary=build('branch-proof-'+str(int(simd)),branch,simd)
 for which in (0,1):
  for length in (0,1,2,16):execute(binary,(which,length),str(max(0,length-1))+'\n')
  for length in (17,(1<<64)-1):execute(binary,(which,length),failed=True)
 # Bounds on a wrapped result must never imply a strictly positive index.
 wrap='index := number/bounded(io/argument-u64(1) + 1, 0, 16)\nio/write(text/format("{}\\n", sequence/get(sequence/iota-bytes(16), index)))\n'
 binary=build('wrapped-index-'+str(int(simd)),wrap,simd)
 for index in (0,1,14,15):execute(binary,(index,),str(index)+'\n')
 for index in (16,(1<<64)-1):execute(binary,(index,),failed=True)
 nowrap='index := number/bounded(io/argument-u64(1), 0, 15) + 1\nio/write(text/format("{}\\n", sequence/get(sequence/iota-bytes(16), index)))\n'
 binary=build('proved-index-'+str(int(simd)),nowrap,simd)
 for index in (0,1,14,15):execute(binary,(index,),str(index)+'\n')
 for index in (16,(1<<64)-1):execute(binary,(index,),failed=True)
 reductions='''More(i<u64>, end<u64>, factor<u64>, sum<u64>, maximum<u64>) = (more<bool>) :=
  more := i < end.
Step(i<u64>, end<u64>, factor<u64>, sum<u64>, maximum<u64>) = (next<u64>, stop<u64>, f<u64>, total<u64>, most<u64>) :=
  next := i + 1
  stop := end
  f := factor
  total := sum + i * factor
  most := number/max(maximum, i * factor).
Range(start<u64>, end<u64>, factor<u64>) = (sum<u64>, maximum<u64>) :=
  (i, stop, f, sum, maximum) := control/while((start, end, factor, 0, 0), More, Step).
(sum, maximum) := parallel/reduce-ranges(io/argument-u64(1), io/argument-u64(2), io/argument-u64(3), Range, io/argument-u64(4), "sum,max")
io/write(text/format("{} {}\\n", sum, maximum))
'''
 binary=build('ranges-'+str(int(simd)),reductions,simd)
 for workers in (1,3,8):
  for first,last,grain,factor in ((0,0,1,2),(0,1,1,17),(3,1003,7,9),(7,2003,1024,13)):
   execute(binary,(first,last,grain,factor),f'{sum(range(first,last))*factor} {max(range(first,last),default=0)*factor}\n',env=ENV|{'RAYON_NUM_THREADS':str(workers)})
 for args in ((2,1,1,1),(0,10,0,1)):execute(binary,args,failed=True)
 for name,value in [('factorial','number/factorial(io/argument-u64(1))'),('division','7 / io/argument-u64(1)'),('byte-overflow','sequence/get(sequence/set(sequence/iota-bytes(16), 1, io/argument-u64(1)), 1)')]:
  binary=build(name+'-'+str(int(simd)),f'io/write(text/format("{{}}\\n", {value}))\n',simd)
  execute(binary,(21 if name=='factorial' else 0 if name=='division' else 256,),failed=True)
 binary=build('signed-'+str(int(simd)),'io/write(text/format("{:i64}\\n", 0 - io/argument-u64(1)))\n',simd);execute(binary,(7,),'-7\n')
# Independent permutation order/flip reference, not the compiler's loop structure.
def reference(n):
 def permutations(values):
  if len(values)==1:yield values;return
  for _ in range(len(values)):
   for p in permutations(values[:-1]):yield p+values[-1:]
   values=values[1:]+values[:1]
 total=0;maximum=0
 for index,p in enumerate(permutations(list(range(n)))):
  flips=0
  while p[0]:p=p[:p[0]+1][::-1]+p[p[0]+1:];flips+=1
  total+=flips if index%2==0 else -flips;maximum=max(maximum,flips)
 return f'{total}\nPfannkuchen({n}) = {maximum}\n'
for policy in ('simd','scalar'):
 binary=run.BUILD/('mech-fannkuch-'+policy)
 for n in range(1,9):execute(binary,(n,),reference(n))
 for n in (0,17):execute(binary,(n,),failed=True)
report=dict(compiler_executable_sha256=run.sha(COMPILER),byte_index_policy=ENV.get("MECH_CONTROL_BYTE_INDEX","vector"),all_passed=True,checked_results=checks,byte_shapes=[1,2,15,16,17,32,33,64],all_prefix_lengths=True,all_byte_values=True,huge_rotation_distance=True,immutable_aliases=True,branch_proof_scope=True,positive_index_wrap_proof=True,scalar_and_simd=True,range_partitions_and_workers=True,independent_permutation_reference_n=list(range(1,9)))
(run.HERE/'results/control-conformance.json').write_text(json.dumps(report,indent=2)+'\n');print('All native control checks passed:',checks,flush=True)
