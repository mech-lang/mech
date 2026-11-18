"""Regenerate the qualified smaller object using a fresh control rebuild."""
from hashlib import sha256
from pathlib import Path
import argparse,json,os,subprocess

here=Path(__file__).resolve().parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('control',type=Path,help='output of rebuild.py --variant m1-literal-frame-cleanup')
parser.add_argument('output',type=Path,help='new candidate output directory')
args=parser.parse_args()
base=json.loads((args.control/'rebuild.json').read_text())
pin=json.loads((here/'candidate.json').read_text())
assert base['full_integrated_source_identity'] and base['object_text_bytes']==64444
sha=lambda path:sha256(Path(path).read_bytes()).hexdigest()
original=base['phases']['source_to_native']['command']
assert sha(original[0])==base['compiler_sha256']
out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
flags=base['native_flags']|{'MECH_COMPACT_PREFER_PRESERVED_REGS':'1'}
command=[original[0],original[1],str(out/'frontend'),'compact']
env={key:value for key,value in os.environ.items() if not key.startswith('MECH_')}|flags
with (out/'compile.log').open('w') as log:subprocess.run(command,env=env,stdout=log,stderr=log,check=True)
artifacts={name:sha(out/'frontend'/name) for name in ('program.o','metadata.json','program.mecb')}
expected={'program.o':pin['artifact_sha256']['program.o'],'metadata.json':pin['artifact_sha256']['metadata.json'],'program.mecb':pin['artifact_sha256']['program.mecb']}
record={'command':command,'flags':flags,'control_source_commit':base['source_commit'],'compiler_sha256':sha(original[0]),'artifacts':artifacts,'matches_archived_candidate':artifacts==expected,'runtime_wins':'not measured by this command; qualification is recorded separately'}
(out/'replay.json').write_text(json.dumps(record,indent=2)+'\n')
assert artifacts==expected,record
print('Reproduced the 63,804 B candidate object, complete metadata and canonical bytecode.')
