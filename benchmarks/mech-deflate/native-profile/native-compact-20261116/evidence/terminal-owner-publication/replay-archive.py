"""Recompute the recorded focused decisions from both sets of raw blocks."""

from pathlib import Path
import sys

archive = Path(__file__).resolve().parent
packet = archive.parent.parent
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run

policy_file = packet / "qualification-policy-v1.json"
policy = read_json(policy_file)
for variant in ("terminal-owner-only", "terminal-owner-cold-origins"):
    study = archive / variant
    plan_file = study / "plan.json"
    replay = read_json(study / "focus-replay.json")
    assert digest(plan_file) == (study / "plan.sha256").read_text().strip() == replay["plan"]["sha256"]
    plan = read_json(plan_file)
    assert digest(policy_file) == plan["policy"]["sha256"]
    for field, filename in (("driver", "timing.py"), ("timer", "compare-library.py"),
                            ("as_run_inventory", "fixtures-as-run.json"), ("pinned_peers", "peer-pins.json")):
        assert digest(archive / "as-run" / filename) == plan[field]["sha256"]
    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    evidence = Evidence(archive, findings)
    for field in ("environment", "contract"):
        assert digest(study / f"{field}.json") == plan["identities"][f"{field}_sha256"]
    content = study / "fixture-content.json"
    assert digest(content) == plan["fixture_content_sha256"]
    frozen = read_json(content)
    assert frozen.keys() == fixtures.keys()
    for name, files in frozen.items():
        assert set(files) == {"input", "prefix", "expected"}
        for field, sha in files.items():
            assert sha == digest(packet / fixtures[name][field]), (name, field)
    runs = []
    for entry in plan["runs"]:
        if entry["kind"] != "focus":
            continue
        run_id = entry["id"]
        receipt = replay["receipts"][run_id]
        original = receipt["original_pins"] | {"path": f"{run_id}/pins.json"}
        normalized = receipt["pins"] | {"path": f"{variant}/canonical-pins/{run_id}.json"}
        before = evidence.json(original, f"{run_id} original pins")
        after = evidence.json(normalized, f"{run_id} canonical pins")
        assert before["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
        assert after == before | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
        run = runtime_run(receipt | {"pins": normalized}, fixtures, evidence, plan["identities"], policy, entry)
        assert run is not None and set(run["cases"]) == set(plan["focus_cases"])
        runs.append(run)
    assert len(runs) == 2
    result = decide_runtime(runs, [], policy, fixtures, findings)
    assert all(value == replay[field] for field, value in result.items())
    assert findings.decision == replay["decision"]
    assert findings.rejected == replay["rejected"] and findings.unresolved == replay["unresolved"]
    print(f"{variant}: {findings.decision}")
    for reason in findings.rejected + findings.unresolved:
        print(reason)
