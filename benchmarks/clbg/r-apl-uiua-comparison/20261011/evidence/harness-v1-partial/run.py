"""Resumable guarded R/APL/Uiua full runs, completing short benchmarks first."""
import argparse,fcntl,json,os,random,signal,statistics,subprocess,tempfile,threading,time,math
from datetime import datetime,timezone
from pathlib import Path
from prepare import G,C,BENCHES,LANGUAGES,WORK,REQUIRED,LOCK,INPUTS,compact,sha,save

def now():return datetime.now(timezone.utc).isoformat()
class PinGuard:
 def __init__(self,pins):self.pins=pins;self.states={}
 def verify(self):
  for path,digest in self.pins.items():
   st=os.stat(path);state=(st.st_dev,st.st_ino,st.st_size,st.st_mtime_ns,st.st_ctime_ns)
   if self.states.get(path)!=state:
    assert sha(path)==digest,('Pinned file changed',path);self.states[path]=state
class ResourceGuard:
 def __init__(self,pid,mib):
  self.pid=pid;self.limit=int(mib*1024);self.stop=threading.Event();self.peak=0;self.violation=None;self.errors=[]
  self.thread=threading.Thread(target=self.poll,daemon=True)
 def poll(self):
  while not self.stop.wait(1):
   try:
    listing=subprocess.check_output(['ps','-eo','pid,pgid,rss,comm'],text=True)
    own=[]
    for line in listing.splitlines()[1:]:
     f=line.split(None,3)
     if len(f)==4 and int(f[1])==self.pid:own.append({'pid':int(f[0]),'rss_kib':int(f[2]),'executable':Path(f[3]).name})
    rss=sum(r['rss_kib'] for r in own);self.peak=max(self.peak,rss)
    if rss>self.limit:
     self.violation={'at':now(),'rss_kib':rss,'limit_kib':self.limit,'processes':own}
     try:os.killpg(self.pid,signal.SIGKILL)
     except ProcessLookupError:pass
     return
   except Exception as error:self.errors.append(str(error));return
 def start(self):self.thread.start()
 def finish(self):self.stop.set();self.thread.join()

def validate(bench,full,path):
 size=path.seek(0,os.SEEK_END);path.seek(0)
 import hashlib
 digest=hashlib.file_digest(path,'sha256').hexdigest();path.seek(0)
 validation={'bytes':size,'sha256':digest,'reference_match':False}
 if bench=='nbody':
  text=path.read().decode();expected=[-0.169075164,-0.169059907] if full else [-0.169075164,-0.169087605]
  compact.common.validate(text,expected);validation.update(energies=compact.common.energies(text),tolerance=1e-8)
 elif bench in ['spectralnorm','fannkuchredux']:
  actual=path.read();expected={'spectralnorm':[b'1.274219991\n',b'1.274224153\n'],'fannkuchredux':[b'228\nPfannkuchen(7) = 16\n',b'3968050\nPfannkuchen(12) = 65\n']}[bench][int(full)]
  assert actual==expected,(bench,repr(actual[:500]),repr(expected))
 elif bench in ['pidigits','regexredux']:
  reference=C/bench/'inputs'/('output-full.txt' if full else bench+'-output-small.txt')
  assert (size,digest)==(reference.stat().st_size,sha(reference)),(bench,size,digest)
 else:assert (size,digest)==tuple(compact.HASHES[bench][int(full)]),(bench,size,digest,compact.HASHES[bench][int(full)])
 validation['reference_match']=True;return validation

