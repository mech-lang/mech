"""Replay the exact two reduced-size scorer screens under qualification v1."""

from argparse import ArgumentParser
from pathlib import Path
import sys

archive = Path(__file__).resolve().parent
parser = ArgumentParser(description=__doc__)
parser.add_argument("--packet", type=Path, default=archive.parent.parent)
packet = parser.parse_args().packet.resolve(strict=True)
sys.path.insert(0, str(packet))
from qualification_records import Evidence, Findings, digest, load_fixtures, read_json
from qualification_timings import decide_runtime, runtime_run

study = archive / "shrink"
plan_file = study / "plan.json"
plan = read_json(plan_file)
record = read_json(study / "focus-replay.json")
assert digest(plan_file) == (study / "plan.sha256").read_text().strip()
assert digest(plan_file) == record["plan"]["sha256"]
for key, name in {
    "timer": "compare-library.py",
    "driver": "timing.py",
    "as_run_inventory": "fixtures-as-run.json",
    "pinned_peers": "peer-pins.json",
    "scorer_probe": "scorer-probe.py",
    "link_and_verify": "link-and-verify.py",
}.items():
    assert digest(archive / "as-run" / name) == plan[key]["sha256"], key
policy_path = packet / "qualification-policy-v1.json"
assert digest(policy_path) == plan["policy"]["sha256"]
policy = read_json(policy_path)
findings = Findings()
fixtures = load_fixtures(Evidence(packet, findings), policy)
evidence = Evidence(archive, findings)
for key in ("environment", "contract"):
    assert digest(study / f"{key}.json") == plan["identities"][f"{key}_sha256"]
frozen = study / "fixture-content.json"
assert digest(frozen) == plan["fixture_content_sha256"]
digests = read_json(frozen)
assert digests.keys() == fixtures.keys()
for case, contents in digests.items():
    assert set(contents) == {"input", "prefix", "expected"}
    for field, sha in contents.items():
        assert digest(packet / fixtures[case][field]) == sha, (case, field)

for actor, name in (("mech-control", "control"), ("mech-experiment", "shrink")):
    files = plan["bindings"][actor]
    for filename in ("metadata.json", "runtime.json"):
        assert digest(archive / "identities" / name / filename) == files[filename]["sha256"]

def at(reference, path):
    return reference | {"path": path}

runs = []
for item in plan["runs"]:
    if item["kind"] != "focus":
        continue
    rid = item["id"]
    ref = record["receipts"][rid]
    dirname = f"raw/{rid}"
    orig = at(ref["original_pins"], f"{dirname}/pins.json")
    normalized = at(ref["pins"], f"shrink/canonical-pins/{rid}.json")
    before = evidence.json(orig, f"{rid} original bindings")
    after = evidence.json(normalized, f"{rid} canonical bindings")
    assert before["input_inventory_sha256"] == plan["as_run_inventory"]["sha256"]
    assert after == before | {"input_inventory_sha256": policy["fixture_inventory_sha256"]}
    rebased = ref | {
        "original_pins": orig, "pins": normalized,
        "summary": at(ref["summary"], f"{dirname}/summary.json"),
        "complete": at(ref["complete"], f"{dirname}/complete.json"),
        "observations": {case: at(file, f"{dirname}/{case}/observations.json.gz")
                         for case, file in ref["observations"].items()},
        "cleanup": {actor: at(file, f"{dirname}/{actor}.stderr")
                    for actor, file in ref["cleanup"].items()},
    }
    result = runtime_run(rebased, fixtures, evidence, plan["identities"], policy, item)
    assert result is not None and set(result["cases"]) == set(plan["focus_cases"])
    runs.append(result)
assert {run["order"] for run in runs} == {"forward", "reverse"}
actual = decide_runtime(runs, [], policy, fixtures, findings)
assert all(value == record[key] for key, value in actual.items())
assert findings.decision == record["decision"] == "Reject"
assert findings.rejected == record["rejected"] and findings.unresolved == record["unresolved"]
for line in (f"Focused v1 replay: {findings.decision}", *findings.rejected, *findings.unresolved):
    print(line)
