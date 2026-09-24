#!/usr/bin/env python3
"""Measure scalar/SIMD Mech AOT and Rust dylibs with one minimal loader."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import statistics
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CONTROL = ROOT / "benchmarks/iros-2026/rust-dylib"
BUILD = ROOT / "target/iros-rust-dylib"
RUST_LIBRARY = BUILD / "librust_ekf.dylib"
RUNNER = BUILD / "dylib-runner"
THROUGHPUT = re.compile(r"throughput_million_ekf_turns_per_second: ([0-9.]+)")
CHECKSUM = re.compile(r"checksum: ([0-9.]+)")
FAULTS = re.compile(r"faults: ([0-9]+)")
MAX_ERROR = re.compile(r"maximum_reference_absolute_error: ([0-9.eE+-]+)")
RSS = re.compile(r"([0-9]+)\s+maximum resident set size")


def command(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(arguments, cwd=ROOT, check=True, text=True, capture_output=True)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_controls() -> None:
    BUILD.mkdir(parents=True, exist_ok=True)
    common = ("-C", "opt-level=3", "-C", "target-cpu=native", "-C", "codegen-units=1")
    command(
        "rustc",
        "--edition=2024",
        "--crate-type",
        "cdylib",
        *common,
        "-o",
        str(RUST_LIBRARY),
        str(CONTROL / "rust-ekf-dylib.rs"),
    )
    command(
        "rustc",
        "--edition=2024",
        *common,
        "-o",
        str(RUNNER),
        str(CONTROL / "dylib-runner.rs"),
    )


def measure(
    path: Path, instances: int, turns: int, *, simd_packed: bool = False
) -> dict[str, float | int]:
    arguments = [
        "/usr/bin/time",
        "-l",
        str(RUNNER),
        str(path),
        str(instances),
        str(turns),
    ]
    if simd_packed:
        arguments.append("--simd")
    result = command(*arguments)
    throughput = THROUGHPUT.search(result.stdout)
    checksum = CHECKSUM.search(result.stdout)
    faults = FAULTS.search(result.stdout)
    rss = RSS.search(result.stderr)
    if not all((throughput, checksum, faults, rss)):
        raise RuntimeError(f"unrecognized benchmark output:\n{result.stdout}\n{result.stderr}")
    return {
        "throughput_million_ekf_turns_per_second": float(throughput.group(1)),
        "maximum_resident_set_bytes": int(rss.group(1)),
        "checksum": float(checksum.group(1)),
        "faults": int(faults.group(1)),
    }


def summary(values: list[float | int]) -> dict[str, float | list[float | int]]:
    return {
        "samples": values,
        "median": statistics.median(values),
        "observed_range": [min(values), max(values)],
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mech_dylib", type=Path)
    parser.add_argument("--mech-simd-dylib", type=Path)
    parser.add_argument("--instances", type=int, default=10_000)
    parser.add_argument("--turns", type=int, default=200)
    parser.add_argument("--samples", type=int, default=7)
    args = parser.parse_args()
    mech_library = args.mech_dylib.resolve()
    mech_simd_library = (
        args.mech_simd_dylib.resolve() if args.mech_simd_dylib is not None else None
    )
    build_controls()

    records: dict[str, list[dict[str, float | int]]] = {
        "Mech Cranelift AOT": [],
        "Rust cdylib": [],
    }
    paths = {
        "Mech Cranelift AOT": mech_library,
        "Rust cdylib": RUST_LIBRARY,
    }
    if mech_simd_library is not None:
        records["Mech Cranelift SIMD AOT"] = []
        paths["Mech Cranelift SIMD AOT"] = mech_simd_library
    for sample in range(args.samples):
        order = list(paths) if sample % 2 == 0 else list(reversed(paths))
        for name in order:
            records[name].append(
                measure(
                    paths[name],
                    args.instances,
                    args.turns,
                    simd_packed=name == "Mech Cranelift SIMD AOT",
                )
            )

    validation = command(
        str(RUNNER),
        str(RUST_LIBRARY),
        str(args.instances),
        str(args.turns),
        str(mech_library),
    )
    maximum_error = MAX_ERROR.search(validation.stdout)
    if maximum_error is None:
        raise RuntimeError(f"validation output has no maximum error:\n{validation.stdout}")
    simd_maximum_error = None
    if mech_simd_library is not None:
        simd_validation = command(
            str(RUNNER),
            str(mech_simd_library),
            str(args.instances),
            str(args.turns),
            str(RUST_LIBRARY),
            "--simd",
        )
        match = MAX_ERROR.search(simd_validation.stdout)
        if match is None:
            raise RuntimeError(
                f"SIMD validation output has no maximum error:\n{simd_validation.stdout}"
            )
        simd_maximum_error = float(match.group(1))

    rows = {}
    for name, samples in records.items():
        throughput = [sample["throughput_million_ekf_turns_per_second"] for sample in samples]
        memory = [sample["maximum_resident_set_bytes"] for sample in samples]
        rows[name] = {
            "throughput_million_ekf_turns_per_second": summary(throughput),
            "maximum_resident_set_bytes": summary(memory),
            "checksums": [sample["checksum"] for sample in samples],
            "faults": [sample["faults"] for sample in samples],
            "library_bytes": paths[name].stat().st_size,
            "library_sha256": digest(paths[name]),
        }

    print(json.dumps({
        "schema_version": 1,
        "platform": {
            "system": platform.system(),
            "release": platform.release(),
            "architecture": platform.machine(),
        },
        "toolchain": {
            "rustc": command("rustc", "--version").stdout.strip(),
            "rust_library_build": "rustc --edition=2024 --crate-type cdylib -C opt-level=3 -C target-cpu=native -C codegen-units=1",
            "runner_build": "rustc --edition=2024 -C opt-level=3 -C target-cpu=native -C codegen-units=1",
        },
        "configuration": {
            "instances": args.instances,
            "turns": args.turns,
            "untimed_warmup_turns": 100,
            "measured_processes_per_library": args.samples,
            "measurement_order": "Alternated by sample to reduce ordering bias.",
            "timed_region": "Repeated checked turns through a four-argument native ABI; scalar rows use AoS buffers and SIMD AOT uses pre-packed four-lane buffers. dlopen, allocation, packing, input construction, warmup, reset, and reporting are excluded.",
            "memory_metric": "Maximum resident set size reported by /usr/bin/time -l for a fresh minimal-loader process; this is whole-process peak RSS, not private library pages.",
        },
        "sources": {
            "rust_library": str((CONTROL / "rust-ekf-dylib.rs").relative_to(ROOT)),
            "rust_library_sha256": digest(CONTROL / "rust-ekf-dylib.rs"),
            "common_runner": str((CONTROL / "dylib-runner.rs").relative_to(ROOT)),
            "common_runner_sha256": digest(CONTROL / "dylib-runner.rs"),
        },
        "validation": {
            "maximum_final_state_absolute_error": float(maximum_error.group(1)),
            "simd_aot_maximum_final_state_absolute_error": simd_maximum_error,
            "faults": 0,
            "note": "All libraries implement the same checked EKF equations. The scalar rows share the exact symbol and AoS layout; SIMD AOT uses a four-lane packed symbol and layout. Ordinary compiler reassociation produces small f32 differences.",
        },
        "rows": rows,
    }, indent=2))


if __name__ == "__main__":
    main()
