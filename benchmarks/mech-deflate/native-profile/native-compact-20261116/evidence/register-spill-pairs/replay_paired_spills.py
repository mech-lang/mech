"""Regenerate the paired-spill encoder from a locked smaller-recipe rebuild."""
import argparse
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess

def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


here = Path(__file__).resolve().parent
arguments = argparse.ArgumentParser(description=__doc__)
arguments.add_argument("baseline_rebuild", type=Path)
arguments.add_argument("output", type=Path)
args = arguments.parse_args()
baseline = args.baseline_rebuild.resolve(strict=True)
output = args.output.resolve()
receipt = json.loads((baseline / "rebuild.json").read_text())
measured = json.loads((here / "paired/receipt.json").read_text())
bytecode_pin = json.loads((here / "paired/runtime.json").read_text())["image_sha256"]
flags = dict(receipt["native_flags"])
assert receipt["variant"] == "fresh-owner-counts-cold-tails"
assert receipt["full_integrated_source_identity"]
symbols = [item.split("@", 1)[0] for item in flags["MECH_COMPACT_PHASE_BLOCK_LAYOUT"].split(";")]
profiles = [here / f"{name}-paired-block-layout.json" for name in symbols]
for profile in profiles:
    assert profile.is_file()
flags["MECH_COMPACT_PHASE_BLOCK_LAYOUT"] = ";".join(
    f"{symbol}@{profile}" for symbol, profile in zip(symbols, profiles, strict=True)
)
flags["MECH_COMPACT_PAIR_REGISTER_SPILLS"] = "1"
assert not any("LLVM" in name for name in flags)
emitter = Path(receipt["phases"]["native"]["command"][0]).resolve(strict=True)
bytecode = baseline / "frontend/program.mecb"
assert sha(bytecode) == bytecode_pin
assert sha(baseline / "native/program.o") == receipt["expected_object_sha256"]
assert sha(baseline / "native/metadata.json") == receipt["expected_metadata_sha256"]
environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
environment.update(flags, PATH="/usr/bin:/bin")
output.mkdir()
command = list(map(str, (emitter, bytecode, output)))
with (output / "emit.log").open("wb") as log:
    subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)

identities = {"object_sha256": sha(output / "program.o"),
              "metadata_sha256": sha(output / "metadata.json"),
              "bytecode_sha256": sha(bytecode)}
assert identities["object_sha256"] == measured["object_sha256"]
assert identities["metadata_sha256"] == measured["metadata_sha256"]
result = {"command": command, "native_flags": flags, **identities}
(output / "identity.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(identities, indent=2))
