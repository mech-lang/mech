"""Freeze, acquire, and replay the checked owner exchange timing comparison."""

from importlib import import_module
import os
from pathlib import Path
import sys

study = Path(__file__).resolve().parent
q = study.parent
sys.path.insert(0, str(q / "source-frame-copy-tails"))
runtime = import_module("full-timing")
workers = study / "worker-checks"
candidate = workers / "owner-exchange"
cases = [
    "empty", "one-stored", "seven-stored-prefix", "eight-literals", "32-high-entropy-stored",
    "320-overlap", "random-64", "random-1024", "prefix-random-4096", "prefix-random-8192",
    "prefix-natural-4096", "prefix-natural-8192", "prefix-natural-16384", "prefix-periodic-1024",
    "prefix-periodic-16384", "heldout-window-32767", "heldout-random-65537",
    "heldout-mixed-131073", "prefix-random-1048576",
]
action, scope = sys.argv[1:]
assert scope in ("focus", "full")
runtime.control = workers / ("size-control" if scope == "focus" else "retained")
runtime.focus = cases
label = f"source-owner-exchange-71288-{scope}"
folder = study / label
plan_file = folder / "plan.json"
plan_sha = folder / "plan.sha256"

if action == "prepare":
    checks = runtime.read_json(study / "linked-proofs/result.json")
    assert checks["matches_success_observations"] == 162 and len(checks["checked_failure_cases"]) == 6
    scratch = folder.with_name(folder.name + "-prepare")
    runtime.prepare(label, candidate, scratch)
    plan = runtime.read_json(scratch / "plan.json")
    for name in ("source_commit", "source_reproduction", "metadata_reconciliation", "original_focus",
                 "original_repeatability", "history"):
        plan.pop(name)
    plan.update(
        objective="Measure the checked nullable retain/release helper against the fixed peers.",
        review_commit="78b064c5bb70937db72db21ebd3ab2fd556816c3",
        performance_scope=scope,
        control_scope=("Identical unpadded options, with only the owner exchange flag varied."
                       if scope == "focus" else "Exact 71,768-byte published reference with its original padding."),
        driver=runtime.ref(Path(__file__).resolve()),
        qualification_driver=runtime.ref(Path(runtime.__file__)),
        source=runtime.ref(study / "source.json"),
        linked_proofs=runtime.ref(study / "linked-proofs/result.json"),
        native_static_screen=runtime.ref(workers / "variants.json"),
        runs=[{"id": f"{label}-{order}", "kind": scope, "order": order, "blocks_per_case": 96}
              for order in ("forward", "reverse")],
    )
    if scope == "full":
        focused = study / "source-owner-exchange-71288-focus/focus-replay.json"
        assert runtime.read_json(focused)["decision"] == "Unresolved"
        plan["focus_screen"] = runtime.ref(focused)
    (scratch / "plan.json").unlink()
    value = runtime.put(scratch / "plan.json", plan)
    (scratch / "plan.sha256").write_text(value + "\n")
    os.rename(scratch, folder)
    print(label, value, flush=True)
else:
    assert runtime.digest(plan_file) == plan_sha.read_text().strip()
    plan = runtime.read_json(plan_file)
    for name in ("driver", "qualification_driver", "policy", "timer", "as_run_inventory", "pinned_peers",
                 "source", "linked_proofs", "native_static_screen"):
        pin = plan[name]
        assert runtime.digest(q / pin["path"]) == pin["sha256"], name
    if action == "acquire":
        runtime.acquire(plan, candidate, scope)
    elif action == "replay":
        runtime.replay(plan, folder, scope)
    else:
        raise SystemExit("expected prepare, acquire or replay")
