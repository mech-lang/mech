"""Link the frozen Rust worker around exactly the diagnostic scorer object."""

from argparse import ArgumentParser
from hashlib import file_digest
import json
import os
from pathlib import Path
from shutil import copyfile
import subprocess
import sys

q = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")
reference = q / "native-size-20261117/source-aligned"
EXPECTED_WORKER = "674d88602946ae07bbb00ccb56a60e2a5353f73b5e2d3b5db9a2b6b98ac237c9"


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--relink-control", action="store_true")
    args = parser.parse_args()
    candidate = args.candidate.resolve(strict=True)
    proof = json.loads((candidate / "object-proof.json").read_text())
    assert sha(candidate / "program.o") == proof["candidate_object_sha256"]
    assert sha(reference / "program.o") == proof["reference_object_sha256"]
    assert sha(reference / "mech-worker") == EXPECTED_WORKER
    assert not (candidate / "mech-worker").exists()
    receipt = json.loads((reference / "runtime.json").read_text())
    env = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}

    def link(source, output):
        original = receipt["link"]
        object_arg = f"link-arg={reference / 'program.o'}"
        assert original.count(object_arg) == original.count(str(reference / "mech-worker")) == 1
        command = [
            str(output) if arg == str(reference / "mech-worker")
            else f"link-arg={source}" if arg == object_arg
            else arg
            for arg in original
        ]
        subprocess.run(command, check=True, env=env)
        return command

    if args.relink_control:
        control = candidate / "control-link/mech-worker"
        control.parent.mkdir()
        link(reference / "program.o", control)
        assert sha(control) == EXPECTED_WORKER

    copyfile(reference / "metadata.json", candidate / "metadata.json")
    command = link(candidate / "program.o", candidate / "mech-worker")
    hashes = {name: sha(candidate / name) for name in ("program.o", "metadata.json", "mech-worker")}
    (candidate / "runtime.json").write_text(json.dumps(
        receipt | {"link": command, "diagnostic_scorer": proof, "worker_hashes": hashes},
        indent=2,
    ) + "\n")

    # Use the original 162 exact-call checks with all outputs private. Its
    # fixtures and executable operands are the same paths as in the baseline.
    (candidate / "fixtures.json").symlink_to(q / "fixtures.json")
    local_verify = candidate / "verify-worker.py"
    copyfile(q / "verify-worker.py", local_verify)
    subprocess.run([
        sys.executable, "-B", local_verify,
        candidate / "mech-worker", candidate / "metadata.json", "scorer-invariants",
    ], check=True, env=env)
    print(json.dumps({"runtime_link": command, "sha256": hashes}, indent=2))


if __name__ == "__main__":
    main()
