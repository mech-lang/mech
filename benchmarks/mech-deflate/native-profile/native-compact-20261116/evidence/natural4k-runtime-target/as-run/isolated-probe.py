"""Pause only the verified local test build while measuring the real source-built workers."""
from pathlib import Path
import json,os,signal,subprocess,sys,time
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent
record={'paused':[],'resumed':[],'scope':'Only the verified test Cargo process and its descendants are paused. No Rust source changes while the test build exists.'}
def processes():
 out={}
 for line in subprocess.check_output(['ps','-axo','pid=,ppid=,state=,command='],text=True).splitlines():
  a=line.split(None,3)
  if len(a)==4:out[int(a[0])]={'parent':int(a[1]),'state':a[2],'command':a[3]}
 return out
def save():(ROOT/'probe-pointer-isolation.json').write_text(json.dumps(record,indent=2)+'\n')
try:
 table=processes();jobs=[pid for pid,row in table.items() if Path(row['command'].split()[0]).name=='cargo' and ' test --locked --offline --release ' in row['command'] and str(LAB/'target') in row['command'] and '--lib ordered_reduction --no-run' in row['command']]
 assert len(jobs)<=1,jobs
 if jobs:
  job=jobs[0];members={job}
  while True:
   more={pid for pid,row in table.items() if row['parent'] in members};new=members|more
   if new==members:break
   members=new
  # Stop Cargo before descendants so it cannot launch a replacement compiler.
  for pid in [job,*sorted(members-{job})]:
   current=processes().get(pid)
   if current is None:continue
   assert current['command']==table[pid]['command'];os.kill(pid,signal.SIGSTOP);record['paused'].append({'pid':pid,'command':current['command']})
  for _ in range(20):
   current=processes()
   if all(row['pid'] not in current or 'T' in current[row['pid']]['state'] or 'Z' in current[row['pid']]['state'] for row in record['paused']):break
   time.sleep(.05)
  else:raise RuntimeError('Could not confirm the test build stopped; no timings acquired')
  record['stopped_states']={str(r['pid']):current.get(r['pid'],{}).get('state','exited') for r in record['paused']};save()
 with (ROOT/'probe-pointer.log').open('w') as log:done=subprocess.run([sys.executable,str(ROOT/'probe-pointer.py')],stdout=log,stderr=log)
 record['probe_exit_code']=done.returncode
 if done.returncode:raise RuntimeError('Runtime probe failed; inspect probe-pointer.log')
finally:
 for row in reversed(record['paused']):
  current=processes().get(row['pid'])
  if current is not None and current['command']==row['command']:os.kill(row['pid'],signal.SIGCONT);record['resumed'].append(row['pid'])
 save()
print('Isolated probe completed; test build resumed.',flush=True)
