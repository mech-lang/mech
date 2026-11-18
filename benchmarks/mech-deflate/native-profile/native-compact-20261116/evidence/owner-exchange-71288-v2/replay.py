"""Recompute both preserved owner-exchange orders under policy v2."""
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parent
packet = root.parents[1]
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run


def check(path, expected):
    assert digest(path) == expected, path


def main():
    folder = root / "backend-work-hardened/source-owner-exchange-71288-v2"
    original = read_json(folder / "full-replay.json")
    plan_path = folder / "plan.json"
    check(plan_path, (folder / "plan.sha256").read_text().strip())
    check(plan_path, original["plan"]["sha256"])
    plan = read_json(plan_path)
    policy_path = packet / "qualification-policy-v2.json"
    check(policy_path, plan["policy"]["sha256"])
    policy = read_json(policy_path)
    assert plan["policy_version"] == policy["version"] == 2
    assert policy["require_case_nonregression"] is False

    for key in ("environment", "contract"):
        check(folder / f"{key}.json", plan["identities"][f"{key}_sha256"])
    cpu = read_json(folder / "environment.json")["cpu_source"]
    check(root / cpu["path"], cpu["sha256"])
    for name in ("driver", "timer", "qualification_driver", "as_run_inventory", "pinned_peers",
                 "source", "linked_proofs", "native_static_screen", "original_v1_focus", "timer_layout_focus"):
        pin = plan[name]
        check(root / pin["path"], pin["sha256"])
    variants = {row["role"]: row for row in read_json(root / plan["native_static_screen"]["path"])}
    for actor, role in (("mech-control", "retained"), ("mech-experiment", "owner-exchange")):
        binding = plan["bindings"][actor]
        assert plan["identities"][actor] == {"worker": binding["mech-worker"]["sha256"],
                                              "metadata": binding["metadata.json"]["sha256"]}
        for name in ("runtime.json", "metadata.json"):
            check(root / binding[name]["path"], binding[name]["sha256"])
        for name in ("program.o", "metadata.json", "mech-worker"):
            assert variants[role]["sha256"][name] == binding[name]["sha256"]
    peer_pins = read_json(root / plan["pinned_peers"]["path"])["actors"]
    for actor in ("roc", "libdeflate"):
        pin = peer_pins[actor]
        assert plan["identities"][actor] == {"worker": pin["binary_sha256"],
                                              "metadata": pin.get("metadata_sha256")}
    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    fixture_content = folder / "fixture-content.json"
    check(fixture_content, plan["fixture_content_sha256"])
    for name, digests in read_json(fixture_content).items():
        for key, sha in digests.items():
            check(packet / fixtures[name][key], sha)
    evidence = Evidence(root, findings)
    runs = []
    for entry in plan["runs"]:
        receipt = original["receipts"][entry["id"]]
        raw, canonical = (evidence.json(receipt[key], key) for key in ("original_pins", "pins"))
        assert raw["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
        assert canonical == raw | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
        run = runtime_run(receipt, fixtures, evidence, plan["identities"], policy, entry)
        assert run is not None and len(run["cases"]) == policy["named_cases"]
        runs.append(run)
    assert len(runs) == 2 and {run["order"] for run in runs} == {"forward", "reverse"}
    result = decide_runtime(runs, [], policy, fixtures, findings)
    result.update(decision=findings.decision, rejected=findings.rejected, unresolved=findings.unresolved)
    for key, value in result.items():
        assert original[key] == value, key
    print(json.dumps({key: result[key] for key in ("decision", "rejected", "unresolved", "runs")}, indent=2))


if __name__ == "__main__":
    main()
