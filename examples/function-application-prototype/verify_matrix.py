#!/usr/bin/env python3
"""Independent checks for matrix producer fusion, captures, shapes and error order."""
import argparse, json, math, os, pathlib, subprocess, tempfile
HERE=pathlib.Path(__file__).resolve().parent;ROOT=HERE.parents[1]
TARGET_DIR=pathlib.Path(os.environ.get('MECH_TARGET_DIR',os.environ.get('CARGO_TARGET_DIR',ROOT/'target'))).expanduser()
if not TARGET_DIR.is_absolute():TARGET_DIR=ROOT/TARGET_DIR
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--cli',type=pathlib.Path,default=os.environ.get('MECH_CLI',TARGET_DIR/'release'/('mech.exe' if os.name=='nt' else 'mech')))
parser.add_argument('--report',type=pathlib.Path,default=HERE/'matrix-verification-current.json')
options=parser.parse_args();CLI=options.cli.expanduser()
ENV=os.environ|{'CARGO_TARGET_DIR':str(TARGET_DIR),'RUSTFLAGS':'-D warnings','MECH_NUMERIC_UNROLL':'4','MECH_NUMERIC_ROWS':'4','RAYON_NUM_THREADS':'8'}
def build(source,path,scalar=False,valid=True,unroll=None):
 source_path=path.with_suffix('.mec');source_path.write_text(source)
 args=[str(CLI),'build',str(source_path),'--backend','cranelift','--no-config','--offline','-o',str(path)]
 if scalar:args+=['--scalar']
 build_env=ENV if unroll is None else ENV|{'MECH_NUMERIC_UNROLL':str(unroll)}
 result=subprocess.run(args,cwd=ROOT,env=build_env,capture_output=True)
 if valid and result.returncode:raise RuntimeError(result.stderr.decode()+result.stdout.decode())
 if not valid:assert result.returncode,(source,'invalid program accepted')
 return result

def run(binary,*args,partitions=1,input=b''):
 return subprocess.run([str(binary),*map(str,args)],env=ENV|{'MECH_SEQUENCE_PARTITIONS':str(partitions)},input=input,capture_output=True)
def vector_line(values):return ','.join(f'{x:.9f}' for x in values)+'\n'
def print_vector(expression):return f'io/write(string/concat(text/join(number/format({expression}, 9), ","), "\\n"))\n'

