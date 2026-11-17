"""Apply runtime policy v2 to the exact archived 71,768-byte reference orders."""
from argparse import ArgumentParser
import json
from pathlib import Path
import sys


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("packet", type=Path)
    cli.add_argument("output", type=Path)
    args = cli.parse_args()
    packet = args.packet.resolve()
    sys.path.insert(0, str(packet))
    from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
    from qualification_timings import decide_runtime, runtime_run

    archive = packet / "evidence/bound-71768-runtime"
    original = read_json(archive / "full-replay.json")
    plan = read_json(archive / "plan.json")
    assert digest(archive / "plan.json") == (archive / "plan.sha256").read_text().strip()
    assert digest(archive / "plan.json") == original["plan"]["sha256"]
    v1_path = packet / "qualification-policy-v1.json"
    v2_path = packet / "qualification-policy-v2.json"
    assert digest(v1_path) == plan["policy"]["sha256"]
    v1, policy = read_json(v1_path), read_json(v2_path)
    assert policy == v1 | {"version": 2, "require_case_nonregression": False}

    for name, filename in (("timer", "compare-library.py"), ("driver", "timing.py"),
                           ("as_run_inventory", "fixtures-as-run.json"), ("pinned_peers", "peer-pins.json")):
        assert digest(archive / "as-run" / filename) == plan[name]["sha256"]
    for key in ("environment", "contract"):
        assert digest(archive / f"{key}.json") == plan["identities"][f"{key}_sha256"]
    for actor, bindings in plan["bindings"].items():
        for filename in ("runtime.json", "metadata.json"):
            assert digest(archive / "identities" / actor / filename) == bindings[filename]["sha256"]
        assert plan["identities"][actor] == {"worker": bindings["mech-worker"]["sha256"],
                                              "metadata": bindings["metadata.json"]["sha256"]}

    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    fixture_content = archive / "fixture-content.json"
    assert digest(fixture_content) == plan["fixture_content_sha256"]
    frozen = read_json(fixture_content)
    assert frozen.keys() == fixtures.keys()
    for name, files in frozen.items():
        assert set(files) == {"input", "prefix", "expected"}
        for key, sha in files.items():
            assert digest(packet / fixtures[name][key]) == sha
    evidence = Evidence(archive, findings)
    runs = []
    for entry in plan["runs"]:
        run_id = entry["id"]
        receipt = original["receipts"][run_id]
        assert entry["kind"] == "full" and entry["blocks_per_case"] == 96
        raw = receipt["original_pins"] | {"path": f"{run_id}/pins.json"}
        pins = receipt["pins"] | {"path": f"canonical-pins/{run_id}.json"}
        observed = evidence.json(raw, "original pins")
        assert observed["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
        assert evidence.json(pins, "portable pins") == observed | {
            "input_inventory_sha256": policy["fixture_inventory_sha256"],
        }
        run = runtime_run(receipt | {"pins": pins}, fixtures, evidence, plan["identities"], policy, entry)
        assert run is not None and len(run["cases"]) == 81
        runs.append(run)
    assert len(runs) == 2 and {run["order"] for run in runs} == {"forward", "reverse"}
    result = decide_runtime(runs, [], policy, fixtures, findings)
    assert result["runs"] == original["runs"]
    result.update(
        decision=findings.decision, rejected=findings.rejected, unresolved=findings.unresolved,
        policy_sha256=digest(v2_path), original_plan_sha256=digest(archive / "plan.json"),
        original_full_replay_sha256=digest(archive / "full-replay.json"), reference=plan["identities"]["mech-experiment"],
        acquisition="Historical complete orders. No new timing samples.",
        qualification_scope="Runtime component. Source/build/stack/ordinary integration are independently gated.",
    )
    with args.output.open("x") as f:
        json.dump(result, f, indent=2)
        f.write("\n")
    print(result["decision"], result["rejected"], result["unresolved"])


if __name__ == "__main__":
    main()
