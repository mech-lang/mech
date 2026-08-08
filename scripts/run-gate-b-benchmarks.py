#!/usr/bin/env python3
"""Run and summarize the controlled Gate B benchmark lanes."""

from __future__ import annotations

import argparse
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SAMPLE_PREFIX = "GATE_B_SAMPLE "
FROZEN_BASE = "437f6c6c636d9818729597342165dfc9af5eb4a7"
FROZEN_B0_BRANCH = "test/resident-ekf-efficacy-contract"
FROZEN_B1_BRANCH = "feat/engine-resident-ekf-substrate"
FROZEN_B1_BASE = "c4f7cb1d27b9645b3f669d944c7e49bcd0829ccc"
EPISODE_LENGTH = 4_096
SCALED_INSTANCES = (1, 8, 64)
THREAD_VARIABLES = (
    "OMP_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "BLIS_NUM_THREADS",
    "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
    "NUMEXPR_NUM_THREADS",
    "RAYON_NUM_THREADS",
)


def command_output(command: list[str]) -> str:
    return subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout.strip()


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    rank = fraction * (len(ordered) - 1)
    lower = math.floor(rank)
    upper = math.ceil(rank)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (rank - lower)


def cargo_target_directory(environment: dict[str, str]) -> Path:
    configured = Path(environment.get("CARGO_TARGET_DIR", "target"))
    if not configured.is_absolute():
        configured = ROOT / configured
    return configured.resolve()


def clear_gate_b_criterion_results(target_dir: Path) -> None:
    criterion_root = target_dir / "criterion"
    if not criterion_root.exists():
        return
    for child in criterion_root.iterdir():
        if child.name != "gate_b" and not child.name.startswith("gate_b_"):
            continue
        if child.is_symlink():
            child.unlink()
        elif child.is_dir():
            shutil.rmtree(child)


def criterion_samples(target_dir: Path) -> dict[str, dict[str, Any]]:
    criterion_root = target_dir / "criterion"
    summaries: dict[str, dict[str, Any]] = {}
    if not criterion_root.exists():
        return summaries
    sample_paths = list(criterion_root.glob("gate_b/**/new/sample.json"))
    sample_paths.extend(criterion_root.glob("gate_b_*/**/new/sample.json"))
    for sample_path in sorted(sample_paths):
        payload = json.loads(sample_path.read_text(encoding="utf-8"))
        iterations = payload.get("iters", [])
        times = payload.get("times", [])
        if len(iterations) != len(times):
            raise ValueError(f"mismatched Criterion sample arrays in {sample_path}")
        per_episode = [
            float(elapsed) / float(iteration)
            for iteration, elapsed in zip(iterations, times)
            if float(iteration) > 0.0
        ]
        relative = sample_path.parent.parent.relative_to(criterion_root)
        parts = list(relative.parts)
        if parts[0].startswith("gate_b_"):
            parts = ["gate_b", parts[0][len("gate_b_") :], *parts[1:]]
        benchmark = "/".join(parts)
        summaries[benchmark] = {
            "benchmark": benchmark,
            "sample_count": len(per_episode),
            "median_episode_ns": statistics.median(per_episode)
            if per_episode
            else 0.0,
            "p95_episode_ns": percentile(per_episode, 0.95),
        }
    return summaries


def parse_probe_samples(output: str) -> dict[tuple[str, int], dict[str, Any]]:
    samples: dict[tuple[str, int], dict[str, Any]] = {}
    for line in output.splitlines():
        marker = line.find(SAMPLE_PREFIX)
        if marker < 0:
            continue
        sample = json.loads(line[marker + len(SAMPLE_PREFIX) :])
        samples[(sample["lane"], int(sample["instances"]))] = sample
    return samples


def merge_structural_probes(
    timed: dict[tuple[str, int], dict[str, Any]],
    structural: dict[tuple[str, int], dict[str, Any]],
) -> dict[tuple[str, int], dict[str, Any]]:
    merged = {key: value.copy() for key, value in timed.items()}
    legacy = {
        *(("mech-legacy-atomic", instances) for instances in SCALED_INSTANCES),
        ("mech-legacy-atomic-full-write", 1),
    }
    resident = {
        *(("mech-resident-kernel", instances) for instances in SCALED_INSTANCES),
        ("mech-resident-kernel-full-write", 1),
    }
    for key in legacy | resident:
        if key not in merged:
            raise ValueError(f"missing timed structural probe {key[0]}/{key[1]}")
        if key not in structural:
            raise ValueError(f"missing untimed structural probe {key[0]}/{key[1]}")
        fields = (
            ("commit_runtime_call_count", "legacy_journal_capture_count")
            if key in legacy
            else STRUCTURAL_FIELDS
        )
        for field in fields:
            merged[key][field] = structural[key].get(field)
    return merged


