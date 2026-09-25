#!/usr/bin/env python3
"""Build and measure the Rust-hosted hand-written MSL EKF control."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import statistics
import subprocess
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CONTROL = HERE / "rust-metal"
BINARY = CONTROL / "target/release/iros-rust-metal-ekf"
THROUGHPUT = re.compile(r"throughput_million_ekf_turns_per_second: ([0-9.]+)")
CHECKSUM = re.compile(r"checksum: ([0-9.]+)")
FAULTS = re.compile(r"faults: ([0-9]+)")


def command(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(arguments, cwd=ROOT, check=True, text=True, capture_output=True)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def measure(mode: str, instances: int, turns: int) -> dict[str, float | int]:
    result = command(str(BINARY), str(instances), str(turns), mode)
    throughput = THROUGHPUT.search(result.stdout)
    checksum = CHECKSUM.search(result.stdout)
    faults = FAULTS.search(result.stdout)
    if not all((throughput, checksum, faults)):
        raise RuntimeError(f"unrecognized benchmark output:\n{result.stdout}")
    return {
        "throughput_million_ekf_turns_per_second": float(throughput.group(1)),
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
    command(
        "cargo",
        "build",
        "--manifest-path",
        str(CONTROL / "Cargo.toml"),
        "--release",
    )

    records: dict[str, list[dict[str, float | int]]] = {
        "checked": [],
        "unchecked": [],
    }
    for sample in range(args.samples):
        order = ("checked", "unchecked") if sample % 2 == 0 else ("unchecked", "checked")
        for mode in order:
            records[mode].append(measure(mode, args.instances, args.turns))

    source = CONTROL / "src/main.rs"
    kernel = CONTROL / "src/ekf.metal"
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
                    "rustc": command("rustc", "--version").stdout.strip(),
                    "metal_crate": "0.28.0",
                    "kernel_language": "hand-written Metal Shading Language",
                },
                "configuration": {
                    "instances": args.instances,
                    "turns": args.turns,
                    "samples_per_mode": args.samples,
                    "warmup_turns": 5,
                    "threadgroup_size": 64,
                    "state": "f32 structure-of-arrays, resident double buffer",
                    "boundary": "one Metal command buffer and host completion wait per turn",
                    "checked_contract": "finite candidate, positive covariance diagonal, covariance symmetry; reject before publication and return compact fault status",
                    "measurement_order": "checked and unchecked process order alternated by sample",
                },
                "classification": "Rust host plus hand-written MSL; stable Rust does not compile Rust kernels directly to Apple Metal",
                "sources": {
                    "host": str(source.relative_to(ROOT)),
                    "host_sha256": digest(source),
                    "kernel": str(kernel.relative_to(ROOT)),
                    "kernel_sha256": digest(kernel),
                },
                "rows": {mode: summarize(samples) for mode, samples in records.items()},
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
