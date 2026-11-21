from pathlib import Path
from hashlib import sha256
import json,os,subprocess,sys
L=Path(__file__).resolve().parent;role=sys.argv[1];out=L/role;out.mkdir(exist_ok=True);read=lambda p:json.loads(p.read_text());base=read(L/'fused/build.json');env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')};W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');env|={'CARGO_MANIFEST_DIR':str(W/'src/syntax'),'CARGO_PKG_VERSION':'0.3.5'}
cmd=[x.replace(str(L/'fused'),str(out)).replace('metadata=sealed-base-fused','metadata=sealed-base-'+role) for x in base['command']];link=[x.replace(str(L/'fused'),str(out)) for x in base['link']]
for name,command in [('build',cmd),('link',link)]:
 with (out/(name+'.log')).open('w') as log:subprocess.run(command,env=env,cwd=W,stdout=log,stderr=log,check=True)
(out/'build.json').write_text(json.dumps({'command':cmd,'link':link,'source_commit':base['source_commit'],'source_sha256':sha256((W/'src/syntax/src/document/parser/canonical/base/continuation.rs').read_bytes()).hexdigest(),'library_sha256':sha256((out/'libmech_syntax.rlib').read_bytes()).hexdigest(),'binary_sha256':sha256((out/'parser').read_bytes()).hexdigest()},indent=2)+'\n');print('Built '+role,flush=True)
