#!/usr/bin/env python3
"""Measure the Julia Metal.jl EKF with a shared compact fault status."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import statistics
import subprocess
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SOURCE = HERE / "julia-metal-matched.jl"
THREADGROUP_SIZE = 64
THROUGHPUT = re.compile(r"throughput: ([0-9.eE+-]+)")
CHECKSUM = re.compile(r"checksum: ([0-9.eE+-]+)")
FAULTS = re.compile(r"faults: ([0-9]+)")


def command(*arguments: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
        env=env,
    )


def measure(mode: str, instances: int, turns: int) -> dict[str, float | int]:
    environment = os.environ.copy()
    environment.update(JULIA_NUM_THREADS="8")
    result = command(
        "julia",
        "--startup-file=no",
        str(SOURCE),
        str(instances),
        str(turns),
        mode,
        env=environment,
    )
    throughput = THROUGHPUT.search(result.stdout)
    checksum = CHECKSUM.search(result.stdout)
    faults = FAULTS.search(result.stdout)
    if not all((throughput, checksum, faults)):
        raise RuntimeError(f"unrecognized Julia benchmark output:\n{result.stdout}")
    return {
        "throughput_million_ekf_turns_per_second": float(throughput.group(1)) / 1_000_000.0,
        "checksum": float(checksum.group(1)),
        "faults": int(faults.group(1)),
    }


def summarize(samples: list[dict[str, float | int]]) -> dict:
    throughput = [sample["throughput_million_ekf_turns_per_second"] for sample in samples]
    return {
        "samples_million_ekf_turns_per_second": throughput,
        "median_million_ekf_turns_per_second": statistics.median(throughput),
        "observed_range_million_ekf_turns_per_second": [min(throughput), max(throughput)],
        "checksums": [sample["checksum"] for sample in samples],
        "faults": [sample["faults"] for sample in samples],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--samples", type=int, default=7)
    args = parser.parse_args()
    records: dict[str, list[dict[str, float | int]]] = {
        "checked": [],
        "unchecked": [],
    }
    for sample in range(args.samples):
        order = ("checked", "unchecked") if sample % 2 == 0 else ("unchecked", "checked")
        for mode in order:
            records[mode].append(measure(mode, args.instances, args.turns))

    version = command("julia", "--version").stdout.strip()
    digest = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
    print(
        json.dumps(
            {
                "schema_version": 1,
                "generated_at": "2026-09-24",
                "platform": {
                    "system": platform.system(),
                    "release": platform.release(),
                    "architecture": platform.machine(),
                    "device": "Apple M1 GPU",
                },
                "toolchain": {
                    "julia": version,
                    "metal_jl": "1.10.3",
                    "metal": "3.2 MSL / 2.7 AIR / 1.2.8 metallib",
                },
                "configuration": {
                    "instances": args.instances,
                    "turns": args.turns,
                    "samples_per_mode": args.samples,
                    "warmup_turns": 5,
                    "threadgroup_size": THREADGROUP_SIZE,
                    "state": "one packed f32 structure-of-arrays buffer; both modes use the same resident double buffers",
                    "boundary": "one Metal.jl submission and synchronization per turn",
                    "checked_status": "two-word SharedStorage fault array read directly by the host after synchronization",
                    "checked_difference": "candidate predicates and compact fault status only; allocation, binding layout, publication buffers, and synchronization match unchecked mode",
                    "measurement_order": "checked and unchecked process order alternated by sample",
                },
                "sources": {
                    "kernel_and_host": str(SOURCE.relative_to(ROOT)),
                    "kernel_and_host_sha256": digest,
                },
                "rows": {mode: summarize(samples) for mode, samples in records.items()},
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
