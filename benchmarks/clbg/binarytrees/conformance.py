#!/usr/bin/env python3
"""Validate source tree work, opaque reference lifetimes and closed worker scopes."""
import hashlib,json,os,shutil,subprocess
from pathlib import Path
import run
HERE=run.HERE;ROOT=run.ROOT;BUILD=run.BUILD/'conformance';BUILD.mkdir(parents=True,exist_ok=True)
MECH=ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'
ENV=os.environ|{'RUSTFLAGS':'-D warnings -C target-cpu=native','CARGO_TARGET_DIR':str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'RAYON_NUM_THREADS':'8'}
checks=[]
def build(name,source):
 p=BUILD/(name+'.mec');p.write_text(source);binary=BUILD/name;project=Path(str(binary)+'.project')
 if project.exists():shutil.rmtree(project)
 r=subprocess.run([str(MECH),'build',str(p),'--backend','cranelift','--offline','--keep-project','--no-config','-o',str(binary)],cwd=ROOT,env=ENV,capture_output=True,text=True)
 if r.returncode:raise ValueError(r.stdout+r.stderr)
 return binary
def output(binary,*args):return subprocess.check_output([str(binary),*map(str,args)],env=ENV)
def reject(name,source):
 try:build(name,source)
 except ValueError:checks.append([name,'compile-rejected'])
 else:raise AssertionError(name+' admitted')
def fail(name,source,status):
 binary=build(name,source);r=subprocess.run([str(binary)],capture_output=True,env=ENV)
 assert r.returncode and f'native control error {status}'.encode() in r.stderr,(name,r.stderr)
 checks.append([name,'runtime-guard',status])
def expected(n):
 n=max(6,n);s=f'stretch tree of depth {n+1}\t check: {(1<<(n+2))-1}\n';allocated=(1<<(n+2))-1+(1<<(n+1))-1
 for d in range(4,n+1,2):
  iterations=1<<(n-d+4);count=iterations*((1<<(d+1))-1);allocated+=count;s+=f'{iterations}\t trees of depth {d}\t check: {count}\n'
 return (s+f'long lived tree of depth {n}\t check: {(1<<(n+1))-1}\n').encode(),allocated

def scoped(body, capacity=3, arity=2):
 return 'Test(n<u64>) = (result<u64>) :=\n'+''.join('  '+line+'\n' for line in body.splitlines())+'\nresult := memory/with-ref-arena('+str(capacity)+', '+str(arity)+', Test, 0)\nio/print("{}\\n", result)\n'
