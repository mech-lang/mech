#!/usr/bin/env python3
"""Small selection pilot; none of these samples belong to the final campaign."""

import contextlib
import datetime
import hashlib
import io
import json
import math
import os
import platform
import random
import statistics
import subprocess
import sys
import time
from pathlib import Path

import numpy as np


OUTPUT = Path(__file__).resolve().parent
HERE = OUTPUT.parents[1]
ROOT = HERE.parents[1]
PREVIOUS = OUTPUT.parent / "apple-m1-2026-10-11"
THREAD_ENV = {name: "1" for name in (
    "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS", "BLIS_NUM_THREADS",
)}
SEED = 2026101102
ATOL = 1e-4
RTOL = 1e-5


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def capture(command):
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    return {"command": command, "returncode": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def main():
    destination = OUTPUT / "results.json"
    if destination.exists():
        raise SystemExit("Refusing to overwrite an existing pilot")
    (OUTPUT / "validation").mkdir(exist_ok=True)
    config_text = io.StringIO()
    with contextlib.redirect_stdout(config_text):
        np.show_config()
    report = {
        "schema_version": 1,
        "purpose": "implementation-selection pilot only; final measurements must use new independent samples",
        "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "selection_rule": "Highest geometric mean of median filter-turns/s over 10,000 and 100,000 filters, including unchanged baseline as a candidate",
        "workloads": [{"instances": n, "turns": 20} for n in (10000, 100000)],
        "warmup_turns": 5, "reset_after_warmup": True,
        "discarded_processes_per_lane_per_workload": 1,
        "measured_processes_per_lane_per_workload": 3,
        "seed": SEED, "thread_environment": THREAD_ENV,
        "python": sys.version, "python_executable": sys.executable,
        "numpy_version": np.__version__, "numpy_build": config_text.getvalue(),
        "platform": platform.platform(),
        "machine": capture(["sysctl", "-n", "machdep.cpu.brand_string"]),
        "source_revision": capture(["git", "rev-parse", "HEAD"]),
        "source_sha256": {str(path.relative_to(ROOT)): sha(path) for path in (
            Path(__file__).resolve(), HERE / "numpy_batched.py", HERE / "numpy_variants.py",
            HERE / "test_numpy_variants.py",
        )},
        "shared_inputs": {str(n): {"path": str(PREVIOUS / "inputs" / f"inputs-{n}.f32"),
                                   "sha256": sha(PREVIOUS / "inputs" / f"inputs-{n}.f32")}
                          for n in (10000, 100000)},
        "validation_tolerance": {"absolute": ATOL, "relative": RTOL},
        "discarded": [], "measured": [], "validation": [], "summaries": {},
    }
    commands = {
        "numpy-batched": [sys.executable, str(HERE / "numpy_batched.py")],
        **{f"numpy-{variant}": [sys.executable, str(HERE / "numpy_variants.py"), "--variant", variant]
           for variant in ("einsum-aos", "einsum-soa", "matmul-soa")},
    }
    environment = {**os.environ, **THREAD_ENV}
    rng = random.Random(SEED)

    def save():
        destination.write_text(json.dumps(report, indent=2) + "\n")

    def run(lane, instances, sample, discarded=False):
        command = commands[lane] + ["--instances", str(instances), "--turns", "20", "--warmup", "5",
                                    "--input", report["shared_inputs"][str(instances)]["path"]]
        dump = OUTPUT / "validation" / f"{lane}-{instances}-20.f32"
        if discarded:
            command.extend(["--dump", str(dump)])
        began = datetime.datetime.now(datetime.timezone.utc).isoformat()
        wall_started = time.perf_counter()
        completed = subprocess.run(command, cwd=ROOT, env=environment, text=True, capture_output=True)
        record = {
            "lane": lane, "instances": instances, "turns": 20, "round": sample,
            "command": command, "started_at": began,
            "process_wall_seconds": time.perf_counter() - wall_started,
            "returncode": completed.returncode,
            "stdout": completed.stdout, "stderr": completed.stderr,
        }
        report["discarded" if discarded else "measured"].append(record)
        save()
        if completed.returncode != 0:
            raise RuntimeError(record)
        result = json.loads(completed.stdout.strip().splitlines()[-1])
        assert result["elapsed_s"] > 0 and math.isfinite(result["elapsed_s"])
        assert math.isfinite(result["checksum"])
        record["result"] = result
        if discarded:
            reference_path = PREVIOUS / "validation" / f"numpy-batched-{instances}-20.f32"
            reference = np.fromfile(reference_path, dtype="<f4").astype(np.float64)
            actual = np.fromfile(dump, dtype="<f4").astype(np.float64)
            assert actual.size == instances * 12 and np.isfinite(actual).all()
            difference = np.abs(actual - reference)
            bound = ATOL + RTOL * np.abs(reference)
            valid = bool(np.all(difference <= bound))
            validation = {
                "lane": lane, "instances": instances, "turns": 20, "passed": valid,
                "reference": str(reference_path), "reference_sha256": sha(reference_path),
                "dump": str(dump.relative_to(OUTPUT)), "dump_sha256": sha(dump),
                "max_absolute_error": float(difference.max()),
                "max_fraction_of_tolerance_bound": float(np.max(difference / bound)),
            }
            report["validation"].append(validation)
            save()
            if not valid:
                raise RuntimeError(f"Validation failed: {validation}")
        save()
        print(f"{instances:>6} {lane:<17} {'discard' if discarded else 'sample ' + str(sample)}: "
              f"{result['filter_turns_per_second'] / 1e6:.6f} M filter-turns/s", flush=True)

    for instances in (10000, 100000):
        order = list(commands)
        rng.shuffle(order)
        for lane in order:
            run(lane, instances, -1, discarded=True)
        for sample in range(3):
            order = list(commands)
            rng.shuffle(order)
            for lane in order:
                run(lane, instances, sample)
    for lane in commands:
        medians = []
        workloads = {}
        for instances in (10000, 100000):
            values = [entry["result"]["filter_turns_per_second"] for entry in report["measured"]
                      if entry["lane"] == lane and entry["instances"] == instances]
            median = statistics.median(values)
            medians.append(median)
            workloads[str(instances)] = {"median_filter_turns_per_second": median,
                                         "mad_filter_turns_per_second": statistics.median(abs(x - median) for x in values),
                                         "samples": values}
        report["summaries"][lane] = {"workloads": workloads,
                                       "geometric_mean_median_throughput": math.sqrt(medians[0] * medians[1])}
    report["selected_candidate"] = max(report["summaries"], key=lambda lane: report["summaries"][lane]["geometric_mean_median_throughput"])
    report["completed_at"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    save()
    print("SELECTED:", report["selected_candidate"], flush=True)
    print(json.dumps(report["summaries"], indent=2), flush=True)


if __name__ == "__main__":
    main()
