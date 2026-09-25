#!/usr/bin/env python3
"""Measure the matched Taichi Metal EKF control in fresh processes."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import statistics
import subprocess
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SOURCE = HERE / "taichi-metal-matched.py"
THROUGHPUT = re.compile(
    r"throughput_million_ekf_turns_per_second: ([0-9.eE+-]+)"
)
CHECKSUM = re.compile(r"checksum: ([0-9.eE+-]+)")
FAULTS = re.compile(r"faults: ([0-9]+)")
FAULT_WORD = re.compile(r"fault_word: ([0-9]+)")
RESIDENT_BYTES = re.compile(r"resident_bytes: ([0-9]+)")


def command(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
    )


def measure(
    python: str,
    mode: str,
    instances: int,
    turns: int,
    arch: str,
    block: int,
    cpu_threads: int,
) -> dict[str, float | int]:
    result = command(
        python,
        str(SOURCE),
        str(instances),
        str(turns),
        mode,
        "--arch",
        arch,
        "--block-dim",
        str(block),
        "--cpu-threads",
        str(cpu_threads),
    )
    patterns = {
        "throughput_million_ekf_turns_per_second": THROUGHPUT,
        "checksum": CHECKSUM,
        "faults": FAULTS,
        "fault_word": FAULT_WORD,
        "resident_bytes": RESIDENT_BYTES,
    }
    matches = {name: pattern.search(result.stdout) for name, pattern in patterns.items()}
    if not all(matches.values()):
        raise RuntimeError(f"unrecognized Taichi benchmark output:\n{result.stdout}")
    return {
        "throughput_million_ekf_turns_per_second": float(
            matches["throughput_million_ekf_turns_per_second"].group(1)
        ),
        "checksum": float(matches["checksum"].group(1)),
        "faults": int(matches["faults"].group(1)),
        "fault_word": int(matches["fault_word"].group(1)),
        "resident_bytes": int(matches["resident_bytes"].group(1)),
    }


def summarize(samples: list[dict[str, float | int]]) -> dict:
    throughput = [
        sample["throughput_million_ekf_turns_per_second"] for sample in samples
    ]
    return {
        "samples_million_ekf_turns_per_second": throughput,
        "median_million_ekf_turns_per_second": statistics.median(throughput),
        "observed_range_million_ekf_turns_per_second": [
            min(throughput),
            max(throughput),
        ],
        "checksums": [sample["checksum"] for sample in samples],
        "faults": [sample["faults"] for sample in samples],
        "fault_words": [sample["fault_word"] for sample in samples],
        "resident_bytes": samples[0]["resident_bytes"],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--python", default=shutil.which("python3"))
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--arch", choices=("metal", "cpu"), default="metal")
    parser.add_argument("--threadgroup-size", type=int, default=64)
    parser.add_argument("--cpu-threads", type=int, default=8)
    args = parser.parse_args()
    if not args.python:
        parser.error("Python was not found; pass --python /path/to/python")

    records: dict[str, list[dict[str, float | int]]] = {
        "checked": [],
        "unchecked": [],
    }
    for sample in range(args.samples):
        order = (
            ("checked", "unchecked")
            if sample % 2 == 0
            else ("unchecked", "checked")
        )
        for mode in order:
            records[mode].append(
                measure(
                    args.python,
                    mode,
                    args.instances,
                    args.turns,
                    args.arch,
                    args.threadgroup_size,
                    args.cpu_threads,
                )
            )

    version = command(
        args.python,
        "-c",
        "import platform, taichi; version='.'.join(map(str, taichi.__version__)); print(f'Taichi {version}, LLVM 15.0.7, Python {platform.python_version()}')",
    ).stdout.splitlines()[-1]
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
                    "device": "Apple M1 GPU" if args.arch == "metal" else "Apple M1 CPU",
                },
                "toolchain": {
                    "taichi": version,
                    "backend": "native Metal" if args.arch == "metal" else "LLVM CPU",
                },
                "configuration": {
                    "instances": args.instances,
                    "turns": args.turns,
                    "samples_per_mode": args.samples,
                    "warmup_turns": 5,
                    "threadgroup_size": args.threadgroup_size,
                    "cpu_threads": args.cpu_threads if args.arch == "cpu" else None,
                    "state": "one packed component-major f32 field with identical resident double buffers in both modes and backend-specialized axis order",
                    "boundary": f"one Taichi {args.arch} kernel and explicit synchronization per turn",
                    "checked_status": "two-word device status; cumulative fault count avoids a per-turn reset transfer",
                    "checked_difference": "candidate predicates, atomics on fault only, and one compact status read after synchronization",
                    "publication": "the destination state is published only when the cumulative fault count did not change during the turn",
                    "measurement_order": "checked and unchecked process order alternated by sample",
                },
                "sources": {
                    "kernel_and_host": str(SOURCE.relative_to(ROOT)),
                    "kernel_and_host_sha256": digest,
                },
                "rows": {
                    mode: summarize(samples) for mode, samples in records.items()
                },
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
