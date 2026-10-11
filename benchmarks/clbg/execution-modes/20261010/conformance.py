#!/usr/bin/env python3
"""Independent semantic fixtures for new general execution paths."""
from pathlib import Path
import hashlib, json, os, subprocess
HERE=Path(__file__).resolve().parent; CLBG=HERE.parents[1]; ROOT=CLBG.parents[1]
BIN='/tmp/mech-execution-modes-target/release/mech-execution-modes-compiled'
ENV=os.environ|{'RAYON_NUM_THREADS':'8','MECH_CONTROL_SCAN_CACHE':'0','MECH_CONTROL_EXACT_FLOAT_COMPARE':'0','MECH_SEQUENCE_ASYNC':'1','MECH_SEQUENCE_BUFFER_REUSE':'1'}
fixtures=[]
def case(name,source,expected=None,args=(),error=False,stdin=None):fixtures.append(dict(name=name,source=source,expected=expected,args=args,error=error,stdin=stdin))
case('parallel-state-copy','''More(a<u64>, b<u64>, i<u64>) => <bool> := i < 5
Step(a<u64>, b<u64>, i<u64>) = (x<u64>, y<u64>, j<u64>) :=
  x := b
  y := a
  j := i + 1.
(a, b, i) := control/while((11, 29, 0), More, Step)
io/print("{} {} {}\\n", a, b, i)
''',b'29 11 5\n')
case('lazy-branch','''Good(x<u64>) => <u64> := x + 3
Bad(x<u64>) => <u64> := sequence/get(sequence/zero-bytes(2), 3)
result := control/if(true, Good, Bad, 4)
io/print("{}\\n", result)
''',b'7\n')
case('partial-pattern','''f(x<u64>) => <u64>
  | 1 => 9.
io/print("{}\\n", f(io/argument-u64(1)))
''',args=('2',),error=True)
case('integer-range-limit','''n := stats/sum([1 | i <- 18446744073709551614..=18446744073709551615])
io/print("{}\\n", n)
''',b'2\n')
case('integer-range-empty','''n := stats/sum([i | i <- 9..2..=4])
io/print("{}\\n", n)
''',b'0\n')
case('invalid-bounded','''n := number/bounded(io/argument-u64(1), 3, 7)
io/print("{}\\n", n)
''',args=('8',),error=True)
case('float-cast-range','''n := stats/sum([number/to-f64(i) / 2.0 | i <- 0..=4])
io/print("{}\\n", control/select(n == 5.0, 1, 0))
''',b'1\n')
for name,expected in [('effects',b'touch:0\ntouch:1\ntouch:2\nresult:3\n'),('allocation',b'result:14\n')]:
 case(name,(CLBG/f'function-application-audit/20261009T171437Z/application-v3/builds/generic/{name}.project/source.mec').read_text(),expected)
case('signed-bigint-alias','''a := bigint/new(3)
b := bigint/new(7)
x := bigint/multiply-u64-into(a, a, 4)
y := bigint/subtract-multiple-u64(x, b, 3)
negative := control/select(bigint/less-equal(y, bigint/new(0)), 1, 0)
io/print("{}:", negative)
z := bigint/add-multiple-u64(y, b, 2)
quotient := bigint/floor-u64(z, b)
io/print("{}\\n", quotient)
''',b'1:0\n')
case('counted-scan-boundaries','''Byte(i<u64>) = (next<u64>, byte<u64>) :=
  next := i + 1
  byte := number/mod(i, 3) + 97.
last := io/write-scan(19, 0, Byte, 5, "|", 7)
io/print("end:{}\\n", last)
''',b'abcab|cabca|bcabc|abca|end:19\n')
case('rectangular-matrix','''rows := 0..2
columns := 0..3
C(i<u64>, j<u64>) => <f64> := number/to-f64(i * 3 + j + 1)
A := [[C(i, j) | j <- columns]' | i <- rows]'
x := [2.0 | j <- columns]'
y := A ** x
z := A' ** y
io/write(text/join-terminated(number/format(z, 0), " "))
''',b'132 174 216 ')
case('coefficient-unused-parameter','''rows := 0..2
C(i<u64>, j<u64>) => <f64> := number/to-f64(i + 1)
A := [[C(i, j) | j <- rows]' | i <- rows]'
x := [1.0 | j <- rows]'
io/write(text/join-terminated(number/format(A ** x, 0), " "))
''',b'2 4 ')
case('eager-coefficient-overflow','''rows := 0..2
C(i<u64>, j<u64>) => <f64> := number/to-f64(i - j)
A := [[C(i, j) | j <- rows]' | i <- rows]'
x := [1.0 | j <- rows]'
io/write(text/join-terminated(number/format(A ** x, 0), " "))
''',error=True)
case('captured-iteration','''bias := 2.0
Step(x<[f64]>) => <[f64]> := x + bias
x := [1.0 | i <- 0..3]'
y := sequence/iterate(x, 4, Step)
io/write(text/join-terminated(number/format(y, 0), " "))
''',b'9 9 9 ')
# Histograms are checked with an independently computed overlapping count,
# including widths crossing batch and partition boundaries and short inputs.
for size,width,bits,alphabet in [(3,4,1,'AB'),(1109,3,1,'AB'),(2061,7,3,'ABCDEFGH'),(1113,16,4,'ABCDEFGHIJKLMNOP')]:
 text=''.join(alphabet[(i*i+3*i+7)%len(alphabet)] for i in range(size));needle=text[:width];expected=sum(text[i:i+width]==needle for i in range(max(0,len(text)-width+1))) if len(needle)==width else 0
 case(f'histogram-{bits}-{width}-{size}',f'''s := bytes/encode("{text}", "{alphabet}", "exact")
h := map/histogram(sequence/packed-windows(s, {bits}, {width}))
io/write(number/format(map/get(h, sequence/pack(bytes/encode("{needle}", "{alphabet}", "exact"), {bits})), 0))
''',str(expected).encode())
# Source activation evaluates the two reactive assignments once; five turns swap again.
case('numeric-state-and-tail','''~left := [1.0, 2.0, 3.0]
~right := [10.0, 20.0, 30.0]
next-left := right
next-right := left
left = next-left
right = next-right
left[1] + left[2] + left[3] + 2.0 * (right[1] + right[2] + right[3])
''',b'72.000000000\n126.000000000\n',args=('5',))
checks=[]
for fixture in fixtures:
 path=HERE/'fixtures'/f"{fixture['name']}.mec";path.parent.mkdir(exist_ok=True);path.write_text(fixture['source'])
 for mode in ['jit','resident']:
  run=subprocess.run([BIN,str(path),mode,*fixture['args']],input=fixture['stdin'],capture_output=True,env=ENV,cwd=ROOT,timeout=60)
  correct=run.returncode!=0 and b'autonomous source requires' not in run.stderr if fixture['error'] else run.returncode==0 and run.stdout==fixture['expected']
  row={'case':fixture['name'],'mode':mode,'correct':correct,'exit_code':run.returncode,'stdout':run.stdout.decode(errors='replace'),'stderr':run.stderr.decode(),'source_sha256':hashlib.sha256(path.read_bytes()).hexdigest()};checks.append(row)
  print(fixture['name'],mode,'PASS' if correct else 'FAIL', '' if correct else str(row),flush=True)
(HERE/'conformance.json').write_text(json.dumps(checks,indent=2)+'\n')
assert all(row['correct'] for row in checks)
print('Passed',len(checks),'semantic checks',flush=True)
