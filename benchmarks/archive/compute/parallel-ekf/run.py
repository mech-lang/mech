#!/usr/bin/env python3
"""Run the Mech backend and cross-language parallel EKF comparisons."""

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
    parser.add_argument("--backend-gpu-turns", type=int, default=120)
    parser.add_argument(
        "--taichi-python",
        type=Path,
        help="Python interpreter with Taichi installed; runs the synchronized GPU controls",
    )
    parser.add_argument(
        "--taichi-arch",
        default="gpu",
        help="Taichi backend name (for example metal, cuda, vulkan, or cpu)",
    )
    parser.add_argument(
        "--taichi-cpu-threads",
        type=int,
        help="pin Taichi LLVM CPU workers when --taichi-arch=cpu (run 1 for SIMD-only)",
    )
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--luajit-samples", type=int, default=5)
    parser.add_argument(
        "--mech-binary",
        type=Path,
        default=ROOT / "target/release/examples/parallel_ekf_benchmark",
    )
    parser.add_argument(
        "--rust-simd-binary",
        type=Path,
        help="use a prebuilt packed Rust SIMD control instead of compiling it",
    )
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument(
        "--evidence-output",
        type=Path,
        help="write commands, metadata, warmups, and every measured stdout as JSON",
    )
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
    julia_simd_check = subprocess.run(
        [required["julia"], "--startup-file=no", "-e", "using StaticArrays; using SIMD"],
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    if julia_simd_check.returncode != 0:
        raise RuntimeError(
            "Julia SIMD comparison requires StaticArrays and SIMD.jl; install "
            "them in the Julia environment used by the runner (Pkg.add([\"StaticArrays\", \"SIMD\"]))"
        )
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
                "-C",
                "codegen-units=1",
                str(ROOT / "hosts/gpu/examples/parallel_ekf_rust_scalar.rs"),
                "-o",
                str(rust_binary),
            ],
            environment,
        )
        rust_simd_binary = args.rust_simd_binary or Path(temporary) / "rust-simd"
        if args.rust_simd_binary is None:
            wide_libraries = sorted((ROOT / "target/release/deps").glob("libwide-*.rlib"))
            if not wide_libraries:
                raise RuntimeError(
                    "the Rust SIMD control needs the repository's wide dependency; "
                    "build the mech-gpu release artifacts first"
                )
            output(
                [
                    required["rustc"],
                    "--edition=2024",
                    "-C",
                    "opt-level=3",
                    "-C",
                    "target-cpu=native",
                    "-C",
                    "codegen-units=1",
                    str(ROOT / "hosts/gpu/examples/parallel_ekf_rust_simd.rs"),
                    "--extern",
                    f"wide={wide_libraries[-1]}",
                    "-L",
                    f"dependency={ROOT / 'target/release/deps'}",
                    "-o",
                    str(rust_simd_binary),
                ],
                environment,
            )
        if not rust_simd_binary.is_file():
            raise RuntimeError(f"Rust SIMD control does not exist: {rust_simd_binary}")
        common = [str(args.scalar_instances), str(args.scalar_turns)]
        language_commands = {
            "Mech scalar": None,
            "Rust optimized fixed-shape": [str(rust_binary), *common],
            "Rust packed SIMD unchecked": [str(rust_simd_binary), *common, "unchecked"],
            "Rust packed SIMD checked": [str(rust_simd_binary), *common, "checked"],
            "NumPy scalar outer loop": [
                args.python,
                str(HERE / "numpy_scalar.py"),
                *common,
            ],
            "NumPy vectorized fixed-shape unchecked": [
                args.python,
                str(HERE / "numpy_vectorized.py"),
                *common,
                "unchecked",
            ],
            "NumPy vectorized fixed-shape checked": [
                args.python,
                str(HERE / "numpy_vectorized.py"),
                *common,
                "checked",
            ],
            "Julia generic unchecked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_scalar.jl"),
                *common,
                "unchecked",
            ],
            "Julia generic checked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_scalar.jl"),
                *common,
                "checked",
            ],
            "Julia fixed-shape unchecked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_flat.jl"),
                *common,
                "unchecked",
            ],
            "Julia fixed-shape checked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_flat.jl"),
                *common,
                "checked",
            ],
            "Julia fixed-shape SIMD unchecked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_simd.jl"),
                *common,
                "unchecked",
            ],
            "Julia fixed-shape SIMD checked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_simd.jl"),
                *common,
                "checked",
            ],
            "Julia SIMD.jl intrinsics unchecked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_simd_intrinsics.jl"),
                *common,
                "unchecked",
            ],
            "Julia SIMD.jl intrinsics checked": [
                required["julia"],
                "--startup-file=no",
                str(HERE / "julia_simd_intrinsics.jl"),
                *common,
                "checked",
            ],
            "LuaJIT scalar outer loop": [
                required["luajit"],
                str(HERE / "luajit_scalar.lua"),
                *common,
            ],
            "LuaJIT fixed-shape flat unchecked": [
                required["luajit"],
                str(HERE / "luajit_fast.lua"),
                *common,
                "unchecked",
            ],
            "LuaJIT fixed-shape flat checked": [
                required["luajit"],
                str(HERE / "luajit_fast.lua"),
                *common,
                "checked",
            ],
        }
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
                str(args.backend_gpu_turns),
            ],
            args.samples,
            environment,
            evidence_runs,
            "mech_backend_settings",
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
        for label, pattern in (
            ("Mech Cranelift SIMD-JIT", r"Mech Cranelift SIMD-JIT throughput: ([0-9.]+) million"),
            (
                "Mech Cranelift SIMD-JIT checked fast",
                r"Mech Cranelift SIMD-JIT checked fast throughput: ([0-9.]+) million",
            ),
            (
                "Mech Cranelift SIMD-JIT unchecked fast",
                r"Mech Cranelift SIMD-JIT unchecked fast throughput: ([0-9.]+) million",
            ),
        ):
            scalar[label] = (
                statistics.median(number(text, pattern) for text in scalar_mech_outputs)
                * 1e6,
                statistics.median(
                    number(text, r"Mech Cranelift SIMD-JIT checksum: ([0-9.eE+-]+)")
                    for text in scalar_mech_outputs
                ),
            )
        for label, pattern, checksum_pattern in (
            (
                "Mech Cranelift SIMD-JIT parallel",
                r"Mech Cranelift SIMD-JIT parallel throughput: ([0-9.]+) million",
                r"Mech Cranelift SIMD-JIT parallel checksum: ([0-9.eE+-]+)",
            ),
            (
                "Mech Cranelift SIMD-JIT parallel unchecked fast",
                r"Mech Cranelift SIMD-JIT parallel unchecked fast throughput: ([0-9.]+) million",
                r"Mech Cranelift SIMD-JIT parallel unchecked fast checksum: ([0-9.eE+-]+)",
            ),
        ):
            scalar[label] = (
                statistics.median(number(text, pattern) for text in scalar_mech_outputs)
                * 1e6,
                statistics.median(
                    number(text, checksum_pattern) for text in scalar_mech_outputs
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
        scalar_checksum_tolerance = max(0.1, args.scalar_instances * 2.0e-5)
        for lane, (_, checksum) in scalar.items():
            if abs(checksum - reference) > scalar_checksum_tolerance:
                raise RuntimeError(
                    f"{lane} checksum {checksum} differs from Rust {reference} "
                    f"beyond aggregate f32 tolerance {scalar_checksum_tolerance}"
                )

        backend = {
            "Mech scalar": statistics.median(
                number(text, r"Mech scalar throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech Cranelift JIT": statistics.median(
                number(text, r"Mech Cranelift JIT throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech Cranelift SIMD-JIT": statistics.median(
                number(text, r"Mech Cranelift SIMD-JIT throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech Cranelift SIMD-JIT checked fast": statistics.median(
                number(
                    text,
                    r"Mech Cranelift SIMD-JIT checked fast throughput: ([0-9.]+) million",
                )
                for text in backend_mech_outputs
            ),
            "Mech Cranelift SIMD-JIT unchecked fast": statistics.median(
                number(
                    text,
                    r"Mech Cranelift SIMD-JIT unchecked fast throughput: ([0-9.]+) million",
                )
                for text in backend_mech_outputs
            ),
            "Mech SIMD (4xf32)": statistics.median(
                number(text, r"Mech SIMD throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech GPU, checked one-turn API call": statistics.median(
                number(text, r"GPU checked one-turn throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech GPU, checked repeated API call": statistics.median(
                number(
                    text,
                    r"GPU checked repeated throughput \(per-turn validation\): ([0-9.]+) million",
                )
                for text in backend_mech_outputs
            ),
            "Mech GPU, unchecked one-turn API call": statistics.median(
                number(text, r"GPU unchecked one-turn throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech GPU, unchecked repeated dispatches": statistics.median(
                number(text, r"GPU unchecked repeated throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
            "Mech GPU, unchecked one submission": statistics.median(
                number(text, r"GPU unchecked one-submit throughput: ([0-9.]+) million")
                for text in backend_mech_outputs
            ),
        }
        mech_gpu_checksums = {
            "checked": statistics.median(
                number(text, r"Mech GPU checked checksum: ([0-9.eE+-]+)")
                for text in backend_mech_outputs
            ),
            "unchecked": statistics.median(
                number(text, r"Mech GPU unchecked checksum: ([0-9.eE+-]+)")
                for text in backend_mech_outputs
            ),
        }
        if args.taichi_python is not None:
            taichi_environment = environment.copy()
            taichi_environment["TAICHI_ARCH"] = args.taichi_arch
            taichi_commands = {
                "Taichi Vector/Matrix resident unchecked": [
                    str(args.taichi_python),
                    str(HERE / "taichi_comparable.py"),
                    str(args.backend_instances),
                    str(args.backend_gpu_turns),
                    "unchecked",
                ],
                "Taichi Vector/Matrix resident checked": [
                    str(args.taichi_python),
                    str(HERE / "taichi_comparable.py"),
                    str(args.backend_instances),
                    str(args.backend_gpu_turns),
                    "checked",
                ],
                "Taichi Vector/Matrix resident unchecked batched": [
                    str(args.taichi_python),
                    str(HERE / "taichi_comparable.py"),
                    str(args.backend_instances),
                    str(args.backend_gpu_turns),
                    "unchecked-batched",
                ],
            }
            if args.taichi_arch.lower() == "cpu" and args.taichi_cpu_threads is not None:
                for command in taichi_commands.values():
                    command.extend(["--cpu-threads", str(args.taichi_cpu_threads)])
            taichi_checksums = {}
            for mode, lane in (
                ("unchecked", "Taichi Vector/Matrix resident unchecked"),
                ("checked", "Taichi Vector/Matrix resident checked"),
                ("unchecked-batched", "Taichi Vector/Matrix resident unchecked batched"),
            ):
                taichi_outputs = sample(
                    taichi_commands[lane],
                    args.samples,
                    taichi_environment,
                    evidence_runs,
                    lane,
                )
                throughput_value, checksum_value = medians(taichi_outputs)
                backend[lane] = throughput_value / 1e6
                taichi_checksums[mode] = checksum_value
            backend_checksum_tolerance = max(0.1, args.backend_instances * 2.0e-5)
            for mode, checksum in taichi_checksums.items():
                reference_mode = "unchecked" if mode == "unchecked-batched" else mode
                if abs(checksum - mech_gpu_checksums[reference_mode]) > backend_checksum_tolerance:
                    raise RuntimeError(
                        f"Taichi {mode} checksum {checksum} differs from Mech GPU "
                        f"{mech_gpu_checksums[reference_mode]} beyond f32 tolerance {backend_checksum_tolerance}"
                    )
        else:
            backend_checksum_tolerance = max(0.1, args.backend_instances * 2.0e-5)
        print("| Mech backend | Million EKF-turns/s |")
        print("| --- | ---: |")
        for lane, throughput in backend.items():
            print(f"| {lane} | {throughput:.3f} |")
        print("\n| Execution lane | Million EKF-turns/s |")
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
            if args.taichi_python is not None:
                metadata_commands["taichi"] = [
                    str(args.taichi_python),
                    "-c",
                    "import taichi; print(taichi.__version__)",
                ]
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
                    "backend_gpu_turns": args.backend_gpu_turns,
                    "taichi_arch": args.taichi_arch,
                    "taichi_cpu_threads": args.taichi_cpu_threads,
                    "samples": args.samples,
                    "luajit_samples": args.luajit_samples,
                    "scalar_checksum_tolerance": scalar_checksum_tolerance,
                    "backend_checksum_tolerance": backend_checksum_tolerance,
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