source='''rows := io/argument-u64(1)
columns := io/argument-u64(2)
ri := 0..rows
cj := 0..columns
A := [[number/to-f64((i + 1) * (j + 2)) | j <- cj]' | i <- ri]'
step(x<[f64]>) => <[f64]> := A' ** (A ** x)
x := [1.5 | j <- cj]'
y := A ** x
z := A' ** y
iterated := sequence/iterate(x, 2, step)
'''+print_vector('y')+print_vector('z')+print_vector('iterated')
checked=0;cases=[]
with tempfile.TemporaryDirectory(prefix='mech-matrix-proof-') as temp:
 temp=pathlib.Path(temp)
 for scalar in (True,False):
  binary=temp/('rect-scalar' if scalar else 'rect-simd');build(source,binary,scalar)
  for rows,columns in [(0,0),(0,3),(3,0),(1,1),(2,3),(3,2),(7,9),(9,7),(8,8),(17,15)]:
   def multiply(x,transpose=False):
    if transpose:return [sum(x[i]*(i+1)*(j+2) for i in range(rows)) for j in range(columns)]
    return [sum(x[j]*(i+1)*(j+2) for j in range(columns)) for i in range(rows)]
   x=[1.5]*columns;y=multiply(x);z=multiply(y,True);v=x
   for _ in range(2):v=multiply(multiply(v),True)
   expected=(vector_line(y)+vector_line(z)+vector_line(v)).encode()
   for partitions in (1,3,8):
    result=run(binary,rows,columns,partitions=partitions);assert not result.returncode and result.stdout==expected,(scalar,rows,columns,partitions,result.stdout,expected,result.stderr)
    checked+=3;cases.append({'scalar':scalar,'rows':rows,'columns':columns,'partitions':partitions})
 captures='''offset := io/argument-u64(1)
helper(x<f64>) => <f64> := x + number/to-f64(offset)
shadow(offset<f64>) => <f64> := helper(offset)
step(x<f64>) => <f64> := shadow(x)
nested(x<f64>) => <f64> := sequence/iterate(x, 2, step)
a := sequence/iterate(1.0, 3, step)
b := sequence/iterate(1.0, 2, nested)
c := 1.0 |> shadow
io/write(string/concat(number/format(a, 9), "\\n"))
io/write(string/concat(number/format(b, 9), "\\n"))
io/write(string/concat(number/format(c, 9), "\\n"))
'''
 build(captures,temp/'captures')
 for offset in (0,2,7):
  result=run(temp/'captures',offset);expected=f'{1+3*offset:.9f}\n{1+4*offset:.9f}\n{1+offset:.9f}\n'.encode()
  assert not result.returncode and result.stdout==expected,(result,expected)
  checked+=3
 # Validation must occur where a coefficient matrix is constructed, before IO.
 for name,formula,message in [('overflow','number/to-f64(18446744073709551615 + i + 1)','overflow'),('division','number/to-f64(i / (j - j))','division by zero')]:
  error_source=f'''ri := 0..2
cj := 0..3
A := [[{formula} | j <- cj]' | i <- ri]'
io/write("should-not-appear")
x := [1.0 | j <- cj]'
y := A ** x
'''+print_vector('y')
  build(error_source,temp/name);result=run(temp/name)
  assert result.returncode==1 and message.encode() in result.stderr and not result.stdout,(name,result)
 # Competing exceptions must follow source generator order: i then j.
 ordered="""ri := 0..2
cj := 0..2
A := [[number/to-f64((18446744073709551615 - i) + j) + number/to-f64(1 / (1 - i + j)) | j <- cj]' | i <- ri]'
x := [1.0 | j <- cj]'
y := A ** x
"""+print_vector('y')
 build(ordered,temp/'ordered');result=run(temp/'ordered');assert result.returncode==1 and b'overflow' in result.stderr and not result.stdout,result
 # Conservative range proof fails for cancellation, but scalar validation succeeds.
 formula='number/to-f64((j + j) - j + i + 1)'
 good=f'''ri := 0..2
cj := 0..3
A := [[{formula} | j <- cj]' | i <- ri]'
x := [1.0 | j <- cj]'
y := A ** x
'''+print_vector('y')
 build(good,temp/'fallback');result=run(temp/'fallback');assert not result.returncode and result.stdout==b'6.000000000,9.000000000\n',result
 mismatch=source.replace('x := [1.5 | j <- cj]\'', 'x := sequence/fill(columns + 1, 1.5)')
 build(mismatch,temp/'mismatch');result=run(temp/'mismatch',2,3);assert result.returncode==1 and b'shapes differ' in result.stderr and not result.stdout,result
 impure=source.replace('number/to-f64((i + 1) * (j + 2))','number/to-f64(io/argument-u64(1))')
 build(impure,temp/'impure',valid=False)
 # Exact floating topology: multiplying a stored reciprocal must retain its
 # rounding. Unroll one makes a two-column input exercise the SIMD body.
 rounding='''ri := 0..1
cj := 0..2
A := [[1.0 / 10.0 | j <- cj]' | i <- ri]'
x := [3.0 | j <- cj]'
y := A ** x
observed := stats/dot(y, sequence/fill(1, 1.0)) * 1e17
io/write(string/concat(number/format(observed, 0), "\\n"))
'''
 for scalar in (True,False):
  build(rounding,temp/'rounding',scalar,unroll=1)
  result=run(temp/'rounding')
  assert not result.returncode and result.stdout==b'60000000000000008\n',(scalar,result)
  checked+=1
 # Two conversion boundaries, an invariant row cast, sqrt, division, and
 # cancellation all exercise the general vectorized floating suffix.
 suffix='''rows := io/argument-u64(1)
columns := io/argument-u64(2)
ri := 0..rows
cj := 0..columns
A := [[math/sqrt(number/to-f64(i + 1) + number/to-f64(j + 1)) / number/to-f64(j + 10) - 0.16 | j <- cj]' | i <- ri]'
x := [3.125 | j <- cj]'
y := A ** x
'''+print_vector('y').replace('format(y, 9)','format(y, 12)')
 for scalar in (True,False):
  build(suffix,temp/'suffix',scalar,unroll=1)
  for rows,columns in [(1,2),(3,2),(7,9),(8,8),(9,17)]:
   expected=[sum(3.125*(math.sqrt(float(i+1)+float(j+1))/float(j+10)-0.16) for j in range(columns)) for i in range(rows)]
   for partitions in (1,8):
    result=run(temp/'suffix',rows,columns,partitions=partitions)
    assert not result.returncode,(scalar,rows,columns,result)
    actual=[float(v) for v in result.stdout.decode().strip().split(',')]
    assert len(actual)==len(expected) and all(math.isclose(a,e,rel_tol=2e-14,abs_tol=5.1e-13) for a,e in zip(actual,expected)),(scalar,rows,columns,actual,expected)
    checked+=rows
 # A deliberately inconclusive interval selects the checked numeric kernel.
 # Lane zero divides by zero after a safe addition; lane one would overflow
 # at its earlier addition. Finishing the integer prefix lane-by-lane must
 # preserve division-by-zero as the first error.
 checked_suffix='''entry(i<u64>, j<u64>) => <f64> := number/to-f64(18446744073709551615 + j) + number/to-f64(1 / j)
x := sequence/fill(2, 3.0)
y := matrix/apply(x, 1, entry, "multiply")
'''+print_vector('y')
 for scalar in (True,False):
  build(checked_suffix,temp/'checked-suffix',scalar,unroll=1)
  result=run(temp/'checked-suffix')
  assert result.returncode==1 and b'division by zero' in result.stderr and not result.stdout,(scalar,result)
 for scalar in (True,False):
  build((HERE/'spectralnorm-matrix-proposed.mec').read_text(),temp/'norm',scalar)
  sizes=[1,2,3,5,7,8,9,15,16,17,31,100,5500]
  for n in sizes:
   def mult(v,transpose=False):return [sum(v[j]/((i+j)*(i+j+1)//2+(j if transpose else i)+1) for j in range(n)) for i in range(n)]
   if n<=100:
    v=[1.0]*n
    for _ in range(19):v=mult(mult(v),True)
    u=mult(mult(v),True);expected=f'{math.sqrt(sum(a*b for a,b in zip(u,v))/sum(a*a for a in v)):.9f}\n'.encode()
   else:expected=b'1.274224153\n'
   result=run(temp/'norm',n,partitions=8);assert not result.returncode and result.stdout==expected,(scalar,n,result.stdout,expected,result.stderr)
   checked+=1
report={'cases':cases,'checked_outputs':checked,'scalar_and_simd':True,'rectangular_empty_and_tail_shapes':True,'nested_immutable_captures_and_lexical_shadowing':True,'eager_unsigned_error_order':True,'conservative_bounds_validation_fallback':True,'shape_errors_and_impure_formulas_rejected':True,'simd_float_suffix_sqrt_casts_cancellation_and_reciprocal_rounding':True,'checked_integer_lane_error_order':True,'spectralnorm_sizes':sizes,'all_passed':True}
options.report.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='cases'},indent=2))
