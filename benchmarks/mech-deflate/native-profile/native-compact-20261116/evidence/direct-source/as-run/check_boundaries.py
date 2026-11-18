"""Exercise the benchmark driver's other modes and check the aligned worker identity."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
output = study / "boundary-checks"
output.mkdir()
target = q / "host-opt-levels/opt3-cargo/release"
fixture = q / "native-review-pr/checkout/tests/fixtures/syntax-source-boundary/executable.mec"
invalid = output / "invalid.mec"
invalid.write_text("broken := (1 +\n")
env = {k: v for k, v in os.environ.items() if not k.startswith("MECH_")}


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def run(name, argv, succeeds=True):
    log_path = output / (name + ".log")
    with log_path.open("wb") as log:
        result = subprocess.run(argv, env=env, stdout=log, stderr=subprocess.STDOUT)
    assert (result.returncode == 0) == succeeds, (name, result.returncode, log_path.read_text()[-1400:])
    return {"command": list(map(str, argv)), "code": result.returncode, "log_sha256": sha(log_path)}


records = {
    "managed": run("managed", [target / "compile_profile", fixture, output / "managed", "managed"]),
    "frontend": run("frontend", [target / "compile_profile", fixture, output / "frontend", "frontend"]),
    "split_fixture_frontend": run("split_fixture_frontend", [
        target / "native_compact_frontend", fixture, output / "split_fixture_frontend", "frontend",
    ]),
    "invalid_direct": run("invalid_direct", [
        target / "compile_profile", invalid, output / "invalid_direct", "compact",
    ], False),
    "invalid_split": run("invalid_split", [
        target / "native_compact_frontend", invalid, output / "invalid_split", "frontend",
    ], False),
}
fixture_bytecode = sha(output / "managed/program.mecb")
for folder in ("frontend", "split_fixture_frontend"):
    assert sha(output / folder / "program.mecb") == fixture_bytecode
for folder in ("invalid_direct", "invalid_split"):
    assert not (output / folder / "program.mecb").exists()
assert (output / "managed/program.o").stat().st_size

direct_program = study / "variant-fresh-owner-counts-cold-tails-paired-spills/native"
reference = q / "aligned-register-spills-native/final-candidate"
runtime = json.loads((reference / "runtime.json").read_text())
# Keep the executable basename because the Mach-O ad hoc signature uses it.
worker = output / "mech-worker"
link = runtime["link"]
old_output = link[link.index("-o") + 1]
link = [
    str(worker) if arg == old_output else f"link-arg={direct_program / 'program.o'}" if arg.endswith("/program.o") else arg
    for arg in link
]
records["aligned_link"] = run("aligned_link", link)
assert sha(worker) == sha(reference / "mech-worker"), sha(worker)
assert sha(direct_program / "metadata.json") == sha(reference / "metadata.json")
records["aligned_correctness"] = run("aligned_correctness", [
    sys.executable, "-B", q / "verify-worker.py", worker, direct_program / "metadata.json", "direct-source-historical-aligned",
])
records["aligned_correctness"].update(json.loads((q / "direct-source-historical-aligned-qualification.json").read_text()))
records["fixture_bytecode_sha256"] = fixture_bytecode
(output / "receipt.json").write_text(json.dumps(records, indent=2) + "\n")
print(json.dumps({name: row["code"] for name, row in records.items() if isinstance(row, dict)}, indent=2))
