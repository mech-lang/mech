#!/usr/bin/env python3
"""Rebuild prior programs and compare native objects, full outputs and timing."""
import argparse,hashlib,json,os,random,shutil,statistics,subprocess,tempfile,time
from pathlib import Path
import run
HERE=run.HERE;ROOT=run.ROOT;CLBG=HERE.parent;BUILD=run.BUILD/'regressions';COMPILER=ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'
ENV=os.environ|{'RAYON_NUM_THREADS':'8','CARGO_TARGET_DIR':str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'RUSTFLAGS':'-D warnings'}
def digest_bytes(raw):return hashlib.sha256(raw).hexdigest()
def execute(row,small=False):
 argv=row['command']+row['small_args' if small else 'full_args'];stdin=row.get('small_input' if small else 'full_input')
 with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
  input_file=open(stdin,'rb') if stdin else subprocess.DEVNULL
  try:
   start=time.perf_counter_ns();run.process.run_checked(argv,cwd=ROOT,env=ENV|row['runtime_environment'],stdin=input_file,stdout=out,stderr=err,timeout=300);elapsed=(time.perf_counter_ns()-start)/1e9
  finally:
   if stdin:input_file.close()
  out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest();expected=row['small_sha256' if small else 'full_sha256'];assert digest==expected,(row['id'],small,digest,expected)
 return elapsed

def build_row(row,source,env,obj,flags=()):
 binary=BUILD/row['id'];project=Path(str(binary)+'.project')
 if project.exists():shutil.rmtree(project)
 cmd=[str(COMPILER),'build',str(source),'--backend','cranelift','--offline','--keep-project','--no-config','-o',str(binary),*flags]
 subprocess.run(cmd,cwd=ROOT,env=ENV|env,check=True,stdout=subprocess.DEVNULL)
 return row|{'id':row['id']+'-current','command':[str(binary)],'native_object_sha256':run.sha(project/obj),'generation_command':cmd,'build_environment':env}