def execute(row,full,timeout,max_rss_mib,n_override=None):
 bench=row['benchmark'];n=n_override if n_override is not None else WORK[bench][int(full)]
 command=[s.replace('{n}',str(n)) for s in row['command']]
 input_file=INPUTS[bench][int(full)] if bench in INPUTS else None
 monitor=compact.CompilerMonitor(REQUIRED);resource=None;proc=None
 attempt={'key':row['key'],'id':row['id'],'benchmark':bench,'language':row['language'],'source_sha256':row['source_sha256'],'command':command,'cwd':row.get('cwd',str(compact.ROOT)),'started_at':now(),'workload_n':n,'full_workload':bool(full and n_override is None),'timeout_seconds':timeout,'maximum_rss_mib':max_rss_mib,'power_at_start':compact.power_profile()}
 with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
  stdin=input_file.open('rb') if input_file else subprocess.DEVNULL
  def expired(signum,frame):raise TimeoutError('Full process reached the recorded wall-clock timeout')
  old=signal.signal(signal.SIGALRM,expired)
  try:
   monitor.start();start=time.perf_counter_ns()
   proc=subprocess.Popen(command,stdin=stdin,stdout=out,stderr=err,cwd=attempt['cwd'],env=compact.ENV|row.get('runtime_environment',{}),start_new_session=True)
   resource=ResourceGuard(proc.pid,max_rss_mib);resource.start()
   signal.setitimer(signal.ITIMER_REAL,timeout)
   status=proc.wait();attempt['seconds']=(time.perf_counter_ns()-start)/1e9;attempt['exit_status']=status
   if resource.violation:raise RuntimeError('Benchmark process group exceeded RSS ceiling')
   if status:raise RuntimeError('Benchmark exited with status '+str(status))
  except BaseException as error:
   if 'start' in locals():attempt['seconds']=(time.perf_counter_ns()-start)/1e9
   attempt['error']=str(error);attempt['error_type']=type(error).__name__
   if proc:
    try:os.killpg(proc.pid,signal.SIGKILL)
    except ProcessLookupError:pass
    proc.wait();attempt['exit_status']=proc.returncode
  finally:
   signal.setitimer(signal.ITIMER_REAL,0);signal.signal(signal.SIGALRM,old)
   if resource:resource.finish()
   try:monitor.finish()
   except Exception as error:attempt['monitor_finish_error']=str(error)
   if input_file:stdin.close()
  attempt.update(competing_compilers=monitor.observations,power_changes=monitor.power_changes,power_at_end=compact.power_profile(),monitor_errors=monitor.errors,peak_process_group_rss_kib=resource.peak if resource else None,resource_violation=resource.violation if resource else None,resource_monitor_errors=resource.errors if resource else [])
  err.seek(0);attempt['stderr']=err.read(65536).decode(errors='replace')
  out.seek(0,os.SEEK_END);attempt['stdout_bytes']=out.tell();out.seek(0)
  import hashlib
  attempt['stdout_sha256']=hashlib.file_digest(out,'sha256').hexdigest();out.seek(0)
  if not attempt.get('error') and n_override is None:
   try:attempt['validation']=validate(bench,full,out)
   except Exception as error:attempt.update(error=str(error),error_type='OutputValidationError')
  if attempt.get('error'):
   out.seek(0);attempt['stdout_prefix']=out.read(2048).decode(errors='replace')
  attempt['finished_at']=now()
  attempt['valid_environment']=not (attempt['competing_compilers'] or attempt['power_changes'] or attempt['monitor_errors'] or attempt['resource_monitor_errors'] or attempt.get('monitor_finish_error')) and attempt['power_at_start']==REQUIRED and attempt['power_at_end']==REQUIRED
  return attempt

def open_report(language,samples,max_rss_mib):
 audit_file=G/('source-audit-'+language+'.json');audit=json.loads(audit_file.read_text());assert audit['status']=='reviewed'
 path=G/'results'/('timings-'+language+'.json')
 rows={r['key']:dict(r,samples_seconds=[],sample_validation=[],attempts=[],measurement_status='pending') for r in audit['programs']+audit['controls']}
 if path.exists():
  report=json.loads(path.read_text());assert report['source_audit_sha256']==sha(audit_file);assert report['sample_count']==samples;assert set(report['programs'])==set(rows);assert report['maximum_rss_mib']==max_rss_mib
 else:
  report={'status':'running','language':language,'created_at':now(),'sample_count':samples,'maximum_rss_mib':max_rss_mib,'source_audit':str(audit_file),'source_audit_sha256':sha(audit_file),'programs':rows,'orders':{},'attempts':[],'discarded':[],'completed_benchmarks':[],'benchmark_order':BENCHES,'workloads':WORK,'shared_timing_lock':str(LOCK),'protocol':audit['timing_boundary'],'baseline_endpoints':audit['baseline_endpoints'],'machine':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip()}
 guard=PinGuard(audit['pins']);guard.verify();save(path,report)
 return {'audit':audit,'path':path,'report':report,'guard':guard}

