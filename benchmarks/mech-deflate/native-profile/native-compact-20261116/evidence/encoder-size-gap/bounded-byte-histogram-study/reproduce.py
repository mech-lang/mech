"""Reproduce normal and allocation-checked guarded-histogram artifacts from source."""
from pathlib import Path
from hashlib import sha256
import argparse,json,os,subprocess
here=Path(__file__).resolve().parent;packet=here.parents[2]
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('compiler',type=Path);parser.add_argument('output',type=Path);args=parser.parse_args()
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();pin=json.loads((here/'emission.json').read_text());compiler=args.compiler.resolve(strict=True);source=packet/'program.mec';assert sha(source)==pin['source_sha256'];output=args.output.resolve();output.mkdir(parents=True)
flags=pin['native_flags_as_run'].copy();profiles=[]
for row in pin['profiles']:
 path=here/row['path'];assert sha(path)==row['sha256'];profiles.append(row['symbol']+'@'+str(path))
flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(profiles);results=[]
for name,mode in [('normal','1'),('checked','checked')]:
 folder=output/name;folder.mkdir();front=folder/'frontend';command=[str(compiler),str(source),str(front),'compact'];env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':mode}
 with (folder/'compile.log').open('w') as log:subprocess.run(command,env=env,stdout=log,stderr=log,check=True)
 artifacts={name:sha(front/name) for name in pin['artifact_sha256']};assert artifacts==pin['artifact_sha256'],artifacts;results.append({'mode':name,'compiler_sha256':sha(compiler),'command':command,'artifacts':artifacts})
(output/'source-reproduction.json').write_text(json.dumps({'source_sha256':sha(source),'variants':results,'scope':'Source artifact replay only; runtime and complete-build qualification are separate.'},indent=2)+'\n');print('Normal and allocation-checked source builds reproduce object, full metadata and canonical bytecode.')