def prepare():
 rows=[];past=CLBG/'fannkuchredux/.build';old=json.loads((past/'previous-regressions/prepared.json').read_text())
 for r in old:
  if not r['id'].endswith('current'):continue
  benchmark=r['benchmark'];id=r['id'][:-len('-current')];source=CLBG/benchmark/r['source']['paths'][0]
  common=dict(id=id,benchmark=benchmark,runtime_environment=r['runtime_environment'],command=[str((ROOT/r['command'][0]).resolve())],native_object_sha256=r['native_object_sha256'],small_sha256=run.sha(CLBG/benchmark/('upstream/mandelbrot-200.pbm' if benchmark=='mandelbrot' else 'inputs/knucleotide-output-1000.txt')),full_sha256=r['expected_full_sha256'],small_args=['200','8'] if benchmark=='mandelbrot' else [],full_args=['16000','8'] if benchmark=='mandelbrot' else [],source_sha256=run.sha(source))
  if benchmark=='knucleotide':common.update(small_input=str(CLBG/'knucleotide/inputs/knucleotide-input-1000.txt'),full_input=str(CLBG/'knucleotide/.build/knucleotide-input25000000.txt'))
  flags=['--scalar'] if id.endswith('compact') else ['--fp-contract-loop'] if 'loop-fma' in id else []
  current=build_row(common,source,r['build_environment'],'grid.o' if benchmark=='mandelbrot' else 'mech-sequence.o',flags);rows.extend([common|{'id':id+'-previous'},current])
 spec=json.loads((CLBG/'spectralnorm/results/apple-m3-optimized-5500.json').read_text())
 saved=json.loads((past/'extra-regressions/prepared.json').read_text())
 for id in ('mech-integer-simd1-u4-r4-p8-g4','mech-optimized-simd1-u4-r4-p8'):
  original=next(r for r in spec['programs'] if r['id']==id);r=next(r for r in saved if r['id']==id+'-current');source=CLBG/'spectralnorm'/original['path']
  common=dict(id=id,benchmark='spectralnorm',runtime_environment=r['runtime_environment'],command=r['command'][:1],native_object_sha256=r['native_object_sha256'],small_args=['100'],full_args=['5500'],small_sha256=digest_bytes(b'1.274219991\n'),full_sha256=digest_bytes(b'1.274224153\n'),source_sha256=run.sha(source))
  rows.extend([common|{'id':id+'-previous'},build_row(common,source,original['build_environment'],'mech-sequence.o')])
 rev=run.module('revcomp_reference',CLBG/'revcomp/run.py');small,full,small_sha,full_sha,_=rev.references()
 r=next(r for r in json.loads((past/'revcomp-regression/prepared.json').read_text()) if r['id']=='revcomp-current');original=next(r for r in json.loads((CLBG/'revcomp/results/apple-m3-final-25000000.json').read_text())['programs'] if r['id']=='mech-revcomp-optimized');source=CLBG/'revcomp'/original['path']
 common=dict(id='revcomp',benchmark='revcomp',runtime_environment=r['runtime_environment'],command=r['command'],native_object_sha256=r['native_object_sha256'],small_args=[],full_args=[],small_input=str(small),full_input=str(full),small_sha256=small_sha,full_sha256=full_sha,source_sha256=run.sha(source))
 rows.extend([common|{'id':'revcomp-previous'},build_row(common,source,original['build_environment'],'mech-sequence.o')])
 report=json.loads((CLBG/'fannkuchredux/results/apple-m3-final-12.json').read_text())
 for path in ('fannkuchredux.mec','fannkuchredux-even.mec'):
  choices=[r for r in report['programs'] if r['language']=='mech' and r['path']==path];r=min(choices,key=lambda r:r['median_seconds']);source=CLBG/'fannkuchredux'/path
  common=dict(id=r['id'],benchmark='fannkuchredux',runtime_environment=r.get('runtime_environment',{}),command=r['command'],native_object_sha256=r['native_object_sha256'],small_args=['7'],full_args=['12'],small_sha256=digest_bytes(b'228\nPfannkuchen(7) = 16\n'),full_sha256=digest_bytes(b'3968050\nPfannkuchen(12) = 65\n'),source_sha256=run.sha(source))
  rows.extend([common|{'id':r['id']+'-previous'},build_row(common,source,r['build_environment'],'mech-control.o',[] if r.get('simd',True) else ['--scalar'])])
 # Emit the existing nbody source through the numeric backend and identical policy.
 driver=CLBG/'nbody/mech-driver/target/revised-checks/release/mech-clbg-nbody-compiled'
 subprocess.run(['cargo','build','--release','--locked','--offline','--manifest-path',str(CLBG/'nbody/mech-driver/Cargo.toml')],cwd=ROOT,env=ENV|{'RUSTFLAGS':'-D warnings -C target-cpu=native','CARGO_TARGET_DIR':str(driver.parents[1])},check=True,stdout=subprocess.DEVNULL)
 source=CLBG/'nbody/nbody-factored.mec';c=BUILD/'nbody.c';obj=BUILD/'nbody.o';binary=BUILD/'nbody'
 subprocess.run([str(driver),'0',str(source),'emit-cranelift-simd',str(c),str(obj),'--output','energy','--fp-contract','--algebraic-powers'],env=ENV,cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
 sdk=subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip();subprocess.run(['/opt/homebrew/opt/llvm@22/bin/clang-22','-O3','-mcpu=native','-ffp-contract=off','-fno-math-errno',str(c),str(obj),'-lm','-o',str(binary),'-isysroot',sdk],check=True)
 r=next(r for r in saved if r['id']=='nbody-current');common=dict(id='nbody',benchmark='nbody',runtime_environment={},small_args=['1000'],full_args=['50000000'],small_sha256=digest_bytes(b'-0.169075164\n-0.169087605\n'),full_sha256=digest_bytes(b'-0.169075164\n-0.169059907\n'),source_sha256=run.sha(source))
 rows.extend([common|{'id':'nbody-previous','command':r['command'][:1],'native_object_sha256':r['native_object_sha256']},common|{'id':'nbody-current','command':[str(binary)],'native_object_sha256':run.sha(obj)}])
 for row in rows:row['executable_sha256']=run.sha(row['command'][0])
 (BUILD/'prepared.json').write_text(json.dumps(rows,indent=2)+'\n')
 return rows

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--reuse-builds',action='store_true');ap.add_argument('--samples',type=int,default=3);ap.add_argument('--only');ap.add_argument('--output',type=Path,default=HERE/'results/regressions.json');a=ap.parse_args();BUILD.mkdir(parents=True,exist_ok=True)
 rows=json.loads((BUILD/'prepared.json').read_text()) if a.reuse_builds else prepare()
 if a.prepare_only:return
 if a.only:rows=[r for r in rows if __import__('re').search(a.only,r['id'])]
 for row in rows:
  assert run.sha(row['command'][0])==row['executable_sha256'];execute(row,True);execute(row);row['samples_seconds']=[];print('full passed',row['id'],flush=True)
 rng=random.Random(20261007);orders=[]
 for sample in range(a.samples):
  shuffled=rows.copy();rng.shuffle(shuffled);orders.append([r['id'] for r in shuffled])
  for row in shuffled:row['samples_seconds'].append(execute(row));print(sample+1,row['id'],row['samples_seconds'][-1],flush=True)
 for row in rows:row['median_seconds']=statistics.median(row['samples_seconds'])
 pairs=[]
 for row in rows:
  if not row['id'].endswith('-current'):continue
  old=next(r for r in rows if r['id']==row['id'][:-len('-current')]+'-previous');pairs.append(dict(benchmark=row['benchmark'],id=row['id'],native_object_identical=row['native_object_sha256']==old['native_object_sha256'],current_to_previous=row['median_seconds']/old['median_seconds']))
 output=a.output;output.write_text(json.dumps(dict(programs=rows,pairs=pairs,all_outputs_passed=True,samples=a.samples,sample_order=orders,compiler_snapshot=run.snapshot(output),protocol='Identical source and native policy. Sequential fresh processes, complete output and regular-file IO; validation and hashing excluded from timed region.'),indent=2)+'\n')
if __name__=='__main__':main()
