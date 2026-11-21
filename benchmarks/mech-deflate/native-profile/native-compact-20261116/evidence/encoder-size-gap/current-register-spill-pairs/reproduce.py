"""Rebuild the current paired objects from this checkout's compiler; no LLVM tools."""
from pathlib import Path
from hashlib import sha256
import argparse,json,os,subprocess
here=Path(__file__).resolve().parent;packet=here.parents[2]
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('compiler',type=Path);parser.add_argument('output',type=Path);args=parser.parse_args()
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();pin=json.loads((here/'candidate.json').read_text())
source=packet/'program.mec';assert sha(source)==pin['source_sha256'];compiler=args.compiler.resolve(strict=True);out=args.output.resolve();out.mkdir(parents=True)
flags=pin['native_flags_as_run'].copy();profiles=[]
for p in pin['profiles']:
 path=here/p['path'];assert sha(path)==p['sha256'];profiles.append(p['symbol']+'@'+str(path))
flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(profiles)
results=[]
for name,mode in [('normal','1'),('checked','checked')]:
 folder=out/name;folder.mkdir();front=folder/'frontend';command=[str(compiler),str(source),str(front),'compact'];env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':mode}
 with (folder/'compile.log').open('w') as log:subprocess.run(command,env=env,stdout=log,stderr=log,check=True)
 actual={n:sha(front/n) for n in pin['artifact_sha256']};assert actual==pin['artifact_sha256'],actual
 results.append({'mode':name,'command':command,'compiler_sha256':sha(compiler),'artifacts':actual})
(out/'source-reproduction.json').write_text(json.dumps({'source_sha256':sha(source),'variants':results,'runtime_wins':'not measured by this replay'},indent=2)+'\n')
print('Normal and allocation-checked source builds reproduce the paired object, metadata and bytecode.')
