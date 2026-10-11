#!/usr/bin/env python3
"""Compare current spectral-norm/nbody objects and full outputs with preserved builds."""
import argparse,json,os,random,shutil,statistics,subprocess,time
from pathlib import Path
import run
HERE=run.HERE;ROOT=run.ROOT;BUILD=HERE/'.build/extra-regressions'
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--prepare-only',action='store_true');ap.add_argument('--reuse-builds',action='store_true');ap.add_argument('--samples',type=int,default=5);ap.add_argument('--workers',type=int,default=8);ap.add_argument('--output',type=Path,default=HERE/'results/nbody-spectral-regression.json');args=ap.parse_args();BUILD.mkdir(parents=True,exist_ok=True)
 env=os.environ|{'RAYON_NUM_THREADS':str(args.workers),'CARGO_TARGET_DIR':str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'RUSTFLAGS':'-D warnings'}
 if not args.reuse_builds:
  rows=[];report=json.loads((HERE.parent/'spectralnorm/results/apple-m3-optimized-5500.json').read_text())
  for id in ('mech-integer-simd1-u4-r4-p8-g4','mech-optimized-simd1-u4-r4-p8'):
   choices=[r for r in report['programs'] if r['id']==id]
   if not choices and 'float' in id:choices=[r for r in report['programs'] if r['language']=='mech' and r['path']=='spectralnorm.mec' and r['unroll']==4 and r.get('rows',r.get('rows_per_kernel',1))==4 and r['grain']==16]
   old=choices[0];source=HERE.parent/'spectralnorm'/old['path'];binary=BUILD/old['id']
   project=Path(str(binary)+'.project')
   if project.exists():shutil.rmtree(project)
   build_env=env|old['build_environment'];subprocess.run([str(ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),'build',str(source),'--backend','cranelift','--offline','--keep-project','--no-config','-o',str(binary)],env=build_env,cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
   runtime={'MECH_SEQUENCE_PARTITIONS':str(old['partitions']),'MECH_ARRAY_GRAIN':str(old.get('grain',16))}
   common={'benchmark':'spectralnorm','source_sha256':run.sha(source),'runtime_environment':runtime,'expected_small':b'1.274219991\n'.decode(),'expected_full':b'1.274224153\n'.decode()}
   rows+=[common|{'id':old['id']+'-previous','command':old['command']+[str(5500)],'native_object_sha256':old['native_object_sha256']},common|{'id':old['id']+'-current','command':[str(binary),'5500'],'native_object_sha256':run.sha(Path(str(binary)+'.project')/'mech-sequence.o')}]
  # Nbody's native backend is independent of the changed collection ABI.
  # Re-emit its unchanged full source using the current semantic/compiler build.
  driver=ROOT/'benchmarks/clbg/nbody/mech-driver/target/revised-checks/release/mech-clbg-nbody-compiled'
  subprocess.run(['cargo','build','--release','--locked','--offline','--manifest-path',str(HERE.parent/'nbody/mech-driver/Cargo.toml')],cwd=ROOT,env=env|{'RUSTFLAGS':'-D warnings -C target-cpu=native','CARGO_TARGET_DIR':str(driver.parents[1])},check=True,stdout=subprocess.DEVNULL)
  source=HERE.parent/'nbody/nbody-factored.mec';c=BUILD/'nbody.c';obj=BUILD/'nbody.o';binary=BUILD/'nbody'
  subprocess.run([str(driver),'0',str(source),'emit-cranelift-simd',str(c),str(obj),'--output','energy','--fp-contract','--algebraic-powers'],env=env,cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
  sdk=subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip();subprocess.run(['/opt/homebrew/opt/llvm@22/bin/clang-22','-O3','-mcpu=native','-ffp-contract=off','-fno-math-errno',str(c),str(obj),'-lm','-o',str(binary),'-isysroot',sdk],check=True)
  old_report=json.loads((HERE.parent/'nbody/results/apple-m3-v04-native-optimized-50m.json').read_text());old=next(r for r in old_report['programs'] if r['id']=='mech-aot-simd-factored-v04')
  common={'benchmark':'nbody','source_sha256':run.sha(source),'runtime_environment':{},'expected_small':'-0.169075164\n-0.169087605\n','expected_full':'-0.169075164\n-0.169059907\n'}
  rows +=[common|{'id':'nbody-previous','command':old['command'],'native_object_sha256':run.sha(HERE.parent/'nbody/.build/mech-aot-simd-factored-v04.o')},common|{'id':'nbody-current','command':[str(binary),'50000000'],'native_object_sha256':run.sha(obj)}]
  (BUILD/'prepared.json').write_text(json.dumps(rows,indent=2)+'\n')
  if args.prepare_only:return
 else:rows=json.loads((BUILD/'prepared.json').read_text())
 for r in rows:
  argv=r['command'][:-1]+['100' if r['benchmark']=='spectralnorm' else '1000'];out=subprocess.check_output(argv,env=env|r['runtime_environment']);assert out.decode()==r['expected_small'],(r['id'],out)
  out=subprocess.check_output(r['command'],env=env|r['runtime_environment']);assert out.decode()==r['expected_full'],(r['id'],out);r['samples_seconds']=[]
 rng=random.Random(902)
 for sample in range(args.samples):
  shuffled=rows.copy();rng.shuffle(shuffled)
  for r in shuffled:
   start=time.perf_counter_ns();out=subprocess.check_output(r['command'],env=env|r['runtime_environment']);elapsed=(time.perf_counter_ns()-start)/1e9;assert out.decode()==r['expected_full'];r['samples_seconds'].append(elapsed);print(r['id'],elapsed,flush=True)
 for r in rows:r['median_seconds']=statistics.median(r['samples_seconds'])
 args.output.write_text(json.dumps({'programs':rows,'all_outputs_passed':True,'samples':args.samples,'workers':args.workers,'protocol':'Sequential fresh processes, builds and conformance completed before sampling.','compiler_snapshot':run.snapshot(args.output)},indent=2)+'\n')
if __name__=='__main__':main()
