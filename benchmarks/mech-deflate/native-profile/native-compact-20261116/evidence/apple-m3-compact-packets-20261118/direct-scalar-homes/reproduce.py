"""Reproduce the source-built direct-scalar-home encoder."""
from argparse import ArgumentParser
from hashlib import sha256
import json,os
from pathlib import Path
import subprocess
p=ArgumentParser(description=__doc__);p.add_argument('compiler',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
here=Path(__file__).resolve().parent;recipe=json.loads((here/'recipe.json').read_text());source=here.parents[2]/'program.mec'
def digest(p):return sha256(p.read_bytes()).hexdigest()
assert digest(source)==recipe['source_sha256']
flags=recipe['flags'].copy();profiles=[]
for row in recipe['profiles']:
 path=here/row['path'];assert digest(path)==row['sha256'];profiles.append(row['symbol']+'@'+str(path))
flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(profiles)
env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')};env.update(flags)
subprocess.run([str(a.compiler.resolve()),str(source),str(a.output),'compact'],env=env,check=True)
for name,pin in recipe['expected'].items():assert digest(a.output/name)==pin,name
print('Reproduced the 67,648-byte encoder object, metadata and bytecode.')
