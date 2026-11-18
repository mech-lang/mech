"""Rebind the new parser's exact native output to the historical timed worker."""
from hashlib import file_digest
import json
from pathlib import Path
import subprocess

study = Path(__file__).resolve().parent
q = study.parents[1]
native = study / "candidate/rebuild/native"
pinned = q / "refused-ssa-maintained/worker-checks/refused-ssa"
runtime = json.loads((pinned / "runtime.json").read_bytes())


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


for name in ("program.o", "metadata.json"):
    assert sha(native / name) == sha(pinned / name)
link = runtime["link"]
old_object, = [arg for arg in link if arg.startswith("link-arg=") and arg.endswith("/program.o")]
old_worker = link[link.index("-o") + 1]
worker = study / "mech-worker"
link = [f"link-arg={native / 'program.o'}" if arg == old_object else str(worker) if arg == old_worker else arg
        for arg in link]
with (study / "link-worker.log").open("wb") as log:
    subprocess.run(link, stdout=log, stderr=subprocess.STDOUT, check=True)
assert sha(worker) == sha(pinned / "mech-worker") == "674d88602946ae07bbb00ccb56a60e2a5353f73b5e2d3b5db9a2b6b98ac237c9"
verify = ["python3", "-B", q / "verify-worker.py", worker, native / "metadata.json", "definition-replay-reference"]
with (study / "worker-checks.log").open("wb") as log:
    subprocess.run(verify, stdout=log, stderr=subprocess.STDOUT, check=True)
assert "162 exact linked calls" in (study / "worker-checks.log").read_text()
(study / "worker.json").write_text(json.dumps({
    "link": link, "verify": [str(argument) for argument in verify],
    "worker_sha256": sha(worker), "output_sha256": {name: sha(native / name) for name in ("program.o", "metadata.json")},
    "exact_linked_calls": 162,
}, indent=2) + "\n")
