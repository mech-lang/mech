#!/usr/bin/env python3
"""Build and measure the matched Halide Metal EKF control."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import statistics
import subprocess
import tempfile
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SOURCE = HERE / "halide-metal-matched.cpp"
HALIDE = Path("/opt/homebrew/opt/halide")
THROUGHPUT = re.compile(r"throughput: ([0-9.eE+-]+)")
CHECKSUM = re.compile(r"checksum: ([0-9.eE+-]+)")
FAULTS = re.compile(r"faults: ([0-9]+)")
FAULT_WORD = re.compile(r"fault_word: ([0-9]+)")


def command(
    *arguments: str, env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
        env=env,
    )


def measure(
    binary: Path, mode: str, instances: int, turns: int, environment: dict[str, str]
) -> dict[str, float | int]:
    result = command(
        str(binary), str(instances), str(turns), mode, env=environment
    )
    throughput = THROUGHPUT.search(result.stdout)
    checksum = CHECKSUM.search(result.stdout)
    faults = FAULTS.search(result.stdout)
    fault_word = FAULT_WORD.search(result.stdout)
    if not all((throughput, checksum, faults, fault_word)):
        raise RuntimeError(f"unrecognized Halide benchmark output:\n{result.stdout}")
    return {
        "throughput_million_ekf_turns_per_second": float(throughput.group(1))
        / 1_000_000.0,
        "checksum": float(checksum.group(1)),
        "faults": int(faults.group(1)),
        "fault_word": int(fault_word.group(1)),
    }


def summarize(samples: list[dict[str, float | int]]) -> dict:
    throughput = [sample["throughput_million_ekf_turns_per_second"] for sample in samples]
    return {
        "samples_million_ekf_turns_per_second": throughput,
        "median_million_ekf_turns_per_second": statistics.median(throughput),
        "observed_range_million_ekf_turns_per_second": [min(throughput), max(throughput)],
        "checksums": [sample["checksum"] for sample in samples],
        "faults": [sample["faults"] for sample in samples],
        "fault_words": [sample["fault_word"] for sample in samples],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--backend", choices=("metal", "cpu"), default="metal")
    parser.add_argument("--cpu-threads", type=int, default=8)
    args = parser.parse_args()

    environment = os.environ.copy()
    environment["DYLD_LIBRARY_PATH"] = str(HALIDE / "lib")
    environment["HALIDE_BACKEND"] = args.backend
    if args.backend == "cpu":
        environment["HL_NUM_THREADS"] = str(max(1, args.cpu_threads))
    with tempfile.TemporaryDirectory(prefix="iros-halide-metal-") as temporary:
        binary = Path(temporary) / "halide-metal-matched"
        command(
            "clang++",
            "-O3",
            "-std=c++17",
            str(SOURCE),
            f"-I{HALIDE / 'include'}",
            f"-L{HALIDE / 'lib'}",
            "-lHalide",
            "-o",
            str(binary),
        )
        records: dict[str, list[dict[str, float | int]]] = {
            "checked": [],
            "unchecked": [],
        }
        for sample in range(args.samples):
            order = ("checked", "unchecked") if sample % 2 == 0 else (
                "unchecked",
                "checked",
            )
            for mode in order:
                records[mode].append(
                    measure(binary, mode, args.instances, args.turns, environment)
                )

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
                    "device": "Apple M1 GPU" if args.backend == "metal" else "Apple M1 CPU",
                },
                "toolchain": {
                    "halide": command("brew", "list", "--versions", "halide").stdout.strip(),
                    "compiler": command("clang++", "--version").stdout.splitlines()[0],
                    "backend": "native Metal" if args.backend == "metal" else "native CPU",
                },
                "configuration": {
                    "instances": args.instances,
                    "turns": args.turns,
                    "samples_per_mode": args.samples,
                    "warmup_turns": 5,
                    "threadgroup_size": 256 if args.backend == "metal" else None,
                    "cpu_threads": max(1, args.cpu_threads) if args.backend == "cpu" else None,
                    "state": "one packed component-major f32 buffer with identical resident double buffers in both modes",
                    "boundary": f"one Halide {args.backend} callable and completed publication per turn",
                    "checked_status": "resident per-lane fault plane copied and scanned after synchronization; publication buffer swaps only when clear",
                    "checked_difference": "candidate predicates plus fault-plane observation; state layout, allocation, launch geometry, and synchronization match unchecked mode",
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
