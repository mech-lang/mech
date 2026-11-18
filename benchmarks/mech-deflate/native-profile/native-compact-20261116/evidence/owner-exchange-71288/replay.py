"""Replay the exact owner exchange observation blocks."""

import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parent
packet = root.parents[1]
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run

def replay(label, driver):
    folder = root / "backend-work-hardened" / label
    expected = read_json(folder / "focus-replay.json")
    plan = read_json(folder / "plan.json")
    assert digest(folder / "plan.json") == (folder / "plan.sha256").read_text().strip() == expected["plan"]["sha256"]
    assert digest(folder / "fixture-content.json") == plan["fixture_content_sha256"]
    policy_path = packet / "qualification-policy-v1.json"
    assert digest(policy_path) == plan["policy"]["sha256"]
    policy = read_json(policy_path)
    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    for name, checksums in read_json(folder / "fixture-content.json").items():
        for key, value in checksums.items():
            assert digest(packet / fixtures[name][key]) == value
    for name in ("source", "linked_proofs", "native_static_screen"):
        binding = plan[name]
        assert digest(root / binding["path"]) == binding["sha256"]
    for name in ("environment", "contract"):
        assert digest(folder / f"{name}.json") == plan["identities"][f"{name}_sha256"]
    for name, filename in (("timer", "compare-library.py"), ("driver", driver),
                           ("qualification_driver", "full-timing.py"), ("as_run_inventory", "fixtures.json"),
                           ("pinned_peers", "pins.json")):
        assert digest(root / "as-run" / filename) == plan[name]["sha256"]

    for name in ("original_unpadded_outcome", "padding_profile", "padding_proof", "padded_exact_calls"):
        if name in plan:
            pin = plan[name]
            assert digest(root / pin["path"]) == pin["sha256"]
    evidence = Evidence(root, findings)
    runs = []
    for entry in plan["runs"]:
        receipt = expected["receipts"][entry["id"]]
        feature_pins = read_json(root / receipt["pins"]["path"])
        raw_pins = read_json(root / receipt["original_pins"]["path"])
        assert digest(root / receipt["original_pins"]["path"]) == receipt["original_pins"]["sha256"]
        assert feature_pins == raw_pins | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
        run = runtime_run(receipt, fixtures, evidence, plan["identities"], policy, entry)
        assert run is not None and set(run["cases"]) == set(plan["focus_cases"])
        runs.append(run)
    report = decide_runtime(runs, [], policy, fixtures, findings)
    report.update(decision=findings.decision, rejected=findings.rejected, unresolved=findings.unresolved)
    for key, value in report.items():
        assert expected[key] == value, key
    print(json.dumps({k: report[k] for k in ("decision", "rejected", "unresolved", "runs")}, indent=2))


replay("source-owner-exchange-71288-focus", "owner-timing.py")
replay("source-owner-exchange-71368-focus", "layout-owner-timing.py")
