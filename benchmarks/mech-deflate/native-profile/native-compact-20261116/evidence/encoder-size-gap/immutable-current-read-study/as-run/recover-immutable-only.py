"""Explicit allocation refusals alternating with successful recovery for the actual encoder."""
from pathlib import Path
from runpy import run_path
from hashlib import sha256
import json,os,subprocess,time
ROOT=Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/consolidation-995/collector-read-lifetimes/immutable-only')
LAB=ROOT.parent.parent
PACKET=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116')
h=run_path(str(PACKET/'rebuild_checks.py'),run_name='early_recovery_checks')
import sys
role=sys.argv[1]
actor=ROOT/role/'bound'
recipe=json.loads((actor/'runtime.json').read_text());command=recipe['link'].copy()
old=[a for a in command if a.endswith('.rs') and Path(a).is_file()];assert len(old)==1
assert sha256(Path(old[0]).read_bytes()).hexdigest()==sha256((PACKET/'worker.rs').read_bytes()).hexdigest()
command[command.index(old[0])]=PACKET/'worker.rs'
out=ROOT/role/'recoveries';out.mkdir()
fixtures=json.loads((PACKET/'fixtures.json').read_text())
for case in fixtures:
 for key in ['input','prefix','expected']:case[key]=str(PACKET/case[key])
fixture_file=out/'fixtures.json';fixture_file.write_text(json.dumps(fixtures))
clean={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
def run(command,log,env):
 args=list(map(str,command));start=time.perf_counter()
 with log.open('w') as stream:subprocess.run(args,env=env,stdout=stream,stderr=stream,check=True)
 return {'command':args,'seconds':time.perf_counter()-start}
result=h['check_faults'](PACKET,out,command,actor/'metadata.json',fixture_file,fixtures,clean,run)
result.update(object_sha256=recipe['object_sha256'],metadata_sha256=recipe['metadata_sha256'],benchmark_worker_sha256=recipe['worker_sha256'])
(out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k!='link'},indent=2))
