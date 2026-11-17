#!/bin/zsh
set -euo pipefail
q=${0:A:h:h}
study=${0:A:h}
emitter="$q/host-opt-levels/opt3-cargo/release/emit_compact_size"
image="$q/stale-cache-accounting/integrated-rebuild/frontend/program.mecb"
pinned="$q/stale-cache-accounting/pinned-smaller"
aligned="$q/aligned-register-spills-native/final-candidate"
profile_layout="$(cat "$study/phase-layout-profiles.txt")"

probe() {
  local reference="$1"
  shift
  python3 -B "$q/probe-literal-reuse.py" --native-reference "$reference" --emitter "$emitter" "$@"
}

probe "$pinned" "$aligned" "$image" MECH_COMPACT_PAIR_REGISTER_SPILLS=1 \
  "MECH_COMPACT_PHASE_BLOCK_LAYOUT=$profile_layout"
for file in program.o metadata.json mech-worker; do
  cmp "$aligned/$file" "$q/spill-pairs-residue-64-native/final-candidate/$file"
done
probe "$pinned" "$study/disabled" "$image"
for file in program.o metadata.json mech-worker; do
  cmp "$study/disabled/$file" "$pinned/$file"
done
def="$q/stale-cache-accounting/pinned-retained"
probe "$def" "$study/retained" "$image"
for file in program.o metadata.json mech-worker; do
  cmp "$study/retained/$file" "$def/$file"
done
probe "$pinned" "$study/checked" "$image" MECH_COMPACT_PAIR_REGISTER_SPILLS=checked \
  "MECH_COMPACT_PHASE_BLOCK_LAYOUT=$profile_layout" MECH_COMPACT_INLINE_SINGLE_CALLS=preselected
for file in program.o metadata.json mech-worker; do
  cmp "$study/checked/$file" "$aligned/$file"
done
python3 - "$aligned" "$q/spill-pairs-residue-64-native/final-candidate" "$study" <<'PY'
from hashlib import file_digest
import json
from pathlib import Path
import sys

aligned, offline, study = map(Path, sys.argv[1:])

def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()

checks = {"candidate": aligned, "placement_diagnostic": offline}
checks.update({label: study / label for label in ('disabled', 'retained', 'checked')})
identities = {label: {f: sha(path/f) for f in ('program.o','metadata.json','mech-worker')}
              for label,path in checks.items()}
assert identities['candidate'] == identities['placement_diagnostic'] == identities['checked']
assert identities['disabled']['mech-worker'] == '439a28b3b6ea9aec3254735de80b1c214e0d52a56f3d174fce6dd48681761b41'
assert identities['retained']['mech-worker'] == '06dfffa92a21f59bf3fe4ef509a75ce6ae5844a79d52507592ac9e7386ce57e6'
for label,path in checks.items():
    r=json.loads((path/'receipt.json').read_text())
    r['libdeflate_object_text_bytes'] = 37080
    r['object_text_x_libdeflate'] = r['object_text_bytes']/37080
    (study/f'{label}-receipt.json').write_text(json.dumps(r,indent=2)+'\n')
(study/'preflight-identities.json').write_text(json.dumps(identities,indent=2)+'\n')
print(json.dumps(identities,indent=2),flush=True)
PY
