"""Reproduce normal and allocation-checked clamped input-window artifacts from Mech source."""
from pathlib import Path
from hashlib import sha256
import argparse,json,os,subprocess
here=Path(__file__).resolve().parent;packet=here.parents[2];parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('compiler',type=Path);parser.add_argument('output',type=Path);parser.add_argument('--role',default='window-clean-probe-profiled');args=parser.parse_args();result=json.loads((here/'result.json').read_text());pin=result['roles'][args.role];compiler=args.compiler.resolve(strict=True);output=args.output.resolve();output.mkdir(parents=True);source=packet/'program.mec';digest=lambda p:sha256(Path(p).read_bytes()).hexdigest();assert digest(source)==result['source_sha256'];flags=pin['flags_as_run'].copy();profiles=[]
for row in pin['profiles']:
 path=here/row['path'];assert digest(path)==row['sha256'];profiles.append(row['symbol']+'@'+str(path))
flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(profiles);rows=[]
for name,mode in [('normal','1'),('checked','checked')]:
 out=output/name;out.mkdir();command=[str(compiler),str(source),str(out/'frontend'),'compact']
 with (out/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':mode},stdout=log,stderr=log,check=True)
 artifacts={name:digest(out/'frontend'/name) for name in pin['artifact_sha256']};assert artifacts==pin['artifact_sha256'];rows.append({'mode':name,'command':command,'compiler_sha256':digest(compiler),'artifacts':artifacts})
(output/'source-reproduction.json').write_text(json.dumps({'role':args.role,'source_sha256':digest(source),'variants':rows,'scope':'Exact source-artifact replay; runtime and complete-build measurements are separate.'},indent=2)+'\n');print(args.role,'normal and checked object,full metadata and canonical bytecode reproduced.')