def copy_aliases(report):
 for row in report['programs'].values():
  target_key=row.get('timing_alias_of')
  if not target_key:continue
  target=report['programs'][target_key]
  for field in ['samples_seconds','sample_validation','median_seconds','sample_stdev_seconds','full_validation','full_failure','measurement_status']:
   if field in target:row[field]=json.loads(json.dumps(target[field]))
  row['aliased_sample_source']=target_key

def run_pilots(state,args):
 report=state['report'];path=state['path'];estimates=report.setdefault('runtime_planning_pilots',[])
 sizes={'nbody':100000,'fannkuchredux':9,'binarytrees':14}
 for row in report['programs'].values():
  if row['language']=='mech' or row.get('timing_alias_of') or row['benchmark'] not in sizes:continue
  if any(p['key']==row['key'] and p['valid_environment'] for p in estimates):continue
  while True:
   with LOCK.open('a+') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX);compact.wait_for_environment(REQUIRED);state['guard'].verify()
    print('PLANNING ONLY',row['key'],sizes[row['benchmark']],flush=True)
    result=execute(row,False,min(600,args.timeout),args.max_rss_mib,n_override=sizes[row['benchmark']])
    state['guard'].verify()
   if result['valid_environment']:break
   result.update(planning_only=True,never_chart_eligible=True);estimates.append(result);save(path,report)
  small=sizes[row['benchmark']];full=WORK[row['benchmark']][1]
  factor=full/small if row['benchmark']=='nbody' else math.factorial(full)/math.factorial(small) if row['benchmark']=='fannkuchredux' else 2**(full-small)
  result.update(planning_only=True,never_chart_eligible=True,scaling_assumption='Proportional work estimate for scheduling only; startup and changed average work make this uncertain.',work_scale=factor)
  if not result.get('error'):result.update(rough_full_seconds=result['seconds']*factor,rough_full_hours=result['seconds']*factor/3600)
  estimates.append(result);save(path,report)
  print('PLANNING ESTIMATE',row['key'],f"{result.get('seconds',0):.3f}s observed; ~{result.get('rough_full_hours',float('nan')):.2f}h full (not a measured benchmark)",result.get('error',''),flush=True)

