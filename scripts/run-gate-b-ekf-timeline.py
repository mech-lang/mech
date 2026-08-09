#!/usr/bin/env python3
"""Collect ordered, steady-state Gate B EKF timing samples across runtimes."""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EPISODE_LENGTH = 4_096


def run_lines(command: list[str]) -> list[dict[str, Any]]:
    result = subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
    )
    rows = []
    for line in result.stdout.splitlines():
        line = line.strip()
        if line.startswith("{"):
            rows.append(json.loads(line))
    return rows


def collect_rust(samples: int) -> list[dict[str, Any]]:
    return run_lines(
        [
            "cargo",
            "bench",
            "-p",
            "mech-runtime",
            "--bench",
            "ekf_timeline",
            "--features",
            "source_default,runtime_bench_gate_b",
            "--",
            "--samples",
            str(samples),
        ]
    )


def collect_numpy(python: str, samples: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    script = ROOT / "benchmarks/runtime/gate-b/numpy/ekf_v1.py"
    requests = "\n".join(
        (
            json.dumps({"command": "benchmark", "instances": 1, "samples": samples}),
            json.dumps({"command": "quit"}),
            "",
        )
    )
    result = subprocess.run(
        [python, str(script)],
        cwd=ROOT,
        input=requests,
        check=True,
        text=True,
        capture_output=True,
    )
    responses = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    description = responses[0]
    benchmark = responses[1]
    rows = []
    for sample, elapsed_ns in enumerate(benchmark["samples_ns"]):
        rows.append(
            {
                "lane": "numpy-persistent",
                "sample": sample,
                "turns": EPISODE_LENGTH,
                "elapsed_ns": elapsed_ns,
                "gc_ns": benchmark["gc_samples_ns"][sample],
            }
        )
    return rows, description


def command_version(command: list[str]) -> str:
    result = subprocess.run(command, check=True, text=True, capture_output=True)
    return (result.stdout or result.stderr).splitlines()[0].strip()


def find_julia() -> str | None:
    candidates = (
        os.environ.get("JULIA"),
        shutil.which("julia"),
        "/private/tmp/julia-1.12.6/bin/julia",
    )
    return next(
        (
            candidate
            for candidate in candidates
            if candidate and Path(candidate).is_file()
        ),
        None,
    )


def find_numpy_python() -> str | None:
    candidates = (
        os.environ.get("NUMPY_PYTHON"),
        shutil.which("python3"),
        "/private/tmp/mech-benchmark-venv/bin/python",
        "/private/tmp/mech-matrix-bench-py314/bin/python",
    )
    for candidate in candidates:
        if not candidate or not Path(candidate).is_file():
            continue
        result = subprocess.run(
            [candidate, "-c", "import numpy"],
            text=True,
            capture_output=True,
        )
        if result.returncode == 0:
            return candidate
    return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples", type=int, default=60)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.samples < 1:
        raise SystemExit("--samples must be positive")
    python = find_numpy_python()
    lua = shutil.which("lua")
    luajit = shutil.which("luajit")
    julia = find_julia()
    missing = [
        name
        for name, path in (
            ("python with NumPy", python),
            ("lua", lua),
            ("luajit", luajit),
            ("julia", julia),
        )
        if not path
    ]
    if missing:
        raise SystemExit(f"missing benchmark runtimes: {', '.join(missing)}")

    rows = collect_rust(args.samples)
    numpy_rows, numpy_description = collect_numpy(python, args.samples)
    rows.extend(numpy_rows)
    rows.extend(
        run_lines(
            [
                python,
                "benchmarks/runtime/gate-b/python/ekf_timeline.py",
                "--samples",
                str(args.samples),
            ]
        )
    )
    rows.extend(
        run_lines(
            [
                python,
                "benchmarks/runtime/gate-b/python/ekf_fixed.py",
                "--samples",
                str(args.samples),
            ]
        )
    )
    rows.extend(
        run_lines(
            [
                lua,
                "benchmarks/runtime/gate-b/lua/ekf_timeline.lua",
                "--samples",
                str(args.samples),
            ]
        )
    )
    rows.extend(
        run_lines(
            [
                luajit,
                "benchmarks/runtime/gate-b/lua/ekf_timeline.lua",
                "--samples",
                str(args.samples),
            ]
        )
    )
    for runtime in (lua, luajit):
        rows.extend(
            run_lines(
                [
                    runtime,
                    "benchmarks/runtime/gate-b/lua/ekf_fixed.lua",
                    "--samples",
                    str(args.samples),
                ]
            )
        )
    rows.extend(
        run_lines(
            [
                julia,
                "benchmarks/runtime/gate-b/julia/ekf_v1.jl",
                "--timeline",
                "--instances",
                "1",
                "--samples",
                str(args.samples),
            ]
        )
    )
    rows.extend(
        run_lines(
            [
                julia,
                "--startup-file=no",
                f"--project={ROOT / 'benchmarks/runtime/gate-b/julia'}",
                "benchmarks/runtime/gate-b/julia/ekf_fixed.jl",
                "--samples",
                str(args.samples),
            ]
        )
    )

    expected = {
        "rust-raw",
        "rust-fixed-fused",
        "mech-resident-fused",
        "mech-resident-complete",
        "mech-current-atomic",
        "numpy-persistent",
        "python-scalar",
        "python-fixed-preallocated",
        "lua-scalar",
        "lua-fixed-preallocated",
        "luajit-scalar",
        "luajit-fixed-preallocated",
        "julia-persistent",
        "julia-staticarrays",
    }
    counts = {lane: sum(row["lane"] == lane for row in rows) for lane in expected}
    if any(count != args.samples for count in counts.values()):
        raise RuntimeError(f"incomplete timeline samples: {counts}")

    optimization_classes = {
        "rust-raw": "portable-generic-control",
        "rust-fixed-fused": "fixed-shape-specialized-control",
        "mech-resident-fused": "transactional-fused-prototype",
        "mech-resident-complete": "transactional-resident-prototype",
        "mech-current-atomic": "transactional-current-runtime",
        "numpy-persistent": "preallocated-library-dispatch",
        "python-scalar": "allocating-generic-scalar",
        "python-fixed-preallocated": "preallocated-fixed-shape-scalar",
        "lua-scalar": "allocating-generic-scalar",
        "lua-fixed-preallocated": "preallocated-fixed-shape-scalar",
        "luajit-scalar": "allocating-generic-jit",
        "luajit-fixed-preallocated": "preallocated-fixed-shape-jit",
        "julia-persistent": "preallocated-dynamic-array",
        "julia-staticarrays": "preallocated-fixed-shape-specialized",
    }
    for row in rows:
        row["optimization_class"] = optimization_classes[row["lane"]]

    payload = {
        "schema_version": 1,
        "workload": "resident-ekf-v1",
        "collected_at": datetime.now(timezone.utc).isoformat(),
        "machine": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "python": command_version([python, "--version"]),
            "numpy": numpy_description["numpy"],
            "lua": command_version([lua, "-v"]),
            "luajit": command_version([luajit, "-v"]),
            "julia": command_version([julia, "--version"]),
            "rust": command_version(["rustc", "--version"]),
        },
        "protocol": {
            "samples": args.samples,
            "turns_per_sample": EPISODE_LENGTH,
            "state_reset_outside_timing": True,
            "setup_outside_timing": True,
            "gc_enabled": True,
            "x_axis": "cumulative EKF turns",
            "lua_clock": "process CPU time; all other lanes use monotonic wall time",
            "missing_lanes": {
                "matlab": "not installed",
                "mech-bytecode": "no current bytecode EKF execution path",
            },
            "optimization_classes": optimization_classes,
        },
        "samples": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
