from pathlib import Path
from hashlib import sha256
import json,os,subprocess,time
L=Path(__file__).resolve().parent;W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');O=L/'pinned-gpu-tests';sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();record=json.loads((O/'build.json').read_text());env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|record['plan']['environment'];assert record['exit_code']==0;assert record['source_inputs']=={p:sha(W/p) for p in record['source_inputs']}
# Cargo prints the exact generated unit-test executable at the end.
import re
log=(O/'build.log').read_text();matches=re.findall(r'Executable `([^` ]*mech_gpu-[^` ]+)',log)
if not matches:
 matches=re.findall(r'--crate-name mech_gpu[^\n]*?--test[^\n]*?--out-dir ([^ ]+)',log)
 raise AssertionError('Inspect Cargo executable output before running tests: '+repr(matches))
binary=Path(matches[-1]);binary=binary if binary.is_absolute() else W/binary;assert binary.exists();record|={'test_binary':str(binary),'test_binary_sha256':sha(binary)};(O/'build.json').write_text(json.dumps(record,indent=2)+'\n');print('Full optimized GPU test binary built',binary,flush=True)
for label,filter in [('defaulted-loads','defaulted_loads'),('private-loan','private_immutable_default_views_require_the_exact_fixed_source_loan')]:
 command=[str(binary),filter,'--test-threads=1'];start=time.perf_counter()
 with (O/(label+'.log')).open('w') as output:r=subprocess.run(command,cwd=W,env=env,stdout=output,stderr=output)
 (O/(label+'.json')).write_text(json.dumps({'command':command,'exit_code':r.returncode,'seconds_diagnostic':time.perf_counter()-start,'output':(O/(label+'.log')).read_text()},indent=2)+'\n');assert r.returncode==0;print((O/(label+'.log')).read_text(),flush=True)
