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

def main():
 for name in ['binarytrees','binarytrees-parallel']:
  source=(HERE/(name+'.mec')).read_text();binary=build(name,source)
  for n in list(range(13))+[15]:
   exp,_=expected(n);assert output(binary,n)==exp,(name,n);checks.append([name,n,'byte-exact'])
   if n==10:assert exp==(HERE/'reference-10.txt').read_bytes()
  # Sequential accounting is observed after all source destruction, before CLI cleanup.
  if name=='binarytrees':
   binary=build('allocation-accounting',source+'\n(allocated, freed, live, peak) := heap/stats()\naccount := io/print("{} {} {} {}\\n", allocated, freed, live, peak)\n')
   for n in range(13):
    exp,allocated=expected(n);peak=(1<<(max(6,n)+2))-1
    assert output(binary,n)==exp+f'{allocated} {allocated} 0 {peak}\n'.encode(),n
    checks.append(['allocation-accounting',n,allocated,'all-nodes-including-leaves'])
 prefix='a := heap/new-refs((heap/null(), heap/null()))\n'
 identity=prefix+'b := heap/new-refs((heap/null(), heap/null()))\nc := heap/new-refs((a, b))\n(l, r) := heap/read-refs(c, 2)\nline := io/print("{} {}\\n", control/select(a != b, 1, 0), control/select(l == a && r == b, 1, 0))\n(x, y) := heap/take-refs(c, 2)\n(al, ar) := heap/take-refs(a, 2)\n(bl, br) := heap/take-refs(b, 2)\n(allocated, freed, live, peak) := heap/stats()\nio/print("{} {} {} {}\\n", allocated, freed, live, peak)\n'
 assert output(build('identity-alias-and-take',identity))==b'1 1\n3 3 0 3\n';checks.append(['identity-alias-and-take','distinct-allocations-and-exact-free'])
 fail('freed-read',prefix+'(l, r) := heap/take-refs(a, 2)\n(x, y) := heap/read-refs(a, 2)\nio/print("{}\\n", 1)\n',9)
 fail('double-take',prefix+'(l, r) := heap/take-refs(a, 2)\n(x, y) := heap/take-refs(a, 2)\nio/print("{}\\n", 1)\n',9)
 fail('wrong-arity',prefix+'(x, y, z) := heap/read-refs(a, 3)\nio/print("{}\\n", 1)\n',9)
 fail('null-read','(x, y) := heap/read-refs(heap/null(), 2)\nio/print("{}\\n", 1)\n',9)
 fail('recursion-limit','Again(root<ref>) = (result<ref>) :=\n  result := Again(root).\n'+prefix+'b := Again(a)\nio/print("{}\\n", 1)\n',8)
 fail('shift-domain','null := heap/null()\nx := number/shift-left(1, 64)\nio/print("{}\\n", x)\n',2)
 reject('integer-forgery','(x, y) := heap/read-refs(1, 2)\nio/print("{}\\n", 1)\n')
 reject('entry-capture','Hidden(x<ref>) = (result<u64>) :=\n  result := io/argument-u64(1).\n'+prefix+'n := Hidden(a)\nio/print("{}\\n", n)\n')
 hidden='Hidden(x<u64>) = (result<u64>) :=\n  ignored := io/print("hidden={}\\n", x)\n  result := x.\nChunk(start<u64>, end<u64>, arg<u64>) = (result<u64>) :=\n  result := Hidden(start).\nn := parallel/reduce-ranges(0, 10, 1, Chunk, 0, "sum")\nio/print("{}\\n", n)\n'
 reject('transitive-worker-io',hidden)
 leak='Chunk(start<u64>, end<u64>, arg<u64>) = (result<u64>) :=\n  root := heap/new-refs((heap/null(), heap/null()))\n  result := end - start.\nn := parallel/reduce-ranges(0, 100, 1, Chunk, 0, "sum")\nio/print("{}\\n", n)\n'
 fail('worker-escaping-record',leak,5)
 ordered='More(i<u64>, n<u64>) = (more<bool>) :=\n  more := i < n.\nStep(i<u64>, n<u64>) = (next<u64>, stop<u64>) :=\n  ignored := io/print("step={}\\n", i)\n  next := i + 1\n  stop := n.\nfirst := io/print("first={}\\n", 1)\n(i, n) := control/while((0, 3), More, Step)\nlast := io/print("last={}\\n", i)\n'
 assert output(build('ordered-source-effects',ordered))==b'first=1\nstep=0\nstep=1\nstep=2\nlast=3\n';checks.append(['ordered-source-effects','unused-effects-and-loop-order'])
 # Identical effectful branch/loop syntax must execute once per occurrence.
 distinct='Leaf(n<u64>) = (root<ref>) :=\n  root := heap/new-refs((heap/null(), heap/null())).\na := control/if(true, Leaf, Leaf, 0)\nb := control/if(true, Leaf, Leaf, 0)\nline := io/print("{}\\n", control/select(a != b, 1, 0))\n(al, ar) := heap/take-refs(a, 2)\n(bl, br) := heap/take-refs(b, 2)\n'
 assert output(build('distinct-effectful-branches',distinct))==b'1\n';checks.append(['distinct-effectful-branches','two-allocations'])
 twice=ordered.replace('last := io/print', '(j, m) := control/while((0, 3), More, Step)\nlast := io/print')
 assert output(build('distinct-effectful-loops',twice))==b'first=1\nstep=0\nstep=1\nstep=2\nstep=0\nstep=1\nstep=2\nlast=3\n';checks.append(['distinct-effectful-loops','both-loops-executed'])
 report={'compiler_sha256':run.sha(MECH),'checks':checks,'passed':len(checks)}
 (HERE/'results/conformance.json').write_text(json.dumps(report,indent=2)+'\n');print('Passed',len(checks),'checks',flush=True)
if __name__=='__main__':main()
