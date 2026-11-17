"""Replay the aligned spill-pair timings with the maintained v1 policy."""

import argparse
import json
from pathlib import Path
import sys

here = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--packet", type=Path, default=here.parent.parent)
parser.add_argument("--archive", type=Path, default=here)
parser.add_argument("--plan-sha256", required=True)
args = parser.parse_args()
packet = args.packet.resolve()
archive = args.archive.resolve()
sys.path.insert(0, str(packet))
from qualification_records import ACTORS, Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, duplicate_study, runtime_run

findings = Findings()
evidence = Evidence(packet, findings)
policy_path = packet / "qualification-policy-v1.json"
policy = read_json(policy_path)
plan_path = archive / "plan.json"
assert digest(plan_path) == args.plan_sha256
plan = read_json(plan_path)
assert plan["policy_version"] == policy["version"] and plan["policy_sha256"] == digest(policy_path)
for field in ("environment", "contract"):
    assert plan[f"{field}_sha256"] == digest(archive / f"{field}.json")
fixtures = load_fixtures(evidence, policy)
campaign = read_json(archive / "runtime-campaign.json")
assert list(campaign["runs"]) == [run["id"] for run in plan["runs"]]
inventory = evidence.json(campaign["fixture_inventory_as_run"], "original timed inventory")
proof = evidence.json(campaign["fixture_content_proof"], "timed file content bindings")
assert proof["timed_inventory_sha256"] == campaign["fixture_inventory_as_run"]["sha256"]
assert proof["frozen_inventory_sha256"] == policy["fixture_inventory_sha256"]
assert [row["name"] for row in inventory] == list(fixtures)
assert set(proof["case_file_sha256"]) == set(fixtures)
for row in inventory:
    frozen = fixtures[row["name"]]
    assert all(row[field] == frozen[field] for field in (
        "input_sha256", "expected_sha256", "input_bytes", "prefix_bytes", "raw_bytes", "capacity"
    ))
    assert proof["case_file_sha256"][row["name"]] == {
        field: digest(packet / frozen[field]) for field in ("input", "prefix", "expected")
    }
identities = {
    actor: {
        "worker": plan["worker_sha256"][actor],
        "metadata": campaign["metadata_sha256"].get(actor),
    }
    for actor in ACTORS
}
for actor in ACTORS[:2]:
    assert identities[actor]["metadata"] == plan["artifacts"][actor]["metadata.json"]
identities["environment_sha256"] = plan["environment_sha256"]
identities["contract_sha256"] = plan["contract_sha256"]
runs = []
for entry in plan["runs"]:
    receipt = campaign["runs"][entry["id"]]
    original = evidence.json(receipt["original_pins"], "raw timing pins")
    canonical = evidence.json(receipt["pins"], "content-normalized timing pins")
    assert original["input_inventory_sha256"] == proof["timed_inventory_sha256"]
    assert canonical == original | {"input_inventory_sha256": proof["frozen_inventory_sha256"]}
    run = runtime_run(receipt, fixtures, evidence, identities, policy, entry)
    assert run is not None
    if entry["kind"] == "focus":
        assert run["cases"].keys() == set(plan["cases"])
    runs.append(run)
repeats = [(entry["id"], duplicate_study(entry, fixtures, evidence, identities, policy))
           for entry in campaign["duplicate_controls"]]
assert len(repeats) <= 1
if repeats:
    study = evidence.json(campaign["duplicate_controls"][0]["study"], "duplicate control plan")
    rule = plan["conditional_repeatability"]
    for key in ("batch_floor", "seed", "repetitions_per_order"):
        assert study[key] == rule[key]
    initial = Findings()
    before = decide_runtime(runs, [], policy, fixtures, initial)
    assert not initial.rejected
    assert set(repeats[0][1]) == {name for name, row in before["cases"].items() if row["decision"] == "unresolved"}
report = decide_runtime(runs, repeats, policy, fixtures, findings)
report["timing_only_findings"] = {"rejected": findings.rejected, "unresolved": findings.unresolved}
report["timing_decision"] = findings.decision
report["plan_sha256"] = args.plan_sha256
report["worker_sha256"] = plan["worker_sha256"]
report["qualification_scope"] = "Runtime timing policy only. The linked correctness, proof, and build receipts are reported separately."
print(json.dumps(report, indent=2))