def controlled_environment(source: dict[str, str]) -> dict[str, str]:
    environment = source.copy()
    for variable in THREAD_VARIABLES:
        environment[variable] = "1"
    return environment


def hardware_description(machine_label: str | None = None) -> str:
    if machine_label and machine_label.strip():
        return machine_label.strip()
    if sys.platform == "darwin":
        try:
            overview = command_output(
                ["system_profiler", "SPHardwareDataType", "-detailLevel", "mini"]
            )
            fields = {}
            for line in overview.splitlines():
                key, separator, value = line.strip().partition(":")
                if separator and key in {"Model Name", "Model Identifier", "Chip"}:
                    fields[key] = value.strip()
            parts = [
                fields.get("Model Name"),
                fields.get("Model Identifier"),
                fields.get("Chip"),
            ]
            specific = [part for part in parts if part]
            if specific:
                return ", ".join(specific)
        except (OSError, subprocess.CalledProcessError):
            pass
    processor = platform.processor().strip()
    generic = {
        "",
        "arm",
        "arm64",
        "aarch64",
        "x86_64",
        "amd64",
        "i386",
        "i686",
        platform.machine().strip().lower(),
    }
    if processor.lower() not in generic:
        return processor
    raise ValueError(
        "a specific CPU/model identity is unavailable; pass --machine-label "
        "with the controlled machine's stable model label"
    )


def _worker_error(process: subprocess.Popen[str], reason: str) -> RuntimeError:
    stderr = ""
    if process.stderr is not None:
        stderr = process.stderr.read().strip()
    detail = f": {stderr}" if stderr else ""
    return RuntimeError(f"persistent NumPy worker {reason}{detail}")


