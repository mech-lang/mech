"""Link the fixed workers using code emitted by the new native compiler."""

from hashlib import file_digest
import json
from pathlib import Path
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
emit = q / "host-opt-levels/opt3-cargo/release/emit_compact_size"
image = q / "parser-queue-snapshot-20261117/identity/frontend-combined/program.mecb"
output = study / "native-work-identities"
output.mkdir()
references = {
    "retained": q / "stale-cache-accounting/pinned-retained",
    "smaller": q / "stale-cache-accounting/pinned-smaller",
    "aligned": q / "aligned-register-spills-native/final-candidate",
}


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream,"sha256").hexdigest()


pins = {}
for role, reference in references.items():
    dest = output / role
    argv = [sys.executable, "-B", q / "probe-literal-reuse.py",
            "--native-reference", reference, "--emitter", emit, dest, image,
            "MECH_COMPACT_INLINE_SINGLE_CALLS=preselected"]
    with (output / f"{role}.log").open("wb") as log:
        subprocess.run(argv, check=True, stdout=log, stderr=subprocess.STDOUT)
    identity = {name:sha(dest/name) for name in ("program.o", "metadata.json", "mech-worker")}
    assert identity == {name:sha(reference/name) for name in identity}, (role,identity)
    receipt = json.loads((dest / "receipt.json").read_text())
    assert receipt["verified_calls"] == 162
    pins[role] = {"artifacts":identity,"command":list(map(str,argv)),"verified_calls":receipt["verified_calls"],
                  "object_text_bytes":receipt["object_text_bytes"]}
profile = json.loads((study / "baseline.json").read_text())
assert profile["emitter_sha256"] != sha(emit)
(output / "receipt.json").write_text(json.dumps({
    "emitter_sha256":sha(emit),"bytecode_sha256":sha(image),"variants":pins,
},indent=2)+"\n")
print(json.dumps(pins,indent=2))
