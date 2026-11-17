"""Replay the two frozen source experiments with the maintained validator."""

import json
from pathlib import Path
import sys

archive = Path(__file__).resolve().parent
packet = archive.parents[1]
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run


def replay(label):
    folder = archive / "mech-unmatched-runs" / f"mech-ordinary-{label}-qual"
    expected = read_json(folder / "focus-replay.json")
    assert digest(folder / "plan.json") == (folder / "plan.sha256").read_text().strip() == expected["plan"]["sha256"]
    plan = read_json(folder / "plan.json")
    assert digest(folder / "fixture-content.json") == plan["fixture_content_sha256"]
    assert digest(archive / "as-run/fixtures.json") == plan["as_run_inventory"]["sha256"]
    assert digest(archive / "as-run/plan.json") == plan["construction_plan"]["sha256"]
    for name, file in (("driver", "focus.py"), ("qualification_driver", "full-timing.py"), ("timer", "compare-library.py")):
        assert digest(archive / "as-run" / file) == plan[name]["sha256"]
    policy_file = packet / "qualification-policy-v1.json"
    assert digest(policy_file) == plan["policy"]["sha256"]
    for key in ("environment", "contract"):
        assert digest(folder / f"{key}.json") == plan["identities"][f"{key}_sha256"]
    policy = read_json(policy_file)
    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    contents = read_json(folder / "fixture-content.json")
    for name, fields in contents.items():
        for key, hash_value in fields.items():
            assert digest(packet / fixtures[name][key]) == hash_value
    evidence = Evidence(archive, findings)
    runs = []
    for entry in plan["runs"]:
        receipt = expected["receipts"][entry["id"]]
        canonical = read_json(archive / receipt["pins"]["path"])
        original = read_json(archive / receipt["original_pins"]["path"])
        assert digest(archive / receipt["original_pins"]["path"]) == receipt["original_pins"]["sha256"]
        assert canonical == original | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
        run = runtime_run(receipt, fixtures, evidence, plan["identities"], policy, entry)
        assert run is not None and set(run["cases"]) == set(plan["focus_cases"])
        runs.append(run)
    result = decide_runtime(runs, [], policy, fixtures, findings)
    result.update(decision=findings.decision, rejected=findings.rejected, unresolved=findings.unresolved)
    for key, value in result.items():
        assert expected[key] == value, (label, key)
    print(json.dumps({"label": label, "decision": result["decision"], "runs": result["runs"]}, indent=2))


for experiment in ("literal-once", "literal-run"):
    replay(experiment)
