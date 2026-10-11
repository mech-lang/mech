"""Independent confirmation of the default fannkuch before/after signal."""
import fcntl,importlib.util,json,os,random,signal,statistics
from datetime import datetime,timezone
from pathlib import Path
H=Path(__file__).resolve().parent;C=H.parents[1];R=C.parents[1]
spec=importlib.util.spec_from_file_location('confirmation_guard',C/'execution-modes/20261010-final/run.py');guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
def now():return datetime.now(timezone.utc).isoformat()
def main():
 baseline=json.loads((H/'baseline.json').read_text());initial=json.loads((H/'timings.json').read_text())
 key='fannkuchredux:mech-fannkuch-default'
 rows={phase:{k:v for k,v in initial['programs'][key+':'+phase].items() if k not in ('samples_seconds','sample_validation','sample_details','median_seconds','sample_stdev_seconds','full_validation')}|dict(samples_seconds=[],sample_validation=[],sample_details=[]) for phase in ('before','after')}
 pins={str(H/'baseline.json'):guard.sha(H/'baseline.json'),str(H/'timings.json'):guard.sha(H/'timings.json'),str(Path(__file__).resolve()):guard.sha(__file__),str(C/'execution-modes/20261010-final/run.py'):guard.sha(C/'execution-modes/20261010-final/run.py'),str(C/'compact/run.py'):guard.sha(C/'compact/run.py'),str(C/'nbody/run.py'):guard.sha(C/'nbody/run.py')}
 pins.update({str(R/p):v for p,v in baseline['source_files'].items()})
 for row in rows.values():pins.update({row['source']:row['source_sha256'],row['binary']:row['binary_sha256']})
 def verify():
  for p,d in pins.items():assert guard.sha(p)==d,p
 path=H/'fannkuch-confirmation.json'
 report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=now(),sample_count=3,power_profile=guard.POWER,workload_n=12,pins=pins,programs=rows,orders=[],discarded=[],protocol='Independent three-round matched randomized confirmation, same pinned before/after binaries and default source/arguments. Canonical n=12, complete output, fresh loaded interpretation with IO included and preparation excluded. Shared timing mutex, AC power, Low Power Mode off, <=8 workers. No earlier sample reused.')
 assert report['pins']==pins
 rng=random.Random(20261012)
 for i in range(3):
  order=['before','after'];rng.shuffle(order)
  if len(report['orders'])<=i:report['orders'].append(order)
  else:assert report['orders'][i]==order
  for phase in order:
   row=report['programs'][phase]
   if len(row['samples_seconds'])>i:continue
   while True:
    with open('/tmp/mech-clbg-timing.lock','a') as lock:
     fcntl.flock(lock,fcntl.LOCK_EX);guard.guard.wait_for_environment(guard.POWER);verify()
     report['active']=dict(phase=phase,sample=i+1,started_at=now());guard.save(path,report);print('RUN',i+1,phase,flush=True)
     result=guard.execute(row,True,600);verify()
    if result['competing_compilers'] or result['power_changes']:report['discarded'].append(result);guard.save(path,report);continue
    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation']);row['sample_details'].append(result)
    row['median_seconds']=statistics.median(row['samples_seconds']);row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds']) if len(row['samples_seconds'])>1 else None
    report.pop('active',None);guard.save(path,report);print('SAMPLE',i+1,phase,f"{result['seconds']:.6f}s",flush=True);break
 before,after=(report['programs'][phase]['median_seconds'] for phase in ('before','after'))
 report.update(status='complete',finished_at=now(),comparison=dict(before_seconds=before,after_seconds=after,after_x_before=after/before));verify();guard.save(path,report);print(json.dumps(report['comparison']),flush=True)
if __name__=='__main__':main()
