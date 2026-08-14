#!/usr/bin/env python3
"""Run the Mech backend and scalar-language parallel EKF comparisons."""

from __future__ import annotations

import argparse
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent


def output(command: list[str], environment: dict[str, str]) -> str:
    return subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout


def number(text: str, pattern: str) -> float:
    match = re.search(pattern, text, flags=re.MULTILINE)
    if match is None:
        raise RuntimeError(f"missing result matching {pattern!r}:\n{text}")
    return float(match.group(1))


def sample(command: list[str], count: int, environment: dict[str, str]) -> list[str]:
    output(command, environment)
    return [output(command, environment) for _ in range(count)]


def medians(outputs: list[str]) -> tuple[float, float]:
    throughputs = [number(text, r"^throughput: ([0-9.eE+-]+)$") for text in outputs]
    checksums = [number(text, r"^checksum: ([0-9.eE+-]+)$") for text in outputs]
    return statistics.median(throughputs), statistics.median(checksums)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scalar-instances", type=int, default=10_000)
    parser.add_argument("--scalar-turns", type=int, default=20)
    parser.add_argument("--backend-instances", type=int, default=100_000)
    parser.add_argument("--backend-cpu-turns", type=int, default=5)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--luajit-samples", type=int, default=5)
    parser.add_argument(
        "--mech-binary",
        type=Path,
        default=ROOT / "target/release/examples/parallel_ekf_benchmark",
    )
    parser.add_argument("--python", default=sys.executable)
    args = parser.parse_args()
    environment = os.environ.copy()
    for name in (
        "OMP_NUM_THREADS",
        "OPENBLAS_NUM_THREADS",
        "MKL_NUM_THREADS",
        "VECLIB_MAXIMUM_THREADS",
    ):
        environment[name] = "1"

    required = {
        "julia": shutil.which("julia"),
        "luajit": shutil.which("luajit"),
        "rustc": shutil.which("rustc"),
    }
    missing = [name for name, path in required.items() if path is None]
    if missing:
        raise RuntimeError(f"missing benchmark tools: {', '.join(missing)}")
    if not args.mech_binary.is_file():
        raise RuntimeError("build parallel_ekf_benchmark with --release --features native first")

    with tempfile.TemporaryDirectory(prefix="mech-ekf-") as temporary:
        rust_binary = Path(temporary) / "rust-scalar"
        output(
            [
                required["rustc"],
                "--edition=2024",
                "-C",
                "opt-level=3",
                "-C",
                "target-cpu=native",
                str(ROOT / "hosts/gpu/examples/parallel_ekf_rust_scalar.rs"),
                "-o",
                str(rust_binary),
            ],
            environment,
        )
        common = [str(args.scalar_instances), str(args.scalar_turns)]
        language_commands = {
            "Mech scalar": None,
            "Rust scalar fixed-shape": [str(rust_binary), *common],
            "NumPy scalar outer loop": [
                args.python,
                str(HERE / "numpy_scalar.py"),
                *common,
            ],
            "Julia scalar outer loop": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_scalar.jl"),
                *common,
            ],
            "LuaJIT scalar outer loop": [
                required["luajit"],
                str(HERE / "luajit_scalar.lua"),
                *common,
            ],
        }
        scalar_mech_outputs = sample(
            [str(args.mech_binary), *common, str(max(20, args.scalar_turns)), "120"],
            args.samples,
            environment,
        )
        backend_mech_outputs = sample(
            [
                str(args.mech_binary),
                str(args.backend_instances),
                str(args.backend_cpu_turns),
                "40",
                "120",
            ],
            args.samples,
            environment,
        )
        scalar = {}
        scalar["Mech scalar"] = (
            statistics.median(
                number(text, r"Mech scalar throughput: ([0-9.]+) million")
                for text in scalar_mech_outputs
            )
            * 1e6,
            statistics.median(
                number(text, r"Mech scalar checksum: ([0-9.eE+-]+)")
                for text in scalar_mech_outputs
            ),
        )
        for lane, command in language_commands.items():
            if command is not None:
                count = (
                    args.luajit_samples if lane.startswith("LuaJIT") else args.samples
                )
                scalar[lane] = medians(sample(command, count, environment))
        reference = scalar["Rust scalar fixed-shape"][1]
        for lane, (_, checksum) in scalar.items():
            if abs(checksum - reference) > 0.1:
                raise RuntimeError(f"{lane} checksum {checksum} differs from Rust {reference}")

        backend = {
            "Mech scalar": statistics.median(
                number(text, r"Mech scalar throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech SIMD (4xf32)": statistics.median(
                number(text, r"Mech SIMD throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech GPU, one submission/turn": statistics.median(
                number(text, r"GPU single-submit throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech GPU, 120 turns/submission": statistics.median(
                number(text, r"GPU batched throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
        }
        print("| Mech backend | Million EKF-turns/s |")
        print("| --- | ---: |")
        for lane, throughput in backend.items():
            print(f"| {lane} | {throughput:.3f} |")
        print("\n| Scalar outer-loop lane | Million EKF-turns/s |")
        print("| --- | ---: |")
        for lane, (throughput, _) in scalar.items():
            print(f"| {lane} | {throughput / 1e6:.3f} |")


if __name__ == "__main__":
    main()
