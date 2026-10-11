#!/usr/bin/env python3
"""Full-workload JIT/resident timings with source-faithful AOT regression controls."""
from pathlib import Path
from datetime import datetime, timezone
import argparse, hashlib, importlib.util, json, os, random, statistics, subprocess, tempfile, time
HERE=Path(__file__).resolve().parent;CLBG=HERE.parents[1];ROOT=CLBG.parents[1]
spec=importlib.util.spec_from_file_location('mode_guard',CLBG/'compact/run.py');guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
WORK=guard.WORK|{'regexredux':(30,5000000),'pidigits':(30,10000)}
POWER=dict(source='AC Power',low_power_mode=0)
ORDER=['revcomp','regexredux','knucleotide','pidigits','nbody','mandelbrot','spectralnorm','binarytrees','fasta','fannkuchredux']
BIN=Path('/tmp/mech-execution-modes-target/release/mech-execution-modes-compiled')
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def save(p,v):
 t=p.with_suffix('.json.tmp');t.write_text(json.dumps(v,indent=2)+'\n');t.replace(p)
def input_path(bench,full):
 if bench in ('knucleotide','revcomp'):return guard.FULL_INPUT if full else guard.SMALL_INPUT
 if bench=='regexredux':return CLBG/('regexredux/.build/input5000000.txt' if full else 'regexredux/inputs/regexredux-input-small.txt')
def validate(bench,full,out):
 out.seek(0,os.SEEK_END);size=out.tell();out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest();result=dict(bytes=size,sha256=digest,reference_match=True)
 if bench=='nbody':
  out.seek(0);text=out.read().decode();expected=[-0.169075164,-0.169059907] if full else [-0.169075164,-0.169087605];guard.common.validate(text,expected);result|=dict(energies=guard.common.energies(text),tolerance=1e-8)
 elif bench in ('spectralnorm','fannkuchredux'):
  out.seek(0);text=out.read().decode();expected={'spectralnorm':('1.274219991\n','1.274224153\n'),'fannkuchredux':('228\nPfannkuchen(7) = 16\n','3968050\nPfannkuchen(12) = 65\n')}[bench][int(full)];assert text==expected,(bench,text,expected)
 elif bench in ('regexredux','pidigits'):
  reference=CLBG/bench/'inputs'/('output-full.txt' if full else bench+'-output-small.txt');assert size==reference.stat().st_size and digest==sha(reference),(bench,size,digest)
 else:
  expected=guard.HASHES[bench][int(full)];assert (size,digest)==tuple(expected),(bench,size,digest,expected)
 return result

