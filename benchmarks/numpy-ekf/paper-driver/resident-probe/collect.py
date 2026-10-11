#!/usr/bin/env python3
"""Validate and sample the small public-resident diagnostic, separately from kernels."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys

import numpy as np

BENCH = Path(__file__).resolve().parents[2]
ROOT = BENCH.parents[1]
sys.path.insert(0, str(BENCH))
from numpy_batched import BatchedEKF, init_inputs  # noqa: E402
from run_comparison import ATOL, RTOL, THREAD_ENV  # noqa: E402


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--paper-build-provenance", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    report_path = output / "results.json"
    if report_path.exists():
        parser.error("choose a new output directory; existing results are immutable")
    sources = [Path(__file__).resolve(), Path(__file__).resolve().with_name("main.rs"),
               BENCH / "mech-driver/ekf-numerical.mec", BENCH / "numpy_batched.py",
               BENCH / "run_comparison.py", BENCH / "paper-driver/Cargo.toml.in",
               BENCH / "paper-driver/prepare.py"]
    report = {
        "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "scope": "Small public resident f32 reactive interpreter diagnostic, not a kernel-only throughput comparison and not the nalgebra compiler planner.",
        "binary": str(binary), "binary_sha256": sha(binary),
        "source_sha256": {str(path.relative_to(ROOT)): sha(path) for path in sources},
        "paper_build_provenance": json.loads(args.paper_build_provenance.read_text()),
        "build_command": "RUSTFLAGS='-D warnings -A dead_code' cargo build --release --offline --manifest-path <paper-build>/Cargo.toml --bin numpy-ekf-resident-probe",
        "thread_environment": THREAD_ENV,
        "contract": {"dtype": "float32", "workers": 1, "warmup_turns": 5,
                     "reset_after_warmup": True, "validation_atol": ATOL,
                     "validation_rtol": RTOL, "integrity_predicates": False,
                     "publication": "Full public prepare_turn_values + publish, including scheduler, snapshot arithmetic, summary and state hash; final host-readable extraction also timed."},
        "validation": [], "discarded": [], "samples": [], "input_sha256": {},
    }

    def save():
        report_path.write_text(json.dumps(report, indent=2) + "\n")

    inputs = {}

    def shared_input(n):
        if n not in inputs:
            values = init_inputs(n)
            path = output / f"inputs-{n}.f32"
            np.concatenate([values[name] for name in ("velocity", "angular_velocity", "bearing")]).astype("<f4").tofile(path)
            inputs[n] = (path, values)
            report["input_sha256"][str(n)] = sha(path)
        return inputs[n]

    def execute(n, turns, dump=None):
        path, _ = shared_input(n)
        command = [str(binary), "--instances", str(n), "--turns", str(turns),
                   "--warmup", "5", "--input", str(path)]
        if dump is not None:
            command += ["--dump", str(dump)]
        completed = subprocess.run(command, cwd=ROOT, env={**os.environ, **THREAD_ENV},
                                   capture_output=True, text=True, timeout=180)
        record = {"command": command, "returncode": completed.returncode,
                  "stdout": completed.stdout, "stderr": completed.stderr}
        if completed.returncode:
            report["failed_execution"] = record
            save()
            raise RuntimeError(f"public resident probe failed: {completed.stderr}")
        result = json.loads(completed.stdout.strip().splitlines()[-1])
        assert result["instances"] == n and result["turns"] == turns
        assert result["workers"] == 1 and result["checked"] is False
        assert result["warmup_turns"] == 5 and result["reset_after_warmup"] is True
        assert np.isfinite(result["elapsed_s"]) and result["elapsed_s"] > 0
        assert np.isfinite(result["checksum"])
        result["filter_turns_per_second"] = n * turns / result["elapsed_s"]
        record["result"] = result
        return record

    try:
        for n, turns in ((4, 20), (256, 20), (256, 100)):
            dump = output / f"resident-{n}-{turns}.f32"
            row = execute(n, turns, dump)
            _, values = shared_input(n)
            reference = BatchedEKF(n, values)
            for _ in range(turns):
                reference.step()
            expected = np.concatenate([reference.state.ravel(), reference.covariance.ravel()])
            actual = np.fromfile(dump, dtype="<f4")
            assert actual.size == expected.size == n * 12
            difference = np.abs(actual.astype(np.float64) - expected.astype(np.float64))
            bound = ATOL + RTOL * np.abs(expected.astype(np.float64))
            passed = bool(np.all(np.isfinite(actual)) and np.all(difference <= bound))
            row.update({"passed": passed, "max_abs_error": float(difference.max()),
                        "max_tolerance_fraction": float(np.max(difference / bound)),
                        "dump_sha256": sha(dump), "reference": "numpy_batched.BatchedEKF"})
            report["validation"].append(row)
            save()
            if not passed:
                raise RuntimeError("public resident full-output validation failed; no timings collected")
        report["discarded"].append(execute(256, 20))
        save()
        for _ in range(5):
            report["samples"].append(execute(256, 20))
            save()
        rates = [sample["result"]["filter_turns_per_second"] for sample in report["samples"]]
        median = statistics.median(rates)
        report["summary"] = {"instances": 256, "turns": 20, "samples": len(rates),
                             "median_filter_turns_per_second": median,
                             "mad_filter_turns_per_second": statistics.median(abs(rate - median) for rate in rates)}
        assert sha(binary) == report["binary_sha256"]
        assert all(sha(ROOT / path) == value for path, value in report["source_sha256"].items())
        report["completed_at"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        save()
        print(json.dumps(report["summary"], indent=2))
    except Exception as error:
        report["error"] = str(error)
        save()
        raise


if __name__ == "__main__":
    main()
