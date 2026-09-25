#!/usr/bin/env python3
"""Run the publication CPU and Metal comparisons in equal fresh-process windows.

The script deliberately builds nothing.  It accepts the prebuilt controls so a
campaign can be run only after every toolchain has been prepared, then executes
one fresh process for every implementation/mode in a deterministic shuffled
order.  Mech's retained CPU and Metal harnesses report both modes from one fresh
process; each of their ten samples still comes from an independent process.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
import os
import platform
import random
import re
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Callable


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARCHIVE = ROOT / "benchmarks/archive/compute/parallel-ekf"
MILLION = 1_000_000.0


def command(
    arguments: list[str],
    *,
    environment: dict[str, str] | None = None,
    stdin: str | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        arguments,
        cwd=ROOT,
        env=environment,
        input=stdin,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )


def value(text: str, pattern: str) -> float:
    match = re.search(pattern, text, flags=re.MULTILINE)
    if match is None:
        raise RuntimeError(f"missing {pattern!r} in output:\n{text}")
    return float(match.group(1))


def integer(text: str, pattern: str) -> int:
    match = re.search(pattern, text, flags=re.MULTILINE)
    if match is None:
        raise RuntimeError(f"missing {pattern!r} in output:\n{text}")
    return int(match.group(1))


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def summarize(records: list[dict[str, object]]) -> dict[str, object]:
    samples = [float(record["throughput_million_ekf_turns_per_second"]) for record in records]
    median = statistics.median(samples)
    return {
        "samples_million_ekf_turns_per_second": samples,
        "median_million_ekf_turns_per_second": median,
        "median_absolute_deviation_million_ekf_turns_per_second": statistics.median(
            abs(sample - median) for sample in samples
        ),
        "observed_range_million_ekf_turns_per_second": [min(samples), max(samples)],
        "checksums": [record.get("checksum") for record in records],
        "faults": [record.get("faults") for record in records],
        "records": records,
    }


def parsed_process(
    arguments: list[str],
    *,
    environment: dict[str, str] | None = None,
    throughput_pattern: str = r"^throughput: ([0-9.eE+-]+)$",
    throughput_is_millions: bool = False,
) -> dict[str, object]:
    result = command(arguments, environment=environment)
    throughput = value(result.stdout, throughput_pattern)
    if not throughput_is_millions:
        throughput /= MILLION
    return {
        "throughput_million_ekf_turns_per_second": throughput,
        "checksum": value(result.stdout, r"^checksum: ([0-9.eE+-]+)$"),
        "faults": integer(result.stdout, r"^faults: ([0-9]+)$"),
        "command": arguments,
        "stdout": result.stdout,
    }


def futhark_input(instances: int, turns: int) -> str:
    columns: list[list[str]] = [[], [], []]
    for index in range(instances):
        phase = 2.0 * math.pi * index / instances
        values = (
            1.0 + 0.05 * math.sin(3.0 * phase),
            0.015 * (1.0 + 0.1 * math.sin(2.0 * phase)),
            -0.55 + 0.01 * math.sin(7.0 * phase) + 0.005 * math.sin(11.0 * phase),
        )
        for column, number in zip(columns, values, strict=True):
            column.append(f"{number:.9g}f32")
    arrays = ["[" + ",".join(column) + "]" for column in columns]
    return f"{' '.join(arrays)} {turns}i32"


def run_campaign(
    name: str,
    tasks: dict[str, Callable[[int], dict[str, list[dict[str, object]]]]],
    samples: int,
    seed: int,
    rows: dict[str, dict[str, list[dict[str, object]]]],
) -> dict[str, object]:
    run_order: list[list[str]] = []
    for round_index in range(samples):
        order = list(tasks)
        random.Random(seed + round_index).shuffle(order)
        run_order.append(order)
        for position, task_name in enumerate(order):
            print(
                f"{name} round {round_index + 1}/{samples}: {position + 1}/{len(order)} {task_name}",
                file=sys.stderr,
                flush=True,
            )
            additions = tasks[task_name](round_index)
            for row_name, modes in additions.items():
                for mode, record_list in modes.items():
                    for record in record_list:
                        record["round"] = round_index + 1
                        record["schedule_position"] = position + 1
                    rows.setdefault(row_name, {}).setdefault(mode, []).extend(record_list)
    return {
        "random_seed": seed,
        "run_order": run_order,
        "rows": {
            row_name: {mode: summarize(records) for mode, records in modes.items()}
            for row_name, modes in rows.items()
        },
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", type=int, default=10)
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--seed", type=int, default=20260924)
    parser.add_argument("--phase", choices=("cpu", "metal", "all"), default="all")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--mech-cpu", type=Path, required=True)
    parser.add_argument("--mech-metal", type=Path, required=True)
    parser.add_argument("--rust-cpu", type=Path, required=True)
    parser.add_argument("--rust-metal", type=Path, required=True)
    parser.add_argument("--mojo-cpu", type=Path, required=True)
    parser.add_argument("--mojo-metal", type=Path, required=True)
    parser.add_argument("--futhark-cpu", type=Path, required=True)
    parser.add_argument("--halide", type=Path, required=True)
    parser.add_argument("--python", required=True)
    parser.add_argument("--julia", default="julia")
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("--samples must be positive")

    binaries = {
        name: path.resolve()
        for name, path in {
            "mech_cpu": args.mech_cpu,
            "mech_metal": args.mech_metal,
            "rust_cpu": args.rust_cpu,
            "rust_metal": args.rust_metal,
            "mojo_cpu": args.mojo_cpu,
            "mojo_metal": args.mojo_metal,
            "futhark_cpu": args.futhark_cpu,
            "halide": args.halide,
        }.items()
    }
    for name, path in binaries.items():
        if not path.is_file():
            parser.error(f"{name} binary not found: {path}")

    base_environment = os.environ.copy()
    cpu_thread_environment = base_environment | {
        "OMP_NUM_THREADS": "1",
        "OPENBLAS_NUM_THREADS": "1",
        "MKL_NUM_THREADS": "1",
        "VECLIB_MAXIMUM_THREADS": "1",
    }
    campaigns: dict[str, object] = {}

    if args.phase in ("cpu", "all"):
        futhark_stdin = futhark_input(args.instances, args.turns)
        futhark_timing = Path("/private/tmp/iros-n10-futhark-timing")

        def mech_cpu(_: int) -> dict[str, list[dict[str, object]]]:
            arguments = [str(binaries["mech_cpu"]), str(args.instances), str(args.turns), "1", "1"]
            environment = cpu_thread_environment | {"MECH_PARALLEL_WORKERS": "8"}
            result = command(arguments, environment=environment)

            def record(pattern: str) -> dict[str, object]:
                return {
                    "throughput_million_ekf_turns_per_second": value(result.stdout, pattern),
                    "checksum": None,
                    "faults": 0,
                    "command": arguments,
                    "stdout": result.stdout,
                }

            return {
                "Mech fused SIMD/JIT": {
                    "checked": [record(r"^Mech Cranelift SIMD-JIT parallel checked fused block throughput: ([0-9.eE+-]+) million")],
                    "unchecked": [record(r"^Mech Cranelift SIMD-JIT parallel unchecked fast block throughput: ([0-9.eE+-]+) million")],
                },
                "Mech per-turn SIMD/JIT": {
                    "checked": [record(r"^Mech Cranelift SIMD-JIT parallel throughput: ([0-9.eE+-]+) million")],
                    "unchecked": [record(r"^Mech Cranelift SIMD-JIT parallel unchecked throughput: ([0-9.eE+-]+) million")],
                },
            }

        def ordinary_cpu(
            row: str,
            mode: str,
            arguments: list[str],
            environment: dict[str, str] | None = None,
            *,
            millions: bool = False,
        ) -> dict[str, list[dict[str, object]]]:
            return {
                row: {
                    mode: [
                        parsed_process(
                            arguments,
                            environment=environment or cpu_thread_environment,
                            throughput_pattern=(
                                r"^throughput_million_ekf_turns_per_second: ([0-9.eE+-]+)$"
                                if millions
                                else r"^throughput: ([0-9.eE+-]+)$"
                            ),
                            throughput_is_millions=millions,
                        )
                    ]
                }
            }

        def futhark_cpu(mode: str, round_index: int) -> dict[str, list[dict[str, object]]]:
            entry = f"main_{mode}"
            timing = futhark_timing.with_name(f"{futhark_timing.name}-{mode}-{round_index}")
            arguments = [
                str(binaries["futhark_cpu"]),
                "--num-threads",
                "8",
                "--entry-point",
                entry,
                "-r",
                "1",
                "-t",
                str(timing),
            ]
            result = command(arguments, environment=cpu_thread_environment, stdin=futhark_stdin)
            microseconds = float(timing.read_text(encoding="utf-8").strip())
            checksum_text = result.stdout.strip().removesuffix("f32").removesuffix("f64")
            return {
                "Futhark ISPC AOT": {
                    mode: [{
                        "throughput_million_ekf_turns_per_second": args.instances * args.turns / microseconds,
                        "checksum": float(checksum_text),
                        "faults": 0,
                        "command": arguments,
                        "timing_microseconds": microseconds,
                        "stdout": result.stdout,
                    }]
                }
            }

        cpu_tasks: dict[str, Callable[[int], dict[str, list[dict[str, object]]]]] = {
            "Mech paired modes": mech_cpu,
        }
        for mode in ("checked", "unchecked"):
            cpu_tasks[f"Rust {mode}"] = lambda _, mode=mode: ordinary_cpu(
                "Rust packed SIMD",
                mode,
                [str(binaries["rust_cpu"]), str(args.instances), str(args.turns), mode, "fused", "8"],
            )
            cpu_tasks[f"Mojo {mode}"] = lambda _, mode=mode: ordinary_cpu(
                "Mojo SIMD-4",
                mode,
                [str(binaries["mojo_cpu"]), str(args.instances), str(args.turns), mode],
            )
            cpu_tasks[f"Julia {mode}"] = lambda _, mode=mode: ordinary_cpu(
                "Julia SIMD.jl",
                mode,
                [args.julia, "--startup-file=no", str(ARCHIVE / "minimal/julia_simd_threads.jl"), str(args.instances), str(args.turns), mode, "fused"],
                cpu_thread_environment | {"JULIA_NUM_THREADS": "8"},
            )
            cpu_tasks[f"Numba {mode}"] = lambda _, mode=mode: ordinary_cpu(
                "NumPy/Numba",
                mode,
                [args.python, str(ARCHIVE / "minimal/numpy_numba.py"), str(args.instances), str(args.turns), mode, "fused"],
                cpu_thread_environment | {"NUMBA_NUM_THREADS": "8"},
            )
            cpu_tasks[f"Futhark {mode}"] = lambda round_index, mode=mode: futhark_cpu(mode, round_index)
            cpu_tasks[f"Taichi {mode}"] = lambda _, mode=mode: ordinary_cpu(
                "Taichi LLVM CPU",
                mode,
                [args.python, str(HERE / "taichi-metal-matched.py"), str(args.instances), str(args.turns), mode, "--arch", "cpu", "--block-dim", "64", "--cpu-threads", "8"],
                cpu_thread_environment,
                millions=True,
            )
            cpu_tasks[f"Halide {mode}"] = lambda _, mode=mode: ordinary_cpu(
                "Halide native CPU",
                mode,
                [str(binaries["halide"]), str(args.instances), str(args.turns), mode],
                cpu_thread_environment | {
                    "DYLD_LIBRARY_PATH": "/opt/homebrew/opt/halide/lib",
                    "HALIDE_BACKEND": "cpu",
                    "HL_NUM_THREADS": "8",
                },
            )
        campaigns["cpu"] = run_campaign(
            "CPU", cpu_tasks, args.samples, args.seed, {}
        )

    if args.phase in ("metal", "all"):
        def mech_metal(_: int) -> dict[str, list[dict[str, object]]]:
            arguments = [str(binaries["mech_metal"]), str(args.instances), str(args.turns), "1", str(args.turns)]
            result = command(arguments, environment=base_environment | {"MECH_METAL_ONLY": "1"})

            def record(mode: str) -> dict[str, object]:
                return {
                    "throughput_million_ekf_turns_per_second": value(
                        result.stdout,
                        rf"^Mech direct Metal {mode} throughput: ([0-9.eE+-]+) million",
                    ),
                    "checksum": None,
                    "faults": 0,
                    "command": arguments,
                    "stdout": result.stdout,
                }

            return {"Mech generated MSL": {mode: [record(mode)] for mode in ("checked", "unchecked")}}

        def ordinary_metal(
            row: str,
            mode: str,
            arguments: list[str],
            environment: dict[str, str] | None = None,
            *,
            millions: bool = False,
        ) -> dict[str, list[dict[str, object]]]:
            return {
                row: {
                    mode: [
                        parsed_process(
                            arguments,
                            environment=environment or base_environment,
                            throughput_pattern=(
                                r"^throughput_million_ekf_turns_per_second: ([0-9.eE+-]+)$"
                                if millions
                                else r"^throughput: ([0-9.eE+-]+)$"
                            ),
                            throughput_is_millions=millions,
                        )
                    ]
                }
            }

        metal_tasks: dict[str, Callable[[int], dict[str, list[dict[str, object]]]]] = {
            "Mech paired modes": mech_metal,
        }
        for mode in ("checked", "unchecked"):
            metal_tasks[f"Rust + MSL {mode}"] = lambda _, mode=mode: ordinary_metal(
                "Rust + hand-written MSL",
                mode,
                [str(binaries["rust_metal"]), str(args.instances), str(args.turns), mode],
                millions=True,
            )
            metal_tasks[f"Mojo {mode}"] = lambda _, mode=mode: ordinary_metal(
                "Mojo native Metal",
                mode,
                [str(binaries["mojo_metal"]), str(args.instances), str(args.turns), mode, "64"],
                millions=True,
            )
            metal_tasks[f"Julia {mode}"] = lambda _, mode=mode: ordinary_metal(
                "Julia Metal.jl",
                mode,
                [args.julia, "--startup-file=no", str(HERE / "julia-metal-matched.jl"), str(args.instances), str(args.turns), mode],
            )
            metal_tasks[f"Taichi {mode}"] = lambda _, mode=mode: ordinary_metal(
                "Taichi native Metal",
                mode,
                [args.python, str(HERE / "taichi-metal-matched.py"), str(args.instances), str(args.turns), mode, "--arch", "metal", "--block-dim", "64", "--cpu-threads", "8"],
                millions=True,
            )
            metal_tasks[f"Halide {mode}"] = lambda _, mode=mode: ordinary_metal(
                "Halide Metal schedule",
                mode,
                [str(binaries["halide"]), str(args.instances), str(args.turns), mode],
                base_environment | {
                    "DYLD_LIBRARY_PATH": "/opt/homebrew/opt/halide/lib",
                    "HALIDE_BACKEND": "metal",
                },
            )
        campaigns["metal"] = run_campaign(
            "Metal", metal_tasks, args.samples, args.seed + 1000, {}
        )

    sources = [
        ARCHIVE / "minimal/rust_simd.rs",
        ARCHIVE / "minimal/julia_simd_threads.jl",
        ARCHIVE / "minimal/numpy_numba.py",
        ARCHIVE / "minimal/futhark_ekf.fut",
        ARCHIVE / "mojo_simd.mojo",
        HERE / "taichi-metal-matched.py",
        HERE / "halide-metal-matched.cpp",
        HERE / "mojo-metal-matched.mojo",
        HERE / "julia-metal-matched.jl",
        HERE / "rust-metal/src/main.rs",
        HERE / "rust-metal/src/ekf.metal",
    ]
    evidence = {
        "schema_version": 1,
        "generated_at": dt.datetime.now().astimezone().isoformat(),
        "platform": {
            "description": platform.platform(),
            "machine": platform.machine(),
            "device": "Apple M1 CPU and integrated GPU",
        },
        "configuration": {
            "instances": args.instances,
            "turns": args.turns,
            "workers": 8,
            "samples_per_implementation_mode": args.samples,
            "summary_statistic": "median",
            "uncertainty_statistic": "median absolute deviation",
            "measurement_order": "Each round contains every implementation/mode once in deterministic shuffled order; CPU and Metal are separate campaigns.",
            "warmup_turns": 5,
            "outlier_policy": "No samples removed or replaced.",
        },
        "mech_evidence_revisions": {
            "cpu_fused_and_per_turn": "eedc1c75b5a780a92d3f50f094be873f93bca6b9",
            "direct_metal": "45a21a62d4f0e68adc40bb537a8f26e1d11df7d5",
            "cpu_harness_patch": "benchmarks/iros-2026/mech-cpu-n10-validation-tolerance.patch",
            "cpu_harness_patch_scope": "Raises only the untimed 40-turn scalar-vs-SIMD validation tolerance from 1e-4 to 2e-4; measured code and timed regions are unchanged.",
        },
        "binary_sha256": {name: digest(path) for name, path in binaries.items()},
        "toolchains": {
            "rustc": command(["rustc", "--version"]).stdout.strip(),
            "julia": command([args.julia, "--version"]).stdout.strip(),
            "futhark": command(["futhark", "--version"]).stdout.strip(),
            "ispc": command(["ispc", "--version"]).stdout.splitlines()[0],
            "python": command([args.python, "--version"]).stdout.strip(),
            "python_packages": command([
                args.python,
                "-c",
                "import numba,numpy,taichi; print(f'numpy {numpy.__version__}; numba {numba.__version__}; taichi {\".\".join(map(str,taichi.__version__))}')",
            ]).stdout.splitlines()[-1],
            "halide": command(["brew", "list", "--versions", "halide"]).stdout.strip(),
            "mojo": "Mojo 1.1.0 (8189361e); MAX 26.6.0",
        },
        "sources": {
            str(source.relative_to(ROOT)): digest(source)
            for source in sources
        },
        "campaigns": campaigns,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        campaign: {
            row: {
                mode: summary["median_million_ekf_turns_per_second"]
                for mode, summary in modes.items()
            }
            for row, modes in data["rows"].items()
        }
        for campaign, data in campaigns.items()
    }, indent=2))


if __name__ == "__main__":
    main()
