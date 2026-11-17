"""Predeclare and check the copy-tail screen's conflicting random-input case."""

import json
from pathlib import Path
import shutil
import sys

study = Path(__file__).resolve().parent
q = study.parent
original = Path("/private/tmp/mech-text-reductions.yYja14")
frozen = original / "bound-plan"
packet = q / "native-review-pr/staging/benchmarks/mech-deflate/native-profile/native-compact-20261116"
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, duplicate_study, runtime_run

driver = q / "measure-cold-epilogues-followup.py"
control = q / "native-size-20261117/source-aligned"
candidate = original / "static-candidate"
inputs = study / "inputs"
duplicate = inputs / "duplicate-control"
out = study / "followup"
plan_file = study / "plan.json"
first_plan = read_json(frozen / "plan.json")
first = read_json(frozen / "focus-replay.json")
case, = [name for name, row in first["cases"].items() if row["decision"] == "unresolved"]
assert case == "prefix-random-4096" and first["cases"][case]["adverse_runs"]
identities = first_plan["identities"]
binding = {key: identities[key] for key in ("environment_sha256", "contract_sha256")}
workers = {"control": identities["mech-control"]["worker"], "duplicate-control": identities["mech-control"]["worker"],
           "candidate": identities["mech-experiment"]["worker"]}
plan = {"case": case, "worker_sha256": workers, "focus_plan_sha256": digest(frozen / "plan.json"),
        "focus_replay_sha256": digest(frozen / "focus-replay.json"),
        "driver_sha256": digest(driver), "wrapper_sha256": digest(Path(__file__).resolve()),
        "batch_floor": 64, "repetitions_per_order": 64, "seed": 11719771492, **binding}


def write(file, value):
    with file.open("x") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def ref(file):
    return {"path": str(file.relative_to(study)), "sha256": digest(file)}


mode, = sys.argv[1:]
assert workers["control"] == digest(control / "mech-worker")
assert workers["candidate"] == digest(candidate / "mech-worker")
for actor, path in (("mech-control", control), ("mech-experiment", candidate)):
    assert digest(path / "metadata.json") == identities[actor]["metadata"]
if mode == "prepare":
    duplicate.mkdir(parents=True)
    for name in ("mech-worker", "metadata.json"):
        shutil.copy2(control / name, duplicate / name)
    for name, source in (("control", control), ("fixtures.json", q / "fixtures.json"),
                         ("full", original / "shared-copy-tails-focus-forward"),
                         ("cold-uncertainty.json", frozen / "focus-replay.json")):
        (inputs / name).symlink_to(source)
    write(plan_file, plan)
    (study / "plan.sha256").write_text(digest(plan_file) + "\n")
    sys.exit()

assert plan == read_json(plan_file)
assert digest(plan_file) == (study / "plan.sha256").read_text().strip()
assert digest(duplicate / "mech-worker") == workers["duplicate-control"]
if mode == "acquire":
    original_driver = driver.read_text()
    site = "worker_sha256=identities, distinct_worker_files=distinct_files, batch_floor=batch_floor,"
    assert original_driver.count(site) == 1
    source = original_driver.replace(site, site + " " + " ".join(f"{k}={v!r}," for k, v in binding.items()))
    module = {"__file__": str(driver), "__name__": "copy_tail_followup"}
    exec(compile(source, str(driver), "exec"), module)
    module["measure"](inputs, out, [case], plan["seed"], plan["repetitions_per_order"], candidate, duplicate, plan["batch_floor"])
else:
    assert mode == "replay"
    receipt = read_json(out / "study.json")
    for key in ("worker_sha256", "seed", "repetitions_per_order", "batch_floor", *binding):
        assert receipt[key] == plan[key], key
    assert receipt["order"] == [case]
    policy = read_json(packet / "qualification-policy-v1.json")
    findings = Findings()
    fixtures = load_fixtures(Evidence(packet, findings), policy)
    raw_focus = Evidence(original, findings)
    runs = [runtime_run(first["receipts"][row["id"]], fixtures, raw_focus, identities, policy, row)
            for row in first_plan["runs"] if row["kind"] == "focus"]
    assert len(runs) == 2 and all(run is not None for run in runs)
    entry = {"id": "copy-tail-distinct-control", "study": ref(out / "study.json"),
             "results": ref(out / "results.json"), "observations": {case: ref(out / f"{case}.json.gz")}}
    repeat = duplicate_study(entry, fixtures, Evidence(study, findings), identities, policy)
    result = decide_runtime(runs, [(entry["id"], repeat)], policy, fixtures, findings)
    assert result["cases"][case]["adverse_runs"] == first["cases"][case]["adverse_runs"]
    result.update(decision=findings.decision, rejected=findings.rejected, unresolved=findings.unresolved,
                  followup=entry, focus_replay_sha256=plan["focus_replay_sha256"], plan_sha256=digest(plan_file))
    write(study / "combined-replay.json", result)
    print(json.dumps({"result": result["decision"], case: result["cases"][case],
                      "rejected": findings.rejected, "unresolved": findings.unresolved}, indent=2))
