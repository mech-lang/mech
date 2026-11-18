"""Run the predeclared repeatability diagnostic with fresh processes per epoch."""

import argparse
from importlib import import_module
import json
from pathlib import Path
import shutil
from time import time_ns

measure = import_module("measure-cold-epilogues-followup")

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("plan", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("comparison_inputs", type=Path)
parser.add_argument("candidate", type=Path)
args = parser.parse_args()

plan = json.loads(args.plan.read_text())
assert plan["resolution"] == measure.RESOLUTION
args.output.mkdir()
(args.output / "plan.json").write_bytes(args.plan.read_bytes())
duplicate = args.output / "duplicate-control"
duplicate.mkdir()
for name in ("mech-worker", "metadata.json"):
    shutil.copy2(args.comparison_inputs / "control" / name, duplicate / name)

epochs = []
for index, settings in enumerate(plan["epochs"], 1):
    start = time_ns()
    output = args.output / f"epoch-{index:02d}"
    measure.measure(
        args.comparison_inputs.resolve(), output.resolve(), plan["cases"], settings["seed"],
        plan["repetitions_per_order"], args.candidate.resolve(),
        duplicate.resolve() if settings["control_files"] == "copied" else None,
        settings["batch_floor"],
    )
    study = json.loads((output / "study.json").read_text())
    assert study["worker_sha256"] == plan["worker_sha256"]
    assert study["distinct_worker_files"] == (3 if settings["control_files"] == "copied" else 2)
    epochs.append({
        "directory": output.name, "settings": settings, "started_unix_ns": start,
        "ended_unix_ns": time_ns(), "cases": json.loads((output / "results.json").read_text()),
    })
    (args.output / "summary.json").write_text(json.dumps(epochs, indent=2) + "\n")
