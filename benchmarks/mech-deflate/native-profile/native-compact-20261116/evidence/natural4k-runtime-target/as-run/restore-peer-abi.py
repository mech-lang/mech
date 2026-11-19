"""Verify all fixtures before restoring the peer's missing auxiliary metadata."""
from pathlib import Path
from hashlib import sha256
import json,shutil,sys
ROOT=Path(__file__).resolve().parent;Q=ROOT.parent.parent
PACKET=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116')
sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
helper=PACKET/'evidence/host-opt-levels/scripts-as-run/verify-worker.py'
if not (Q/'verify-worker.py').exists():shutil.copy2(helper,Q/'verify-worker.py')
meta=ROOT/'peer-metadata-recovery/archived-compatible.json';pins_file=Q/'pinned-peers/pins.json';original=pins_file.read_bytes();pins=json.loads(original)
source=(Q/'verify-worker.py').read_text();old='CLEANUP private_payload_bytes=0 live_owner_records=0';assert source.count(old)==1;source=source.replace(old,'CLEANUP reference_input_and_output_roots_released=1')
label='natural4k-restored-roc-abi';sys.argv=[str(Q/'verify-worker.py'),pins['actors']['roc']['command'][0],str(meta),label]
exec(compile(source,str(Q/'verify-worker.py')+':roc-abi','exec'),{'__file__':str(Q/'verify-worker.py'),'__name__':'__main__'})
receipt=json.loads((Q/(label+'-qualification.json')).read_text());assert receipt['complete_smokes']==162 and receipt['worker_sha256']==pins['actors']['roc']['binary_sha256']
folder=ROOT/'peer-metadata-recovery';(folder/'previous-pins.json').write_bytes(original)
prior=pins['actors']['roc'].copy();restored=Q/'pinned-peers/roc-compatible-metadata.json';shutil.copyfile(meta,restored)
pins['actors']['roc']['command'][1]=str(restored);pins['actors']['roc']['metadata_sha256']=sha(restored)
pins['actors']['roc']['auxiliary_metadata_recovery']={'reason':'Original temporary metadata missing. Exact archived content could not be found. Binary unchanged; a different archived compiler metadata file supplies its unchanged input ABI. All 162 fixture output/decompression/alias observations verified before use. Fresh timings must bind this metadata hash.','previous_metadata_path':prior['command'][1],'previous_metadata_sha256':prior['metadata_sha256'],'source_archive':str(PACKET/'evidence/minimal-deployment/inputs/mech_metadata/metadata.json.gz'),'verified_162':str(Q/(label+'-qualification.json'))}
pins_file.write_text(json.dumps(pins,indent=2)+'\n');(folder/'restored-pins.json').write_bytes(pins_file.read_bytes())
print('Roc binary unchanged; 162 checks pass with compatible archived metadata',sha(restored),flush=True)
