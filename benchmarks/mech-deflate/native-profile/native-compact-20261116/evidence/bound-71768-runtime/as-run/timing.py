"""Requalify the exact 71,768-byte worker in both complete runtime orders."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

study = Path(__file__).resolve().parent
q = study.parent
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
sys.path.insert(0, str(packet))
from qualification_records import ACTORS, Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run

control = q / "stale-cache-accounting/pinned-smaller"
timer = q / "compare-library.py"
fixture_file = q / "fixtures.json"
policy_file = packet / "qualification-policy-v1.json"
peers = q / "pinned-peers/pins.json"
focus = [
    "empty", "one-stored", "seven-stored-prefix", "320-overlap", "prefix-periodic-1024", "random-64",
    "random-1024", "prefix-random-1024", "prefix-random-4096", "prefix-random-8192",
    "prefix-natural-4096", "prefix-natural-8192", "prefix-natural-16384",
    "heldout-window-32767", "heldout-mixed-131073", "prefix-random-1048576",
]


def put(path, value):
    with path.open("x") as out:
        json.dump(value, out, indent=2)
        out.write("\n")
    return digest(path)


def ref(path):
    return {"path": str(path.relative_to(q)), "sha256": digest(path)}


def prepare(label, candidate, folder):
    folder.mkdir()
    base_environment = q / "native-size-20261117/aligned-environment.json"
    previous = read_json(base_environment)
    os_release = subprocess.check_output(["/usr/bin/sw_vers"], text=True).strip()
    assert "15.6.1" in os_release and os.uname().machine == "arm64"
    environment = put(folder / "environment.json", {
        "cpu": previous["cpu"], "cpu_source": ref(base_environment), "os": os_release,
        "architecture": os.uname().machine,
        "clock": "Rust Instant",
        "protocol": "Serial balanced worker orders with separate warmup and private allocation cleanup.",
    })
    contract = put(folder / "contract.json", {
        "scope": "diagnostic", "level": 12,
        "boundary": "compress-into invocation through private heap cleanup",
        "canonical_inventory_sha256": digest(packet / "fixtures.json"),
    })
    files = ("program.o", "metadata.json", "runtime.json", "mech-worker")
    pinned_peers = read_json(peers)["actors"]
    bindings = {actor: {name: ref(path / name) for name in files}
                for actor, path in (("mech-control", control), ("mech-experiment", candidate))}
    identities = {
        actor: {"worker": inputs["mech-worker"]["sha256"], "metadata": inputs["metadata.json"]["sha256"]}
        for actor, inputs in bindings.items()
    }
    for peer in ("roc", "libdeflate"):
        spec = pinned_peers[peer]
        identities[peer] = {"worker": spec["binary_sha256"], "metadata": spec.get("metadata_sha256")}
    identities.update(environment_sha256=environment, contract_sha256=contract)
    frozen = load_fixtures(Evidence(packet, Findings()), read_json(policy_file))
    actual = read_json(fixture_file)
    assert [row["name"] for row in actual] == list(frozen)
    assert set(focus) <= frozen.keys()
    content = {}
    for case in actual:
        name = case["name"]
        contract_case = frozen[name]
        for field in ("input_sha256", "expected_sha256", "input_bytes", "prefix_bytes", "raw_bytes", "capacity"):
            assert case[field] == contract_case[field], (name, field)
        content[name] = {field: digest(Path(case[field])) for field in ("input", "prefix", "expected")}
        assert all(sha == digest(packet / contract_case[field]) for field, sha in content[name].items()), name
    content_sha = put(folder / "fixture-content.json", content)
    plan = {
        "objective": "qualify the 71,768-byte worker against the frozen 71,960-byte control",
        "history": "The earlier timing receipts and unresolved attribution remain in evidence/branch-on-zero. "
                   "This acquisition uses the same exact workers, oracle, bindings, and v1 decision resolution.",
        "focus_cases": focus,
        "runs": [{"id": f"{label}-{kind}-{order}", "kind": kind, "order": order, "blocks_per_case": 96}
                 for kind in ("full",) for order in ("forward", "reverse")],
        "policy": ref(policy_file), "timer": ref(timer), "driver": ref(Path(__file__).resolve()),
        "as_run_inventory": ref(fixture_file), "pinned_peers": ref(peers),
        "fixture_content_sha256": content_sha, "bindings": bindings, "identities": identities,
    }
    plan_sha = put(folder / "plan.json", plan)
    (folder / "plan.sha256").write_text(plan_sha + "\n")


def acquire(plan, candidate, kind):
    for actor in plan["bindings"].values():
        for file in actor.values():
            assert digest(q / file["path"]) == file["sha256"]
    for key in list(os.environ):
        if key.startswith("MECH_"):
            del os.environ[key]
    source = timer.read_text()
    hook = "assert digest(FIXTURES) == pins['input_inventory_sha256']"
    assert source.count(hook) == 1
    identities = plan["identities"]
    bound = {field: identities[field] for field in ("environment_sha256", "contract_sha256")}
    source = source.replace(hook, f"{hook}\npins.update({bound!r})")
    permutations = "orders = list(permutations(commands))"
    assert source.count(permutations) == 1
    source = source.replace(permutations, permutations + " * 4")
    if kind == "focus":
        selected = "selected = {case['name'] for case in cases}"
        assert source.count(selected) == 1
        source = source.replace(selected, f"selected = set({plan['focus_cases']!r})")
    for run in plan["runs"]:
        if run["kind"] != kind:
            continue
        assert not (q / run["id"]).exists()
        print(f"Acquiring {run['id']}", flush=True)
        sys.argv = [str(timer), run["id"], str(control), str(candidate)]
        if run["order"] == "reverse":
            sys.argv.append("reverse")
        exec(compile(source, str(timer), "exec"), {"__file__": str(timer), "__name__": "__main__"})


def replay(plan, folder, kind):
    policy = read_json(policy_file)
    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    canonical = folder / "canonical-pins"
    canonical.mkdir(exist_ok=True)
    content = read_json(folder / "fixture-content.json")
    assert digest(folder / "fixture-content.json") == plan["fixture_content_sha256"]
    for case in read_json(fixture_file):
        name = case["name"]
        for field, sha in content[name].items():
            assert digest(Path(case[field])) == sha == digest(packet / fixtures[name][field]), (name, field)
    evidence = Evidence(q, findings)
    runs = []
    receipts = {}
    for entry in plan["runs"]:
        if entry["kind"] != kind and kind == "focus":
            continue
        run_id = entry["id"]
        outcome = q / run_id
        pins = read_json(outcome / "pins.json")
        assert pins["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
        normalized = canonical / f"{run_id}.json"
        reconciled = pins | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
        if normalized.exists():
            assert read_json(normalized) == reconciled
        else:
            put(normalized, reconciled)
        receipt = entry | {
            "pins": ref(normalized),
            "original_pins": ref(outcome / "pins.json"),
            "summary": ref(outcome / "summary.json"),
            "complete": ref(outcome / "complete.json"),
            "observations": {p.parent.name: ref(p) for p in sorted(outcome.glob("*/observations.json.gz"))},
            "cleanup": {actor: ref(outcome / f"{actor}.stderr") for actor in ACTORS},
        }
        result = runtime_run(receipt, fixtures, evidence, plan["identities"], policy, entry)
        assert result is not None
        if entry["kind"] == "focus":
            assert set(result["cases"]) == set(focus)
        runs.append(result)
        receipts[run_id] = receipt
    report = decide_runtime(runs, [], policy, fixtures, findings)
    report.update(decision=findings.decision, rejected=findings.rejected, unresolved=findings.unresolved,
                  plan=ref(folder / "plan.json"), receipts=receipts)
    put(folder / f"{kind}-replay.json", report)
    print(json.dumps({k: report[k] for k in ("decision", "rejected", "unresolved", "runs")}, indent=2))


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("action", choices=("prepare", "acquire", "replay"))
    ap.add_argument("label")
    ap.add_argument("--kind", choices=("focus", "full"), default="focus")
    args = ap.parse_args()
    path = q / f"{args.label}-native/final-candidate"
    folder = study / args.label
    if args.action == "prepare":
        prepare(args.label, path, folder)
    else:
        plan_file = folder / "plan.json"
        assert digest(plan_file) == (folder / "plan.sha256").read_text().strip()
        plan = read_json(plan_file)
        for name in ("policy", "timer", "driver", "as_run_inventory", "pinned_peers"):
            assert digest(q / plan[name]["path"]) == plan[name]["sha256"], name
        if args.action == "acquire":
            acquire(plan, path, args.kind)
        else:
            replay(plan, folder, args.kind)
