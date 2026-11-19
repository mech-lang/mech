"""Emit and verify the experimental checked phase from a fresh source control."""
from pathlib import Path
from hashlib import sha256
import argparse,json,os,subprocess

here=Path(__file__).resolve().parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('control',type=Path)
parser.add_argument('output',type=Path)
args=parser.parse_args()
base=json.loads((args.control/'rebuild.json').read_text())
pin=json.loads((here/'codebook-core-private-helpers/receipt.json').read_text())
assert base['full_integrated_source_identity'] and base['object_text_bytes']==63804
sha=lambda path:sha256(Path(path).read_bytes()).hexdigest()
original=base['phases']['source_to_native']['command']
assert sha(original[0])==base['compiler_sha256']
output=args.output.resolve();output.mkdir(parents=True,exist_ok=False)
flags=base['native_flags']|{'MECH_COMPACT_CHECKED_PHASE_WINDOWS':pin['window']}
command=[original[0],original[1],str(output/'frontend'),'compact']
with (output/'compile.log').open('w') as log:
 subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
actual={name:sha(output/'frontend'/name) for name in ['program.o','metadata.json','program.mecb']}
expected=dict(zip(actual,[pin['object_sha256'],pin['metadata_sha256'],pin['bytecode_sha256']]))
record={'command':command,'flags':flags,'compiler_sha256':base['compiler_sha256'],'source_commit':base['source_commit'],'artifacts':actual,'matches_experiment':actual==expected,'qualification':'experimental; runtime wins not measured by this command'}
(output/'replay.json').write_text(json.dumps(record,indent=2)+'\n')
assert actual==expected,record
print('Experimental object, full metadata and canonical bytecode reproduced. No runtime-win claim.')
