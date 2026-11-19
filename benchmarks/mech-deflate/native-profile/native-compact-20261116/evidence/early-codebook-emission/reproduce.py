"""Regenerate all three recorded artifacts from live compiler source."""
from argparse import ArgumentParser
from hashlib import sha256
from pathlib import Path
import json,os,struct,subprocess
HERE=Path(__file__).resolve().parent
PACKET=HERE.parents[1]
read=lambda p:json.loads(p.read_text())
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
parser=ArgumentParser(description=__doc__)
parser.add_argument('--compiler',required=True,type=Path)
parser.add_argument('--output',required=True,type=Path)
parser.add_argument('--case',choices=['selected-control','matched-unsplit','early-codebook'],action='append')
args=parser.parse_args();compiler=args.compiler.resolve(strict=True)
recipe=read(HERE/'emission-recipes.json');source=PACKET/'program.mec'
assert sha(source)==recipe['source_sha256']
manifest=PACKET/recipe['profile_manifest'];profiles=[]
for profile in read(manifest)['profiles']:
 path=(manifest.parent/profile['path']).resolve(strict=True)
 assert sha(path)==profile['sha256']
 profiles.append(profile['symbol']+'@'+str(path))
root=args.output.resolve();root.mkdir(parents=True,exist_ok=False)
results={}
for label in args.case or recipe['cases']:
 case=recipe['cases'][label];folder=root/label;folder.mkdir()
 flags=case['flags']|{'MECH_COMPACT_PHASE_BLOCK_LAYOUT':';'.join(profiles)}
 command=[str(compiler),str(source),str(folder/'frontend'),'compact']
 with (folder/'compile.log').open('w') as log:
  subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
 artifacts={name:sha(folder/'frontend'/name) for name in case['artifacts']}
 assert artifacts==case['artifacts'],(label,artifacts,case['artifacts'])
 data=(folder/'frontend/program.o').read_bytes();assert struct.unpack_from('<I',data)[0]==0xfeedfacf
 at=32;text=None
 for _ in range(struct.unpack_from('<I',data,16)[0]):
  tag,size=struct.unpack_from('<II',data,at)
  if tag==0x19:
   for i in range(struct.unpack_from('<I',data,at+64)[0]):
    section=at+72+80*i
    if data[section:section+16].rstrip(b'\0')==b'__text':text=struct.unpack_from('<Q',data,section+40)[0]
  at+=size
 assert text==case['text_bytes']
 results[label]={'compiler_sha256':sha(compiler),'command':command,'flags':flags,'artifacts':artifacts,'encoder_text_bytes':text,'runtime_qualification':'not performed'}
 print(label,text,'B: exact artifacts reproduced',flush=True)
(root/'result.json').write_text(json.dumps(results,indent=2)+'\n')
