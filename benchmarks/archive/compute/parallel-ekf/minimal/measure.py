#!/usr/bin/env python3
"""Measure the minimized EKF controls with the same lane count and turns."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import os
import re
import shutil
import statistics
import subprocess
import tempfile
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]


def run(command: list[str], *, stdin: str | None = None, env: dict[str, str] | None = None) -> str:
    return subprocess.run(
        command,
        cwd=ROOT,
        input=stdin,
        text=True,
        env=env,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout


def samples(command: list[str], count: int, env: dict[str, str], stdin: str | None = None) -> list[str]:
    run(command, stdin=stdin, env=env)
    return [run(command, stdin=stdin, env=env) for _ in range(count)]


def value(text: str, name: str) -> float:
    match = re.search(rf"^{name}: ([0-9.eE+-]+)$", text, re.MULTILINE)
    if match is None:
        raise RuntimeError(f"missing {name} in output:\n{text}")
    return float(match.group(1))


def futhark_input(n: int, turns: int) -> str:
    vals = []
    for i in range(n):
        phase = 2.0 * math.pi * i / n
        vals.append((1.0 + 0.05 * math.sin(3.0 * phase),
                     0.015 * (1.0 + 0.1 * math.sin(2.0 * phase)),
                     -0.55 + 0.01 * math.sin(7.0 * phase) + 0.005 * math.sin(11.0 * phase)))
    def array(column: int) -> str:
        return "[" + ",".join(f"{row[column]:.9g}f32" for row in vals) + "]"
    return f"{array(0)} {array(1)} {array(2)} {turns}i32"


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--python", default=shutil.which("python3") or "python3")
    p.add_argument("--instances", type=int, default=10_000)
    p.add_argument("--turns", type=int, default=20)
    p.add_argument("--samples", type=int, default=5)
    p.add_argument("--halide-cxx", default=shutil.which("clang++") or "clang++")
    p.add_argument("--halide-simd", action="store_true", help="also measure Halide JIT SIMD with eight workers")
    p.add_argument("--futhark-ispc", action="store_true", help="also measure Futhark ISPC SIMD with eight workers")
    p.add_argument("--output", type=Path, default=HERE.parent / "results/apple-m1-minimal-source-2026-08-31.json")
    args = p.parse_args()
    env = os.environ.copy()
    for name in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS"):
        env[name] = "1"
    result: dict[str, object] = {
        "schema_version": 1,
        "generated_at": dt.datetime.now().astimezone().isoformat(),
        "configuration": {"instances": args.instances, "turns": args.turns, "samples": args.samples},
        "rows": {},
    }
    rows: dict[str, object] = result["rows"]  # type: ignore[assignment]
    for label, script in (("NumPy scalar", HERE / "numpy_scalar.py"), ("NumPy advanced", HERE / "numpy_fast.py")):
        for mode in ("unchecked", "checked"):
            command = [args.python, str(script), str(args.instances), str(args.turns), mode]
            out = samples(command, args.samples, env)
            rows[f"{label} {mode}"] = {"command": command, "throughput": [value(x, "throughput") for x in out], "checksums": [value(x, "checksum") for x in out]}
    with tempfile.TemporaryDirectory(prefix="mech-ekf-minimal-") as temp:
        halide = Path(temp) / "halide-ekf"
        run([args.halide_cxx, "-O3", "-std=c++17", str(HERE / "halide_ekf.cpp"), "-I/opt/homebrew/opt/halide/include", "-L/opt/homebrew/opt/halide/lib", "-lHalide", "-o", str(halide)], env=env)
        halide_env = env | {"DYLD_LIBRARY_PATH": "/opt/homebrew/opt/halide/lib"}
        for mode in ("unchecked", "checked"):
            command = [str(halide), str(args.instances), str(args.turns), mode]
            out = samples(command, args.samples, halide_env)
            rows[f"Halide {mode}"] = {"command": command, "throughput": [value(x, "throughput") for x in out], "checksums": [value(x, "checksum") for x in out]}
        if args.halide_simd:
            simd_halide_env = halide_env | {"HL_NUM_THREADS": "8"}
            for mode in ("unchecked", "checked"):
                command = [str(halide), str(args.instances), str(args.turns), mode]
                out = samples(command, args.samples, simd_halide_env)
                rows[f"Halide JIT SIMD 8 workers {mode}"] = {"command": command, "throughput": [value(x, "throughput") for x in out], "checksums": [value(x, "checksum") for x in out]}
        futhark = Path(temp) / "futhark-ekf"
        run(["futhark", "multicore", str(HERE / "futhark_ekf.fut"), "-o", str(futhark)], env=env)
        inp = futhark_input(args.instances, args.turns)
        for threads in (1, 8):
            for checked, flag in ((False, "false"), (True, "true")):
                times: list[float] = []
                checksums: list[float] = []
                command = [str(futhark), "--num-threads", str(threads), "-r", "1", "-t", str(Path(temp) / "time")]
                data = inp + f" {flag}"
                run(command, stdin=data, env=env)
                # Runtime files are overwritten per invocation; repeat and read each value.
                for _ in range(args.samples):
                    output = run(command, stdin=data, env=env)
                    with open(Path(temp) / "time", encoding="utf-8") as timing:
                        micros = float(timing.read().strip())
                    times.append(args.instances * args.turns / (micros / 1e6))
                    checksums.append(float(output.strip().removesuffix("f32").removesuffix("f64")))
                rows[f"Futhark multicore {threads} threads {'checked' if checked else 'unchecked'}"] = {"command": command, "throughput": times, "checksums": checksums}
        if args.futhark_ispc:
            ispc_path = Path(temp) / "ispc-bin"
            ispc_path.mkdir()
            (ispc_path / "ispc").symlink_to(HERE / "futhark-ispc-compat.sh")
            ispc_env = env | {"PATH": f"{ispc_path}{os.pathsep}{env.get('PATH', '')}"}
            futhark_ispc = Path(temp) / "futhark-ekf-ispc"
            run(["futhark", "ispc", str(HERE / "futhark_ekf.fut"), "-o", str(futhark_ispc)], env=ispc_env)
            for checked, flag in ((False, "false"), (True, "true")):
                times: list[float] = []
                checksums: list[float] = []
                command = [str(futhark_ispc), "--num-threads", "8", "-r", "1", "-t", str(Path(temp) / "time-ispc")]
                data = inp + f" {flag}"
                run(command, stdin=data, env=ispc_env)
                for _ in range(args.samples):
                    output = run(command, stdin=data, env=ispc_env)
                    with open(Path(temp) / "time-ispc", encoding="utf-8") as timing:
                        micros = float(timing.read().strip())
                    times.append(args.instances * args.turns / (micros / 1e6))
                    checksums.append(float(output.strip().removesuffix("f32").removesuffix("f64")))
                rows[f"Futhark ISPC SIMD 8 workers {'checked' if checked else 'unchecked'}"] = {"command": command, "throughput": times, "checksums": checksums}
        opencl = Path(temp) / "futhark-opencl"
        try:
            run(["futhark", "opencl", str(HERE / "futhark_ekf.fut"), "-o", str(opencl)], env=env)
            for checked, flag in ((False, "false"), (True, "true")):
                times: list[float] = []
                checksums: list[float] = []
                command = [str(opencl), "-r", "1", "-t", str(Path(temp) / "time-opencl")]
                for _ in range(args.samples):
                    output = run(command, stdin=inp + f" {flag}", env=env)
                    with open(Path(temp) / "time-opencl", encoding="utf-8") as timing:
                        micros = float(timing.read().strip())
                    times.append(args.instances * args.turns / (micros / 1e6))
                    checksums.append(float(output.strip().removesuffix("f32").removesuffix("f64")))
                rows[f"Futhark OpenCL {'checked' if checked else 'unchecked'}"] = {"command": command, "throughput": times, "checksums": checksums}
        except subprocess.CalledProcessError as error:
            # Futhark can compile OpenCL on hosts whose installed driver cannot execute it.
            # Keep that capability result explicit instead of discarding the rest of the run.
            detail = (error.stdout or str(error)).strip()
            for checked in (False, True):
                rows[f"Futhark OpenCL {'checked' if checked else 'unchecked'}"] = {
                    "available": False,
                    "error": detail,
                }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for label, row in rows.items():
        if "throughput" in row:
            print(f"{label}: {statistics.median(row['throughput']):.3f} turns/s")  # type: ignore[index]
        else:
            print(f"{label}: unavailable ({row['error']})")  # type: ignore[index]


if __name__ == "__main__":
    main()
