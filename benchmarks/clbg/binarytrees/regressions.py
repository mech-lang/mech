#!/usr/bin/env python3
"""Verify the new recursive path preserves prior pure native objects and output."""
import hashlib,json,os,shutil,subprocess,tempfile
from pathlib import Path
import run
ROOT=run.ROOT;HERE=run.HERE;BUILD=run.BUILD/'regressions';BUILD.mkdir(exist_ok=True)
ENV=os.environ|{'RUSTFLAGS':'-D warnings -C target-cpu=native','CARGO_TARGET_DIR':str(ROOT/'benchmarks/clbg/nbody/mech-driver/target'),'RAYON_NUM_THREADS':'8'}
MECH=ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'
def main():
 rows=[]
 report=json.loads((HERE.parent/'fasta/results/apple-m3-streaming-final-25000000.json').read_text())
 for id in ['mech-fasta-final-compact-v2','mech-fasta-final-expanded-v2']:
  row=next(r for r in report['programs'] if r['id']==id);binary=BUILD/id;source=HERE.parent/'fasta'/row['path'];env=ENV|row.get('build_environment',{});env.update({k:ENV[k] for k in ('RUSTFLAGS','CARGO_TARGET_DIR')})
  project=Path(str(binary)+'.project')
  if project.exists():shutil.rmtree(project)
  subprocess.run([str(MECH),'build',str(source),'--backend','cranelift','--offline','--keep-project','--no-config','-o',str(binary)],env=env,cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
  obj=Path(str(binary)+'.project/mech-control.o');assert run.sha(obj)==row['native_object_sha256'],id
  with tempfile.TemporaryFile() as out:
   run.process.run_checked([str(binary),'25000000'],env=env,cwd=ROOT,stdout=out,stderr=subprocess.DEVNULL,timeout=300);out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest()
  assert digest==json.loads((HERE.parent/'fasta/reference-pins.json').read_text())['full']['sha256'];rows.append({'id':id,'native_object_identical':True,'full_output_sha256':digest});print('passed',id,flush=True)
 report=json.loads((HERE.parent/'fannkuchredux/results/apple-m3-final-12.json').read_text())
 for source in ['fannkuchredux.mec','fannkuchredux-even.mec']:
  row=next(r for r in report['programs'] if r['id']==('mech-fannkuch-default' if source=='fannkuchredux.mec' else 'mech-fannkuch-even'));binary=BUILD/source[:-4];env=ENV|row.get('build_environment',{});env.update({k:ENV[k] for k in ('RUSTFLAGS','CARGO_TARGET_DIR')})
  project=Path(str(binary)+'.project')
  if project.exists():shutil.rmtree(project)
  subprocess.run([str(MECH),'build',str(HERE.parent/'fannkuchredux'/source),'--backend','cranelift','--offline','--keep-project','--no-config','-o',str(binary)],env=env,cwd=ROOT,check=True,stdout=subprocess.DEVNULL)
  assert run.sha(Path(str(binary)+'.project/mech-control.o'))==row['native_object_sha256'],source
  assert subprocess.check_output([str(binary),'7'],env=env)==b'228\nPfannkuchen(7) = 16\n';rows.append({'id':source,'native_object_identical':True,'correctness_n':7});print('passed',source,flush=True)
 (HERE/'results/regressions.json').write_text(json.dumps({'compiler_sha256':run.sha(MECH),'passed':len(rows),'checks':rows},indent=2)+'\n')
if __name__=='__main__':main()
