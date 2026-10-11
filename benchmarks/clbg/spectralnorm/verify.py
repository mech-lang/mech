#!/usr/bin/env python3
"""Independent native array conformance, using non-benchmark coefficient functions."""
import argparse,json,math,os,pathlib,subprocess,tempfile
HERE=pathlib.Path(__file__).resolve().parent;ROOT=HERE.parents[2]
ap=argparse.ArgumentParser();ap.add_argument('--mech',default=str(ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'));ap.add_argument('--output',type=pathlib.Path,default=HERE/'results/array-conformance.json');args=ap.parse_args()
def build(source,binary,simd,unroll,rows=1):
 argv=[args.mech,'build',str(source),'--backend','cranelift','--no-config','--offline','-o',str(binary)]
 if not simd:argv+=['--scalar']
 env=os.environ|{'CARGO_TARGET_DIR':str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'RUSTFLAGS':'-D warnings','MECH_NUMERIC_UNROLL':str(unroll),'MECH_NUMERIC_ROWS':str(rows)}
 result=subprocess.run(argv,env=env,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 if result.returncode:raise RuntimeError(result.stderr.decode()+result.stdout.decode())
formulas=[('row','i + 1.0',lambda i,j:i+1),('column','j + 2.0',lambda i,j:j+2),('mixed','(i + 1.0) * (j + 2.0) + 0.25',lambda i,j:(i+1)*(j+2)+.25),('ratio','(i + 1.0) / (j + 2.0)',lambda i,j:(i+1)/(j+2)),('root','math/sqrt((i + 1.0) * (j + 2.0))',lambda i,j:math.sqrt((i+1)*(j+2)))]
source=['index(i<f64>, j<f64>) = (out<f64>) := out := i + 1.0.']
formulas += [('urow','number/to-f64(i + 1)',lambda i,j:float(i+1)),('ucolumn','number/to-f64(j + 2)',lambda i,j:float(j+2)),('umixed','number/to-f64((i + 1) * (j + 2) + 1)',lambda i,j:float((i+1)*(j+2)+1)),('uratio','number/to-f64(i + 1) / number/to-f64(j + 2)',lambda i,j:(i+1)/(j+2)),('udivide','number/to-f64((i + 1) * (j + 2) / 3 + 1)',lambda i,j:float((i+1)*(j+2)//3+1)),('udynamic','number/to-f64((i + 10) / (j + 1) + 1)',lambda i,j:float((i+10)//(j+1)+1)),('usubtract','number/to-f64((i + 20) - j)',lambda i,j:float(i+20-j)),('uconservative','number/to-f64((i + i) - i + 1)',lambda i,j:float(i+1))]
source += [f'{name}(i<{"u64" if name.startswith("u") else "f64"}>, j<{"u64" if name.startswith("u") else "f64"}>) = (out<f64>) := out := {formula}.' for name,formula,_ in formulas]
more=[('uquadratic','number/to-f64((i + 2) * (j * j + j + 1))',lambda i,j:float((i+2)*(j*j+j+1))),('usquare','number/to-f64((i + j + 2) * (i + j + 2))',lambda i,j:float((i+j+2)**2)),('utwo','number/to-f64((i + j + 1) * (i + j + 2)) + number/to-f64((j + 3) * (j + 4))',lambda i,j:float((i+j+1)*(i+j+2)+(j+3)*(j+4)))]
formulas += more
source += [f'{name}(i<u64>, j<u64>) = (out<f64>) := out := {formula}.' for name,formula,_ in more]
expected=[];cases=[]
for columns,rows in [(0,0),(0,7),(1,1),(1,5),(3,2),(5,7),(7,3),(8,9),(9,8),(15,17),(17,15)]:
 vector=f'v{columns}-{rows}'
 source.append(f'{vector} := matrix/apply(sequence/fill(1, 1.0), {columns}, index, "multiply")')
 for name,_,formula in formulas:
  for transpose in (False,True):
   for mode in ('multiply','divide'):
    op='matrix/apply-transpose' if transpose else 'matrix/apply'
    source.append(f'io/write(string/concat(text/join(number/format({op}({vector}, {rows}, {name}, "{mode}"), 9), ","), "\\n"))')
    coefficients=lambda i,j:formula(j,i) if transpose else formula(i,j)
    expected.append([sum((j+1)/coefficients(i,j) if mode=='divide' else (j+1)*coefficients(i,j) for j in range(columns)) for i in range(rows)])
    cases.append({'columns':columns,'rows':rows,'formula':name,'transpose':transpose,'mode':mode})
# IEEE exceptional coefficients must retain scalar semantics under SIMD.
for name,formula,want in [('zero','0.0',math.inf),('negative-zero','0.0 / (0.0 - 1.0)',-math.inf),('infinity','1.0 / 0.0',0.0),('nan','0.0 / 0.0',math.nan)]:
 source.append(f'{name}(i<f64>, j<f64>) = (out<f64>) := out := {formula}.')
 source.append(f'io/write(string/concat(text/join(number/format(matrix/apply(sequence/fill(7, 1.0), 5, {name}, "divide"), 9), ","), "\\n"))')
 expected.append([want]*5)
source += ['scale(v<[f64]:_>) = (out<[f64]:_>) := out := v * 1.25 + 0.5.', 'identity(v<[f64]:_>) = (out<[f64]:_>) := out := v.', 'nested(v<[f64]:_>) = (out<[f64]:_>) := out := sequence/iterate(v, 2, scale).', 'scalar(x<f64>) = (out<f64>) := out := math/sqrt(x * x) + 0.5.']
for count in (0,1,2,9,21):
 for fn in ('scale','identity','nested'):
  source.append(f'io/write(string/concat(text/join(number/format(sequence/iterate(sequence/fill(7, 0.75), {count}, {fn}), 9), ","), "\\n"))')
  value=.75
  for _ in range(count*(2 if fn=='nested' else 1)):
   if fn!='identity':value=value*1.25+.5
  expected.append([value]*7)
 source.append(f'io/write(string/concat(number/format(sequence/iterate(0.75, {count}, scalar), 9), "\\n"))');expected.append([.75+count*.5])
source += ['io/write(string/concat(number/format(stats/dot(sequence/fill(7, 1.5), sequence/fill(7, 2.0)), 9), "\\n"))'];expected.append([21.0])
configs=[(False,1,1),(True,1,1),(True,2,1),(True,4,1),(True,8,1),(False,1,2),(False,1,4),(True,1,2),(True,2,2),(True,4,2),(True,1,4),(True,2,4),(True,4,4)];checked=0
with tempfile.TemporaryDirectory(prefix='mech-array-conformance-') as temp:
 temp=pathlib.Path(temp);p=temp/'generic.mec';p.write_text('\n'.join(source)+'\n')
 for simd,unroll,rows in configs:
  binary=temp/'generic';build(p,binary,simd,unroll,rows)
  for partitions,grain in [(p,g) for p in (1,3,8) for g in (1,16,64)]:
   run=subprocess.run([str(binary)],env=os.environ|{'MECH_SEQUENCE_PARTITIONS':str(partitions),'RAYON_NUM_THREADS':'8','MECH_ARRAY_GRAIN':str(grain)},check=True,capture_output=True)
   actual=run.stdout.decode().splitlines();assert len(actual)==len(expected),(len(actual),len(expected))
   for index,(line,values) in enumerate(zip(actual,expected)):
    floats=[] if not line else list(map(float,line.split(',')));assert len(floats)==len(values)
    for actual,want in zip(floats,values):assert (math.isnan(actual) and math.isnan(want)) or actual==want or math.isclose(actual,want,rel_tol=2e-10,abs_tol=1e-8),(simd,unroll,partitions,index,actual,want)
   checked+=len(expected);print(f'simd={simd} unroll={unroll} rows={rows} partitions={partitions} grain={grain}: {len(expected)} independent results passed',flush=True)
  # Shape errors are checked before entering emitted pointer loops.
  p2=temp/'bad-shape.mec';p2.write_text('io/write(number/format(stats/dot(sequence/fill(3, 1.0), sequence/fill(4, 1.0)), 9))\n');bad=temp/'bad';build(p2,bad,simd,unroll,rows)
  result=subprocess.run([str(bad)],capture_output=True);assert result.returncode==1 and b'shapes differ' in result.stderr
 # Scalar and matrix unsigned arithmetic retain checked overflow/division semantics.
 errors=[('18446744073709551615 + 1','overflow'),('0 - 1','overflow'),('18446744073709551615 * 2','overflow'),('1 / 0','division by zero')]
 for expression,message in errors:
  p.write_text(f'io/write(number/format({expression}, 0))\n');build(p,temp/'bad-integer',True,2,2)
  result=subprocess.run([str(temp/'bad-integer')],capture_output=True);assert result.returncode==1 and message.encode() in result.stderr and not result.stdout,(expression,result)
 for expression,message in [('number/to-f64(i - 1)','overflow'),('number/to-f64(18446744073709551615 + i)','overflow'),('number/to-f64(18446744073709551615 * (i + 1))','overflow'),('number/to-f64(i / j)','division by zero')]:
  p.write_text(f'f(i<u64>, j<u64>) = (out<f64>) := out := {expression}.\nio/write(text/join(number/format(matrix/apply(sequence/fill(7, 1.0), 5, f, "multiply"), 0), ","))\n');build(p,temp/'bad-integer',True,2,2)
  for workers in (1,8):
   result=subprocess.run([str(temp/'bad-integer')],env=os.environ|{'RAYON_NUM_THREADS':str(workers),'MECH_SEQUENCE_PARTITIONS':'8'},capture_output=True);assert result.returncode==1 and message.encode() in result.stderr and not result.stdout,(expression,result)
 p.write_text('io/write(number/format((18446744073709551615 - 18446744073709551614) * 17 / 3 + 2, 0))\n');build(p,temp/'unsigned',True,2,2);result=subprocess.run([str(temp/'unsigned')],capture_output=True,check=True);assert result.stdout==b'7'
 p.write_text('f(i<u64>, j<u64>) = (out<f64>) := out := number/to-f64(18446744073709551615 + i + 1).\nio/write(text/join(number/format(matrix/apply(sequence/fill(0, 1.0), 5, f, "multiply"), 0), ","))\n');build(p,temp/'empty-unsigned',True,2,2);result=subprocess.run([str(temp/'empty-unsigned')],capture_output=True,check=True);assert result.stdout==b'0,0,0,0,0'
 # Arbitrary-input spectral norm checks include all SIMD tail boundaries.
 for source_name in ('spectralnorm.mec','spectralnorm-integer.mec'):
  p.write_text((HERE/source_name).read_text());build(p,temp/'norm',True,4,4)
  for n in (1,2,3,5,7,8,9,15,16,17,31,100):
   def mult(v,transpose=False):return [sum(v[j]/(((i+j)*(i+j+1)/2)+(j if transpose else i)+1) for j in range(n)) for i in range(n)]
   def b(v):return mult(mult(v),True)
   u=[1.0]*n
   for _ in range(10):v=b(u);u=b(v)
   want=math.sqrt(sum(x*y for x,y in zip(u,v))/sum(x*x for x in v));run=subprocess.run([str(temp/'norm'),str(n)],capture_output=True,check=True);assert run.stdout==f'{want:.9f}\n'.encode(),(n,run.stdout,want)
report={'nonbenchmark_matrix_cases':cases,'native_policies':[{'simd':s,'unroll':u,'rows':r} for s,u,r in configs],'row_partitions':[1,3,8],'array_grains':[1,16,64],'ieee_nan_infinity_signed_zero_coefficients':True,'checked_results':checked,'rectangular_and_empty_vectors':True,'nested_scalar_vector_iterations_and_zero_count':True,'dot_shape_errors_rejected':True,'independent_spectral_sizes':[1,2,3,5,7,8,9,15,16,17,31,100],'all_passed':True}
report.update(unsigned_coefficients_and_scalar_arithmetic=True,unsigned_overflow_and_division_errors=True,conservative_interval_checked_fallback=True)
args.output.parent.mkdir(exist_ok=True);args.output.write_text(json.dumps(report,indent=2)+'\n')
