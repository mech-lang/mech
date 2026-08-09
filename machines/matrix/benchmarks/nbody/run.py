#!/usr/bin/env python3

import argparse
import csv
import os
import pathlib
import statistics
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
EXACT = HERE / "benchmarksgame"
BUILD = HERE / "build"


def run_once(command, environment):
    started = time.perf_counter()
    completed = subprocess.run(command, check=True, capture_output=True, text=True, env=environment)
    elapsed = time.perf_counter() - started
    values = [float(line) for line in completed.stdout.splitlines() if line.strip()]
    if len(values) != 2:
        raise RuntimeError(f"expected two energy lines from {command}, got {completed.stdout!r}")
    return elapsed, values


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--steps", type=int, default=500_000)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument("--lua", default="lua")
    parser.add_argument("--luajit", default="luajit")
    parser.add_argument("--julia", default="julia")
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--output")
    args = parser.parse_args()
    if args.steps <= 0 or args.repetitions <= 0:
        parser.error("steps and repetitions must be positive")

    BUILD.mkdir(exist_ok=True)
    rust_binary = BUILD / "nbody-rust"
    subprocess.run(
        [args.rustc, "-O", "-C", "target-cpu=native", str(EXACT / "rust.rs"), "-o", str(rust_binary)],
        check=True,
    )
    environment = os.environ.copy()
    environment.update(
        {
            "VECLIB_MAXIMUM_THREADS": "1",
            "OPENBLAS_NUM_THREADS": "1",
            "OMP_NUM_THREADS": "1",
            "JULIA_NUM_THREADS": "1",
            "JULIA_LLVM_ARGS": "-unroll-threshold=500",
        }
    )
    steps = str(args.steps)
    commands = [
        ("rust-benchmarksgame-2", [str(rust_binary), steps]),
        ("python-benchmarksgame-1", [args.python, "-OO", str(EXACT / "python.py"), steps]),
        ("numpy-vectorized", [args.python, str(HERE / "numpy_benchmark.py"), steps]),
        ("lua-benchmarksgame-2", [args.lua, str(EXACT / "lua.lua"), steps]),
        ("luajit-benchmarksgame-2", [args.luajit, str(EXACT / "lua.lua"), steps]),
        ("julia-benchmarksgame-5", [args.julia, "--startup-file=no", str(EXACT / "julia.jl"), steps]),
    ]

    rows = []
    reference = None
    for runtime, command in commands:
        samples = []
        energies = None
        for _ in range(args.repetitions):
            elapsed, current_energies = run_once(command, environment)
            samples.append(elapsed)
            if energies is None:
                energies = current_energies
            elif max(abs(a - b) for a, b in zip(energies, current_energies)) > 1.0e-12:
                raise RuntimeError(f"{runtime} produced nondeterministic energies")
        if reference is None:
            reference = energies
        elif max(abs(a - b) for a, b in zip(reference, energies)) > 1.0e-8:
            raise RuntimeError(f"{runtime} energy mismatch: expected {reference}, got {energies}")
        rows.append(
            {
                "runtime": runtime,
                "steps": args.steps,
                "median_seconds": f"{statistics.median(samples):.9f}",
                "min_seconds": f"{min(samples):.9f}",
                "max_seconds": f"{max(samples):.9f}",
                "repetitions": args.repetitions,
                "initial_energy": f"{energies[0]:.9f}",
                "final_energy": f"{energies[1]:.9f}",
            }
        )

    fields = list(rows[0])
    destination = open(args.output, "w", newline="") if args.output else sys.stdout
    try:
        writer = csv.DictWriter(destination, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)
    finally:
        if args.output:
            destination.close()


if __name__ == "__main__":
    main()
