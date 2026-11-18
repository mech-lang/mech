"""Acquire a new complete owner-exchange campaign under predeclared policy v2."""

from importlib import import_module
from pathlib import Path
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q / "source-frame-copy-tails"))
runtime = import_module("full-timing")
runtime.policy_file = runtime.packet / "qualification-policy-v2.json"
runtime.control = study / "worker-checks/retained"
candidate = study / "worker-checks/owner-exchange"
label = "source-owner-exchange-71288-v2"
folder = study / label
plan_file = folder / "plan.json"
plan_sha = folder / "plan.sha256"

action, = sys.argv[1:]
policy = runtime.read_json(runtime.policy_file)
assert policy["version"] == 2 and policy["require_case_nonregression"] is False
assert policy["peer_win_floors"] == {"roc": 81, "libdeflate": 61}
if action == "prepare":
    publish = runtime.read_json(q / "native-review-pr/publication.json")
    assert publish["published"] and len(publish["policy_v2_head"]) == 40
    checks = runtime.read_json(study / "linked-proofs/result.json")
    assert checks["matches_success_observations"] == 162 and len(checks["checked_failure_cases"]) == 6
    assert checks["peak_stack_bytes"]["owner-exchange"] <= checks["peak_stack_bytes"]["retained"]
    runtime.prepare(label, candidate, folder)
    plan = runtime.read_json(plan_file)
    for key in ("source_commit", "source_reproduction", "metadata_reconciliation", "original_focus",
                "original_repeatability", "history"):
        plan.pop(key, None)
    contract = folder / "contract.json"
    contract.unlink()
    plan["identities"]["contract_sha256"] = runtime.put(contract, {
        "scope": "diagnostic", "level": 12,
        "timing_boundary": "compress-into invocation through private heap cleanup",
        "fixture_inventory_sha256": policy["fixture_inventory_sha256"],
    })
    plan.update(
        policy_version=policy["version"], policy_commit=publish["policy_v2_head"],
        review_commit=publish["head"], objective="encoder_text_bytes",
        expected_scope="Runtime gate only. Build, checked artefact provenance, and other promotion evidence apply separately.",
        driver=runtime.ref(Path(__file__).resolve()),
        qualification_driver=runtime.ref(Path(runtime.__file__)),
        source=runtime.ref(study / "source.json"),
        linked_proofs=runtime.ref(study / "linked-proofs/result.json"),
        native_static_screen=runtime.ref(study / "worker-checks/variants.json"),
        original_v1_focus=runtime.ref(study / "source-owner-exchange-71288-focus/focus-replay.json"),
        timer_layout_focus=runtime.ref(study / "source-owner-exchange-71368-focus/focus-replay.json"),
        runs=[{"id": f"{label}-full-{order}", "kind": "full", "order": order, "blocks_per_case": 96}
              for order in ("forward", "reverse")],
    )
    plan_file.unlink()
    value = runtime.put(plan_file, plan)
    plan_sha.write_text(value + "\n")
    print(label, value, flush=True)
else:
    assert runtime.digest(plan_file) == plan_sha.read_text().strip()
    plan = runtime.read_json(plan_file)
    assert plan["policy_version"] == policy["version"]
    for name in ("policy", "timer", "driver", "qualification_driver", "as_run_inventory",
                 "pinned_peers", "source", "linked_proofs", "native_static_screen",
                 "original_v1_focus", "timer_layout_focus"):
        pin = plan[name]
        assert runtime.digest(q / pin["path"]) == pin["sha256"], name
    for name in ("environment", "contract"):
        assert runtime.digest(folder / f"{name}.json") == plan["identities"][f"{name}_sha256"]
    if action == "acquire":
        runtime.acquire(plan, candidate, "full")
    elif action == "replay":
        runtime.replay(plan, folder, "full")
    else:
        raise SystemExit("expected prepare, acquire or replay")