def run_numpy_controls(
    python: str, samples: int, environment: dict[str, str]
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    command = [
        python,
        str(ROOT / "benchmarks/runtime/gate-b/numpy/ekf_v1.py"),
    ]
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        if process.stdout is None or process.stdin is None:
            raise _worker_error(process, "did not expose its protocol pipes")
        ready_line = process.stdout.readline()
        if not ready_line:
            raise _worker_error(process, "exited before its ready message")
        ready = json.loads(ready_line)
        if ready.get("type") != "ready" or ready.get("protocol") != "gate-b-numpy-v1":
            raise RuntimeError(f"unexpected persistent NumPy ready message: {ready}")
        results = []
        for instances in SCALED_INSTANCES:
            request = {
                "command": "benchmark",
                "instances": instances,
                "samples": samples,
            }
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()
            response_line = process.stdout.readline()
            if not response_line:
                raise _worker_error(process, f"exited during the {instances}-instance lane")
            response = json.loads(response_line)
            if response.get("type") == "error":
                raise RuntimeError(
                    "persistent NumPy worker rejected the "
                    f"{instances}-instance lane: {response.get('message')}"
                )
            if response.get("type") != "benchmark-result":
                raise RuntimeError(f"unexpected persistent NumPy result: {response}")
            results.append(response)
        process.stdin.write(json.dumps({"command": "quit"}) + "\n")
        process.stdin.flush()
        bye = json.loads(process.stdout.readline())
        if bye.get("type") != "bye":
            raise RuntimeError(f"unexpected persistent NumPy shutdown: {bye}")
        if process.wait(timeout=10) != 0:
            raise _worker_error(process, "exited unsuccessfully")
        return ready, results
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


STRUCTURAL_FIELDS = (
    "candidate_seed_bytes",
    "candidate_written_bytes",
    "published_buffer_copy_bytes",
    "publication_store_count",
    "receipt_bytes",
    "commit_runtime_call_count",
    "legacy_journal_capture_count",
    "abort_output_hash",
)


def lane_record(
    lane: str,
    instances: int,
    sample_count: int,
    median_episode_ns: float,
    p95_episode_ns: float,
    probe: dict[str, Any],
    reference_hash: str,
) -> dict[str, Any]:
    turns = int(probe["turns"])
    allocations = probe.get("allocation_count")
    allocated_bytes = probe.get("allocated_bytes")
    return {
        "lane": lane,
        "instances": instances,
        "sample_count": sample_count,
        "turns_per_sample": turns,
        "timing": {
            "median_ns_per_turn": median_episode_ns / turns,
            "p95_ns_per_turn": p95_episode_ns / turns,
        },
        "allocation": {
            "allocations_per_turn": (
                float(allocations) / turns if allocations is not None else None
            ),
            "allocated_bytes_per_turn": (
                float(allocated_bytes) / turns
                if allocated_bytes is not None
                else None
            ),
            "episode_allocation_count": allocations,
            "episode_allocated_bytes": allocated_bytes,
        },
        "correctness": bool(probe["correctness"]),
        "quantized_state_hash": probe["quantized_state_hash"],
        "reference_quantized_state_hash": reference_hash,
        "structural": {field: probe.get(field) for field in STRUCTURAL_FIELDS},
    }


def assemble_lanes(
    criterion: dict[str, dict[str, Any]],
    probes: dict[tuple[str, int], dict[str, Any]],
    numpy_results: list[dict[str, Any]],
    reference_hash: str,
) -> list[dict[str, Any]]:
    lanes = []
    rust_lanes = (
        ("rust-kernel", "gate_b/rust-kernel/{instances}"),
        ("rust-epoch", "gate_b/rust-epoch/{instances}"),
        ("mech-resident-kernel", "gate_b/mech-resident-kernel/{instances}"),
        ("mech-legacy-atomic", "gate_b/mech-legacy-atomic/{instances}"),
    )
    for lane, benchmark_template in rust_lanes:
        for instances in SCALED_INSTANCES:
            benchmark = benchmark_template.format(instances=instances)
            if benchmark not in criterion:
                raise ValueError(f"missing Criterion result {benchmark}")
            key = (lane, instances)
            if key not in probes:
                raise ValueError(f"missing structural probe {lane}/{instances}")
            timing = criterion[benchmark]
            lanes.append(
                lane_record(
                    lane,
                    instances,
                    int(timing["sample_count"]),
                    float(timing["median_episode_ns"]),
                    float(timing["p95_episode_ns"]),
                    probes[key],
                    reference_hash,
                )
            )
    for lane, benchmark in (
        ("rust-epoch-full-write", "gate_b/full-write/rust-epoch"),
        ("mech-legacy-atomic-full-write", "gate_b/full-write/mech-legacy-atomic"),
        ("mech-resident-kernel-full-write", "gate_b/full-write/mech-resident-kernel"),
    ):
        if benchmark not in criterion:
            raise ValueError(f"missing Criterion result {benchmark}")
        key = (lane, 1)
        if key not in probes:
            raise ValueError(f"missing structural probe {lane}/1")
        timing = criterion[benchmark]
        lanes.append(
            lane_record(
                lane,
                1,
                int(timing["sample_count"]),
                float(timing["median_episode_ns"]),
                float(timing["p95_episode_ns"]),
                probes[key],
                probes[key]["quantized_state_hash"],
            )
        )
    for result in numpy_results:
        durations = [float(value) for value in result["samples_ns"]]
        probe = {
            **result,
            "allocation_count": None,
            "allocated_bytes": None,
        }
        lanes.append(
            lane_record(
                "numpy-persistent",
                int(result["instances"]),
                len(durations),
                statistics.median(durations),
                percentile(durations, 0.95),
                probe,
                result["reference_quantized_state_hash"],
            )
        )
    return sorted(lanes, key=lambda lane: (lane["lane"], lane["instances"]))


def primary_median(lanes: list[dict[str, Any]], name: str) -> float:
    matches = [
        lane
        for lane in lanes
        if lane["lane"] == name and lane["instances"] == 1
    ]
    if len(matches) != 1:
        raise ValueError(f"expected exactly one primary {name} lane")
    return float(matches[0]["timing"]["median_ns_per_turn"])


def legacy_denominator(lanes: list[dict[str, Any]]) -> dict[str, Any]:
    legacy = primary_median(lanes, "mech-legacy-atomic")
    raw_epoch = primary_median(lanes, "rust-epoch")
    denominator = legacy - raw_epoch
    return {
        "mech_legacy_atomic_ns_per_turn": legacy,
        "rust_epoch_ns_per_turn": raw_epoch,
        "legacy_denominator_ns_per_turn": denominator,
        "positive": denominator > 0.0,
    }


def b1_progression(lanes: list[dict[str, Any]]) -> dict[str, Any]:
    resident = primary_median(lanes, "mech-resident-kernel")
    rust_kernel = primary_median(lanes, "rust-kernel")
    rust_epoch = primary_median(lanes, "rust-epoch")
    multiplier = 1.05
    limit = multiplier * rust_epoch
    return {
        "resident_kernel_ns_per_turn": resident,
        "rust_kernel_ns_per_turn": rust_kernel,
        "rust_epoch_ns_per_turn": rust_epoch,
        "resident_kernel_ratio": resident / rust_kernel,
        "resident_kernel_vs_raw_epoch": resident / rust_epoch,
        "limit_multiplier": multiplier,
        "limit_ns_per_turn": limit,
        "passed": resident <= limit,
    }


def worktree_changes() -> str:
    return command_output(["git", "status", "--porcelain=v1", "--untracked-files=all"])


def frozen_base_error(commit: str, branch: str) -> str | None:
    expected_base = {
        FROZEN_B0_BRANCH: FROZEN_BASE,
        FROZEN_B1_BRANCH: FROZEN_B1_BASE,
    }.get(branch)
    if expected_base is None:
        return f"Gate B controls cannot run on unapproved branch {branch}"
    merge_base = command_output(["git", "merge-base", commit, expected_base])
    if merge_base != expected_base:
        return (
            f"Gate B commit {commit} is not based on frozen base "
            f"{expected_base}; merge-base is {merge_base}"
        )
    return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        help="write the report here; otherwise print it to stdout",
    )
    parser.add_argument(
        "--raw-output",
        type=Path,
        help="write combined Cargo/Criterion output here (defaults under target)",
    )
    parser.add_argument(
        "--raw-structural-output",
        type=Path,
        help="write the untimed probe-run output here (defaults under target)",
    )
    parser.add_argument(
        "--python",
        default=sys.executable,
        help="Python executable containing NumPy",
    )
    parser.add_argument("--sample-size", type=int, default=10)
    parser.add_argument("--warm-up-time", type=float, default=1.0)
    parser.add_argument("--measurement-time", type=float, default=3.0)
    parser.add_argument(
        "--machine-label",
        help="stable controlled-machine model label when automatic detection is unavailable",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.sample_size < 10:
        print("Criterion sample size must be at least 10", file=sys.stderr)
        return 2
    if args.warm_up_time <= 0.0 or args.measurement_time <= 0.0:
        print("warm-up and measurement times must be positive", file=sys.stderr)
        return 2
    try:
        hardware = hardware_description(args.machine_label)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2
    dirty = worktree_changes()
    if dirty:
        print(
            "refusing to attribute Gate B benchmark results to a dirty worktree:\n"
            f"{dirty}",
            file=sys.stderr,
        )
        return 2

    commit = command_output(["git", "rev-parse", "HEAD"])
    branch = command_output(["git", "symbolic-ref", "--short", "HEAD"])
    base_error = frozen_base_error(commit, branch)
    if base_error:
        print(base_error, file=sys.stderr)
        return 2
    environment = controlled_environment(os.environ)
    target_dir = cargo_target_directory(environment)
    raw_output = args.raw_output or (
        target_dir / "gate-b-benchmark-runs" / commit / "b0-controls.log"
    )
    if not raw_output.is_absolute():
        raw_output = (ROOT / raw_output).resolve()
    raw_output.parent.mkdir(parents=True, exist_ok=True)
    raw_structural_output = args.raw_structural_output or (
        target_dir / "gate-b-benchmark-runs" / commit / "b0-structural.log"
    )
    if not raw_structural_output.is_absolute():
        raw_structural_output = (ROOT / raw_structural_output).resolve()
    raw_structural_output.parent.mkdir(parents=True, exist_ok=True)

    command = [
        "cargo",
        "bench",
        "-p",
        "mech-runtime",
        "--bench",
        "resident_ekf",
        "--features",
        "source_default,runtime_bench_gate_b",
        "--",
        "--noplot",
        "--sample-size",
        str(args.sample_size),
        "--warm-up-time",
        str(args.warm_up_time),
        "--measurement-time",
        str(args.measurement_time),
        "--nresamples",
        "10000",
    ]
    clear_gate_b_criterion_results(target_dir)
    process = subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    raw_output.write_text(process.stdout, encoding="utf-8")
    if process.returncode != 0:
        sys.stderr.write(process.stdout)
        return process.returncode

    structural_command = [
        "cargo",
        "bench",
        "-p",
        "mech-runtime",
        "--bench",
        "resident_ekf",
        "--features",
        "source_default,runtime_bench_gate_b,runtime_bench_probes",
        "--",
        "--noplot",
    ]
    structural_environment = environment.copy()
    structural_environment["MECH_GATE_B_STRUCTURAL_ONLY"] = "1"
    structural_process = subprocess.run(
        structural_command,
        cwd=ROOT,
        env=structural_environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    raw_structural_output.write_text(
        structural_process.stdout, encoding="utf-8"
    )
    if structural_process.returncode != 0:
        sys.stderr.write(structural_process.stdout)
        return structural_process.returncode

    try:
        ready, numpy_results = run_numpy_controls(
            args.python, args.sample_size, environment
        )
        manifest = json.loads(
            (ROOT / "benchmarks/runtime/gate-b/ekf-v1.json").read_text(
                encoding="utf-8"
            )
        )
        probes = merge_structural_probes(
            parse_probe_samples(process.stdout),
            parse_probe_samples(structural_process.stdout),
        )
        lanes = assemble_lanes(
            criterion_samples(target_dir),
            probes,
            numpy_results,
            manifest["reference"]["quantized_trajectory_sha256"],
        )
        derived = legacy_denominator(lanes)
    except (OSError, ValueError, RuntimeError, KeyError, json.JSONDecodeError) as error:
        print(f"Gate B result assembly failed: {error}", file=sys.stderr)
        return 2

    summary = {
        "schema_version": 1,
        "gate": "B",
        "phase": "B0-controls" if branch == FROZEN_B0_BRANCH else "B1-resident-kernel",
        "git_commit": commit,
        "git_branch": branch,
        "machine": {
            "identity": hardware,
            "os": platform.platform(),
            "architecture": platform.machine(),
        },
        "toolchain": {
            "rustc": command_output(["rustc", "-Vv"]),
            "RUSTFLAGS": os.environ.get("RUSTFLAGS", ""),
            "CARGO_ENCODED_RUSTFLAGS": os.environ.get(
                "CARGO_ENCODED_RUSTFLAGS", ""
            ),
            "python": ready["python"],
            "python_executable": args.python,
            "numpy": ready["numpy"],
            "numpy_config": ready["numpy_config"],
            "blas_lapack_provider": ready["blas_lapack_provider"],
        },
        "thread_environment": {
            variable: environment[variable] for variable in THREAD_VARIABLES
        },
        "trace": {
            "sha256": manifest["trace"]["sha256"],
            "file": manifest["trace"]["file"],
        },
        "workload": {
            "version": manifest["workload"],
            "episode_length": EPISODE_LENGTH,
            "scaled_instances": list(SCALED_INSTANCES),
            "scalar": "f64",
            "matrix_storage": "column-major",
        },
        "sample_protocol": {
            "criterion_sample_size": args.sample_size,
            "numpy_sample_size": args.sample_size,
            "warm_up_seconds": args.warm_up_time,
            "measurement_seconds": args.measurement_time,
            "turns_per_sample": EPISODE_LENGTH,
            "fixture_setup_included_in_timing": False,
            "correctness_included_in_timing": False,
            "profile": "release",
        },
        "benchmark_arguments": command,
        "structural_probe_arguments": structural_command,
        "raw_criterion_directory": str(target_dir / "criterion"),
        "raw_output": str(raw_output),
        "raw_structural_probe_output": str(raw_structural_output),
        "lanes": lanes,
        "derived": derived,
        "stop_condition": {
            "name": "positive-legacy-denominator",
            "passed": bool(derived["positive"]),
        },
    }
    if branch == FROZEN_B1_BRANCH:
        summary["b1_progression"] = b1_progression(lanes)
    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if args.output:
        output = args.output if args.output.is_absolute() else ROOT / args.output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    if not derived["positive"]:
        print(
            "Gate B B0 stop: T_mech-legacy-atomic - T_rust-epoch is non-positive",
            file=sys.stderr,
        )
        return 3
    if branch == FROZEN_B1_BRANCH and not summary["b1_progression"]["passed"]:
        print(
            "Gate B B1 stop: mech-resident-kernel exceeds 1.05 x rust-epoch",
            file=sys.stderr,
        )
        return 4
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
