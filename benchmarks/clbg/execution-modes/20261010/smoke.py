#!/usr/bin/env python3
import json, os, subprocess, hashlib, time
from pathlib import Path
HERE=Path(__file__).resolve().parent; CLBG=HERE.parents[1]; ROOT=CLBG.parents[1]
SITE=Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site/dist/data.json')
BIN='/tmp/mech-execution-modes-target/release/mech-execution-modes-compiled'
d=json.loads(SITE.read_text()); old=json.loads((CLBG/'additional-languages/20261009-full-suite/results/timings.json').read_text())
env=os.environ|{'RAYON_NUM_THREADS':'8','MECH_TIMINGS':'1','MECH_SEQUENCE_PARTITIONS':'4','MECH_SEQUENCE_NARROW':'1','MECH_SEQUENCE_STD_MAP':'1','MECH_SEQUENCE_ASYNC':'1','MECH_SEQUENCE_BUFFER_REUSE':'1'}
result=[]
for b in d['benchmarks']:
 bench=b['key']; control=old['programs'][bench+':mech-control']; n=old['workloads'][bench][0]
 ip=CLBG/('regexredux/inputs/regexredux-input-small.txt' if bench=='regexredux' else 'knucleotide/inputs/knucleotide-input-1000.txt') if bench in ('knucleotide','revcomp','regexredux') else None
 group=next(g for g in b['groups'] if g['language']=='mech')
 for key in group['programs']:
  p=d['programs'][key]; path=CLBG/d['files'][p['files'][0]]['path']
  flags=['--fp-contract-loop'] if bench=='mandelbrot' and 'simd' in p['id'] else ['--algebraic-powers','--fp-contract','--reassociate-constants','--schedule','--pack-state-tails'] if bench=='nbody' else []
  for mode in ['jit','resident']:
   argv=[BIN,str(path),mode]+([] if ip else [str(n)])+flags
   start=time.perf_counter(); inp=ip.open('rb') if ip else subprocess.DEVNULL
   try:r=subprocess.run(argv,stdin=inp,capture_output=True,env=env,cwd=ROOT,timeout=60)
   finally:
    if ip:inp.close()
   digest=hashlib.sha256(r.stdout).hexdigest(); expected=control['small_validation']['sha256']
   ok=not r.returncode and (digest==expected or bench=='nbody' and [float(x) for x in r.stdout.decode().split()]==[-0.169075164,-0.169087605])
   row={'key':key,'mode':mode,'correct':ok,'wall_seconds':time.perf_counter()-start,'bytes':len(r.stdout),'sha256':digest,'stderr':r.stderr.decode(),'command':argv};result.append(row)
   print(key,mode,'PASS' if ok else 'FAIL',round(row['wall_seconds'],3),r.stderr.decode().strip()[:500],flush=True)
   if not ok:print('output',repr(r.stdout[:500]),'expected',expected,flush=True)
   (HERE/'smoke.json').write_text(json.dumps(result,indent=2)+'\n')
print('PASSED',sum(r['correct'] for r in result),'of',len(result),flush=True)
