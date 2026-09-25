#!/usr/bin/env python3
"""Collect matched checked/unchecked Mech backend samples without building.

Each of ten cases runs once per deterministically shuffled round, sequentially,
in a fresh OS process. Compilation, warmup, allocation, and result readback are
outside the reported resident-turn interval. Results are saved after every
process, including failures. No samples are trimmed or removed as outliers.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
import os
import platform
import random
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BACKENDS = ("evaluator", "scalar-jit", "simd-aot", "simd-jit-8w", "metal")
MODES = ("checked", "unchecked")
THREAD_ENVIRONMENT = {
    "OMP_NUM_THREADS": "1",
    "OPENBLAS_NUM_THREADS": "1",
    "MKL_NUM_THREADS": "1",
    "VECLIB_MAXIMUM_THREADS": "1",
    "BLIS_NUM_THREADS": "1",
    "NUMBA_NUM_THREADS": "1",
    "RAYON_NUM_THREADS": "1",
}
BUILD_COMMAND = [
    "cargo", "build", "--profile", "kernel-bench", "--no-default-features",
    "--features", "kernel-benchmarks", "--example", "mech_backend_pairs",
]
SOURCE_SUFFIXES = {
    ".rs", ".py", ".toml", ".lock", ".mec", ".mcfg", ".c", ".cc", ".cpp",
    ".h", ".hpp", ".metal", ".wgsl", ".sh",
}


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def digest(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher.hexdigest()


def capture(arguments: list[str]) -> dict:
    try:
        result = subprocess.run(
            arguments, cwd=ROOT, text=True, capture_output=True, check=False
        )
        return {
            "command": arguments,
            "returncode": result.returncode,
            "stdout": result.stdout,
            "stderr": result.stderr,
        }
    except OSError as error:
        return {"command": arguments, "error": str(error)}


def provenance(binary: Path) -> dict:
    diff = capture(["git", "diff", "--binary", "HEAD", "--"])
    status = capture(["git", "status", "--porcelain=v1", "--untracked-files=all"])
    source_files = {
        "examples/embedded_ekf/ekf.mec",
        "benchmarks/iros-2026/mech_backend_pairs.rs",
        "benchmarks/iros-2026/measure_mech_backend_pairs.py",
        "Cargo.toml", "Cargo.lock",
    }
    changed = capture(["git", "diff", "--name-only", "-z", "HEAD", "--"])
    untracked = capture(["git", "ls-files", "--others", "--exclude-standard", "-z"])
    for result in (changed, untracked):
        if result.get("returncode") != 0:
            raise RuntimeError(f"cannot capture source provenance: {result}")
        source_files.update(
            name for name in result["stdout"].split("\0")
            if name and Path(name).suffix in SOURCE_SUFFIXES
        )
    snapshots = {}
    for name in sorted(source_files):
        path = ROOT / name
        if path.is_file():
            content = path.read_bytes()
            snapshots[name] = {
                "sha256": hashlib.sha256(content).hexdigest(),
                "content_utf8": content.decode("utf-8"),
            }
        else:
            snapshots[name] = {"deleted": True}
    return {
        "binary": str(binary),
        "binary_sha256": digest(binary),
        "source_files_sha256": {
            name: snapshot["sha256"] for name, snapshot in snapshots.items()
            if "sha256" in snapshot
        },
        "changed_and_untracked_source_snapshots": snapshots,
        "source_snapshot_scope": "fixed harness/fixture/manifests plus every tracked changed or untracked implementation source file",
        "git_head": capture(["git", "rev-parse", "HEAD"]),
        "git_branch": capture(["git", "branch", "--show-current"]),
        "git_status": status,
        "tracked_diff_sha256": hashlib.sha256(
            diff.get("stdout", "").encode("utf-8")
        ).hexdigest(),
        "tracked_diff_command": diff["command"],
        "tracked_diff_returncode": diff.get("returncode"),
        "tracked_diff_content": diff.get("stdout"),
        "build": {
            "reproduction_command": BUILD_COMMAND,
            "command_note": "Recorded reproduction command; this campaign executes an existing binary and does not invoke the build.",
            "profile": "kernel-bench",
            "inherits": "dev",
            "default_dependency_opt_level": 0,
            "package_opt_levels": {name: 3 for name in ("mech", "mech-gpu", "mech-compute", "wide")},
            "debug": False,
            "incremental": False,
            "strip": "symbols",
            "debug_assertions": True,
            "overflow_checks": True,
            "cargo_toml_snapshot": "Cargo.toml",
        },
        "machine": {
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": sys.version,
            "sw_vers": capture(["sw_vers"]),
            "uname": capture(["uname", "-a"]),
            "sysctl": {
                name: capture(["sysctl", "-n", name])
                for name in (
                    "machdep.cpu.brand_string", "hw.model", "hw.memsize",
                    "hw.physicalcpu", "hw.logicalcpu",
                )
            },
            "rustc_vv": capture(["rustc", "-Vv"]),
            "cargo_version": capture(["cargo", "--version"]),
            "cc_version": capture([os.environ.get("CC", "cc"), "--version"]),
        },
        "environment_baseline": {
            "captured_at": now(),
            "processes": capture(["ps", "-axo", "pid,ppid,pcpu,pmem,comm"]),
            "thermal_state": capture(["pmset", "-g", "therm"]),
            "note": "Read-only snapshots; the collector does not stop or alter other processes.",
        },
        "thread_environment": THREAD_ENVIRONMENT,
        "explicit_simd_jit_workers": 8,
        "aot_cache_directory": os.environ.get("MECH_AOT_CACHE_DIR"),
        "compiler_environment_note": "Installed rustc/cc versions are recorded; binary SHA identifies the executable actually run.",
    }


def summarize(records: list[dict]) -> dict:
    result = {}
    for backend in BACKENDS:
        result[backend] = {}
        for mode in MODES:
            selected = [
                record for record in records
                if record.get("status") == "ok"
                and record["backend"] == backend and record["mode"] == mode
            ]
            if not selected:
                continue
            samples = [
                record["measurement"]["throughput_million_filter_turns_per_second"]
                for record in selected
            ]
            median = statistics.median(samples)
            result[backend][mode] = {
                "n": len(samples),
                "median_million_filter_turns_per_second": median,
                "mad_million_filter_turns_per_second": statistics.median(
                    abs(value - median) for value in samples
                ),
                "minimum": min(samples), "maximum": max(samples),
                "samples_million_filter_turns_per_second": samples,
                "samples_elapsed_ns": [record["measurement"]["elapsed_ns"] for record in selected],
                "checksums": [record["measurement"]["checksum"] for record in selected],
            }
    return result


def save(path: Path, document: dict) -> None:
    document["updated_at"] = now()
    document["summary"] = summarize(document["records"])
    # Replace only this campaign's known output, never an unrelated file.
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent,
        prefix=path.name + ".", suffix=".tmp", delete=False,
    ) as stream:
        temporary = Path(stream.name)
        json.dump(document, stream, indent=2, allow_nan=False)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)


def check_measurement(measurement: dict, backend: str, mode: str, instances: int, turns: int) -> None:
    expected = {
        "backend": backend, "mode": mode, "instances": instances,
        "turns": turns, "total_filter_turns": instances * turns,
        "attempted_turns": turns + 5, "faults": 0,
        "warmup_turns": 5, "warmup_in_measured_session": True,
        "measured_start_after_turn": 5,
    }
    for key, value in expected.items():
        if measurement.get(key) != value:
            raise ValueError(f"{key}: expected {value!r}, got {measurement.get(key)!r}")
    elapsed = measurement["elapsed_ns"]
    if not isinstance(elapsed, int) or elapsed <= 0:
        raise ValueError("elapsed_ns must be a positive integer")
    throughput = measurement["throughput_million_filter_turns_per_second"]
    if not math.isfinite(throughput) or throughput <= 0:
        raise ValueError("non-finite or nonpositive throughput")
    expected_throughput = instances * turns * 1000.0 / elapsed
    if not math.isclose(throughput, expected_throughput, rel_tol=1.0e-12):
        raise ValueError("throughput is inconsistent with elapsed_ns")
    if not math.isfinite(measurement["checksum"]):
        raise ValueError("non-finite checksum")


def run_case(binary: Path, environment: dict, backend: str, mode: str,
             instances: int, turns: int, validate: bool) -> dict:
    command = [
        str(binary), "--backend", backend, "--mode", mode,
        "--instances", str(instances), "--turns", str(turns),
    ]
    if validate:
        command.append("--validate")
    record = {"backend": backend, "mode": mode, "command": command, "started_at": now()}
    try:
        process = subprocess.run(
            command, cwd=ROOT, env=environment, text=True,
            capture_output=True, check=False,
        )
        record.update(returncode=process.returncode, stdout=process.stdout, stderr=process.stderr)
        if process.returncode != 0:
            raise RuntimeError(f"benchmark process exited {process.returncode}")
        measurement = json.loads(process.stdout)
        check_measurement(measurement, backend, mode, instances, turns)
        library = measurement.get("library_path")
        if backend == "simd-aot" and not library:
            raise ValueError("AOT sample did not report its loaded library path")
        if library:
            library_path = Path(library)
            if not library_path.is_absolute():
                library_path = ROOT / library_path
            record["loaded_library"] = {
                "path": str(library_path.resolve()),
                "sha256": digest(library_path),
                "size_bytes": library_path.stat().st_size,
            }
        if validate:
            validation = measurement.get("validation")
            if not isinstance(validation, dict) or not validation.get("passed"):
                raise ValueError("preflight did not report passing numerical validation")
        record.update(status="ok", measurement=measurement)
    except (OSError, RuntimeError, ValueError, KeyError, TypeError) as error:
        record.update(status="failed", error=str(error))
    record["finished_at"] = now()
    return record


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=10)
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--seed", type=int, default=20260925)
    parser.add_argument(
        "--preflight-only", action="store_true",
        help="run all ten small numerical validation cases and skip full measured samples",
    )
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    if not binary.is_file():
        parser.error(f"binary not found: {binary}")
    if output.exists():
        parser.error(f"output already exists; choose a new campaign path: {output}")
    if args.samples < 1 or args.turns < 1:
        parser.error("--samples and --turns must be positive")
    if args.instances < 4 or args.instances % 4 or args.instances > 2**32 - 1:
        parser.error("--instances must be positive, divisible by four, and fit u32")
    output.parent.mkdir(parents=True, exist_ok=True)
    randomizer = random.Random(args.seed)
    cases = [(backend, mode) for backend in BACKENDS for mode in MODES]
    schedule = []
    for _ in range(args.samples):
        order = cases.copy()
        randomizer.shuffle(order)
        schedule.append(order)
    document = {
        "schema_version": 1, "started_at": now(), "status": "preflight",
        "instances": args.instances, "turns": args.turns, "samples_per_case": args.samples,
        "preflight_only": args.preflight_only,
        "random_seed": args.seed, "schedule": schedule,
        "provenance": provenance(binary), "preflight_records": [], "records": [],
        "methods": {
            "sample_unit": "one fresh OS process per backend and mode",
            "execution_order": "sequential; all ten cases once per deterministically shuffled round",
            "summary": "median throughput plus unscaled median absolute deviation",
            "outlier_removal": "none",
            "warmup": "5 turns in the same session and compiled backend, followed immediately by 40 measured turns by default; no reset or recompilation between warmup and measurement",
            "timed_work": "resident numerical turns, declared checks when enabled, completed publication every turn",
            "excluded_work": "source/backend compilation, dynamic library load, allocation, packing, worker startup, final state readback and checksum",
            "unchecked": "only the three named EKF source guards and guard-only instructions removed; state publication retained",
            "input_precision": "common host-generated f32 arrays; phase and trigonometric arguments calculated in f32",
            "validation": "all state components for 45 turns on at most 4092 instances against scalar checked; abs 2e-4 + rel 1e-5; checked NaN rejection preserves initial published state and reports the verified observed fault lane; 4092 exercises a partial Metal group and uneven 8-worker partitions",
            "preflight_samples": "separate fresh processes before the measured campaign; excluded from the summary",
        },
    }
    environment = os.environ.copy() | THREAD_ENVIRONMENT
    save(output, document)
    try:
        for backend, mode in cases:
            print(f"preflight: {backend} {mode}", file=sys.stderr, flush=True)
            record = run_case(binary, environment, backend, mode, min(args.instances, 4092), 40, True)
            document["preflight_records"].append(record)
            save(output, document)
            if record["status"] != "ok":
                raise RuntimeError(f"preflight failed: {backend} {mode}: {record['error']}")
        if not args.preflight_only:
            document["status"] = "measuring"
            save(output, document)
            for round_index, order in enumerate(schedule, 1):
                for position, (backend, mode) in enumerate(order, 1):
                    print(f"round {round_index}/{args.samples} case {position}/10: {backend} {mode}",
                          file=sys.stderr, flush=True)
                    record = run_case(binary, environment, backend, mode, args.instances, args.turns, False)
                    record.update(round=round_index, position=position)
                    document["records"].append(record)
                    save(output, document)
                    if record["status"] != "ok":
                        raise RuntimeError(f"sample failed: {backend} {mode}: {record['error']}")
        if digest(binary) != document["provenance"]["binary_sha256"]:
            raise RuntimeError("binary changed during campaign")
        for name, expected_hash in document["provenance"]["source_files_sha256"].items():
            if digest(ROOT / name) != expected_hash:
                raise RuntimeError(f"source file changed during campaign: {name}")
        document["status"] = "preflight-complete" if args.preflight_only else "complete"
    except (Exception, KeyboardInterrupt) as error:
        document["status"] = "failed" if isinstance(error, Exception) else "interrupted"
        document["error"] = str(error)
        save(output, document)
        raise
    document["finished_at"] = now()
    save(output, document)
    print(output)


if __name__ == "__main__":
    main()
