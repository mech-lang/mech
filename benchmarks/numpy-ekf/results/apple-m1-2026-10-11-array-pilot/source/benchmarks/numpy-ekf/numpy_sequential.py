#!/usr/bin/env python3
"""Matched harness around the unmodified archived small-matrix NumPy step."""
import argparse
import importlib.util
import json
from pathlib import Path
import time

import numpy as np

SOURCE = Path(__file__).resolve().parents[1] / "archive/compute/parallel-ekf/numpy_scalar.py"
spec = importlib.util.spec_from_file_location("archived_numpy_scalar", SOURCE)
scalar = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scalar)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instances", type=int, default=10_000)
    parser.add_argument("--turns", type=int, default=20)
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--dump", type=Path)
    args = parser.parse_args()
    if args.instances < 1 or args.turns < 1 or args.warmup < 0:
        parser.error("instances/turns must be positive and warmup nonnegative")
    inputs = np.fromfile(args.input, dtype="<f4")
    if inputs.size != args.instances * 3:
        parser.error("input must contain three float32 arrays of length instances")
    velocity, angular_velocity, bearing = inputs.reshape(3, args.instances)
    state = np.empty((args.instances, 3), dtype=np.float32)
    covariance = np.empty((args.instances, 3, 3), dtype=np.float32)
    scratch = scalar.Scratch()
    initial_state = np.array([55, 25, 0.4], dtype=np.float32)
    initial_covariance = np.diag(np.array([100, 100, 0.15], dtype=np.float32))

    def reset():
        state[:] = initial_state
        covariance[:] = initial_covariance

    def run(turns):
        for _ in range(turns):
            for lane in range(args.instances):
                scalar.step(state[lane], covariance[lane], velocity[lane],
                            angular_velocity[lane], bearing[lane], scratch)

    reset()
    run(args.warmup)
    reset()
    started = time.perf_counter()
    run(args.turns)
    elapsed = time.perf_counter() - started
    if args.dump:
        with args.dump.open("wb") as stream:
            state.astype("<f4", copy=False).tofile(stream)
            covariance.astype("<f4", copy=False).tofile(stream)
    print(json.dumps({
        "backend": "numpy-sequential", "instances": args.instances,
        "turns": args.turns, "warmup": args.warmup, "checked": False,
        "elapsed_s": elapsed,
        "filter_turns_per_second": args.instances * args.turns / elapsed,
        "checksum": float(state.sum(dtype=np.float64) + covariance.sum(dtype=np.float64)),
    }))


if __name__ == "__main__":
    main()
