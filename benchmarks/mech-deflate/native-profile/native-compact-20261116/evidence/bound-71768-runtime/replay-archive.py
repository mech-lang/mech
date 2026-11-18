"""Replay both complete orders and the separately declared duplicate controls."""

from pathlib import Path
import sys

archive = Path(__file__).resolve().parent
packet = archive.parent.parent
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, duplicate_study, runtime_run


def bound_plan(stem):
    file = archive / f"{stem}.json"
    assert digest(file) == (archive / f"{stem}.sha256").read_text().strip()
    return read_json(file)


def at(reference, relative):
    return reference | {"path": relative}


def match(record, result, state):
    for field, value in result.items():
        assert value == record[field], field
    assert state.decision == record["decision"]
    assert state.rejected == record["rejected"] and state.unresolved == record["unresolved"]


original = read_json(archive / "full-replay.json")
final = read_json(archive / "full-and-repeat-replay.json")
plan = bound_plan("plan")
repeat_plan = bound_plan("followup-plan")
assert digest(archive / "plan.json") == original["plan"]["sha256"] == repeat_plan["initial_plan_sha256"]
assert digest(archive / "full-replay.json") == repeat_plan["initial_replay_sha256"] == final["initial"]["sha256"]
assert digest(archive / "followup-plan.json") == final["plan"]["sha256"]
for record, files in (
    (plan, {"driver": "timing.py", "timer": "compare-library.py", "as_run_inventory": "fixtures-as-run.json",
            "pinned_peers": "peer-pins.json"}),
    (repeat_plan, {"driver": "measure-cold-epilogues-followup.py", "driver_wrapper": "repeatability.py"}),
):
    for key, filename in files.items():
        assert digest(archive / "as-run" / filename) == record[key]["sha256"]

policy_path = packet / "qualification-policy-v1.json"
assert digest(policy_path) == plan["policy"]["sha256"]
policy = read_json(policy_path)
findings = Findings()
fixtures = load_fixtures(Evidence(packet, findings), policy)
evidence = Evidence(archive, findings)
for field in ("environment", "contract"):
    key = f"{field}_sha256"
    assert digest(archive / f"{field}.json") == plan["identities"][key] == repeat_plan[key]
frozen = archive / "fixture-content.json"
assert digest(frozen) == plan["fixture_content_sha256"]
fixture_digests = read_json(frozen)
assert fixture_digests.keys() == fixtures.keys()
for name, files in fixture_digests.items():
    assert set(files) == {"input", "prefix", "expected"}
    for field, sha in files.items():
        assert sha == digest(packet / fixtures[name][field]), (name, field)
for actor, inputs in plan["bindings"].items():
    for filename in ("metadata.json", "runtime.json"):
        assert digest(archive / "identities" / actor / filename) == inputs[filename]["sha256"]

runs = []
for entry in plan["runs"]:
    run_id = entry["id"]
    assert entry["kind"] == "full"
    receipt = original["receipts"][run_id]
    before = evidence.json(at(receipt["original_pins"], f"{run_id}/pins.json"), f"{run_id} original pins")
    pins = at(receipt["pins"], f"canonical-pins/{run_id}.json")
    after = evidence.json(pins, f"{run_id} canonical pins")
    assert before["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
    assert after == before | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
    run = runtime_run(receipt | {"pins": pins}, fixtures, evidence, plan["identities"], policy, entry)
    assert run is not None and len(run["cases"]) == policy["named_cases"]
    runs.append(run)
assert len(runs) == 2 and {run["order"] for run in runs} == {"forward", "reverse"}
initial_result = decide_runtime(runs, [], policy, fixtures, findings)
match(original, initial_result, findings)
assert set(repeat_plan["cases"]) == {
    name for name, case in initial_result["cases"].items() if case["decision"] == "unresolved"
}

findings = Findings()
evidence.findings = findings
followup = final["followup"]
raw = {key: at(followup[key], f"followup/{Path(followup[key]['path']).name}") for key in ("study", "results")}
actual = evidence.json(raw["study"], "duplicate-control acquisition")
for field in ("seed", "repetitions_per_order", "batch_floor", "worker_sha256"):
    assert actual[field] == repeat_plan[field], field
assert set(actual["order"]) == set(repeat_plan["cases"])
for actor in ("control", "duplicate-control", "candidate"):
    log = archive / "followup" / f"{actor}.stderr"
    assert log.read_text().rstrip().endswith("CLEANUP private_payload_bytes=0 live_owner_records=0")
raw["observations"] = {name: at(ref, f"followup/{name}.json.gz") for name, ref in followup["observations"].items()}
repeated = duplicate_study(followup | raw, fixtures, evidence, plan["identities"], policy)
result = decide_runtime(runs, [(followup["id"], repeated)], policy, fixtures, findings)
match(final, result, findings)
print(f"Recorded runtime component: {findings.decision}")
for reason in findings.rejected + findings.unresolved:
    print(reason)
