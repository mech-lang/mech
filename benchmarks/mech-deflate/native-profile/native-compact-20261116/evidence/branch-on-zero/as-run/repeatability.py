"""Resolve the cases still open after the two full aligned timing orders."""

from hashlib import file_digest
from importlib import import_module
import json
from pathlib import Path
import shutil
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q))
measure = import_module("measure-cold-epilogues-followup")


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


full_replay = study / "aligned-full-replay.json"
first = study / "aligned-full-initial-replay.json"
assert sys.argv[1:] in ([], ["--bound"])
bound = bool(sys.argv[1:])
prefix = "bound-" if bound else ""
if not bound:
    assert not first.exists()
    shutil.copyfile(full_replay, first)
report = json.loads(first.read_text())
assert not report["rejected"]
cases = [name for name, row in report["cases"].items() if row["decision"] == "unresolved"]
assert cases
inputs = study / (prefix + "repeatability-inputs")
inputs.mkdir()
control = q / "stale-cache-accounting/pinned-smaller"
candidate = q / "branch-on-zero-aligned-native/final-candidate"
duplicate = inputs / "duplicate-control"
duplicate.mkdir()
for name in ("mech-worker", "metadata.json"):
    shutil.copy2(control / name, duplicate / name)
(inputs / "control").symlink_to(control)
(inputs / "fixtures.json").symlink_to(q / "fixtures.json")
(inputs / "full").symlink_to(q / "branch-on-zero-aligned-full")
shutil.copyfile(first, inputs / "cold-uncertainty.json")
plan_path = study / (prefix + "repeatability-plan.json")
plan = {
    "trigger": "Every unresolved per-case bound in the first two full timing orders.",
    "initial_replay_sha256": sha(first),
    "cases": cases,
    "repetitions_per_order": 64, "batch_floor": 32, "seed": 99371768 + int(bound),
    "resolution": measure.RESOLUTION,
    "worker_sha256": {name: sha(folder / "mech-worker") for name, folder in (
        ("control", control), ("duplicate-control", duplicate), ("candidate", candidate),
    )},
}
if bound:
    full_bindings = study / "aligned-full-bindings.json"
    full_sha = (study / "aligned-full-bindings.sha256").read_text().strip()
    assert sha(full_bindings) == full_sha
    plan["full_bindings_sha256"] = full_sha
    verified = json.loads(full_bindings.read_text())
    keys = ("environment_sha256", "contract_sha256")
    binding = {key: verified[key] for key in keys}
    for name in ("environment", "contract"):
        assert sha(study / f"aligned-{name}.json") == binding[f"{name}_sha256"]
    plan.update(binding)
    # Reuse the original acquisition code and add only the receipt bindings.
    origin = Path(measure.__file__)
    code = origin.read_text()
    anchor = "worker_sha256=identities, distinct_worker_files=distinct_files, batch_floor=batch_floor,"
    assert code.count(anchor) == 1
    values = " ".join(f"{key}={value!r}," for key, value in binding.items())
    namespace = {"__file__": str(origin), "__name__": "bound_repeatability"}
    exec(compile(code.replace(anchor, anchor + " " + values), str(origin), "exec"), namespace)
    acquire = namespace["measure"]
else:
    acquire = measure.measure
plan_path.write_text(json.dumps(plan, indent=2) + "\n")
(study / (prefix + "repeatability-plan.sha256")).write_text(sha(plan_path) + "\n")
output = study / (prefix + "repeatability")
acquire(inputs, output, cases, plan["seed"], plan["repetitions_per_order"],
        candidate, duplicate, plan["batch_floor"])
receipt = json.loads((output / "study.json").read_text())
assert receipt["worker_sha256"] == plan["worker_sha256"] and receipt["distinct_worker_files"] == 3
if bound:
    assert all(receipt[key] == plan[key] for key in keys)
