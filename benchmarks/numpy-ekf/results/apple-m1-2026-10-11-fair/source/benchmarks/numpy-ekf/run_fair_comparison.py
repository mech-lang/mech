#!/usr/bin/env python3
"""Expanded, independently sampled numerical-kernel comparison.

Build the pinned native driver and finish the separate NumPy selection pilot first.
Do not run this alongside compilation or another benchmark.
"""
import argparse
import contextlib
import datetime
import io
import json
import os
from pathlib import Path
import random
import shutil
import statistics
import subprocess
import sys
import time

import numpy as np

from numpy_batched import init_inputs
from run_comparison import ATOL, RTOL, HERE, ROOT, THREAD_ENV, command_output, sha

PAPER_REVISION = "0d85319d60c460cdecb81e03c181d62ecfe1ef39"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--paper-binary", type=Path, required=True)
    parser.add_argument("--paper-build-provenance", type=Path, required=True)
    parser.add_argument("--pilot", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=10)
    parser.add_argument("--large-samples", type=int, default=7)
    parser.add_argument("--seed", type=int, default=20261012)
    parser.add_argument("--validate-only", action="store_true")
    args = parser.parse_args()
    for field in ("paper_binary", "paper_build_provenance", "pilot", "output"):
        setattr(args, field, getattr(args, field).resolve())
    for binary in (args.paper_binary,):
        if not binary.is_file():
            parser.error(f"build the native driver first: {binary}")
    if min(args.samples, args.large_samples) < 1:
        parser.error("sample counts must be positive")
    if (args.output / "results.json").exists():
        parser.error("use a new output directory to preserve the existing run")
    pilot = json.loads(args.pilot.read_text())
    # The selected candidate is fixed before this collector's independent runs.
    selected = max(pilot["summaries"], key=lambda lane: pilot["summaries"][lane]["geometric_mean_median_throughput"])
    if pilot.get("selected_candidate") != "numpy-einsum-soa" or selected != "numpy-einsum-soa":
        parser.error("expected the separately recorded component-major einsum selection pilot")
    for relative, expected_hash in pilot["source_sha256"].items():
        if sha(ROOT / relative) != expected_hash:
            parser.error(f"pilot source changed before independent sampling: {relative}")
    paper_build = json.loads(args.paper_build_provenance.read_text())
    if paper_build["revision"] != PAPER_REVISION:
        parser.error("paper build does not match the declared source revision")
    args.output.mkdir(parents=True, exist_ok=True)
    input_dir = args.output / "inputs"
    dump_dir = args.output / "validation"
    input_dir.mkdir(exist_ok=True)
    dump_dir.mkdir(exist_ok=True)
    provenance_dir = args.output / "paper-build"
    provenance_dir.mkdir(exist_ok=True)
    paper_manifest = Path(paper_build["manifest"])
    for path in (args.paper_build_provenance, paper_manifest, paper_manifest.with_name("Cargo.lock")):
        shutil.copy2(path, provenance_dir / path.name)
    env = {**os.environ, **THREAD_ENV}
    bases = {
        "numpy-matmul-aos": [sys.executable, str(HERE / "numpy_batched.py")],
        "numpy-einsum-soa": [sys.executable, str(HERE / "numpy_variants.py"), "--variant", "einsum-soa"],
        "mech-scalar-ir": [str(args.paper_binary), "--backend", "scalar"],
        "mech-simd-ir": [str(args.paper_binary), "--backend", "simd"],
        "mech-scalar-jit": [str(args.paper_binary), "--backend", "jit"],
        "mech-simd-jit": [str(args.paper_binary), "--backend", "jit-simd"],
        "mech-simd-aot": [str(args.paper_binary), "--backend", "aot-simd"],
    }
    reference_base = [sys.executable, str(HERE / "numpy_sequential.py")]
    input_paths = {}

    def shared_input(n):
        if n not in input_paths:
            path = input_dir / f"inputs-{n}.f32"
            values = init_inputs(n)
            with path.open("wb") as stream:
                for name in ("velocity", "angular_velocity", "bearing"):
                    values[name].astype("<f4", copy=False).tofile(stream)
            input_paths[n] = path
        return input_paths[n]

    def execute(lane, base, n, turns, dump=None):
        command = base + ["--instances", str(n), "--turns", str(turns),
                          "--warmup", "5", "--input", str(shared_input(n))]
        if dump is not None:
            command += ["--dump", str(dump)]
        start = time.perf_counter()
        result = subprocess.run(command, cwd=ROOT, env=env, text=True, capture_output=True)
        wall = time.perf_counter() - start
        if result.returncode:
            raise RuntimeError(f"{lane} failed ({result.returncode}):\n{result.stdout}\n{result.stderr}")
        output = json.loads(result.stdout.strip().splitlines()[-1])
        elapsed = float(output["elapsed_s"])
        checksum = float(output["checksum"])
        if not (elapsed > 0 and np.isfinite(elapsed) and np.isfinite(checksum)):
            raise RuntimeError(f"invalid measurement: {output}")
        if output["instances"] != n or output["turns"] != turns:
            raise RuntimeError(f"incorrect work count: {output}")
        if output.get("checked") is not False or output.get("warmup", output.get("warmup_turns")) != 5:
            raise RuntimeError(f"incorrect checking or warmup contract: {output}")
        if lane.startswith("mech-") and output.get("workers") != 1:
            raise RuntimeError(f"incorrect Mech worker count: {output}")
        output["filter_turns_per_second"] = n * turns / elapsed
        return {"lane": lane, "command": command, "process_wall_seconds": wall,
                "result": output, "stdout": result.stdout, "stderr": result.stderr}

    numpy_info = io.StringIO()
    with contextlib.redirect_stdout(numpy_info):
        np.show_config()
        np.show_runtime()
    source_files = [HERE / name for name in (
        "numpy_batched.py", "numpy_variants.py", "numpy_sequential.py",
        "run_comparison.py", "run_fair_comparison.py",
        "mech-driver/Cargo.toml", "mech-driver/Cargo.lock", "mech-driver/src/main.rs")]
    source_files.append(ROOT / "benchmarks/archive/compute/parallel-ekf/numpy_scalar.py")
    for directory in (HERE / "mech-driver", HERE / "paper-driver"):
        if directory.exists():
            for path in directory.rglob("*"):
                if "target" in path.parts or not path.is_file():
                    continue
                if path.suffix in (".rs", ".mec", ".toml", ".lock", ".py", ".sh", ".json") or path.name == "Cargo.toml.in":
                    source_files.append(path)
    for path in sorted(set(source_files)):
        destination = args.output / "source" / path.relative_to(ROOT)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, destination)
    report = {
        "schema_version": 2,
        "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "machine": {"platform": sys.platform,
                    "os": command_output(["sw_vers"]),
                    "cpu": command_output(["sysctl", "-n", "machdep.cpu.brand_string"]),
                    "memory_bytes": command_output(["sysctl", "-n", "hw.memsize"]),
                    "logical_cpus": command_output(["sysctl", "-n", "hw.logicalcpu"])},
        "python": sys.version, "numpy": np.__version__, "numpy_build_runtime": numpy_info.getvalue(),
        "rustc": command_output(["rustc", "--version", "--verbose"]),
        "harness_checkout_revision": command_output(["git", "rev-parse", "HEAD"]),
        "all_mech_backends_revision": PAPER_REVISION,
        "build": {"rustflags": "-D warnings -A dead_code", "profile": "release",
                  "opt_level": 3, "codegen_units": 1, "lto": "thin", "debug": False},
        "paper_build_provenance": paper_build,
        "paper_build_sha256": {path.name: sha(path) for path in sorted(provenance_dir.iterdir())},
        "tracked_worktree_changes": command_output(["git", "diff", "--name-only"]),
        "binary_sha256": {"paper": sha(args.paper_binary)},
        "source_sha256": {str(path.relative_to(ROOT)): sha(path) for path in sorted(set(source_files))},
        "pilot": {"path": str(args.pilot), "sha256": sha(args.pilot), "selected": "numpy-einsum-soa",
                  "selection": "Geometric mean of median throughput at both pilot batch sizes; final samples independent."},
        "thread_environment": THREAD_ENV,
        "numpy_disabled_cpu_features": env.get("NPY_DISABLE_CPU_FEATURES", ""),
        "seed": args.seed,
        "interpreter_scope": "Scalar/SIMD rows evaluate lowered fixed-shape instructions, not the general reactive runtime or a nalgebra source interpreter.",
        "contract": {
            "dtype": "float32", "user_integrity_predicates": False,
            "mech_source": "Common constraint-free numerical EKF source; no predicate-only helpers.",
            "warmup_turns": 5, "reset_after_warmup": True,
            "inputs": "Three shared contiguous little-endian f32 arrays: velocity, angular velocity, bearing; reused each turn.",
            "outputs": "Complete final state and covariance; canonical dump has C-order state (N,3) then covariance (N,3,3).",
            "validation_atol": ATOL, "validation_rtol": RTOL,
            "timing": "Twenty dependent state updates; final public host-readable state included. Excludes compilation, initial allocation/setup, input binding, warmup/reset, checksums and file output. Includes allocations inside execution.",
            "publication": "Final host-readable output only; no inter-turn host snapshot or turn fusion. Public API snapshot conversion cost is included where required.",
            "optimization": "Native SIMD, batching, layout choices and existing compilation pathways allowed. No performance-derived tolerance or fastest-sample selection.",
        },
        "validation": [], "discarded_processes": [], "campaigns": [],
    }

    def save():
        report["input_sha256"] = {str(n): sha(p) for n, p in input_paths.items()}
        (args.output / "results.json").write_text(json.dumps(report, indent=2) + "\n")

    expected_checksums = {}
    scenarios = [(4, 1), (20, 4), (256, 20), (260, 20), (260, 100), (10000, 20), (100000, 20)]
    for n, turns in scenarios:
        # A pure-NumPy reference, independent of the selected faster contraction.
        reference_name = "numpy-sequential-reference" if n <= 10000 else "numpy-matmul-reference"
        reference = reference_base if n <= 10000 else bases["numpy-matmul-aos"]
        lanes = {reference_name: reference, **bases}
        expected = None
        for lane, base in lanes.items():
            dump = dump_dir / f"{lane}-{n}-{turns}.f32"
            execution = execute(lane, base, n, turns, dump)
            values = np.fromfile(dump, dtype="<f4")
            if values.size != n * 12 or not np.isfinite(values).all():
                raise RuntimeError(f"invalid full output: {lane}, {n}, {turns}")
            if expected is None:
                expected = values.astype(np.float64)
            delta = np.abs(values.astype(np.float64) - expected)
            bound = ATOL + RTOL * np.abs(expected)
            passed = bool(np.all(delta <= bound))
            row = {"lane": lane, "instances": n, "turns": turns, "reference_lane": reference_name,
                   "passed": passed, "max_state_abs_error": float(delta[:3*n].max()),
                   "max_covariance_abs_error": float(delta[3*n:].max()),
                   "max_tolerance_fraction": float((delta / bound).max()),
                   "dump_sha256": sha(dump), "execution": execution}
            report["validation"].append(row)
            save()
            print(f"Validate {lane} {n}×{turns}: {passed}; bound fraction {row['max_tolerance_fraction']:.3g}", flush=True)
            if not passed:
                raise RuntimeError("Full-output validation failed; no performance result may be collected")
            expected_checksums[(lane, n, turns)] = execution["result"]["checksum"]
    if args.validate_only:
        return

    rng = random.Random(args.seed)
    campaigns = [(256, args.samples), (10000, args.samples), (100000, args.large_samples)]
    for n, samples in campaigns:
        lanes = dict(bases)
        campaign = {"instances": n, "turns": 20, "samples_per_lane": samples,
                    "lanes": list(lanes), "runs": [], "summaries": {}}
        report["campaigns"].append(campaign)
        for lane, base in lanes.items():
            report["discarded_processes"].append(execute(lane, base, n, 20))
        for round_number in range(samples):
            order = list(lanes)
            rng.shuffle(order)
            for lane in order:
                execution = execute(lane, lanes[lane], n, 20)
                if execution["result"]["checksum"] != expected_checksums[(lane, n, 20)]:
                    raise RuntimeError(f"timed checksum changed: {lane}")
                execution["round"] = round_number
                campaign["runs"].append(execution)
                save()
                print(f"{n}×20 round {round_number+1}/{samples}: {lane} {execution['result']['filter_turns_per_second']/1e6:.6f} M/s", flush=True)
        for lane in lanes:
            rates = [r["result"]["filter_turns_per_second"] for r in campaign["runs"] if r["lane"] == lane]
            median = statistics.median(rates)
            campaign["summaries"][lane] = {"median_filter_turns_per_second": median,
                "mad_filter_turns_per_second": statistics.median(abs(r - median) for r in rates),
                "minimum": min(rates), "maximum": max(rates), "n": len(rates)}
        save()
    if sha(args.paper_binary) != report["binary_sha256"]["paper"]:
        raise RuntimeError("native binary changed during measurement")
    for relative, expected_hash in report["source_sha256"].items():
        if sha(ROOT / relative) != expected_hash:
            raise RuntimeError(f"source changed during measurement: {relative}")
    report["finished_at"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    save()
    print(json.dumps([{key: c[key] for key in ("instances", "summaries")} for c in report["campaigns"]], indent=2))


if __name__ == "__main__":
    main()
