#!/usr/bin/env python3
"""Run and summarize the controlled Gate A Criterion benchmarks."""

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
SAMPLE_PREFIX = "GATE_A_SAMPLE "


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


def criterion_samples(target_dir: Path) -> list[dict[str, Any]]:
    criterion_root = target_dir / "criterion"
    summaries: list[dict[str, Any]] = []
    if not criterion_root.exists():
        return summaries
    for sample_path in sorted(criterion_root.glob("**/new/sample.json")):
        payload = json.loads(sample_path.read_text(encoding="utf-8"))
        iterations = payload.get("iters", [])
        times = payload.get("times", [])
        if len(iterations) != len(times):
            raise ValueError(f"mismatched Criterion sample arrays in {sample_path}")
        per_iteration = [
            float(time) / float(iteration)
            for iteration, time in zip(iterations, times)
            if float(iteration) > 0
        ]
        benchmark = sample_path.parent.parent.relative_to(criterion_root).as_posix()
        if not (benchmark.startswith("gate_a/") or benchmark.startswith("gate_a_")):
            continue
        summaries.append(
            {
                "benchmark": benchmark,
                "sample_count": len(per_iteration),
                "median_ns": statistics.median(per_iteration) if per_iteration else 0.0,
                "p95_ns": percentile(per_iteration, 0.95),
            }
        )
    return summaries


def clear_gate_a_criterion_results(target_dir: Path) -> None:
    """Remove only generated Gate A results before starting a controlled run."""
    criterion_root = target_dir / "criterion"
    if not criterion_root.exists():
        return
    for child in criterion_root.iterdir():
        if child.name != "gate_a" and not child.name.startswith("gate_a_"):
            continue
        if child.is_symlink():
            child.unlink()
        elif child.is_dir():
            shutil.rmtree(child)


def worktree_changes() -> str:
    return command_output(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"]
    )


def cargo_target_directory(environment: dict[str, str]) -> Path:
    configured = Path(environment.get("CARGO_TARGET_DIR", "target"))
    if not configured.is_absolute():
        configured = ROOT / configured
    return configured.resolve()


def parse_probe_samples(output: str) -> list[dict[str, Any]]:
    samples: dict[tuple[str, int], dict[str, Any]] = {}
    for line in output.splitlines():
        marker = line.find(SAMPLE_PREFIX)
        if marker < 0:
            continue
        sample = json.loads(line[marker + len(SAMPLE_PREFIX) :])
        samples[(sample["operation"], sample["history"])] = sample
    return [samples[key] for key in sorted(samples)]


def hardware_description(machine_label: str | None = None) -> str:
    if machine_label and machine_label.strip():
        return machine_label.strip()
    if sys.platform == "darwin":
        try:
            return command_output(["sysctl", "-n", "machdep.cpu.brand_string"])
        except (OSError, subprocess.CalledProcessError):
            pass
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        help="write summary JSON here; otherwise print it to stdout",
    )
    parser.add_argument(
        "--raw-output",
        type=Path,
        help="write combined Cargo/Criterion output here (defaults under target)",
    )
    parser.add_argument(
        "--filter",
        help="optional Criterion benchmark filter",
    )
    parser.add_argument(
        "--sample-size",
        type=int,
        help="optional Criterion sample size override",
    )
    parser.add_argument(
        "--extended",
        action="store_true",
        help="include the opt-in 1,000,000-record direct-store sweep",
    )
    parser.add_argument(
        "--machine-label",
        help="stable controlled-machine model label when automatic detection is unavailable",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        hardware = hardware_description(args.machine_label)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2
    dirty = worktree_changes()
    if dirty:
        print(
            "refusing to attribute Gate A benchmark results to a dirty worktree:\n"
            f"{dirty}",
            file=sys.stderr,
        )
        return 2
    commit = command_output(["git", "rev-parse", "HEAD"])
    target_dir = cargo_target_directory(os.environ)
    raw_output = args.raw_output or (
        target_dir
        / "gate-a-benchmark-runs"
        / commit
        / ("history_scaling-extended.log" if args.extended else "history_scaling.log")
    )
    if not raw_output.is_absolute():
        raw_output = (ROOT / raw_output).resolve()
    raw_output.parent.mkdir(parents=True, exist_ok=True)

    command = [
        "cargo",
        "bench",
        "-p",
        "mech-runtime",
        "--bench",
        "history_scaling",
        "--features",
        "source_default,runtime_bench_probes",
        "--",
        "--noplot",
    ]
    sample_size = args.sample_size or 10
    if args.filter:
        command.append(args.filter)
    command.extend(["--sample-size", str(sample_size)])

    clear_gate_a_criterion_results(target_dir)
    environment = os.environ.copy()
    if args.extended:
        environment["MECH_GATE_A_EXTENDED"] = "1"
    else:
        environment.pop("MECH_GATE_A_EXTENDED", None)
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

    summary = {
        "schema_version": 1,
        "git_commit": commit,
        "rustc": command_output(["rustc", "-Vv"]),
        "os": platform.platform(),
        "architecture": platform.machine(),
        "hardware": hardware,
        "target_cpu": {
            "RUSTFLAGS": os.environ.get("RUSTFLAGS", ""),
            "CARGO_ENCODED_RUSTFLAGS": os.environ.get("CARGO_ENCODED_RUSTFLAGS", ""),
        },
        "runtime_limits": {
            "default": "RuntimeConfig::default()",
            "lane_overrides": [
                {
                    "operation": "context_event_retention_steady",
                    "history": limit,
                    "max_in_memory_events": limit,
                }
                for limit in (32, 1_024, 16_384, 100_000)
            ],
        },
        "sample_protocol": {
            "criterion_sample_size": sample_size,
            "fixed_history_per_sample": True,
            "measured_operations_per_fixture": {
                "single_operation_lanes": 1,
                "context_event_retention_steady": (
                    "max(criterion_requested_iterations, retention_limit)"
                ),
            },
            "setup_included_in_timing": False,
            "profile": "release",
            "extended_direct_store_sweep": args.extended,
        },
        "benchmark_arguments": command,
        "raw_criterion_directory": str(target_dir / "criterion"),
        "raw_output": str(raw_output),
        "criterion": criterion_samples(target_dir),
        "probes": parse_probe_samples(process.stdout),
    }
    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if args.output:
        output = args.output if args.output.is_absolute() else ROOT / args.output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
