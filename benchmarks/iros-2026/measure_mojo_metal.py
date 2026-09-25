#!/usr/bin/env python3
"""Build and measure the matched native-Mojo Metal EKF control."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import statistics
import subprocess
import tempfile
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SOURCE = HERE / "mojo-metal-matched.mojo"
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
    binary: Path, mode: str, instances: int, turns: int, block: int
) -> dict[str, float | int]:
    result = command(
        str(binary), str(instances), str(turns), mode, str(block)
    )
    values = {
        "throughput_million_ekf_turns_per_second": THROUGHPUT,
        "checksum": CHECKSUM,
        "faults": FAULTS,
        "fault_word": FAULT_WORD,
        "resident_bytes": RESIDENT_BYTES,
    }
    matches = {name: pattern.search(result.stdout) for name, pattern in values.items()}
    if not all(matches.values()):
        raise RuntimeError(f"unrecognized Mojo benchmark output:\n{result.stdout}")
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
    parser.add_argument("--mojo", default=shutil.which("mojo"))
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--samples", type=int, default=7)
    parser.add_argument("--threadgroup-size", type=int, default=64)
    args = parser.parse_args()
    if not args.mojo:
        parser.error("Mojo was not found; pass --mojo /path/to/mojo")

    with tempfile.TemporaryDirectory(prefix="iros-mojo-metal-") as temporary:
        binary = Path(temporary) / "mojo-metal-matched"
        command(args.mojo, "build", str(SOURCE), "-o", str(binary))
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
                        binary,
                        mode,
                        args.instances,
                        args.turns,
                        args.threadgroup_size,
                    )
                )

    version = command(args.mojo, "--version").stdout.strip()
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
                    "mojo": version,
                    "max": "26.6.0",
                    "backend": "native Metal",
                },
                "configuration": {
                    "instances": args.instances,
                    "turns": args.turns,
                    "samples_per_mode": args.samples,
                    "warmup_turns": 5,
                    "threadgroup_size": args.threadgroup_size,
                    "state": "one packed component-major f32 state buffer with identical resident double buffers in both modes",
                    "boundary": "one native Mojo Metal submission and synchronization per turn",
                    "math": "fast AIR sin, cos, and atan2 intrinsics, matching the hand-written MSL control's default transcendental lowering",
                    "checked_status": "two-word device status accessed zero-copy through unsafe_host_ptr on Apple unified memory",
                    "checked_difference": "candidate predicates and two compact atomics on fault only; allocation, binding layout, publication buffers, and synchronization match unchecked mode",
                    "publication": "the destination state is published only after a zero fault count; otherwise the source remains current",
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
