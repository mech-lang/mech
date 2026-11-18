"""Replay the frozen shared-copy experiment from its archived observations."""

from pathlib import Path
import sys

archive = Path(__file__).resolve().parent
packet = archive.parent.parent
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run

plan_file = archive / "plan.json"
plan = read_json(plan_file)
assert digest(plan_file) == (archive / "plan.sha256").read_text().strip()
record = read_json(archive / "focus-replay.json")
assert record["plan"]["sha256"] == digest(plan_file)
policy_file = packet / "qualification-policy-v1.json"
assert digest(policy_file) == plan["policy"]["sha256"]
policy = read_json(policy_file)
for key, path in {
    "driver": "as-run/timing.py",
    "timer": "as-run/compare-library.py",
    "as_run_inventory": "as-run/fixtures-as-run.json",
    "pinned_peers": "as-run/peer-pins.json",
    "screen_design": "screen-design.json",
    "static_transform": "as-run/shared_copy_tails.py",
    "static_report": "static-report.json",
    "smoke_report": "checks/shared-copy-tails-qualification.json",
    "failure_report": "checks/checked-failure-report.json",
}.items():
    assert digest(archive / path) == plan[key]["sha256"], key
intent = read_json(archive / "screen-design.json")
assert intent["cases"] == plan["focus_cases"]
for actor, prefix in (("mech-control", "baseline"), ("mech-experiment", "candidate")):
    assert plan["bindings"][actor]["program.o"]["sha256"] == intent[f"{prefix}_object_sha256"]
    for name in ("runtime.json", "metadata.json"):
        assert digest(archive / "identities" / actor / name) == plan["bindings"][actor][name]["sha256"]
static = read_json(archive / "static-report.json")
assert intent["baseline_object_sha256"] == static["reference_sha256"]
assert intent["candidate_object_sha256"] == static["candidate_sha256"]
assert digest(archive / "contract.json") == plan["identities"]["contract_sha256"]
assert digest(archive / "environment.json") == plan["identities"]["environment_sha256"]

findings = Findings()
fixtures = load_fixtures(Evidence(packet, findings), policy)
evidence = Evidence(archive, findings)
content = archive / "fixture-content.json"
assert digest(content) == plan["fixture_content_sha256"]
frozen = read_json(content)
assert frozen.keys() == fixtures.keys()
for name, files in frozen.items():
    assert set(files) == {"input", "prefix", "expected"}
    assert all(sha == digest(packet / fixtures[name][field]) for field, sha in files.items())


def at(reference, path):
    return reference | {"path": str(path)}


results = []
for item in plan["runs"]:
    name = item["id"]
    run = record["receipts"][name]
    folder = Path(name)
    old = evidence.json(at(run["original_pins"], folder / "pins.json"), f"{name} as-run pins")
    pins = at(run["pins"], f"canonical-pins/{name}.json")
    normalized = evidence.json(pins, f"{name} canonical pins")
    assert old["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
    assert normalized == old | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
    run = run | {
        "pins": pins,
        "summary": at(run["summary"], folder / "summary.json"),
        "complete": at(run["complete"], folder / "complete.json"),
        "observations": {case: at(ref, folder / case / "observations.json.gz")
                         for case, ref in run["observations"].items()},
        "cleanup": {actor: at(ref, folder / f"{actor}.stderr") for actor, ref in run["cleanup"].items()},
    }
    result = runtime_run(run, fixtures, evidence, plan["identities"], policy, item)
    assert result is not None and set(result["cases"]) == set(plan["focus_cases"])
    results.append(result)

replay = decide_runtime(results, [], policy, fixtures, findings)
assert all(value == record[key] for key, value in replay.items())
assert findings.decision == record["decision"]
assert findings.rejected == record["rejected"] and findings.unresolved == record["unresolved"]
assert len(results) == 2 and {run["order"] for run in results} == {"forward", "reverse"}
print(f"Recorded runtime screen {findings.decision}")
for finding in findings.rejected + findings.unresolved:
    print(finding)
