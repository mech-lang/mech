"""Compare the old and new verifiers on valid and stale control-flow data."""

from hashlib import file_digest
import json
from pathlib import Path
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
artifacts = [json.loads(line) for line in (study / "cargo.jsonl").read_text().splitlines()
             if line.startswith('{"reason":"compiler-artifact"')]
actual, = (row for row in artifacts if row["target"]["name"] == "cranelift_codegen")
current, = (Path(name) for name in actual["filenames"] if name.endswith(".rlib"))
baseline = study / "verifier-control/libcranelift_codegen.rlib"
deps = q / "host-opt-levels/opt3-cargo/release/deps"
output = study / "cfg-verifier"
output.mkdir()
observations = {}


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


for role, library in (("control", baseline), ("work-reuse", current)):
    exe = output / role
    command = ["rustc", "+nightly-2026-03-03", "--edition=2024", "--crate-name", "cfg_verifier_probe",
               "-D", "warnings", "-L", f"dependency={deps}", "--extern", f"cranelift_codegen={library}",
               study / "cfg-verifier-probe.rs", "-o", exe]
    with (output / f"{role}-build.log").open("wb") as stream:
        subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, check=True)
    log = output / f"{role}.txt"
    with log.open("wb") as stream:
        subprocess.run([exe], stdout=stream, stderr=subprocess.STDOUT, check=True)
    observations[role] = {"build":list(map(str, command)),"library_sha256":sha(library),
                          "probe_sha256":sha(exe),"output_sha256":sha(log)}
assert observations["control"]["library_sha256"] != observations["work-reuse"]["library_sha256"]
assert (output / "control.txt").read_bytes() == (output / "work-reuse.txt").read_bytes()
(output / "receipt.json").write_text(json.dumps(observations, indent=2) + "\n")
print("Matched the valid CFG, stale successor, stale predecessor and instruction type diagnostics")