def main():
 for name in ['binarytrees','binarytrees-parallel']:
  source=(HERE/(name+'.mec')).read_text();binary=build(name,source)
  for n in list(range(13))+[15]:
   exp,_=expected(n);assert output(binary,n)==exp,(name,n);checks.append([name,n,'byte-exact'])
   if n==10:assert exp==(HERE/'reference-10.txt').read_bytes()
 identity=scoped('a := heap/new-refs((heap/null(), heap/null()))\nb := heap/new-refs((heap/null(), heap/null()))\nc := heap/new-refs((a, b))\n(l, r) := heap/read-refs(c, 2)\nresult := control/select(a != b && l == a && r == b, 1, 0).')
 assert output(build('identity-and-aliases',identity))==b'1\n';checks.append(['identity-and-aliases','distinct-leaves-and-child-links'])
 fail('outside-arena','a := heap/new-refs((heap/null(), heap/null()))\nio/print("{}\\n", 1)\n',9)
 fail('capacity-exhaustion',identity.replace('with-ref-arena(3,','with-ref-arena(2,'),9)
 fail('wrong-arity',scoped('a := heap/new-refs((heap/null(), heap/null()))\n(x, y, z) := heap/read-refs(a, 3)\nresult := 1.'),9)
 fail('allocation-arity',identity.replace('with-ref-arena(3, 2,','with-ref-arena(3, 3,'),9)
 fail('null-read',scoped('(x, y) := heap/read-refs(heap/null(), 2)\nresult := 1.'),9)
 fail('zero-capacity',identity.replace('with-ref-arena(3,','with-ref-arena(0,'),9)
 fail('capacity-overflow',identity.replace('with-ref-arena(3,','with-ref-arena(18446744073709551615,'),9)
 fail('frame-outside-arena','Test(n<u64>) = (result<u64>) :=\n  result := n.\nr := memory/with-frame(Test, 1)\nio/print("{}\\n", r)\n',9)
 fail('recursion-limit','Again(n<u64>) = (result<u64>) :=\n  result := Again(n).\nr := Again(0)\nio/print("{}\\n", r)\n',8)
 fail('shift-domain','x := number/shift-left(1, 64)\nio/print("{}\\n", x)\n',2)
 reject('integer-forgery','(x, y) := heap/read-refs(1, 2)\nio/print("{}\\n", 1)\n')
 reject('retired-per-node-api',scoped('a := heap/new-refs((heap/null(), heap/null()))\n(l, r) := heap/take-refs(a, 2)\nresult := 1.'))
 reject('reference-escape','Test(n<u64>) = (result<ref>) :=\n  result := heap/new-refs((heap/null(), heap/null())).\nr := memory/with-ref-arena(1, 2, Test, 0)\nio/print("{}\\n", 1)\n')
 reject('cross-region-reference','Inner(r<ref>) = (result<u64>) :=\n  result := 1.\n'+scoped('a := heap/new-refs((heap/null(), heap/null()))\nresult := memory/with-ref-arena(1, 2, Inner, a).'))
 reject('frame-reference-escape','Inner(n<u64>) = (result<ref>) :=\n  result := heap/new-refs((heap/null(), heap/null())).\n'+scoped('a := memory/with-frame(Inner, n)\nresult := 1.'))
 reject('frame-reference-capture','Inner(r<ref>) = (result<u64>) :=\n  result := 1.\n'+scoped('a := heap/new-refs((heap/null(), heap/null()))\nresult := memory/with-frame(Inner, a).'))
 reject('entry-capture','Hidden(n<u64>) = (result<u64>) :=\n  result := io/argument-u64(1).\nn := Hidden(0)\nio/print("{}\\n", n)\n')
 hidden='Hidden(x<u64>) = (result<u64>) :=\n  ignored := io/print("hidden={}\\n", x)\n  result := x.\nChunk(start<u64>, end<u64>, arg<u64>) = (result<u64>) :=\n  result := Hidden(start).\nn := parallel/reduce-ranges(0, 10, 1, Chunk, 0, "sum")\nio/print("{}\\n", n)\n'
 reject('transitive-worker-io',hidden)
 fail('unscoped-worker-record','Chunk(start<u64>, end<u64>, arg<u64>) = (result<u64>) :=\n  root := heap/new-refs((heap/null(), heap/null()))\n  result := end - start.\nn := parallel/reduce-ranges(0, 100, 1, Chunk, 0, "sum")\nio/print("{}\\n", n)\n',5)
 reuse='Leaf(n<u64>) = (result<u64>) :=\n  a := heap/new-refs((heap/null(), heap/null()))\n  result := control/select(a != heap/null(), 1, 0).\nMore(i<u64>, n<u64>, sum<u64>) = (more<bool>) :=\n  more := i < n.\nStep(i<u64>, n<u64>, sum<u64>) = (next<u64>, stop<u64>, check<u64>) :=\n  count := memory/with-frame(Leaf, 0)\n  next := i + 1\n  stop := n\n  check := sum + count.\n'+scoped('(i, stop, result) := control/while((0, 100, 0), More, Step).',1)
 assert output(build('bounded-frame-reuse',reuse))==b'100\n';checks.append(['bounded-frame-reuse','100-leaf-allocations-in-one-record-capacity'])
 fail('unframed-exhaustion',reuse.replace('memory/with-frame(Leaf, 0)','Leaf(0)'),9)
 nested='Inner(n<u64>) = (result<u64>) :=\n  a := heap/new-refs((heap/null(), heap/null()))\n  result := control/select(a != heap/null(), 1, 0).\n'+scoped('a := heap/new-refs((heap/null(), heap/null()))\nx := memory/with-ref-arena(1, 2, Inner, 0)\n(l, r) := heap/read-refs(a, 2)\nresult := control/select(l == heap/null() && r == heap/null(), x, 0).',1)
 assert output(build('nested-independent-realizations',nested))==b'1\n';checks.append(['nested-independent-realizations','outer-reference-remains-live'])
 ordered='More(i<u64>, n<u64>) = (more<bool>) :=\n  more := i < n.\nStep(i<u64>, n<u64>) = (next<u64>, stop<u64>) :=\n  ignored := io/print("step={}\\n", i)\n  next := i + 1\n  stop := n.\nfirst := io/print("first={}\\n", 1)\n(i, n) := control/while((0, 3), More, Step)\nlast := io/print("last={}\\n", i)\n'
 assert output(build('ordered-source-effects',ordered))==b'first=1\nstep=0\nstep=1\nstep=2\nlast=3\n';checks.append(['ordered-source-effects','unused-effects-and-loop-order'])
 distinct='Leaf(n<u64>) = (root<ref>) :=\n  root := heap/new-refs((heap/null(), heap/null())).\n'+scoped('a := control/if(true, Leaf, Leaf, 0)\nb := control/if(true, Leaf, Leaf, 0)\nresult := control/select(a != b, 1, 0).',2)
 assert output(build('distinct-effectful-branches',distinct))==b'1\n';checks.append(['distinct-effectful-branches','two-allocations'])
 twice=ordered.replace('last := io/print', '(j, m) := control/while((0, 3), More, Step)\nlast := io/print')
 assert output(build('distinct-effectful-loops',twice))==b'first=1\nstep=0\nstep=1\nstep=2\nstep=0\nstep=1\nstep=2\nlast=3\n';checks.append(['distinct-effectful-loops','both-loops-executed'])
 report={'compiler_sha256':run.sha(MECH),'storage':'core MemoryDomain / RealizedMemoryPlan / PlannedArenaProjection<u64>','checks':checks,'passed':len(checks)}
 (HERE/'results/conformance.json').write_text(json.dumps(report,indent=2)+'\n');print('Passed',len(checks),'checks',flush=True)
if __name__=='__main__':main()
