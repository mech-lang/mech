"""Reconstruct ignored canonical stdin files from the visible R FASTA program."""
import fcntl,json,subprocess
from pathlib import Path
from prepare import G,C,LOCK,compact,REQUIRED,sha

def main():
 manifest=json.loads((G/'r-manifest.json').read_text())
 fasta=next(r for r in manifest['programs'] if r['benchmark']=='fasta' and r['variant']=='optimized')
 specifications=[(C/'knucleotide/.build/knucleotide-input25000000.txt',25000000,json.loads((C/'knucleotide/input25000000.json').read_text())['sha256']),(C/'regexredux/.build/input5000000.txt',5000000,json.loads((C/'regexredux/input-pins.json').read_text())['full_input']['sha256'])]
 for target,n,digest in specifications:
  if target.exists():assert sha(target)==digest,target;continue
  target.parent.mkdir(parents=True,exist_ok=True);temporary=target.with_suffix('.generating')
  with LOCK.open('a+') as lock:
   fcntl.flock(lock,fcntl.LOCK_EX);compact.wait_for_environment(REQUIRED)
   assert sha(fasta['source'])==fasta['source_sha256']
   command=[x.replace('{n}',str(n)) for x in fasta['command']]
   with temporary.open('wb') as output:subprocess.run(command,stdout=output,check=True,cwd=fasta.get('cwd',str(compact.ROOT)),env=compact.ENV|fasta['runtime_environment'])
   assert sha(temporary)==digest,('Generated input differs from canonical pin',target)
   temporary.replace(target)
  print('INPUT READY',target,n,flush=True)
if __name__=='__main__':main()
