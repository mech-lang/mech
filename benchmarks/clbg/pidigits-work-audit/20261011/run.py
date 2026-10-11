"""Three guarded canonical Node two-extract runs and fresh Mech AOT controls."""
import fcntl, hashlib, importlib.util, json, math, os, random, signal, statistics, subprocess, tempfile, time
from datetime import datetime, timezone
from pathlib import Path
HERE=Path(__file__).resolve().parent
CLBG=HERE.parents[1]
spec=importlib.util.spec_from_file_location('pi_guard',CLBG/'compact/run.py')
guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
POWER={'source':'AC Power','low_power_mode':0}
LOCK=Path('/tmp/mech-clbg-timing.lock')
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,d):
 tmp=p.with_suffix('.json.tmp');tmp.write_text(json.dumps(d,indent=2)+'\n');tmp.replace(p)
def now():return datetime.now(timezone.utc).isoformat()
def execute(row):
 argv=[v.replace('{n}','10000') for v in row['command']]
 env={k:os.environ[k] for k in ('PATH','HOME','TMPDIR','LANG','LC_ALL') if k in os.environ}
 env|={k:v for k,v in guard.ENV.items() if k.endswith('_NUM_THREADS') or k in ('OMP_DYNAMIC','GOMAXPROCS','PYTHONHASHSEED')}
 env|=row.get('runtime_environment',{})
 monitor=guard.CompilerMonitor(POWER);proc=None
 def interrupted(sig,frame):raise InterruptedError('Termination signal '+str(sig))
 def timeout(sig,frame):raise TimeoutError('Full process timeout')
 previous={s:signal.signal(s,interrupted) for s in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP)}
 previous[signal.SIGALRM]=signal.signal(signal.SIGALRM,timeout)
 with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
  try:
   monitor.start();start=time.perf_counter_ns()
   proc=subprocess.Popen(argv,cwd=row['cwd'],stdin=subprocess.DEVNULL,stdout=out,stderr=err,env=env,start_new_session=True)
   signal.setitimer(signal.ITIMER_REAL,300)
   status=proc.wait();seconds=(time.perf_counter_ns()-start)/1e9
   assert status==0,(row['key'],status)
   try:os.killpg(proc.pid,0)
   except ProcessLookupError:pass
   else:raise RuntimeError('Process group survives main process')
  except BaseException:
   if proc:
    try:os.killpg(proc.pid,signal.SIGKILL)
    except ProcessLookupError:pass
    proc.wait()
   raise
  finally:
   signal.setitimer(signal.ITIMER_REAL,0)
   for s,old in previous.items():signal.signal(s,old)
   monitor.finish()
  assert math.isfinite(seconds) and seconds>0
  out.seek(0,os.SEEK_END);size=out.tell();out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest()
  reference=CLBG/'pidigits/inputs/output-full.txt'
  assert (size,digest)==(reference.stat().st_size,sha(reference)),(row['key'],size,digest)
  err.seek(0);stderr=err.read().decode(errors='replace')
  assert not stderr,(row['key'],stderr)
  return dict(key=row['key'],source_sha256=row['source_sha256'],command=argv,cwd=row['cwd'],seconds=seconds,workload_n=10000,full_workload=True,exit_status=status,validation=dict(bytes=size,sha256=digest,reference_match=True),competing_compilers=monitor.observations,power_changes=monitor.power_changes,power_at_end=guard.power_profile(),valid_environment=not(monitor.observations or monitor.power_changes or monitor.errors) and guard.power_profile()==POWER,finished_at=now())
def main():
 audit_path=HERE/'audit.json';audit=json.loads(audit_path.read_text());source=audit['corrections'][0]
 assert source['small_validation']['passed'] and sha(source['small_validation']['report'])==source['small_validation']['sha256']
 control_audit=CLBG/'r-apl-uiua-comparison/20261011/source-audit-r.json'
 controls=json.loads(control_audit.read_text());control=next(r for r in controls['controls'] if r['benchmark']=='pidigits')
 rows={source['key']:dict(source),control['key']:dict(control)}
 for row in rows.values():row.setdefault('cwd',str(CLBG.parents[1]));row.update(samples_seconds=[],sample_validation=[],sample_details=[])
 pins={str(Path(__file__).resolve()):sha(__file__),str(audit_path):sha(audit_path),str(control_audit):sha(control_audit),str(CLBG/'compact/run.py'):sha(CLBG/'compact/run.py'),str(CLBG/'nbody/run.py'):sha(CLBG/'nbody/run.py'),str(CLBG/'pidigits/inputs/output-full.txt'):sha(CLBG/'pidigits/inputs/output-full.txt')}
 for row in rows.values():
  pins[row['source']]=row['source_sha256']
  runtime=row.get('runtime')
  if runtime:pins[runtime['path']]=runtime['sha256']
  else:pins[row['binary']]=row['binary_sha256']
 def verify():
  for file,digest in pins.items():assert sha(file)==digest,file
 path=HERE/'results/timings.json'
 report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=now(),sample_count=3,power_profile=POWER,source_audit=str(audit_path),source_audit_sha256=sha(audit_path),control_audit=str(control_audit),control_audit_sha256=sha(control_audit),pins=pins,programs=rows,orders=[],discarded=[],protocol='Three randomized sequential fresh-process runs at canonical N=10000. Process startup, full computation, complete output and teardown included for both Node and Mech AOT. Shared timing mutex, AC power, Low Power Mode off, <=8 workers. Both exact extract(3)/extract(4) divisions per tested candidate. No prior samples reused.')
 assert report['pins']==pins and report['sample_count']==3
 for key,row in rows.items():
  for field in ('source','source_sha256','command','cwd','runtime_environment','loc'):
   assert report['programs'][key].get(field)==row.get(field),(key,field)
 verify();save(path,report)
 rng=random.Random(20261011)
 for sample in range(3):
  order=list(rows);rng.shuffle(order)
  if len(report['orders'])<=sample:report['orders'].append(order)
  else:assert report['orders'][sample]==order
  for key in order:
   row=report['programs'][key]
   if len(row['samples_seconds'])>sample:continue
   while True:
    with LOCK.open('a') as lock:
     fcntl.flock(lock,fcntl.LOCK_EX);guard.wait_for_environment(POWER);verify()
     print('RUN',sample+1,key,flush=True);result=execute(row);verify()
    if not result['valid_environment']:report['discarded'].append(result);save(path,report);continue
    row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation']);row['sample_details'].append(result)
    row['median_seconds']=statistics.median(row['samples_seconds']);row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds']) if len(row['samples_seconds'])>1 else None
    save(path,report);print('SAMPLE',sample+1,key,f"{result['seconds']:.6f}s",flush=True);break
 candidate=report['programs'][source['key']];mech=report['programs'][control['key']]
 report['comparison']=dict(seconds=candidate['median_seconds'],time_x_mech=candidate['median_seconds']/mech['median_seconds'],loc=candidate['loc'],loc_x_mech=candidate['loc']/mech['loc'],mech_seconds=mech['median_seconds'],mech_loc=mech['loc'])
 report['status']='complete';report['finished_at']=now();verify();save(path,report);print(json.dumps(report['comparison']),flush=True)
if __name__=='__main__':main()
