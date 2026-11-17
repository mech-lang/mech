"""Archive every planned timing run and preserve its original observations."""

from hashlib import file_digest
import json
from pathlib import Path
import shutil

study = Path(__file__).resolve().parent
q = study.parent
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
archive = packet / "evidence/aligned-register-spills"
archive.mkdir(exist_ok=True)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def ref(path):
    return {"path": str(path.relative_to(packet)), "sha256": digest(path)}


for name in ("contract.json", "environment.json", "plan.json", "plan.sha256", "residue-64-cases.json", "replay_runtime.py"):
    shutil.copyfile(study / name, archive / name)
shutil.copyfile(q / "fixtures.json", archive / "fixtures-as-run.json")
plan = json.loads((study / "plan.json").read_text())
absolute_fixtures = json.loads((q / "fixtures.json").read_text())
frozen_fixtures = json.loads((packet / "fixtures.json").read_text())
start = (study / "plan.sha256").stat().st_mtime_ns
verified = {}
for actual, frozen in zip(absolute_fixtures, frozen_fixtures, strict=True):
    name = actual["name"]
    assert all(actual[field] == frozen[field] for field in (
        "name", "input_sha256", "expected_sha256", "input_bytes", "prefix_bytes", "raw_bytes", "capacity"
    )), name
    verified[name] = {}
    for field in ("input", "prefix", "expected"):
        file = Path(actual[field])
        assert file.stat().st_mtime_ns <= start, file
        sha = digest(file)
        assert sha == digest(packet / frozen[field]), (name, field)
        verified[name][field] = sha
assert len(verified) == 81
fixture_as_run = ref(archive / "fixtures-as-run.json")
frozen_sha = digest(packet / "fixtures.json")
inventory_proof = archive / "fixture-content-bindings.json"
inventory_proof.write_text(json.dumps({
    "timed_inventory_sha256": fixture_as_run["sha256"],
    "frozen_inventory_sha256": frozen_sha,
    "case_file_sha256": verified,
}, indent=2) + "\n")
runs = {}
metadata = {}
actors = tuple(plan["worker_sha256"])
for run in plan["runs"]:
    source = q / run["id"]
    dest = archive / "runs" / run["id"]
    for base in ("summary.json", "complete.json", "pins.json", *(f"{a}.stderr" for a in actors)):
        dest.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source / base, dest / base)
    for file in source.glob("*/observations.json.gz"):
        target = dest / file.parent.name / file.name
        target.parent.mkdir(exist_ok=True)
        shutil.copyfile(file, target)
    pins = json.loads((dest / "pins.json").read_text())
    assert pins["input_inventory_sha256"] == fixture_as_run["sha256"]
    # Keep the acquisition pins intact. Replay uses this separate receipt
    # with the content-verified canonical paths substituted.
    canonical = dest / "pins-canonical.json"
    canonical.write_text(json.dumps(pins | {"input_inventory_sha256": frozen_sha}, indent=2) + "\n")
    for actor in actors:
        worker = pins["actors"][actor]
        assert worker["binary_sha256"] == plan["worker_sha256"][actor]
        if "metadata_sha256" in worker:
            assert metadata.setdefault(actor, worker["metadata_sha256"]) == worker["metadata_sha256"]
    runs[run["id"]] = {
        "id": run["id"],
        "kind": run["kind"],
        "order": run["order"],
        "pins": ref(canonical),
        "original_pins": ref(dest / "pins.json"),
        "summary": ref(dest / "summary.json"),
        "complete": ref(dest / "complete.json"),
        "observations": {p.parent.name: ref(p) for p in sorted(dest.glob("*/observations.json.gz"))},
        "cleanup": {actor: ref(dest / f"{actor}.stderr") for actor in actors},
    }

repeat = study / "repeatability"
duplicates = []
if repeat.is_dir():
    shutil.copytree(repeat, archive / "repeatability", dirs_exist_ok=True)
    repeat = archive / "repeatability"
    duplicates.append({"id": "distinct-file-duplicate-control",
                       "study": ref(repeat / "study.json"),
                       "results": ref(repeat / "results.json"),
                       "observations": {name.removesuffix(".json.gz"): ref(repeat / name)
                                        for name in sorted(p.name for p in repeat.glob("*.json.gz"))}})
campaign = {"runs": runs, "metadata_sha256": metadata, "duplicate_controls": duplicates,
            "fixture_inventory_as_run": fixture_as_run, "fixture_content_proof": ref(inventory_proof)}
(archive / "runtime-campaign.json").write_text(json.dumps(campaign, indent=2) + "\n")
