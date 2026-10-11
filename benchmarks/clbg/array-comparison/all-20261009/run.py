"""Resumable, sequential, guarded full-size BQN/K benchmark measurements."""
import argparse,hashlib,importlib.util,json,os,random,statistics,subprocess,tempfile,time
from datetime import datetime,timezone
from pathlib import Path
HERE=Path(__file__).resolve().parent;CLBG=HERE.parents[1]
spec=importlib.util.spec_from_file_location('compact',CLBG/'compact/run.py')
compact=importlib.util.module_from_spec(spec);spec.loader.exec_module(compact)
WORK=compact.WORK|{'regexredux':(30,5000000),'pidigits':(30,10000)}
REQUIRED=dict(source='AC Power',low_power_mode=0)
BENCHES=['spectralnorm','regexredux','pidigits','mandelbrot','revcomp','fasta','binarytrees','knucleotide','fannkuchredux']
def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def execute(row,full,timeout):
 bench=row['benchmark'];n=WORK[bench][int(full)]
 argv=[s.replace('{n}',str(n)) for s in row['command']]
 ip=None
 if bench in ('knucleotide','revcomp'):ip=compact.FULL_INPUT if full else compact.SMALL_INPUT
 if bench=='regexredux':ip=CLBG/('regexredux/.build/input5000000.txt' if full else 'regexredux/inputs/regexredux-input-small.txt')
 with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
  stdin=ip.open('rb') if ip else subprocess.DEVNULL
  monitor=compact.CompilerMonitor(REQUIRED)
  try:
   monitor.start();start=time.perf_counter_ns()
   compact.run_process(argv,stdin,out,err,timeout,compact.ENV|row.get('runtime_environment',{}))
   seconds=(time.perf_counter_ns()-start)/1e9
  except Exception as error:
   err.seek(0);raise RuntimeError(str(error)+'; '+err.read(2000).decode(errors='replace')) from error
  finally:
   if ip:stdin.close()
   monitor.finish()
  out.seek(0,os.SEEK_END);size=out.tell();out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest()
  validation=dict(bytes=size,sha256=digest,reference_match=False)
  if bench in ('spectralnorm','fannkuchredux'):
   out.seek(0);actual=out.read().decode()
   expected=({'spectralnorm':['1.274219991\n','1.274224153\n'],'fannkuchredux':['228\nPfannkuchen(7) = 16\n','3968050\nPfannkuchen(12) = 65\n']})[bench][int(full)]
   assert actual==expected,(bench,actual,expected)
  elif bench in ('regexredux','pidigits'):
   name='output-full.txt' if full else bench+'-output-small.txt';reference=CLBG/bench/'inputs'/name
   assert size==reference.stat().st_size and digest==sha(reference),(bench,size,digest)
  else:
   rs,rh=compact.HASHES[bench][int(full)];assert size==rs and digest==rh,(bench,size,digest,rs,rh)
  validation['reference_match']=True
  return dict(seconds=seconds,validation=validation,competing_compilers=monitor.observations,power_changes=monitor.power_changes)

