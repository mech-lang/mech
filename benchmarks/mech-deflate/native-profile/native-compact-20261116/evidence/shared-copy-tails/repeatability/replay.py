"""Replay the original focus and the separately bound 4 KiB control study."""

from pathlib import Path
import runpy

folder = Path(__file__).resolve().parent
archive = folder.parent
prior = runpy.run_path(str(archive / "replay-archive.py"))
from qualification_records import Evidence, Findings, digest, read_json
from qualification_timings import decide_runtime, duplicate_study

plan_file = folder / "plan.json"
assert digest(plan_file) == (folder / "plan.sha256").read_text().strip()
plan = read_json(plan_file)
assert digest(archive / "plan.json") == plan["focus_plan_sha256"]
assert digest(archive / "focus-replay.json") == plan["focus_replay_sha256"]
assert digest(folder / "as-run/followup.py") == plan["wrapper_sha256"]
assert digest(folder / "as-run/measure-cold-epilogues-followup.py") == plan["driver_sha256"]
receipt = read_json(folder / "followup/study.json")
for key in ("worker_sha256", "seed", "repetitions_per_order", "batch_floor",
            "environment_sha256", "contract_sha256"):
    assert receipt[key] == plan[key]
assert receipt["order"] == [plan["case"]]
for actor in ("control", "duplicate-control", "candidate"):
    log = folder / "followup" / f"{actor}.stderr"
    assert log.read_text().rstrip().endswith("CLEANUP private_payload_bytes=0 live_owner_records=0")

report = read_json(folder / "combined-replay.json")
assert report["plan_sha256"] == digest(plan_file)
assert report["focus_replay_sha256"] == plan["focus_replay_sha256"]
findings = Findings()
row = report["followup"]
repeat = duplicate_study(row, prior["fixtures"], Evidence(folder, findings),
                         prior["plan"]["identities"], prior["policy"])
result = decide_runtime(prior["results"], [(row["id"], repeat)], prior["policy"], prior["fixtures"], findings)
assert all(value == report[key] for key, value in result.items())
assert findings.decision == report["decision"]
assert findings.rejected == report["rejected"] and findings.unresolved == report["unresolved"]
print(f"Combined focused and duplicate-control replay {findings.decision}")
for reason in findings.rejected + findings.unresolved:
    print(reason)
