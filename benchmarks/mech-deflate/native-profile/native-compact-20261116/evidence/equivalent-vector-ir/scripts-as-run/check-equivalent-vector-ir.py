"""Check vector reuse, serial fallback, and phase disassembly behavior."""

from hashlib import file_digest
import json
from pathlib import Path
import subprocess
import sys

q = Path(__file__).resolve().parent
study = q / "equivalent-vector-phases"
checks = study / "secondary-checks"
emit = q / "host-opt-levels/opt3-cargo/release/emit_compact_size"
root = q / "cranelift-continuation-registers-word-pages-native/final-candidate"
image = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mecb"
flag = "MECH_COMPACT_REUSE_EQUIVALENT_VECTOR_IR=1"
identity_files = ("program.o", "metadata.json")


def sha(path):
    with path.open("rb") as file:
        return file_digest(file, "sha256").hexdigest()


def count(folder):
    return {json.loads(line.partition(": ")[2]) for line in (folder / "emit.stderr").read_text().splitlines()
            if line.startswith("compact vector code reused:")}


def probe(name, *options):
    output = checks / name
    command = [sys.executable, "-B", q / "probe-literal-reuse.py", "--emit-only",
               "--native-reference", root, "--emitter", emit, output, image, *options]
    subprocess.run(command, check=True)
    return output, list(map(str, command))


emitted = study / "retained-reuse"
expected = count(emitted)
assert expected
metadata = json.loads((root / "metadata.json").read_text())
inactive = {row["vector_variant"]["symbol"] for row in metadata["numeric_phases"] if "vector_variant" in row
            and not row["vector_variant"]["selected_by_guard"]}
assert expected <= inactive

symbol = min(expected)
checks.mkdir()
vcode = checks / f"{symbol}.asm"
assembly, command = probe("diagnostic-vcode", flag, f"MECH_INSPECT_PHASE_VCODE={vcode}")
assert count(assembly) == expected - {symbol} and vcode.stat().st_size > 0
assert all((assembly / name).read_bytes() == (root / name).read_bytes() for name in identity_files)

worker, worker_command = probe("one-worker", "MECH_COMPACT_COMPILE_THREADS=1", flag, "MECH_PROFILE_COMPILE_JOBS=1")
assert count(worker) == expected
assert all((worker / name).read_bytes() == (root / name).read_bytes() for name in identity_files)
assert "workers=1" in (worker / "emit.stderr").read_text()

serial, serial_command = probe("serial-control", "MECH_COMPACT_COMPILE_THREADS=0")
serial_reuse, reuse_command = probe("serial-reuse", "MECH_COMPACT_COMPILE_THREADS=0", flag)
assert not count(serial) and not count(serial_reuse)
assert all((serial / name).read_bytes() == (serial_reuse / name).read_bytes() for name in identity_files)
record = {
    "always_preserved_identities": {name: sha(root / name) for name in identity_files},
    "the_historical_machine_code_comparison_discarded": sorted(inactive),
    "matched_prepared_ir": sorted(expected),
    "compiled_and_discarded_after_comparison": sorted(inactive - expected),
    "disassembly_remains_available": {
        "symbol": symbol, "command": command, "assembly_bytes": vcode.stat().st_size,
        "assembly_sha256": sha(vcode), "other_reused_bodies": sorted(count(assembly)),
    },
    "one_worker": {"command": worker_command, "identical_to_historical_encoder": True,
                   "reused_bodies": sorted(count(worker))},
    "serial_fallback": {"commands": [serial_command, reuse_command],
                        "identical_between_arms": {name: sha(serial / name) for name in identity_files}},
}
(study / "diagnostic-checks.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(record, indent=2))
