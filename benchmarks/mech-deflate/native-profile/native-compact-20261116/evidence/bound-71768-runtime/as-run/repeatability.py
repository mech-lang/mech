"""Resolve the inconclusive full-suite bounds using distinct duplicate controls."""

from hashlib import file_digest
import json
from pathlib import Path
import shutil
import sys

study = Path(__file__).resolve().parent
q = study.parent
trial = study / "fully-bound-71768"
sys.path.insert(0, str(trial))
sys.path.insert(0, str(q))
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, duplicate_study, runtime_run

output = study / "followup"
input_dir = study / "followup-inputs"
inputs_plan = study / "followup-plan.json"
initial_plan = read_json(trial / "plan.json")
initial = read_json(trial / "full-replay.json")
assert not initial["rejected"] and initial["decision"] == "Unresolved"
assert len(initial["runs"]) == 2 and all(len(run["peers"]) == 2 for run in initial["runs"])
assert digest(trial / "plan.json") == (trial / "plan.sha256").read_text().strip()
assert digest(packet / "qualification-policy-v1.json") == initial_plan["policy"]["sha256"]
policy = read_json(packet / "qualification-policy-v1.json")
names = [name for name, result in initial["cases"].items() if result["decision"] == "unresolved"]
assert names and all(run["kind"] == "full" for run in initial["runs"])
control = q / "stale-cache-accounting/pinned-smaller"
candidate = q / "native-size-20261117/source-aligned"
duplicate = input_dir / "duplicate-control"
keys = ("environment_sha256", "contract_sha256")
bindings = {key: initial_plan["identities"][key] for key in keys}
driver = q / "measure-cold-epilogues-followup.py"


def put(path, result):
    with path.open("x") as stream:
        json.dump(result, stream, indent=2)
        stream.write("\n")
    return digest(path)


def ref(file):
    return {"path": str(file.relative_to(q)), "sha256": digest(file)}


def prepare():
    duplicate.mkdir(parents=True)
    for name in ("mech-worker", "metadata.json"):
        shutil.copy2(control / name, duplicate / name)
        assert digest(control / name) == digest(duplicate / name)
    (input_dir / "control").symlink_to(control)
    (input_dir / "fixtures.json").symlink_to(q / "fixtures.json")
    (input_dir / "full").symlink_to(q / "fully-bound-71768-full-forward")
    shutil.copyfile(trial / "full-replay.json", input_dir / "cold-uncertainty.json")
    workers = {role: digest(path / "mech-worker") for role, path in (
        ("control", control), ("duplicate-control", duplicate), ("candidate", candidate)
    )}
    assert workers["control"] == initial_plan["identities"]["mech-control"]["worker"]
    assert workers["candidate"] == initial_plan["identities"]["mech-experiment"]["worker"]
    plan = {
        "initial_replay_sha256": digest(trial / "full-replay.json"),
        "initial_plan_sha256": digest(trial / "plan.json"),
        "cases": names, "worker_sha256": workers,
        "repetitions_per_order": 32, "batch_floor": 64, "seed": 9937176802,
        "driver": ref(driver), "driver_wrapper": ref(Path(__file__).resolve()),
        **bindings,
    }
    (study / "followup-plan.sha256").write_text(put(inputs_plan, plan) + "\n")


def acquire(plan):
    raw = driver.read_text()
    site = "worker_sha256=identities, distinct_worker_files=distinct_files, batch_floor=batch_floor,"
    assert raw.count(site) == 1
    edits = " ".join(f"{key}={value!r}," for key, value in bindings.items())
    module = {"__file__": str(driver), "__name__": "qualified_followup"}
    exec(compile(raw.replace(site, site + " " + edits), str(driver), "exec"), module)
    module["measure"](input_dir, output, names, plan["seed"], plan["repetitions_per_order"],
                      candidate, duplicate, plan["batch_floor"])


def replay(plan):
    actual = read_json(output / "study.json")
    for field in ("worker_sha256", "seed", "repetitions_per_order", "batch_floor", *keys):
        assert actual[field] == plan[field], field
    assert len(actual["order"]) == len(names) and set(actual["order"]) == set(names)
    assert actual["distinct_worker_files"] == 3
    findings = Findings()
    evidence = Evidence(q, findings)
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    identities = initial_plan["identities"]
    full = [runtime_run(initial["receipts"][entry["id"]], fixtures, evidence, identities, policy, entry)
            for entry in initial_plan["runs"]]
    assert all(r is not None for r in full)
    follow = {"id": "distinct-control-bound-followup-v1", "study": ref(output / "study.json"),
              "results": ref(output / "results.json"),
              "observations": {name: ref(output / f"{name}.json.gz") for name in names}}
    repeats = duplicate_study(follow, fixtures, evidence, identities, policy)
    report = decide_runtime(full, [(follow["id"], repeats)], policy, fixtures, findings)
    report.update(decision=findings.decision, rejected=findings.rejected, unresolved=findings.unresolved,
                  initial=ref(trial / "full-replay.json"), plan=ref(inputs_plan), followup=follow)
    put(trial / "full-and-repeat-replay.json", report)
    print(json.dumps({key: report[key] for key in ("decision", "rejected", "unresolved", "runs")}, indent=2))


action, = sys.argv[1:]
if action == "prepare":
    prepare()
else:
    assert digest(inputs_plan) == (study / "followup-plan.sha256").read_text().strip()
    plan = read_json(inputs_plan)
    assert names == plan["cases"] and digest(trial / "full-replay.json") == plan["initial_replay_sha256"]
    assert digest(trial / "plan.json") == plan["initial_plan_sha256"]
    assert digest(driver) == plan["driver"]["sha256"]
    assert digest(Path(__file__).resolve()) == plan["driver_wrapper"]["sha256"]
    if action == "acquire":
        acquire(plan)
    else:
        assert action == "replay"
        replay(plan)
