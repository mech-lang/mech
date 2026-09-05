#!/usr/bin/env python3
"""Run the Mech backend and scalar-language parallel EKF comparisons."""

from __future__ import annotations

import argparse
import datetime
import json
import os
import platform
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


def sample(
    command: list[str],
    count: int,
    environment: dict[str, str],
    evidence: dict[str, dict[str, object]],
    name: str,
) -> list[str]:
    warmup = output(command, environment)
    measured = [output(command, environment) for _ in range(count)]
    evidence[name] = {
        "command": command,
        "discarded_warmup_stdout": warmup,
        "measured_stdout": measured,
    }
    return measured


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
    parser.add_argument(
        "--mech-metal-only",
        action="store_true",
        help="run the Mech backend lane through direct Metal without creating a WGPU session",
    )
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument(
        "--pypy",
        help="path to pypy3; when supplied, compare identical scalar sources under CPython and PyPy",
    )
    parser.add_argument(
        "--julia-source",
        type=Path,
        default=HERE / "julia_mojo_style.jl",
        help="Julia control source (defaults to the revised fixed-shape SoA lane)",
    )
    parser.add_argument(
        "--taichi-source",
        type=Path,
        help="optional Taichi control source; use a Python environment with Taichi installed",
    )
    parser.add_argument(
        "--taichi-backend", choices=("cpu", "gpu"), default="cpu"
    )
    parser.add_argument("--taichi-threads", type=int, default=8)
    parser.add_argument("--taichi-sync-each-turn", action="store_true")
    parser.add_argument(
        "--mojo",
        help="path to the Mojo compiler; when supplied, include the compiled Mojo scalar lane",
    )
    parser.add_argument(
        "--evidence-output",
        type=Path,
        help="write commands, metadata, warmups, and every measured stdout as JSON",
    )
    args = parser.parse_args()
    if args.mech_metal_only and platform.system() != "Darwin":
        raise RuntimeError("--mech-metal-only requires a macOS build with the metal-native feature")
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
    mojo = args.mojo or shutil.which("mojo")
    pypy = args.pypy
    missing = [name for name, path in required.items() if path is None]
    if missing:
        raise RuntimeError(f"missing benchmark tools: {', '.join(missing)}")
    if not args.mech_binary.is_file():
        raise RuntimeError(
            "build parallel_ekf_benchmark with --release --features native,jit first"
        )

    evidence_runs: dict[str, dict[str, object]] = {}

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
        mojo_binary = None
        mojo_textbook_fixed_binary = None
        if mojo is not None:
            mojo_binary = Path(temporary) / "mojo-scalar"
            output(
                [
                    mojo,
                    "build",
                    "-O3",
                    "--fp-mode",
                    "contract=off",
                    str(HERE / "mojo_scalar.mojo"),
                    "-o",
                    str(mojo_binary),
                ],
                environment,
            )
            mojo_textbook_fixed_binary = Path(temporary) / "mojo-textbook-fixed"
            output(
                [
                    mojo,
                    "build",
                    "-O3",
                    "--fp-mode",
                    "contract=off",
                    str(HERE / "mojo_textbook_fixed.mojo"),
                    "-o",
                    str(mojo_textbook_fixed_binary),
                ],
                environment,
            )
        julia_lane_name = (
            "Julia fixed-shape SoA"
            if args.julia_source.name == "julia_mojo_style.jl"
            else "Julia scalar outer loop"
        )
        language_commands = {
            "Mech scalar": None,
            "Rust optimized fixed-shape": [str(rust_binary), *common],
            "NumPy scalar outer loop": [
                args.python,
                str(HERE / "numpy_scalar.py"),
                *common,
            ],
            julia_lane_name: [
                required["julia"],
                "--startup-file=no",
                str(args.julia_source),
                *common,
            ],
            "LuaJIT scalar outer loop": [
                required["luajit"],
                str(HERE / "luajit_scalar.lua"),
                *common,
            ],
        }
        if pypy is not None:
            # Run the same source through CPython as a control.  Keeping the
            # command line, workload, and validation mode identical makes the
            # interpreter comparison a runtime comparison rather than a source
            # comparison.
            language_commands["CPython textbook scalar (unchecked)"] = [
                args.python,
                str(HERE / "pypy_textbook.py"),
                *common,
                "unchecked",
            ]
            language_commands["CPython textbook scalar (checked)"] = [
                args.python,
                str(HERE / "pypy_textbook.py"),
                *common,
                "checked",
            ]
            language_commands["CPython optimized scalar (unchecked)"] = [
                args.python,
                str(HERE / "pypy_optimized.py"),
                *common,
                "unchecked",
            ]
            language_commands["CPython optimized scalar (checked)"] = [
                args.python,
                str(HERE / "pypy_optimized.py"),
                *common,
                "checked",
            ]
            language_commands["PyPy textbook scalar (unchecked)"] = [
                pypy,
                str(HERE / "pypy_textbook.py"),
                *common,
                "unchecked",
            ]
            language_commands["PyPy textbook scalar (checked)"] = [
                pypy,
                str(HERE / "pypy_textbook.py"),
                *common,
                "checked",
            ]
            language_commands["PyPy optimized scalar (unchecked)"] = [
                pypy,
                str(HERE / "pypy_optimized.py"),
                *common,
                "unchecked",
            ]
            language_commands["PyPy optimized scalar (checked)"] = [
                pypy,
                str(HERE / "pypy_optimized.py"),
                *common,
                "checked",
            ]
        if mojo_binary is not None:
            language_commands["Mojo fixed-shape scalar (unchecked)"] = [
                str(mojo_binary),
                *common,
                "unchecked",
            ]
            language_commands["Mojo fixed-shape scalar (checked)"] = [
                str(mojo_binary),
                *common,
                "checked",
            ]
        if mojo_textbook_fixed_binary is not None:
            language_commands["Mojo textbook fixed matrix (unchecked)"] = [
                str(mojo_textbook_fixed_binary),
                *common,
                "unchecked",
            ]
            language_commands["Mojo textbook fixed matrix (checked)"] = [
                str(mojo_textbook_fixed_binary),
                *common,
                "checked",
            ]
        if args.taichi_source is not None:
            taichi_command = [
                args.python,
                str(args.taichi_source),
                "--backend",
                args.taichi_backend,
                "--instances",
                str(args.scalar_instances),
                "--turns",
                str(args.scalar_turns),
                "--samples",
                "1",
                "--threads",
                str(max(0, args.taichi_threads)),
            ]
            if args.taichi_sync_each_turn:
                taichi_command.append("--sync-each-turn")
            language_commands["Taichi fixed-shape SoA"] = taichi_command
        scalar_mech_outputs = []
        if not args.mech_metal_only:
            scalar_mech_outputs = sample(
                [str(args.mech_binary), *common, str(max(20, args.scalar_turns)), "120"],
                args.samples,
                environment,
                evidence_runs,
                "mech_scalar_settings",
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
            {
                **environment,
                **({"MECH_METAL_ONLY": "1"} if args.mech_metal_only else {}),
            },
            evidence_runs,
            "mech_backend_settings",
        )
        scalar = {}
        if not args.mech_metal_only:
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
            scalar["Mech Cranelift JIT"] = (
                statistics.median(
                    number(text, r"Mech Cranelift JIT throughput: ([0-9.]+) million")
                    for text in scalar_mech_outputs
                )
                * 1e6,
                statistics.median(
                    number(text, r"Mech Cranelift JIT checksum: ([0-9.eE+-]+)")
                    for text in scalar_mech_outputs
                ),
            )
        for lane, command in language_commands.items():
            if command is not None:
                count = (
                    args.luajit_samples if lane.startswith("LuaJIT") else args.samples
                )
                scalar[lane] = medians(
                    sample(command, count, environment, evidence_runs, lane)
                )
        reference = scalar["Rust optimized fixed-shape"][1]
        for lane, (_, checksum) in scalar.items():
            if abs(checksum - reference) > 0.1:
                raise RuntimeError(f"{lane} checksum {checksum} differs from Rust {reference}")

        if args.mech_metal_only:
            backend = {
                "Mech direct Metal checked": statistics.median(
                    number(
                        text,
                        r"Mech direct Metal checked throughput: ([0-9.]+) million",
                    )
                    for text in backend_mech_outputs
                ),
                "Mech direct Metal unchecked": statistics.median(
                    number(
                        text,
                        r"Mech direct Metal unchecked throughput: ([0-9.]+) million",
                    )
                    for text in backend_mech_outputs
                ),
            }
        else:
            backend = {
                "Mech scalar": statistics.median(
                    number(text, r"Mech scalar throughput: ([0-9.]+) million")
                    for text in backend_mech_outputs
                ),
                "Mech Cranelift JIT": statistics.median(
                    number(text, r"Mech Cranelift JIT throughput: ([0-9.]+) million")
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
                "Mech GPU, checked repeated turns": statistics.median(
                    number(text, r"GPU checked repeated throughput: ([0-9.]+) million")
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

        if args.evidence_output is not None:
            metadata_commands = {
                "git_commit": ["git", "rev-parse", "HEAD"],
                "rustc": [required["rustc"], "--version"],
                "python": [args.python, "--version"],
                "numpy": [
                    args.python,
                    "-c",
                    "import numpy; print(numpy.__version__)",
                ],
                "julia": [required["julia"], "--version"],
                "luajit": [required["luajit"], "-v"],
            }
            if mojo is not None:
                metadata_commands["mojo"] = [mojo, "--version"]
            if pypy is not None:
                metadata_commands["pypy"] = [pypy, "--version"]
            evidence = {
                "schema_version": 1,
                "generated_at": datetime.datetime.now().astimezone().isoformat(),
                "platform": {
                    "description": platform.platform(),
                    "machine": platform.machine(),
                },
                "configuration": {
                    "scalar_instances": args.scalar_instances,
                    "scalar_turns": args.scalar_turns,
                    "backend_instances": args.backend_instances,
                    "backend_cpu_turns": args.backend_cpu_turns,
                    "mech_metal_only": args.mech_metal_only,
                    "samples": args.samples,
                    "luajit_samples": args.luajit_samples,
                    "thread_environment": {
                        name: environment[name]
                        for name in (
                            "OMP_NUM_THREADS",
                            "OPENBLAS_NUM_THREADS",
                            "MKL_NUM_THREADS",
                            "VECLIB_MAXIMUM_THREADS",
                        )
                    },
                },
                "versions": {
                    name: output(command, environment).strip()
                    for name, command in metadata_commands.items()
                },
                "summary": {
                    "mech_backends_million_ekf_turns_per_second": backend,
                    "scalar_outer_loop": {
                        lane: {
                            "ekf_turns_per_second": throughput,
                            "checksum": checksum,
                        }
                        for lane, (throughput, checksum) in scalar.items()
                    },
                },
                "runs": evidence_runs,
            }
            args.evidence_output.parent.mkdir(parents=True, exist_ok=True)
            args.evidence_output.write_text(
                json.dumps(evidence, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )


if __name__ == "__main__":
    main()
