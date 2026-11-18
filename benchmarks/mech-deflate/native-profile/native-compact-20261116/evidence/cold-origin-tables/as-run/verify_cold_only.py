"""Make each packed decoder trap to check the successful benchmark route."""

from importlib import import_module
import json
import os
from pathlib import Path
from struct import pack_into, unpack_from
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q))
macho = import_module("reachability-census")
candidate = q / "cold-origin-packed-native/final-candidate"
output = study / "cold-entry-traps"
output.mkdir()
program = output / "program.o"
worker = output / "mech-worker"
data, sections, _ = macho.load(candidate / "program.o")
_, address, extent, offset, *_ = sections[macho.TEXT]
object_bytes = bytearray(data)
proof = json.loads((study / "link-proof.json").read_text())
decoders = sorted({row["decoder"] for row in proof["cold_origin_map"]})
assert len(decoders) == 3
for pc in decoders:
    assert address <= pc and pc + 4 <= address + extent
    at = offset + pc - address
    assert unpack_from("<I", object_bytes, at)[0] & 0x9f00_000f == 0x1000_0001
    pack_into("<I", object_bytes, at, 0xd420_0000)
program.write_bytes(object_bytes)
runtime = json.loads((candidate / "runtime.json").read_text())
link = [arg.replace(str(candidate / "program.o"), str(program))
        .replace(str(candidate / "mech-worker"), str(worker)) for arg in runtime["link"]]
environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
subprocess.run(link, cwd=q, env=environment, check=True)
subprocess.run([sys.executable, "-B", q / "verify-worker.py", worker,
                candidate / "metadata.json", "cold-origin-decoders-trapped"],
               cwd=q, env=environment, check=True)
observed = json.loads((q / "cold-origin-decoders-trapped-qualification.json").read_text())
assert observed["complete_smokes"] == 162 and observed["verified"]
(study / "cold-entry-proof.json").write_text(json.dumps({
    "scope": "Successful benchmark calls with a trap at each of the three decoder entries.",
    "decoder_offsets": decoders, "linked_smokes": observed,
}, indent=2) + "\n")
