"""Freeze and replay focused diagnostic runs with the same compiler options."""

from importlib import import_module
import os
from pathlib import Path
import sys

study = Path(__file__).resolve().parent
q = study.parent
full_script = q / "source-frame-copy-tails"
sys.path.insert(0, str(full_script))
runtime = import_module("full-timing")

cases = [
    "empty", "seven-stored-prefix", "320-overlap", "random-64", "prefix-random-1024",
    "prefix-random-4096", "prefix-random-8192", "prefix-random-32769",
    "prefix-random-1048576", "prefix-natural-4096", "prefix-natural-8192",
    "prefix-periodic-16384", "heldout-random-65537",
]
options = {
    "literal-once": (study / "native/generic-control", study / "literal-once-proof/literal-once"),
    "literal-run": (study / "expanded-fuel-proof/expanded-control",
                    study / "expanded-fuel-proof/expanded-literal-run"),
}
choice, action = sys.argv[1:]
control, candidate = options[choice]
runtime.control = control
runtime.focus = cases
label = f"mech-ordinary-{choice}-qual"
folder = study / label
plan_file = folder / "plan.json"
plan_sha = folder / "plan.sha256"
for path in (control, candidate):
    assert runtime.read_json(path / "runtime.json")["flags"] == runtime.read_json(study / "native/plan.json")["common_flags"]

if action == "prepare":
    temporary = folder.with_name(folder.name + "-prepare")
    runtime.prepare(label, candidate, temporary)
    plan = runtime.read_json(temporary / "plan.json")
    for name in ("source_commit", "source_reproduction", "metadata_reconciliation", "original_focus",
                 "original_repeatability", "history", "review_commit"):
        plan.pop(name)
    plan.update(
        objective="Isolate the shared-path runtime effect with ID-independent flags. Focused stages only.",
        compiler_source="b72b7419b39b22765fe1eed6b5c636e63964b905",
        acquisition_datasets=(runtime.ref(temporary / "environment.json"),
                              runtime.ref(temporary / "contract.json")),
        construction_plan=runtime.ref(study / "native/plan.json"),
        driver=runtime.ref(Path(__file__).resolve()),
        qualification_driver=runtime.ref(full_script / "full-timing.py"),
        source_control=runtime.ref(control / "source.mec"),
        source_experiment=runtime.ref(candidate / "source.mec"),
        default_parse_admission=choice == "literal-once",
        runs=[{"id": f"{label}-{order}", "kind": "focus", "order": order, "blocks_per_case": 24}
              for order in ("forward", "reverse")],
    )
    for item in plan["acquisition_datasets"]:
        item["path"] = item["path"].replace(temporary.name, folder.name)
    (temporary / "plan.json").unlink()
    digest = runtime.put(temporary / "plan.json", plan)
    (temporary / "plan.sha256").write_text(digest + "\n")
    os.rename(temporary, folder)
    print(label, digest, len(cases), "cases", flush=True)
else:
    assert runtime.digest(plan_file) == plan_sha.read_text().strip()
    plan = runtime.read_json(plan_file)
    for name in ("driver", "qualification_driver", "construction_plan", "policy", "timer",
                 "as_run_inventory", "pinned_peers", "source_control", "source_experiment"):
        binding = plan[name]
        assert runtime.digest(q / binding["path"]) == binding["sha256"], name
    for actor in plan["bindings"].values():
        for binding in actor.values():
            assert runtime.digest(q / binding["path"]) == binding["sha256"]
    if action == "acquire":
        source = runtime.timer.read_text()
        bound = {field: plan["identities"][field] for field in ("environment_sha256", "contract_sha256")}
        for old, new in (
            ("assert digest(FIXTURES) == pins['input_inventory_sha256']",
             f"assert digest(FIXTURES) == pins['input_inventory_sha256']\npins.update({bound!r})"),
            ("selected = {case['name'] for case in cases}", f"selected = set({plan['focus_cases']!r})"),
        ):
            assert source.count(old) == 1
            source = source.replace(old, new)
        for key in list(os.environ):
            if key.startswith("MECH_"):
                del os.environ[key]
        for run in plan["runs"]:
            assert not (q / run["id"]).exists()
            print(f"Acquiring {run['id']}", flush=True)
            sys.argv = [str(runtime.timer), run["id"], str(control), str(candidate)]
            if run["order"] == "reverse":
                sys.argv.append("reverse")
            exec(compile(source, str(runtime.timer), "exec"),
                 {"__file__": str(runtime.timer), "__name__": "__main__"})
    elif action == "replay":
        runtime.replay(plan, folder, "focus")
    else:
        raise SystemExit("expected prepare, acquire, or replay")
