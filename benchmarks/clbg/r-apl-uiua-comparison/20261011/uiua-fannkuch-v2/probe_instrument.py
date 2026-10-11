#!/usr/bin/env python3
"""Bounded locked output probe; no full n=12 workload."""
from pathlib import Path
import fcntl,json,os,subprocess
H=Path(__file__).resolve().parent
N=H.parent
D=json.loads((N/'uiua-manifest.json').read_text())
U=D['runtime']['path']
S=H/'results/instrumented.ua'
E=dict(os.environ,LC_ALL='C',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',VECLIB_MAXIMUM_THREADS='1')
print('Waiting for shared lock',flush=True)
with open('/tmp/mech-clbg-timing.lock','a') as lock:
 fcntl.flock(lock,fcntl.LOCK_EX)
 print('Locked; bounded n1 n2 n3 only',flush=True)
 for n in (2,):
  r=subprocess.run([U,'run','--no-format',str(S),str(n)],env=E,cwd=N,capture_output=True,timeout=30)
  print(n,r.returncode,r.stdout.decode(),r.stderr.decode(),flush=True)
  assert r.returncode==0
