#!/usr/bin/env python3
"""Reproduce the poster's illustrative EKF output with a trusted Rust control library."""

import argparse
import ctypes as c
import hashlib
import json
import math
from pathlib import Path
import platform

import numpy as np


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--library", type=Path, required=True,
                        help="Trusted library exporting the checked Rust mech_fixed_numeric_turn ABI")
    parser.add_argument("--turns", type=int, default=800)
    args = parser.parse_args()
    if args.turns < 1:
        parser.error("--turns must be positive")
    library = args.library.expanduser().resolve(strict=True)
    fixture = {
        "input_order": ["dt", "linear_velocity", "angular_velocity", "bearing", "measurement_noise"],
        "inputs_decimal": [.1, 1., .015, -.54, .25],
        "initial_mean": [55, 25, .4],
        "initial_covariance_rows": [[100, 0, 0], [0, 100, 0], [0, 0, .15]],
        "landmark": [140, 12],
        "process_covariance_rows": [[.01, 0], [0, .0025]],
    }
    lib = c.CDLL(str(library))
    turn = lib.mech_fixed_numeric_turn
    pointer = c.POINTER(c.c_float)
    turn.argtypes = [c.POINTER(pointer), c.POINTER(pointer), c.POINTER(pointer), c.c_size_t]
    turn.restype = c.c_uint64
    inputs = [(c.c_float * 1)(value) for value in fixture["inputs_decimal"]]
    input_pointers = (pointer * 5)(*[c.cast(value, pointer) for value in inputs])
    mean = (c.c_float * 3)(*fixture["initial_mean"])
    covariance = (c.c_float * 9)(100, 0, 0, 0, 100, 0, 0, 0, .15)
    for index in range(args.turns):
        next_mean = (c.c_float * 3)()
        next_covariance = (c.c_float * 9)()
        status = turn(input_pointers, (pointer * 2)(mean, covariance),
                      (pointer * 2)(next_mean, next_covariance), 1)
        if status:
            raise RuntimeError(f"Turn {index + 1} failed with ABI status {status}")
        mean, covariance = next_mean, next_covariance
    rows = [[covariance[column * 3 + row] for column in range(3)] for row in range(3)]

    # Independent matrix evaluation of the Mech equations, retaining f32 arithmetic.
    x = np.array(fixture["initial_mean"], dtype=np.float32)
    cov = np.array(fixture["initial_covariance_rows"], dtype=np.float32)
    dt, velocity, omega, bearing, noise = [np.float32(v) for v in fixture["inputs_decimal"]]
    process = np.array(fixture["process_covariance_rows"], dtype=np.float32)
    landmark = np.array(fixture["landmark"], dtype=np.float32)
    for _ in range(args.turns):
        sine, cosine = np.sin(x[2]), np.cos(x[2])
        distance = velocity * dt
        motion = np.array([[1, 0, -distance * sine], [0, 1, distance * cosine],
                           [0, 0, 1]], dtype=np.float32)
        control = np.array([[cosine * dt, 0], [sine * dt, 0], [0, dt]], dtype=np.float32)
        prior_x = x + np.array([distance * cosine, distance * sine, omega * dt], dtype=np.float32)
        prior_p = motion @ cov @ motion.T + control @ process @ control.T
        delta = landmark - prior_x[:2]
        squared_range = delta @ delta
        predicted_bearing = np.arctan2(delta[1], delta[0]) - prior_x[2]
        raw_innovation = bearing - predicted_bearing
        innovation = np.arctan2(np.sin(raw_innovation), np.cos(raw_innovation))
        observation = np.array([delta[1] / squared_range, -delta[0] / squared_range, -1], dtype=np.float32)
        gain = (prior_p @ observation) / (observation @ prior_p @ observation + noise)
        x = prior_x + gain * innovation
        correction = np.eye(3, dtype=np.float32) - np.outer(gain, observation)
        cov = correction @ prior_p @ correction.T + np.outer(gain, gain) * noise

    a, b, d = rows[0][0], (rows[0][1] + rows[1][0]) / 2, rows[1][1]
    root = math.hypot((a - d) / 2, b)
    eigenvalues = [(a + d) / 2 + root, (a + d) / 2 - root]
    if min(eigenvalues) <= 0:
        raise ArithmeticError("Position covariance is not positive definite")
    angle = math.degrees(.5 * math.atan2(2 * b, a - d))
    result = {
        "schema_version": 1,
        "purpose": "Illustrative poster output from repeated inputs; not a benchmark sample, recorded robot trajectory, or new Mech execution",
        "backend": "Rust checked f32 ABI control",
        "turn": args.turns,
        "instances": 1,
        "fixture": fixture,
        "inputs_f32": [value[0] for value in inputs],
        "library_sha256": hashlib.sha256(library.read_bytes()).hexdigest(),
        "environment": {"python": platform.python_version(), "numpy": np.__version__},
        "result": {"all_status_codes_zero": True, "mean": list(mean), "covariance_rows": rows},
        "independent_cross_check": {
            "method": "NumPy f32 matrix equations",
            "mean": x.tolist(),
            "covariance_rows": cov.tolist(),
            "maximum_mean_absolute_difference": float(np.max(np.abs(x - np.array(mean)))),
            "maximum_covariance_absolute_difference": float(np.max(np.abs(cov - np.array(rows)))),
        },
        "scene": {
            "covariance_block": "Symmetric part of the 2x2 position block",
            "position_eigenvalues": eigenvalues,
            "two_sigma_semiaxes": [2 * math.sqrt(value) for value in eigenvalues],
            "world_major_axis_angle_degrees": angle,
            "svg_major_axis_angle_degrees": -angle,
            "robot_heading_radians": mean[2],
            "svg_robot_heading_degrees": -math.degrees(mean[2]),
        },
    }
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
