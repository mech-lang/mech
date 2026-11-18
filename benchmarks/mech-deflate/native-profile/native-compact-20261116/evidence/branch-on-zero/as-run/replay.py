"""Replay acquired orders and validate every timed call with the v1 policy."""

import json
from pathlib import Path
import sys

study = Path(__file__).resolve().parent
q = study.parent
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
sys.path.insert(0, str(packet))
from qualification_records import ACTORS, Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, duplicate_study, runtime_run

assert sys.argv[1:] in ([], ["--aligned"])
prefix = "aligned-" if sys.argv[1:] else ""
plan_file = study / (prefix + "timing-plan.json")
plan_sha = (study / (prefix + "timing-plan.sha256")).read_text().strip()
assert digest(plan_file) == plan_sha
plan = read_json(plan_file)
policy_path = packet / "qualification-policy-v1.json"
assert digest(policy_path) == plan["policy_sha256"]
policy = read_json(policy_path)
findings = Findings()
fixtures = load_fixtures(Evidence(packet, findings), policy)
evidence = Evidence(q, findings)


def ref(path):
    return {"path": str(path.relative_to(q)), "sha256": digest(path)}


inventory = ref(q / "fixtures.json")
assert inventory["sha256"] == plan["fixture_inventory_sha256"]
as_run = read_json(q / "fixtures.json")
assert [row["name"] for row in as_run] == list(fixtures)
# The acquisition ran against absolute paths. Verify each original file
# before connecting those observations to the frozen content-addressed paths.
fields = ("input_sha256", "expected_sha256", "input_bytes", "prefix_bytes", "raw_bytes", "capacity")
content = {}
for row in as_run:
    name = row["name"]
    frozen = fixtures[name]
    assert all(row[key] == frozen[key] for key in fields), name
    content[name] = {}
    for field in ("input", "prefix", "expected"):
        actual = Path(row[field])
        assert actual.stat().st_mtime_ns <= plan_file.stat().st_mtime_ns, (name, actual)
        sha = digest(actual)
        assert sha == digest(packet / frozen[field]), (name, field)
        content[name][field] = sha
(study / (prefix + "fixture-content.json")).write_text(json.dumps({
    "as_run": inventory, "frozen_sha256": policy["fixture_inventory_sha256"], "files": content,
}, indent=2) + "\n")
normalized = study / "canonical-pins"
normalized.mkdir(exist_ok=True)
identities = {
    actor: {"worker": binding.get("mech-worker", binding.get("binary_sha256")),
            "metadata": binding.get("metadata.json")}
    for actor, binding in plan["actors"].items()
}
identities.update(environment_sha256=None, contract_sha256=None)
bindings_file = study / "aligned-full-bindings.json"
if prefix and bindings_file.is_file():
    bindings_sha = (study / "aligned-full-bindings.sha256").read_text().strip()
    assert digest(bindings_file) == bindings_sha
    bindings = read_json(bindings_file)
    assert bindings["original_plan_sha256"] == plan_sha
    assert bindings["runs"] == [row for row in plan["runs"] if row["kind"] == "full"]
    for name in ("environment", "contract"):
        key = f"{name}_sha256"
        assert digest(study / f"aligned-{name}.json") == bindings[key]
        identities[key] = bindings[key]
runs = []
receipts = {}
for entry in plan["runs"]:
    label = entry["id"]
    source = q / label
    if not source.exists():
        continue
    raw_pins = read_json(source / "pins.json")
    assert raw_pins["input_inventory_sha256"] == inventory["sha256"]
    canonical = normalized / (label + ".json")
    canonical.write_text(json.dumps(raw_pins | {
        "input_inventory_sha256": policy["fixture_inventory_sha256"],
    }, indent=2) + "\n")
    receipt = entry | {
        "pins": ref(canonical),
        "original_pins": ref(source / "pins.json"),
        "summary": ref(source / "summary.json"),
        "complete": ref(source / "complete.json"),
        "observations": {p.parent.name: ref(p) for p in sorted(source.glob("*/observations.json.gz"))},
        "cleanup": {actor: ref(source / f"{actor}.stderr") for actor in ACTORS},
    }
    run = runtime_run(receipt, fixtures, evidence, identities, policy, entry)
    assert run is not None
    if entry["kind"] == "focus":
        assert run["cases"].keys() == set(plan["focus_cases"])
    receipts[label] = receipt
    runs.append(run)
assert len(runs) in (2, 4)
repeats = []
repeat_entries = []
for variant in ("", "bound-"):
    repeat = study / (variant + "repeatability")
    if not prefix or len(runs) != 4 or not (repeat / "results.json").exists():
        continue
    repeat_plan = study / (variant + "repeatability-plan.json")
    assert digest(repeat_plan) == (study / (variant + "repeatability-plan.sha256")).read_text().strip()
    repeat_spec = read_json(repeat_plan)
    prior = study / "aligned-full-initial-replay.json"
    assert digest(prior) == repeat_spec["initial_replay_sha256"]
    assert {name for name, row in read_json(prior)["cases"].items()
            if row["decision"] == "unresolved"} == set(repeat_spec["cases"])
    repeat_receipt = read_json(repeat / "study.json")
    for key in ("seed", "repetitions_per_order", "batch_floor", "worker_sha256"):
        assert repeat_receipt[key] == repeat_spec[key]
    if variant:
        assert repeat_spec["full_bindings_sha256"] == digest(bindings_file)
        for key in ("environment_sha256", "contract_sha256"):
            assert repeat_receipt[key] == repeat_spec[key] == identities[key]
    repeat_entry = {
        "id": variant + "distinct-file-duplicate-control",
        "study": ref(repeat / "study.json"),
        "results": ref(repeat / "results.json"),
        "observations": {name: ref(repeat / f"{name}.json.gz") for name in repeat_spec["cases"]},
    }
    repeat_entries.append(repeat_entry)
    repeats.append((repeat_entry["id"], duplicate_study(repeat_entry, fixtures, evidence, identities, policy)))
report = decide_runtime(runs, repeats, policy, fixtures, findings)
report.update(
    plan_sha256=plan_sha, observed_receipts=receipts, decision=findings.decision,
    rejected=findings.rejected, unresolved=findings.unresolved,
    scope="Runtime diagnostics with frozen artifacts and v1 per-case and peer checks.",
)
if repeats:
    report["repeatability_receipts"] = repeat_entries
(study / (prefix + ("focus-replay.json" if len(runs) == 2 else "full-replay.json"))).write_text(
    json.dumps(report, indent=2) + "\n"
)
print(json.dumps({
    "decision": findings.decision, "rejected": findings.rejected,
    "unresolved": findings.unresolved,
    "wins": [{"order": run["id"], **{peer: detail["point_estimate_wins"]
              for peer, detail in run["peers"].items()}} for run in report["runs"]],
}, indent=2), flush=True)
