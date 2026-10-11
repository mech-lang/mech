#!/usr/bin/env python3
"""Locked odd-width bitmap, native byte-packing and recurrence trace checks only."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess

H=Path(__file__).resolve().parent
N=H.parent
S=H/'sources/fast.ua'
R=H/'results'
B=json.loads((N/'uiua-manifest.json').read_text())
U=Path(B['runtime']['path'])
ENV=dict(LC_ALL='C',OMP_NUM_THREADS='1',OPENBLAS_NUM_THREADS='1',VECLIB_MAXIMUM_THREADS='1')
CASES=(1,2,3,7,8,9,15,16,17,31,33,65,129,200,257)
LOCK=Path('/tmp/mech-clbg-timing.lock')


def sha(path):
 with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def reference(n):
 output=[]
 steps=[]
 for y in range(n):
  bits=[]
  row_steps=[]
  for x in range(n):
   cr=2*x/n-1.5
   ci=2*y/n-1
   zr=zi=0.0
   inside=True
   used=0
   for iteration in range(50):
    zr,zi=zr*zr-zi*zi+cr,2*zr*zi+ci
    used+=1
    if zr*zr+zi*zi>4:inside=False;break
   bits.append(int(inside))
   row_steps.append(used)
  bits.extend([0]*((-n)%8))
  output.append([sum(bit*(1<<(7-j)) for j,bit in enumerate(bits[i:i+8])) for i in range(0,len(bits),8)])
  steps.append(max(row_steps))
 return f'P4\n{n} {n}\n'.encode()+bytes(b for row in output for b in row),output,steps


def execute(source,n,data=None):
 command=[str(U),'run','--no-format',str(source),str(n)]
 done=subprocess.run(command,cwd=N,env=dict(os.environ,**ENV),input=data,capture_output=True,timeout=60)
 return command,done


def persist(source,n,command,done,stem):
 out=R/(stem+'.stdout');err=R/(stem+'.stderr')
 out.write_bytes(done.stdout);err.write_bytes(done.stderr)
 return dict(source=str(source),source_sha256=sha(source),n=n,command=command,cwd=str(N),runtime_environment=ENV,
  returncode=done.returncode,bytes=len(done.stdout),sha256=sha(out),stdout_file=str(out),stderr_file=str(err),stderr_sha256=sha(err))


def instrument(source):
 lines=[]
 for line in source.read_text().splitlines():
  if line.startswith('Iterate ←'):
   lines.extend(['Iterate ← (','  &p ⊂ "ITER=" json .',
                 '  '+line.split('←',1)[1].strip(),')'])
  else:lines.append(line.replace('&fwa "/dev/stdout" Pack','&p ⊂ "ROW=" json Pack'))
 return '\n'.join(lines)+'\n'


def main():
 R.mkdir(parents=True,exist_ok=True)
 pin=sha(S)
 old=N/'sources/uiua/mandelbrot/fast.ua';old_pin=sha(old)
 for path,digest in B['runtime']['pins'].items():assert sha(path)==digest,path
 trace=R/'instrumented.ua';trace.write_text(instrument(S))
 report=dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),
  scope='Complete small bitmaps, row/pixel recurrence traces and generic bit packing only; no full n=16000 timing.',
  source=str(S),source_sha256=pin,original_source=str(old),original_source_sha256=old_pin,
  runtime=str(U),runtime_sha256=sha(U),runtime_environment=ENV,shared_timing_lock=str(LOCK),
  validator=str(Path(__file__).resolve()),validator_sha256=sha(__file__),cases=[],instrumentation=[],primitive_cases=[])
 print('Waiting for common mutex; all reference/runtime probes occur after acquisition.',flush=True)
 with LOCK.open('a') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX)
  print('Locked bounded bitmap/packing validation.',flush=True)
  expected={n:reference(n) for n in CASES}
  for n in CASES:
   command,done=execute(S,n)
   row=persist(S,n,command,done,'output-'+str(n))
   row.update(case='canonical-small' if n==200 else 'odd-width-'+str(n),expected_sha256=hashlib.sha256(expected[n][0]).hexdigest(),
    reference_match=done.returncode==0 and done.stdout==expected[n][0],pixels=n*n,maximum_pixel_iterations=50)
   report['cases'].append(row)
   assert row['reference_match'],(row,done.stderr.decode())
   print(n,'OUTPUT PASS',flush=True)
  for n in (1,9,17,65,129):
   command,done=execute(trace,n)
   row=persist(trace,n,command,done,'work-'+str(n))
   assert done.returncode==0,done.stderr.decode()
   rows=[];step_rows=[];current_steps=[];coordinate=None
   zr=zi=live=None
   for line in done.stdout.decode().splitlines():
    if line.startswith('ITER='):
     observed_r,observed_i,observed_live,cr,cy,iteration=json.loads(line[5:])
     assert len(observed_r)==len(observed_i)==len(observed_live)==len(cr)==n
     if coordinate is None:
      coordinate=cy;zr=[0.0]*n;zi=[0.0]*n;live=[1]*n
     assert abs(cy-coordinate)<1e-15
     assert iteration==len(current_steps)
     for x in range(n):
      assert abs(cr[x]-((x/n)*2-1.5))<1e-14
      assert observed_live[x]==live[x]
      if live[x]:
       assert abs(observed_r[x]-zr[x])<=1e-12*max(1,abs(zr[x]))
       assert abs(observed_i[x]-zi[x])<=1e-12*max(1,abs(zi[x]))
       zr[x],zi[x]=zr[x]*zr[x]-zi[x]*zi[x]+cr[x],(zr[x]*zi[x])*2+cy
       live[x]=int(zr[x]*zr[x]+zi[x]*zi[x]<=4)
     current_steps.append(iteration)
    elif line.startswith('ROW='):
     rows.append(json.loads(line[4:]));step_rows.append(current_steps)
     assert abs(coordinate-(2*(len(rows)-1)/n-1))<1e-14
     coordinate=None;current_steps=[]
   assert rows==expected[n][1]
   assert step_rows==[list(range(k)) for k in expected[n][2]],(n,step_rows,expected[n][2])
   row.update(original_source=str(S),original_source_sha256=pin,instrumented_source=str(trace),instrumented_source_sha256=sha(trace),
    transformation='Print each unchanged Iterate input and serialize the exact Pack row instead of writing bytes; all arithmetic and loop conditions retained.',
    pixels=n*n,rows=n,iteration_counts=[len(x) for x in step_rows],maximum_pixel_iterations=50,
    actual_vector_pixel_iterations=n*sum(len(x) for x in step_rows),full_row_recurrences=True,ordinary_escape_only=True,
    every_active_pixel_state_verified=True,every_iteration_live_mask_verified=True,
    every_packed_byte_equal_reference=True,reference_match=True)
   report['instrumentation'].append(row)
   print(n,'WORK PASS',flush=True)
  keep=[line for line in S.read_text().splitlines() if line.startswith(('N ','Pack '))]
  probe=R/'pack-only.ua';probe.write_text('\n'.join(keep+['&p json Pack °json &fras "/dev/stdin"'])+'\n')
  for n in (1,2,3,7,8,9,15,16,17,31,33,65,129,8001):
   bits=[int((37*i+13)%101<47) for i in range(n)]
   data=json.dumps(bits).encode();padded=bits+[0]*((-n)%8)
   packed=[sum(bit*(1<<(7-j)) for j,bit in enumerate(padded[i:i+8])) for i in range(0,len(padded),8)]
   command,done=execute(probe,n,data)
   assert done.returncode==0 and json.loads(done.stdout)==packed,done.stderr.decode()
   row=persist(probe,n,command,done,'pack-'+str(n))
   inp=R/('pack-'+str(n)+'.input.json');inp.write_bytes(data)
   row.update(operation='native-weight-array-rows-reduce',input_file=str(inp),input_sha256=sha(inp),
    bits=n,bytes_packed=len(packed),tail_padding=(-n)%8,reference_match=True,no_pixel_recurrence=True)
   report['primitive_cases'].append(row)
  assert sha(S)==pin and sha(old)==old_pin
 report.update(status='passed',finished_at=datetime.now(timezone.utc).isoformat(),passed=len(report['cases']),
  work_checks_passed=len(report['instrumentation']),primitive_checks_passed=len(report['primitive_cases']),full_performance_runs=0)
 (R/'validation.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
 print('PASS',report['passed'],'outputs',report['work_checks_passed'],'work traces',report['primitive_checks_passed'],'packing probes',flush=True)


if __name__=='__main__':main()
