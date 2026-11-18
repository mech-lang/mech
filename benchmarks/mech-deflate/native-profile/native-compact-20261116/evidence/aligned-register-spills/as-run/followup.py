"""Run the single predeclared duplicate-control follow-up when required."""

import json
from pathlib import Path
import shutil

study = Path(__file__).resolve().parent
q = study.parent
plan = json.loads((study / "plan.json").read_text())
initial = study / "initial-runtime-policy.json"
report = json.loads(initial.read_text())
assert report["plan_sha256"] == (study / "plan.sha256").read_text().strip()
if report["timing_only_findings"]["rejected"]:
    print("The timing policy found an established failure. The conditional follow-up is not triggered.")
    raise SystemExit(0)
cases = [name for name, row in report["cases"].items() if row["decision"] == "unresolved"]
if not cases:
    print("Every paired case is bounded. The conditional follow-up is not triggered.")
    raise SystemExit(0)
root = study / "repeatability-inputs"
root.mkdir()
(root / "cold-uncertainty.json").symlink_to(initial)
(root / "fixtures.json").symlink_to(q / "fixtures.json")
(root / "full").symlink_to(q / "aligned-register-spills-full")
(root / "control").symlink_to(q / "cranelift-width-16-owners-cold-native/final-candidate")
duplicate = root / "duplicate-control"
duplicate.mkdir()
for name in ("mech-worker", "metadata.json"):
    shutil.copy2(root / "control" / name, duplicate / name)
script = q / "measure-cold-epilogues-followup.py"
source = script.read_text()
point = 'worker_sha256=identities, distinct_worker_files=distinct_files, batch_floor=batch_floor,'
assert source.count(point) == 1
binding = "\n        ".join(f"{field}={plan[field]!r},"
                            for field in ("environment_sha256", "contract_sha256"))
namespace = {"__file__": str(script), "__name__": "aligned_repeatability"}
exec(compile(source.replace(point, point + "\n        " + binding), str(script), "exec"), namespace)
rule = plan["conditional_repeatability"]
namespace["measure"](root, study / "repeatability", cases, rule["seed"],
                     rule["repetitions_per_order"], q / "aligned-register-spills-native/final-candidate",
                     duplicate, rule["batch_floor"])
