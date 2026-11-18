"""Recheck the two recorded cursor screens with qualification policy v1."""

from pathlib import Path
import sys

evidence_dir = Path(__file__).resolve().parent
assert sys.argv[1:] in [['standard-link'], ['aligned-sections']]
archive = evidence_dir / sys.argv[1]
packet = evidence_dir.parent.parent
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run

record = read_json(archive / "focus-replay.json")
plan = read_json(archive / "plan.json")
assert digest(archive / "plan.json") == record["plan"]["sha256"]
assert digest(archive / "as-run-timing.py") == plan["driver"]["sha256"]
assert digest(archive / "as-run-timer.py") == plan["timer"]["sha256"]
policy_file = packet / "qualification-policy-v1.json"
assert digest(policy_file) == plan["policy"]["sha256"]
policy = read_json(policy_file)
findings = Findings()
fixtures = load_fixtures(Evidence(packet, findings), policy)
evidence = Evidence(archive, findings)
frozen = archive / "fixture-content.json"
assert digest(frozen) == plan["fixture_content_sha256"]
fixture_digests = read_json(frozen)
assert fixture_digests.keys() == fixtures.keys()
for name, files in fixture_digests.items():
    assert set(files) == {"input", "prefix", "expected"}
    for field, sha in files.items():
        assert sha == digest(packet / fixtures[name][field]), (name, field)
for key in ("environment", "contract"):
    assert digest(archive / f"{key}.json") == plan["identities"][f"{key}_sha256"]

runs = []
for entry in plan["runs"]:
    if entry["kind"] != "focus":
        continue
    run_id = entry["id"]
    receipt = record["receipts"][run_id]
    original = receipt["original_pins"] | {"path": f"{run_id}/pins.json"}
    pinned = receipt["pins"] | {"path": f"canonical-pins/{run_id}.json"}
    before = evidence.json(original, f"{run_id} original bindings")
    after = evidence.json(pinned, f"{run_id} canonical bindings")
    assert before["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
    assert after == before | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
    run = runtime_run(receipt | {"pins": pinned}, fixtures, evidence, plan["identities"], policy, entry)
    assert run is not None and set(run["cases"]) == set(plan["focus_cases"])
    runs.append(run)

result = decide_runtime(runs, [], policy, fixtures, findings)
assert result["runs"] == record["runs"]
assert result["cases"] == record["cases"]
assert findings.decision == record["decision"]
assert findings.rejected == record["rejected"] and findings.unresolved == record["unresolved"]
assert len(runs) == 2
print(f"Historical screen replay: {findings.decision}.")
for reason in findings.rejected + findings.unresolved:
    print(reason)
