#!/usr/bin/env python3
"""Tiny/reference correctness only. Never runs a full benchmark or times it."""
from pathlib import Path
import subprocess,os,json,hashlib,importlib.util,sys
G=Path(__file__).resolve().parent
CLBG=G.parents[1]
R=G/'.build/toolchains/R-4.6.1/bin/Rscript'
env=os.environ|{'R_LIBS_USER':str(G/'.build/toolchains/r-library'),'R_ENABLE_JIT':'3','OMP_NUM_THREADS':'1','OPENBLAS_NUM_THREADS':'1'}
WORK={'nbody':1000,'spectralnorm':100,'binarytrees':10,'fannkuchredux':7,'fasta':1000,'mandelbrot':200,'knucleotide':1000,'revcomp':1000,'regexredux':30,'pidigits':30}
INPUTS={'knucleotide':CLBG/'knucleotide/inputs/knucleotide-input-1000.txt','revcomp':CLBG/'knucleotide/inputs/knucleotide-input-1000.txt','regexredux':CLBG/'regexredux/inputs/regexredux-input-small.txt'}
HASHES={'mandelbrot':(5011,'97610473750700638fc63d13cfa49d339b67c18e7f26b3f9c9acb61e746472d5'),'knucleotide':(242,'baa63c46addb89c63e68ea0055952847fbccf450e89997b2b62234c90e66f500'),'revcomp':(10245,'549dcdcc7df2685145e6cbdfd682469bd9c12ff1beec93c270f8ba447acd4855'),'fasta':(10245,'62d1e8d0df7938d2aefda9a37887e0389231ea72c099c29a51afb6edca1bdc73'),'binarytrees':(223,'b7f92c56b5d8aeb0a4d698842d1d87a57b4909865c3c84e5e10313e16663c3cb')}
RESULTS=G/'results/r'
RESULTS.mkdir(parents=True,exist_ok=True)
selected=set(sys.argv[1].split(",")) if len(sys.argv)>1 else None
report=json.loads((RESULTS/"small-validation.json").read_text()) if selected is not None else {}
for bench,n in WORK.items():
 if selected is not None and bench not in selected:continue
 for variant in ['compact','optimized']+(['array-compact','array-batched'] if bench=='fannkuchredux' else []):
  source=G/'sources/r'/bench/(variant+'.R')
  args=[str(R),'--vanilla',str(source)]+([] if bench in INPUTS else [str(n)])
  data=INPUTS[bench].read_bytes() if bench in INPUTS else None
  p=subprocess.run(args,input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,timeout=90)
  stem=bench+'-r-'+variant
  (RESULTS/(stem+'.small.stdout')).write_bytes(p.stdout)
  (RESULTS/(stem+'.small.stderr')).write_bytes(p.stderr)
  h=hashlib.sha256(p.stdout).hexdigest()
  ok=p.returncode==0
  if bench in HASHES:ok=ok and (len(p.stdout),h)==HASHES[bench]
  elif bench=='nbody':ok=ok and p.stdout==b'-0.169075164\n-0.169087605\n'
  elif bench=='spectralnorm':ok=ok and p.stdout==b'1.274219991\n'
  elif bench=='fannkuchredux':ok=ok and p.stdout==b'228\nPfannkuchen(7) = 16\n'
  else:ok=ok and p.stdout==(CLBG/bench/'inputs'/(bench+'-output-small.txt')).read_bytes()
  report[stem]={'source':str(source),'command':args,'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'returncode':p.returncode,'bytes':len(p.stdout),'sha256':h,'reference_match':ok,'stderr':p.stderr.decode(),'stdout_file':str(RESULTS/(stem+'.small.stdout'))}
  print(stem,'PASS' if ok else 'FAIL',p.returncode,len(p.stdout),p.stderr.decode()[:500],flush=True)
  if not ok and bench not in ['fasta','mandelbrot','revcomp']:print(repr(p.stdout.decode(errors='replace')),flush=True)
  (RESULTS/'small-validation.json').write_text(json.dumps(report,indent=2)+'\n')
assert all(r['reference_match'] for r in report.values())
