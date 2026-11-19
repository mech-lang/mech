"""Regenerate the recorded source-defined encoder variants with a built compiler."""
from argparse import ArgumentParser
from hashlib import sha256
from pathlib import Path
import json,os,struct,subprocess
HERE=Path(__file__).resolve().parent
PACKET=HERE.parents[1]
read=lambda p:json.loads(p.read_text())
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
parser=ArgumentParser(description=__doc__)
parser.add_argument('--compiler',type=Path,required=True)
parser.add_argument('--output',type=Path,required=True)
parser.add_argument('--case',action='append')
args=parser.parse_args();compiler=args.compiler.resolve(strict=True)
recipe=read(HERE/'emission-recipes.json');source=PACKET/'program.mec'
assert sha(source)==recipe['source_sha256']
output=args.output.resolve();output.mkdir(parents=True,exist_ok=False)
result={}
for label in args.case or recipe['cases']:
 case=recipe['cases'][label];folder=output/label;folder.mkdir();profiles=[]
 for profile in case['profiles']:
  path=(HERE/profile['path']).resolve(strict=True);assert sha(path)==profile['sha256']
  profiles.append(profile['symbol']+'@'+str(path))
 flags=recipe['base_flags']|case['flags']|{'MECH_COMPACT_PHASE_BLOCK_LAYOUT':';'.join(profiles)}
 command=[str(compiler),str(source),str(folder/'frontend'),'compact']
 with (folder/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
 artifacts={n:sha(folder/'frontend'/n) for n in case['artifacts']}
 assert artifacts==case['artifacts'],(label,artifacts,case['artifacts'])
 data=(folder/'frontend/program.o').read_bytes();assert struct.unpack_from('<I',data)[0]==0xfeedfacf
 at=32;text=None
 for _ in range(struct.unpack_from('<I',data,16)[0]):
  kind,size=struct.unpack_from('<II',data,at)
  if kind==0x19:
   for i in range(struct.unpack_from('<I',data,at+64)[0]):
    section=at+72+80*i
    if data[section:section+16].rstrip(b'\0')==b'__text':text=struct.unpack_from('<Q',data,section+40)[0]
  at+=size
 assert text==case['encoder_text_bytes']
 result[label]={'compiler_sha256':sha(compiler),'command':command,'flags':flags,'artifacts':artifacts,'encoder_text_bytes':text,'runtime_qualification':'not performed'}
 print(label,text,'B: exact source-built artifacts reproduced',flush=True)
(output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