def main():
 ap=argparse.ArgumentParser(description=__doc__)
 ap.add_argument('--benchmarks',default=','.join(BENCHES));ap.add_argument('--samples',type=int,default=3);ap.add_argument('--timeout',type=int,default=14400)
 args=ap.parse_args();assert args.samples>=3
 path=HERE/'results/faithful-timings.json';path.parent.mkdir(exist_ok=True)
 audit=json.loads((HERE/'source-audit.json').read_text())
 report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),sample_count=args.samples,protocol='Sequential fresh processes, randomized rounds; complete stdout written to regular files; startup, libraries, interpreter workers and output included; compilation, validation and hashing excluded; AC power, Low Power Mode off; concurrent compiler and power guards. At most eight language workers per program.',workloads=WORK,programs={},orders={},completed_benchmarks=[],source_audit_sha256=sha(HERE/'source-audit.json'),library_pins=audit['libraries'],runtimes=audit['runtimes'],reused_nbody_report='array-comparison/nbody-20261009/comparison-mech-compact.json',reused_nbody_report_sha256=sha(CLBG/'array-comparison/nbody-20261009/comparison-mech-compact.json'))
 assert report['sample_count']==args.samples and report['source_audit_sha256']==sha(HERE/'source-audit.json')
 def save():
  t=path.with_suffix('.json.tmp');t.write_text(json.dumps(report,indent=2)+'\n');t.replace(path)
 def pins():
  for filename,digest in report['library_pins'].items():assert sha(HERE/filename)==digest,filename
  for r in report['runtimes'].values():assert sha(r['path'])==r['sha256']
 builds=json.loads((CLBG/'function-application-audit/20261009T171437Z/application-v3/builds.json').read_text())
 corrections=json.loads((CLBG/'work-audit/20261009/builds.json').read_text())
 save();pins()
 for bench in args.benchmarks.split(','):
  records=[r for r in audit['rows'] if r['benchmark']==bench and r['algorithm']['strict']]
  assert {r['language'] for r in records}=={'bqn','k'},bench
  rows=[]
  for record in records:
   source=HERE/'sources'/record['file'];assert sha(source)==record['sha256']
   lang=record['language'];runtime=report['runtimes'][lang];key=bench+':array-'+lang+'-faithful'
   p=report['programs'].setdefault(key,dict(key=key,id=key,benchmark=bench,language=lang,command=[runtime['path'],str(source)]+([] if bench in ('knucleotide','revcomp','regexredux') else ['{n}']),source=str(source),source_sha256=record['sha256'],loc=record['normalized_loc'],physical_loc=record['physical_loc'],loc_expansions=record.get('expansions',[]),algorithm=record['algorithm'],runtime_sha256=runtime['sha256'],eligible_for_chart=True,samples_seconds=[],sample_validation=[]))
   rows.append(p)
  build=next(p for p in builds['programs'] if p['benchmark']==bench)['builds']['after']
  faithful=[p for p in corrections['programs'].values() if p['benchmark']==bench and p['variant'] in ('four-functions','floating')]
  if faithful:build=max(faithful,key=lambda p:p['loc'])
  key=bench+':mech-control-'+build['binary_sha256'][:12]
  control=report['programs'].setdefault(key,dict(key=key,id=key,benchmark=bench,language='mech',command=[build['binary']]+([] if bench in ('knucleotide','revcomp','regexredux') else ['{n}']),source=build['source'],source_sha256=build['source_sha256'],binary_sha256=build['binary_sha256'],runtime_environment=build.get('runtime_environment',{}),samples_seconds=[],sample_validation=[]))
  for row in [control,*rows]:
   compact.wait_for_environment(REQUIRED)
   if not row.get('small_validation'):
    row['small_validation']=execute(row,False,args.timeout)['validation'];print('SMALL PASS',row['key'],flush=True);save()
   if not row.get('full_validation'):
    print('RUN full validation',row['key'],flush=True)
    result=execute(row,True,args.timeout)
    row['full_validation']=result['validation'];row['validation_seconds']=result['seconds'];print('FULL PASS',row['key'],f'{result["seconds"]:.6f}s',flush=True);save()
  rng=random.Random(20261009+sum(map(ord,bench)))
  for round_no in range(args.samples):
   order=[control,*rows];rng.shuffle(order)
   report['orders'].setdefault(bench,[])
   if len(report['orders'][bench])<=round_no:report['orders'][bench].append([r['key'] for r in order])
   for row in order:
    if len(row['samples_seconds'])>round_no:continue
    while True:
     compact.wait_for_environment(REQUIRED);pins();assert sha(row['source'])==row['source_sha256']
     print(f'RUN sample {round_no+1}/{args.samples}',row['key'],flush=True)
     result=execute(row,True,args.timeout)
     if result['competing_compilers'] or result['power_changes']:
      report.setdefault('discarded',[]).append(dict(key=row['key'],round=round_no+1,result=result));save();continue
     row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation'])
     row['median_seconds']=statistics.median(row['samples_seconds'])
     if len(row['samples_seconds'])>=2:row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds'])
     print(f'SAMPLE {round_no+1}/{args.samples}',row['key'],f'{result["seconds"]:.6f}s',flush=True);save();break
  if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
  save()
 report['status']='complete' if set(report['completed_benchmarks'])==set(BENCHES) else 'partial'
 report['finished_at']=datetime.now(timezone.utc).isoformat();pins();save()
 print('REPORT',path,report['status'],flush=True)
if __name__=='__main__':main()
