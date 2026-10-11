#!/usr/bin/env python3
from pathlib import Path
import hashlib,json,os,subprocess
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[3]
BASE=HERE.with_name('20261010')
EMIT=Path('/tmp/mech-execution-modes-target/release/mech-execution-modes-final-emit')
BIN=HERE/'.build/mech-execution-modes-final'
def sha(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
report=dict(status='checking',runner=str(BIN),runner_sha256=sha(BIN),emitter=str(EMIT),emitter_sha256=sha(EMIT),programs={})
base=json.loads((BASE/'builds.json').read_text())
for key,row in base['programs'].items():
 directory=HERE/'objects'/key.replace(':','-');directory.mkdir(parents=True,exist_ok=True)
 env=os.environ|row['build_environment']
 command=[str(EMIT),row['kind'],row['source'],str(directory),*row['flags']]
 with (directory/'emit.log').open('w') as log:subprocess.run(command,env=env,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
 suffix='numeric' if row['kind']=='numeric' else row['kind']
 obj=directory/f'mech-{suffix}.o'
 identical=sha(obj)==row['native_object_sha256']
 report['programs'][key]=dict(source_sha256=sha(row['source']),native_object_sha256=sha(obj),original_native_object_sha256=row['native_object_sha256'],identical=identical,command=command)
 print('IDENTICAL' if identical else 'CHANGED',key,flush=True)
 (HERE/'objects.json').write_text(json.dumps(report,indent=2)+'\n')
 assert identical,key
report['status']='complete';(HERE/'objects.json').write_text(json.dumps(report,indent=2)+'\n')