def run_sample(language,state,bench,sample,args):
 report=state['report'];path=state['path']
 candidates=[r for r in report['programs'].values() if r['benchmark']==bench and not r.get('timing_alias_of')]
 random.Random(20261011+sample*1000+sum(map(ord,language+bench))).shuffle(candidates)
 rounds=report['orders'].setdefault(bench,[]);order=[r['key'] for r in candidates]
 if len(rounds)<=sample:rounds.append(order)
 else:assert rounds[sample]==order
 for row in candidates:
  if len(row['samples_seconds'])>sample or row.get('full_failure'):continue
  while True:
   print('WAIT LOCK',sample+1,row['id'],flush=True)
   with LOCK.open('a+') as lock:
    fcntl.flock(lock,fcntl.LOCK_EX);compact.wait_for_environment(REQUIRED);state['guard'].verify()
    planning=next((r for r in report.get('runtime_planning_pilots',[]) if r['key']==row['key'] and r.get('rough_full_seconds')),None)
    timeout=min(args.timeout,row.get('recommended_timeout_seconds',args.timeout))
    if planning:timeout=min(timeout,max(300,math.ceil(planning['rough_full_seconds']*2)))
    report['active']={'key':row['key'],'sample':sample+1,'started_at':now(),'timeout_seconds':timeout};save(path,report)
    print('RUN FULL',sample+1,row['id'],'timeout',timeout,flush=True)
    attempt=execute(row,True,timeout,args.max_rss_mib)
    try:state['guard'].verify()
    except Exception as error:attempt.update(error=str(error),error_type='SourcePinError')
    attempt.update(sample=sample+1,attempt_number=len(report['attempts'])+1)
    report['attempts'].append(attempt);row['attempts'].append(attempt['attempt_number']);report.pop('active',None)
    if attempt.get('error_type') in ['KeyboardInterrupt','SystemExit']:
     row['measurement_status']='interrupted';save(path,report);raise KeyboardInterrupt('Interrupted benchmark archived; previous samples remain resumable')
    if attempt.get('error_type')=='SourcePinError':save(path,report);raise RuntimeError(attempt['error'])
    if not attempt['valid_environment']:
     report['discarded'].append(attempt['attempt_number']);save(path,report);print('DISCARDED ENVIRONMENT',row['id'],attempt.get('seconds'),flush=True);continue
    if attempt.get('error'):
     row['full_failure']={k:attempt.get(k) for k in ['error','error_type','seconds','exit_status','attempt_number','finished_at','stdout_bytes','stdout_sha256','stderr','resource_violation']}
     row['measurement_status']='failed';copy_aliases(report);save(path,report);print('FULL FAILURE',row['id'],f"{attempt.get('seconds',0):.6f}s",attempt['error'][:500],flush=True);break
    assert attempt['full_workload'] and attempt['validation']['reference_match']
    row['samples_seconds'].append(attempt['seconds']);row['sample_validation'].append(attempt['validation']);row.setdefault('full_validation',attempt['validation']);row['median_seconds']=statistics.median(row['samples_seconds']);row['measurement_status']='measured' if len(row['samples_seconds'])>=args.samples else 'partial'
    if len(row['samples_seconds'])>=2:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
    copy_aliases(report);save(path,report);print('SAMPLE FULL',sample+1,row['id'],f"{attempt['seconds']:.6f}s",flush=True);break
 group=[r for r in report['programs'].values() if r['benchmark']==bench]
 if all(len(r['samples_seconds'])>=args.samples or r.get('full_failure') for r in group) and bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
 report['updated_at']=now();save(path,report)
 from summarize import summarize
 summarize()

def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--languages',default='r');parser.add_argument('--benchmarks',default=','.join(BENCHES));parser.add_argument('--samples',type=int,default=3);parser.add_argument('--timeout',type=int,default=14400);parser.add_argument('--max-rss-mib',type=int,default=10240);parser.add_argument('--pilot',action='store_true');parser.add_argument('--pilot-only',action='store_true')
 args=parser.parse_args();assert args.samples>=3 and args.timeout>0 and args.max_rss_mib>0
 languages=args.languages.split(',');assert set(languages)<=set(LANGUAGES)
 benches=args.benchmarks.split(',');assert set(benches)<=set(BENCHES)
 states={language:open_report(language,args.samples,args.max_rss_mib) for language in languages}
 if args.pilot or args.pilot_only:
  for state in states.values():run_pilots(state,args)
 if args.pilot_only:return
 # Finish three samples per short benchmark before entering long numerical cases.
 for bench in BENCHES:
  if bench not in benches:continue
  for sample in range(args.samples):
   for language,state in states.items():run_sample(language,state,bench,sample,args)
 for state in states.values():
  report=state['report'];state['guard'].verify();copy_aliases(report)
  terminal=all(len(r['samples_seconds'])>=args.samples or r.get('full_failure') for r in report['programs'].values())
  report['status']=('complete-with-failures' if any(r.get('full_failure') for r in report['programs'].values()) else 'complete') if terminal else 'partial';report['updated_at']=now();save(state['path'],report)
  print('REPORT',report['language'],report['status'],flush=True)
 from summarize import summarize
 summarize()
if __name__=='__main__':main()