def execute(row,full,timeout):
 bench=row['benchmark'];n=WORK[bench][int(full)];argv=[arg.replace('{n}',str(n)) for arg in row['command']];ip=input_path(bench,full)
 with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
  inp=ip.open('rb') if ip else subprocess.DEVNULL;monitor=guard.CompilerMonitor(POWER)
  try:
   monitor.start();start=time.perf_counter_ns();guard.run_process(argv,inp,out,err,timeout,guard.ENV|row['runtime_environment']|{'MECH_TIMINGS':'1'});wall=(time.perf_counter_ns()-start)/1e9
  except Exception as error:
   err.seek(0);raise RuntimeError(str(error)+'; '+err.read(4000).decode(errors='replace')) from error
  finally:
   if ip:inp.close()
   monitor.finish()
  err.seek(0);stderr=err.read().decode();measurements=[json.loads(line.removeprefix('TIMING ')) for line in stderr.splitlines() if line.startswith('TIMING ')]
  if row['mode'] in ('jit','resident'):
   assert len(measurements)==1,(row['key'],stderr);detail=measurements[0];assert detail['kernel_seconds']>0 and detail['prepare_seconds']>=0
  else:detail={}
  seconds=detail['kernel_seconds'] if row['mode']=='resident' else wall
  return dict(seconds=seconds,wall_seconds=wall,**detail,validation=validate(bench,full,out),competing_compilers=monitor.observations,power_changes=monitor.power_changes)

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--samples',type=int,default=3);parser.add_argument('--timeout',type=int,default=14400);parser.add_argument('--benchmarks',default=','.join(ORDER));parser.add_argument('--validate-only',action='store_true');args=parser.parse_args();assert args.samples>=3
 builds=json.loads((HERE/'builds.json').read_text());assert builds['status']=='complete' and len(builds['programs'])==15
 path=HERE/'timings.json';report=json.loads(path.read_text()) if path.exists() else dict(status='running',created_at=datetime.now(timezone.utc).isoformat(),machine=subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),sample_count=args.samples,power_profile=POWER,workloads=WORK,builds_sha256=sha(HERE/'builds.json'),runner_sha256=sha(BIN),programs={},orders={},completed_benchmarks=[],protocol={'jit':'Fresh-process elapsed wall time including startup, source parsing, admission, JIT compilation, execution, input, full output to a regular file, flush and teardown.','resident':'Loaded typed instruction interpreter. Timing starts after source loading, graph optimization and activation; includes input, standard-library operations, interpreted arithmetic/control, output formatting, writing and flush. Full process time and preparation time are retained separately. Fresh initial workload state for every sample; no output/workload caching.','controls':'Old and rebuilt AOT binaries measured in the same randomized rounds, with byte-identical source-generated native objects.','samples':'Three fresh runs per source/mode at the full standard workloads, <=8 workers, randomized sequential rounds, AC power and Low Power Mode off. All full outputs independently checked; compiler/power contention discards samples. Compilation of the implementation, output hashes, and validation comparisons are outside every reported region.'})
 assert report['sample_count']==args.samples and report['runner_sha256']==sha(BIN) and report['builds_sha256']==sha(HERE/'builds.json')
 for source in builds['programs'].values():
  bench=source['benchmark']
  for mode in ('jit','resident','aot-new','aot-original'):
   key=source['key']+':'+mode;env=source['runtime_environment']
   if mode in ('jit','resident'):command=[str(BIN),source['source'],mode]+([] if input_path(bench,False) else ['{n}'])+source['flags'];binary=BIN
   else:binary=Path(source['binary'] if mode=='aot-new' else source['baseline_binary']);command=[str(binary)]+([] if input_path(bench,False) else ['{n}'])+(['8'] if bench=='mandelbrot' else [])
   report['programs'].setdefault(key,dict(key=key,source_key=source['key'],benchmark=bench,mode=mode,language='mech-jit' if mode=='jit' else 'mech-resident' if mode=='resident' else 'mech',source=source['source'],source_sha256=source['source_sha256'],loc=source['loc'],physical_loc=source['physical_loc'],loc_expansions=source['loc_expansions'],binary=str(binary),binary_sha256=sha(binary),command=command,runtime_environment=env,samples_seconds=[],sample_validation=[],sample_details=[],timing_boundary=report['protocol']['resident' if mode=='resident' else 'jit' if mode=='jit' else 'controls'],eligible_for_chart=mode in ('jit','resident'),required_work_validated=True))
 def pins(row=None):
  assert sha(HERE/'builds.json')==report['builds_sha256']
  files={str(BIN):report['runner_sha256']}
  for item in ([row] if row else report['programs'].values()):files|={item['source']:item['source_sha256'],item['binary']:item['binary_sha256']}
  for file,digest in files.items():assert sha(file)==digest,file
 save(path,report);pins();selected=args.benchmarks.split(',');assert set(selected)<=set(ORDER)
 for bench in selected:
  rows=[row for row in report['programs'].values() if row['benchmark']==bench]
  for row in rows:
   if 'small_validation' in row:continue
   guard.wait_for_environment(POWER);print('SMALL',row['key'],flush=True);result=execute(row,False,args.timeout);row['small_validation']=result['validation'];save(path,report)
  if args.validate_only:continue
  rng=random.Random(20261010+sum(map(ord,bench)))
  for round_no in range(args.samples):
   order=rows.copy();rng.shuffle(order);report['orders'].setdefault(bench,[])
   if len(report['orders'][bench])<=round_no:report['orders'][bench].append([r['key'] for r in order])
   else:assert report['orders'][bench][round_no]==[r['key'] for r in order]
   for row in order:
    if len(row['samples_seconds'])>round_no:continue
    while True:
     guard.wait_for_environment(POWER);pins(row);report['active_run']=dict(key=row['key'],sample=round_no+1,started_at=datetime.now(timezone.utc).isoformat());save(path,report);print('RUN',round_no+1,row['key'],flush=True)
     result=execute(row,True,args.timeout)
     if result['competing_compilers'] or result['power_changes']:report.setdefault('discarded',[]).append(dict(key=row['key'],round=round_no+1,result=result));save(path,report);continue
     row['samples_seconds'].append(result['seconds']);row['sample_validation'].append(result['validation']);row['sample_details'].append({k:v for k,v in result.items() if k not in ('validation','competing_compilers','power_changes')});row['full_validation']=result['validation'];row['median_seconds']=statistics.median(row['samples_seconds']);row['sample_stdev_seconds']=statistics.stdev(row['samples_seconds']) if len(row['samples_seconds'])>1 else None
     print('SAMPLE',round_no+1,row['key'],f'{result["seconds"]:.6f}s',f'process {result["wall_seconds"]:.6f}s',flush=True);report.pop('active_run',None);save(path,report);break
  if bench not in report['completed_benchmarks']:report['completed_benchmarks'].append(bench)
  save(path,report)
 report['aot_regressions']={source['key']:dict(native_object_identical=source['identical_native_object'],old_seconds=report['programs'][source['key']+':aot-original'].get('median_seconds'),new_seconds=report['programs'][source['key']+':aot-new'].get('median_seconds'),ratio=(report['programs'][source['key']+':aot-new']['median_seconds']/report['programs'][source['key']+':aot-original']['median_seconds'] if report['programs'][source['key']+':aot-original'].get('median_seconds') and report['programs'][source['key']+':aot-new'].get('median_seconds') else None)) for source in builds['programs'].values()}
 report['status']='validated' if args.validate_only else 'complete' if set(report['completed_benchmarks'])==set(ORDER) else 'partial';report['finished_at']=datetime.now(timezone.utc).isoformat();pins();save(path,report);print('REPORT',path,report['status'],flush=True)
if __name__=='__main__':main()
