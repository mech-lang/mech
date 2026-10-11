#!/usr/bin/env python3
"""Exercise source-defined scans, typed state, byte bounds and ordered effects."""
import argparse,json,os,shutil,subprocess
import math,random
from pathlib import Path
import run
HERE=run.HERE;ROOT=run.ROOT;BUILD=run.BUILD/'conformance'
ENV=os.environ|{'RUSTFLAGS':'-D warnings -C target-cpu=native','CARGO_TARGET_DIR':str(ROOT/'benchmarks/clbg/nbody/mech-driver/target')}
MECH=ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'
def build(name,source,scalar=False):
 p=BUILD/(name+'.mec');p.write_text(source);binary=BUILD/name;project=Path(str(binary)+'.project')
 if project.exists():shutil.rmtree(project)
 result=subprocess.run([str(MECH),'build',str(p),'--backend','cranelift','--offline','--keep-project','--no-config','-o',str(binary)]+(['--scalar'] if scalar else []),cwd=ROOT,env=ENV,capture_output=True,text=True)
 if result.returncode:raise ValueError(name+'\n'+result.stdout+result.stderr)
 return binary

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output',type=Path,default=HERE/'results/conformance.json');args=parser.parse_args()
 BUILD.mkdir(parents=True,exist_ok=True);checks=[]
 generator=run.BUILD/'fasta-gcc-1-llvm'
 ns=[0,1,2,11,12,19,20,21,29,30,31,59,60,61,286,287,288,1000,21845,21846,32768,65535,65536,65537,100001]
 for source in ('fasta.mec','fasta-optimized.mec'):
  binary=build(source[:-4],(HERE/source).read_text())
  for n in ns:
   actual=subprocess.check_output([str(binary),str(n)])
   expected=subprocess.check_output([str(generator),str(n)])
   assert actual==expected,(source,n);checks.append([source,n])
 generic='''Step(i<u64>, f<f64>, b<bool>) = (next<u64>, value<f64>, toggle<bool>, byte<u64>) :=
  next := i + 1
  value := f + 0.5
  toggle := b == false
  byte := control/select(b, 65, 97) + control/select(f >= 4.0, 1, 0).
n := io/argument-u64(1)
width := io/argument-u64(2)
buffer := io/argument-u64(3)
io/write("same\\n")
io/write("same\\n")
(i, f, b) := io/write-scan(n, (0, 1.25, false), Step, width, "|", buffer)
io/write(text/format("count={}\\n", i))
'''
 binary=build('typed-state',generic)
 for n in (0,1,2,5,7,59,60,61,129):
  raw=bytes((65 if i%2 else 97)+(1 if 1.25+i*.5>=4 else 0) for i in range(n))
  for width in (0,1,7,60):
   for block in (1,7,64):
    expected=(b'|'.join(raw[i:i+width] for i in range(0,n,width))+b'|' if n and width else raw)
    expected=b'same\nsame\n'+expected+f'count={n}\n'.encode()
    actual=subprocess.check_output([str(binary),str(n),str(width),str(block)])
    assert actual==expected,(n,width,block,actual,expected);checks.append(['typed-state',n,width,block])
 for block in (0,1048577):
  result=subprocess.run([str(binary),'10','60',str(block)],capture_output=True)
  assert result.returncode!=0 and b'native control error' in result.stderr;checks.append(['invalid-buffer',block])
 for name,byte in [('invalid-byte','256'),('invalid-index','sequence/get(bytes/from-text("a"), i + 1)')]:
  binary=build(name,f'''Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := i + 1
  byte := {byte}.
s := io/write-scan(io/argument-u64(1), 0, Step, 0, "", 7)
''')
  result=subprocess.run([str(binary),'10'],capture_output=True)
  assert result.returncode!=0 and result.stdout==b'';checks.append([name,'guarded-before-block-publication'])
 for index in ('i + 1','0','4'):
  lookup=f'''Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := i + 1
  p := sequence/f64-values((1.25, 2.5, 3.75))
  byte := control/select(sequence/get(p, {index}) > 1.5, 65, 97).
s := io/write-scan(2, io/argument-u64(1), Step, 0, "", 7)
'''
  binary=build('readonly-'+index.replace(' ','').replace('+','plus'),lookup)
  if index=='i + 1':
   assert subprocess.check_output([str(binary),'0'])==b'aA';checks.append(['readonly-f64','valid'])
  for initial in ([2,18446744073709551615] if index=='i + 1' else [0]):
   result=subprocess.run([str(binary),str(initial)],capture_output=True)
   assert result.returncode!=0 and result.stdout==b'';checks.append(['readonly-f64',index,initial,'guarded'])
 # The callback must remain closed and pure: source output cannot hide inside it.
 impure='''Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := i + io/write("hidden")
  byte := 65.
s := io/write-scan(1, 0, Step, 0, "", 7)
'''
 try:build('impure',impure)
 except ValueError:checks.append(['impure-source-callback','rejected'])
 else:raise AssertionError('impure source callback admitted')
 try:build('unused-io',impure.replace('next := i + io/write("hidden")','ignored := io/write("hidden")\n  next := i + 1'))
 except ValueError:checks.append(['unused-IO-in-callback','rejected'])
 else:raise AssertionError('unused callback IO admitted')
 for multiplier,increment,modulus,maximum in [(17,29,97,96),(65535,65535,65537,65536),(1,1,4294967296,4294967295),(4294967295,29573,4294967311,18446744073709551615)]:
  arithmetic=f'''Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := number/mod(i * {multiplier} + {increment}, {modulus})
  byte := number/mod(next, 256).
s := io/write-scan(129, number/bounded(io/argument-u64(1), 0, {maximum}), Step, 0, "", 7)
'''
  for narrow in ('0','1'):
   ENV['MECH_CONTROL_NARROW_UINT']=narrow
   binary=build(f'uint-{modulus}-{narrow}',arithmetic)
   for initial in (0,1,maximum):
    state=initial;expected=bytearray()
    for _ in range(129):
     state=((state*multiplier+increment)%(2**64))%modulus;expected.append(state%256)
    assert subprocess.check_output([str(binary),str(initial)])==expected,(modulus,narrow,initial)
    checks.append(['unsigned-arithmetic',modulus,narrow,initial])
 ENV.pop('MECH_CONTROL_NARROW_UINT',None)
 # Exercise exact quotient and remainder over the complete unsigned 32-bit
 # domain, including divisors that need the backend's fallback recipe.
 rng=random.Random(20261007)
 for divisor in (3,7,17,97,65535,65537,139968,2147483647,2147483649,4294967295):
  arithmetic=f'''Step(x<u64>, q<u64>, r<u64>) = (same<u64>, quotient<u64>, remainder<u64>, byte<u64>) :=
  same := x
  quotient := x / {divisor}
  remainder := number/mod(x, {divisor})
  byte := 0.
(x, q, r) := io/write-scan(1, (number/bounded(io/argument-u64(1), 0, 4294967295), 0, 0), Step, 0, "", 1)
io/write(text/format("{{}} {{}}\\n", q, r))
'''
  ENV['MECH_CONTROL_NARROW_UINT']='1'
  binary=build(f'divisor-{divisor}',arithmetic)
  values=sorted(set([0,1,divisor-1,divisor,divisor+1,2147483647,2147483648,4294967294,4294967295]+[rng.randrange(2**32) for _ in range(128)]))
  for value in (v for v in values if v<2**32):
   expected=b'\0'+f'{value//divisor} {value%divisor}\n'.encode()
   assert subprocess.check_output([str(binary),str(value)])==expected,(divisor,value)
   checks.append(['uint32-quotient-remainder',divisor,value])
 ENV.pop('MECH_CONTROL_NARROW_UINT',None)
 # SIMD Boolean sums must preserve all IEEE predicates, NaNs, infinities,
 # exact equality and wrapping integer sums, including an odd scalar remainder.
 source=''
 operators=['==','!=','<','<=','>','>=']
 for index,op in enumerate(operators):
  source+=f'Count{index}(r<f64>, p<f64>) = (count<u64>) :=\n  count := control/select(r {op} p, 1, 0).\n'
 source+='''Step(i<u64>) = (next<u64>, byte<u64>) :=
  next := i + 1
  r := control/select(i == 3, 1.0 / 0.0, control/select(i == 4, 0.0 / 0.0, number/to-f64(i) / 2.0))
'''
 for group in range(6):
  for j,threshold in enumerate(('0.0 - 1.0','0.0','0.5','1.0','1.0 / 0.0')):
   source+=f'  c{group}{j} := Count{group}(r, {threshold})\n'
 terms=[f'c{group}{j}' for group in range(6) for j in range(5)]
 source+='  byte := number/mod(18446744073709551615 + '+' + '.join(terms)+', 128).\n'
 source+='s := io/write-scan(1, io/argument-u64(1), Step, 0, "", 1)\n'
 compare=[lambda a,b:a==b,lambda a,b:a!=b,lambda a,b:a<b,lambda a,b:a<=b,lambda a,b:a>b,lambda a,b:a>=b]
 for scalar in (False,True):
  binary=build('ieee-scalar' if scalar else 'ieee-simd',source,scalar)
  for i,r in enumerate((0.0,.5,1.0,math.inf,math.nan)):
   count=sum(int(op(r,p)) for op in compare for p in (-1.0,0.0,.5,1.0,math.inf))
   expected=bytes([((2**64-1+count)%(2**64))%128])
   assert subprocess.check_output([str(binary),str(i)])==expected,(scalar,i)
   checks.append(['IEEE-counts',scalar,i])
 args.output.parent.mkdir(parents=True,exist_ok=True)
 args.output.write_text(json.dumps({'checks':checks,'passed':len(checks),'all_passed':True,'compiler_executable_sha256':run.sha(MECH),'compiler_snapshot':run.snapshot(args.output)},indent=2)+'\n')
 print('passed',len(checks),flush=True)
if __name__=='__main__':main()
