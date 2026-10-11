#!/usr/bin/env python3
"""Validate and interleave fresh-process NumPy/Mech EKF measurements.

Run only after building mech-driver in release mode. No compilation or other
benchmarks should run concurrently with this collector.
"""
import argparse
import contextlib
import datetime
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import sys

import numpy as np
from numpy_batched import init_inputs

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
THREAD_ENV = {key: "1" for key in (
    "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS", "NUMEXPR_NUM_THREADS", "BLIS_NUM_THREADS")}
ATOL = 1.0e-4
RTOL = 1.0e-5


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def command_output(command):
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    return result.stdout.strip() if result.returncode == 0 else f"unavailable: {result.stderr.strip()}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mech-binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=10)
    parser.add_argument("--large-samples", type=int, default=7)
    parser.add_argument("--seed", type=int, default=20261011)
    parser.add_argument("--validate-only", action="store_true")
    args = parser.parse_args()
    args.mech_binary = args.mech_binary.resolve()
    args.output = args.output.resolve()
    if not args.mech_binary.is_file():
        parser.error("build the Mech driver before collecting measurements")
    if args.samples < 1 or args.large_samples < 1:
        parser.error("sample counts must be positive")
    if (args.output / "results.json").exists():
        parser.error("results.json already exists; use a new directory to preserve that run")
    args.output.mkdir(parents=True, exist_ok=True)
    environment = {**os.environ, **THREAD_ENV}
    inputs_dir = args.output / "inputs"
    inputs_dir.mkdir(exist_ok=True)
    dumps_dir = args.output / "validation"
    dumps_dir.mkdir(exist_ok=True)
    bases = {
        "numpy-sequential": [sys.executable, str(HERE / "numpy_sequential.py")],
        "numpy-batched": [sys.executable, str(HERE / "numpy_batched.py")],
        "mech-scalar": [str(args.mech_binary), "--backend", "scalar"],
        "mech-jit": [str(args.mech_binary), "--backend", "jit"],
    }
    input_paths = {}

    def shared_input(instances):
        if instances not in input_paths:
            path = inputs_dir / f"inputs-{instances}.f32"
            values = init_inputs(instances)
            with path.open("wb") as stream:
                for name in ("velocity", "angular_velocity", "bearing"):
                    values[name].astype("<f4", copy=False).tofile(stream)
            input_paths[instances] = path
        return input_paths[instances]

    def execute(lane, instances, turns, dump=None):
        command = bases[lane] + ["--instances", str(instances), "--turns", str(turns),
                                "--warmup", "5", "--input", str(shared_input(instances))]
        if dump:
            command += ["--dump", str(dump)]
        result = subprocess.run(command, cwd=ROOT, env=environment, text=True, capture_output=True)
        if result.returncode != 0:
            raise RuntimeError(f"{lane} exited with {result.returncode}:\n{result.stdout}\n{result.stderr}")
        record = json.loads(result.stdout.strip().splitlines()[-1])
        elapsed = float(record["elapsed_s"])
        assert elapsed > 0 and np.isfinite(elapsed), record
        record["filter_turns_per_second"] = instances * turns / elapsed
        return {"lane": lane, "command": command, "result": record,
                "stdout": result.stdout, "stderr": result.stderr}

    config_text = io.StringIO()
    with contextlib.redirect_stdout(config_text):
        np.show_config()
    report = {
        "schema_version": 1,
        "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "machine": {"platform": platform.platform(), "processor": command_output(["sysctl", "-n", "machdep.cpu.brand_string"]),
                    "memory_bytes": command_output(["sysctl", "-n", "hw.memsize"]),
                    "logical_cpus": command_output(["sysctl", "-n", "hw.logicalcpu"])},
        "python": sys.version, "numpy": np.__version__, "numpy_build": config_text.getvalue(),
        "rustc": command_output(["rustc", "--version", "--verbose"]),
        "source_revision": command_output(["git", "rev-parse", "HEAD"]),
        "tracked_worktree_changes": command_output(["git", "diff", "--name-only"]),
        "mech_build": {"command": "RUSTFLAGS='-D warnings -A dead_code' cargo build --release --offline --manifest-path benchmarks/numpy-ekf/mech-driver/Cargo.toml",
                       "opt_level": 3, "codegen_units": 1, "lto": "thin",
                       "note": "dead_code allowance is scoped to this CPU-only diagnostic build; unused GPU-only helpers otherwise trigger the repository's deny-warnings flag."},
        "thread_environment": THREAD_ENV, "seed": args.seed,
        "contract": {"dtype": "float32", "checked": False, "warmup_turns": 5,
                     "unchecked_api": "Mech uses without_integrity_constraints(): validation/fault handling is removed, but the current scalar executor can still evaluate unused predicate intermediates retained in its IR; JIT optimization may eliminate them.",
                     "reset_after_warmup": True, "input_arrays_reused_each_turn": True,
                     "shared_inputs": "Three consecutive little-endian f32 arrays: velocity, angular velocity, bearing",
                     "dump_format": "C-order state (N,3), then C-order covariance (N,3,3)",
                     "validation_atol": ATOL, "validation_rtol": RTOL,
                     "timing": "Execution and per-turn state updates, including allocations performed inside each step; Mech JIT includes its final state-snapshot refresh. Excludes imports, compilation, initial allocation/setup, input binding, five warmups, reset, checksum and dump.",
                     "state_updates": "Every turn updates the batch; no turn fusion. NumPy updates in place, while Mech uses ping-pong state buffers; this does not imply identical atomic publication machinery."},
        "source_sha256": {}, "binary_sha256": sha(args.mech_binary),
        "validation": [], "discarded_processes": [], "campaigns": [],
    }
    source_files = [HERE / name for name in ("numpy_batched.py", "numpy_sequential.py", "run_comparison.py",
                    "mech-driver/Cargo.toml", "mech-driver/Cargo.lock", "mech-driver/src/main.rs")]
    source_files += [ROOT / "benchmarks/archive/compute/parallel-ekf/numpy_scalar.py", ROOT / "hosts/gpu/fixtures/ekf-kernel.mec"]
    for source in source_files:
        if source.is_file():
            report["source_sha256"][str(source.relative_to(ROOT))] = sha(source)

    def save():
        report["input_sha256"] = {str(n): sha(path) for n, path in input_paths.items()}
        (args.output / "results.json").write_text(json.dumps(report, indent=2) + "\n")

    # Check non-vector-width batch sizes and longer trajectories before timing.
    validation_cases = [(n, turns, list(bases)) for n, turns in
                        ((1, 1), (17, 4), (257, 20), (257, 100), (10000, 20))]
    validation_cases.append((100000, 20, list(bases)[1:]))
    for instances, turns, validation_lanes in validation_cases:
        reference = None
        for lane in validation_lanes:
            dump = dumps_dir / f"{lane}-{instances}-{turns}.f32"
            record = execute(lane, instances, turns, dump)
            values = np.fromfile(dump, dtype="<f4")
            assert values.size == instances * 12 and np.isfinite(values).all(), (lane, values.shape)
            if reference is None:
                reference = values.copy()
            delta = np.abs(values.astype(np.float64) - reference.astype(np.float64))
            allowed = ATOL + RTOL * np.abs(reference.astype(np.float64))
            state_error = float(delta[:instances * 3].max())
            covariance_error = float(delta[instances * 3:].max())
            passed = bool(np.all(delta <= allowed))
            report["validation"].append({"instances": instances, "turns": turns, "lane": lane,
                "reference_lane": validation_lanes[0],
                "passed": passed, "max_state_abs_error": state_error, "max_covariance_abs_error": covariance_error,
                "max_tolerance_fraction": float((delta / allowed).max()), "dump_sha256": sha(dump),
                "execution": record})
            save()
            print(f"Validation {lane} {instances}×{turns}: {passed}; state {state_error:.3g}, covariance {covariance_error:.3g}", flush=True)
            if not passed:
                raise RuntimeError("Elementwise validation failed; do not collect or publish performance results")
    if args.validate_only:
        return

    rng = random.Random(args.seed)
    for instances, sample_count, lanes in ((10000, args.samples, list(bases)),
                                           (100000, args.large_samples, list(bases)[1:])):
        turns = 20
        campaign = {"instances": instances, "turns": turns, "samples_per_lane": sample_count,
                    "lanes": lanes, "runs": [], "summaries": {}}
        report["campaigns"].append(campaign)
        for lane in lanes:
            print(f"Discarded process warmup: {lane} {instances}×{turns}", flush=True)
            report["discarded_processes"].append(execute(lane, instances, turns))
        for round_number in range(sample_count):
            order = lanes.copy()
            rng.shuffle(order)
            for lane in order:
                record = execute(lane, instances, turns)
                record["round"] = round_number
                campaign["runs"].append(record)
                rate = record["result"]["filter_turns_per_second"] / 1e6
                print(f"{instances}×{turns} round {round_number + 1}/{sample_count} {lane}: {rate:.6f} M/s", flush=True)
                save()
        for lane in lanes:
            rates = [row["result"]["filter_turns_per_second"] for row in campaign["runs"] if row["lane"] == lane]
            median = statistics.median(rates)
            campaign["summaries"][lane] = {"median_filter_turns_per_second": median,
                "mad_filter_turns_per_second": statistics.median(abs(value - median) for value in rates),
                "minimum": min(rates), "maximum": max(rates), "n": len(rates)}
        save()
    report["finished_at"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    save()
    print(json.dumps([{"instances": c["instances"], "summaries": c["summaries"]} for c in report["campaigns"]], indent=2))


if __name__ == "__main__":
    main()
