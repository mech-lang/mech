#!/usr/bin/env python3
"""Pure NumPy stacked-matrix control for the archived, unchecked EKF.

All filters are independent. The only Python loop during execution is over
turns; the filter dimension is handled by NumPy broadcasting and stacked
matrix products. Scratch arrays and the identity matrix are allocated once.
The equations, float32 state, inputs, and Joseph covariance update match
../archive/compute/parallel-ekf/numpy_scalar.py.
"""

import argparse
import json
import math
import time
from pathlib import Path

import numpy as np


DT = np.float32(0.1)
R = np.float32(0.25)
Q = np.array([[0.01, 0.0], [0.0, 0.0025]], dtype=np.float32)
INITIAL_STATE = np.array([55.0, 25.0, 0.4], dtype=np.float32)
INITIAL_COVARIANCE = np.diag(np.array([100.0, 100.0, 0.15], dtype=np.float32))
INPUT_NAMES = ("velocity", "angular_velocity", "bearing")


def init_inputs(instances):
    """Use the original control's float32 input construction, element for element."""
    if instances < 1:
        raise ValueError("instances must be positive")
    phase = np.float32(2.0 * math.pi) * np.arange(instances, dtype=np.float32) / np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(phase * np.float32(3.0))
    angular_velocity = np.float32(0.015) * (np.float32(1.0) + np.float32(0.1) * np.sin(phase * np.float32(2.0)))
    bearing = np.float32(-0.55) + np.float32(0.01) * np.sin(phase * np.float32(7.0)) + np.float32(0.005) * np.sin(phase * np.float32(11.0))
    return dict(zip(INPUT_NAMES, (velocity, angular_velocity, bearing)))


def load_inputs(path, instances):
    """Read little-endian f32 velocity[N], angular_velocity[N], bearing[N]."""
    data = np.fromfile(path, dtype="<f4")
    if data.size != 3 * instances:
        raise ValueError(f"input contains {data.size} floats; expected {3 * instances}")
    return {name: data[index * instances:(index + 1) * instances]
            for index, name in enumerate(INPUT_NAMES)}


class BatchedEKF:
    def __init__(self, instances, inputs=None):
        if instances < 1:
            raise ValueError("instances must be positive")
        self.instances = instances
        supplied = init_inputs(instances) if inputs is None else inputs
        self.inputs = {}
        for name in INPUT_NAMES:
            value = np.asarray(supplied[name], dtype=np.float32)
            if value.shape != (instances,):
                raise ValueError(f"{name} must have shape ({instances},)")
            self.inputs[name] = np.ascontiguousarray(value)
        self.state = np.empty((instances, 3), dtype=np.float32)
        self.covariance = np.empty((instances, 3, 3), dtype=np.float32)
        self.identity = np.eye(3, dtype=np.float32)
        self.f = np.broadcast_to(self.identity, self.covariance.shape).copy()
        self.g = np.zeros((instances, 3, 2), dtype=np.float32)
        self.g[:, 2, 1] = DT
        for name in ("left", "predicted_p", "process_p", "a", "ap", "corrected_p", "measurement_p"):
            setattr(self, name, np.empty_like(self.covariance))
        self.process_left = np.empty_like(self.g)
        for name in ("predicted_state", "h", "pht", "gain", "state_correction"):
            setattr(self, name, np.empty_like(self.state))
        for name in ("sin_theta", "cos_theta", "distance", "control", "delta_x", "delta_y",
                     "squared_range", "squared_y", "predicted_bearing", "innovation",
                     "sin_innovation", "cos_innovation"):
            setattr(self, name, np.empty(instances, dtype=np.float32))
        self.innovation_variance_matrix = np.empty((instances, 1, 1), dtype=np.float32)
        self.innovation_variance = self.innovation_variance_matrix[:, 0, 0]
        # These are views, not transposed copies or per-turn allocations.
        self.ft = self.f.swapaxes(1, 2)
        self.gt = self.g.swapaxes(1, 2)
        self.at = self.a.swapaxes(1, 2)
        self.h_row = self.h[:, None, :]
        self.h_column = self.h[:, :, None]
        self.pht_column = self.pht[:, :, None]
        self.gain_row = self.gain[:, None, :]
        self.gain_column = self.gain[:, :, None]
        self.reset()

    def reset(self):
        self.state[:] = INITIAL_STATE
        self.covariance[:] = INITIAL_COVARIANCE

    def step(self):
        """Compute and publish one unchecked turn for every filter."""
        np.sin(self.state[:, 2], out=self.sin_theta)
        np.cos(self.state[:, 2], out=self.cos_theta)
        np.multiply(self.inputs["velocity"], DT, out=self.distance)
        np.multiply(self.distance, self.cos_theta, out=self.control)
        np.add(self.state[:, 0], self.control, out=self.predicted_state[:, 0])
        np.multiply(self.distance, self.sin_theta, out=self.control)
        np.add(self.state[:, 1], self.control, out=self.predicted_state[:, 1])
        np.multiply(self.inputs["angular_velocity"], DT, out=self.control)
        np.add(self.state[:, 2], self.control, out=self.predicted_state[:, 2])

        np.multiply(self.distance, self.sin_theta, out=self.f[:, 0, 2])
        np.negative(self.f[:, 0, 2], out=self.f[:, 0, 2])
        np.multiply(self.distance, self.cos_theta, out=self.f[:, 1, 2])
        np.multiply(self.cos_theta, DT, out=self.g[:, 0, 0])
        np.multiply(self.sin_theta, DT, out=self.g[:, 1, 0])
        np.matmul(self.f, self.covariance, out=self.left)
        np.matmul(self.left, self.ft, out=self.predicted_p)
        np.matmul(self.g, Q, out=self.process_left)
        np.matmul(self.process_left, self.gt, out=self.process_p)
        np.add(self.predicted_p, self.process_p, out=self.predicted_p)

        np.subtract(np.float32(140.0), self.predicted_state[:, 0], out=self.delta_x)
        np.subtract(np.float32(12.0), self.predicted_state[:, 1], out=self.delta_y)
        np.multiply(self.delta_x, self.delta_x, out=self.squared_range)
        np.multiply(self.delta_y, self.delta_y, out=self.squared_y)
        np.add(self.squared_range, self.squared_y, out=self.squared_range)
        np.arctan2(self.delta_y, self.delta_x, out=self.predicted_bearing)
        np.subtract(self.predicted_bearing, self.predicted_state[:, 2], out=self.predicted_bearing)
        np.subtract(self.inputs["bearing"], self.predicted_bearing, out=self.innovation)
        np.sin(self.innovation, out=self.sin_innovation)
        np.cos(self.innovation, out=self.cos_innovation)
        np.arctan2(self.sin_innovation, self.cos_innovation, out=self.innovation)
        np.divide(self.delta_y, self.squared_range, out=self.h[:, 0])
        np.negative(self.delta_x, out=self.h[:, 1])
        np.divide(self.h[:, 1], self.squared_range, out=self.h[:, 1])
        self.h[:, 2] = np.float32(-1.0)
        np.matmul(self.predicted_p, self.h_column, out=self.pht_column)
        np.matmul(self.h_row, self.pht_column, out=self.innovation_variance_matrix)
        np.add(self.innovation_variance, R, out=self.innovation_variance)
        np.divide(self.pht, self.innovation_variance[:, None], out=self.gain)
        np.multiply(self.gain, self.innovation[:, None], out=self.state_correction)
        np.add(self.predicted_state, self.state_correction, out=self.state)

        # Joseph form, exactly as in the sequential small-matrix control.
        np.multiply(self.gain_column, self.h_row, out=self.a)
        np.subtract(self.identity, self.a, out=self.a)
        np.matmul(self.a, self.predicted_p, out=self.ap)
        np.matmul(self.ap, self.at, out=self.corrected_p)
        np.multiply(self.gain_column, self.gain_row, out=self.measurement_p)
        np.multiply(self.measurement_p, R, out=self.measurement_p)
        np.add(self.corrected_p, self.measurement_p, out=self.covariance)

    def checksum(self):
        return float(np.sum(self.state, dtype=np.float64) + np.sum(self.covariance, dtype=np.float64))

    def dump(self, path):
        """Write little-endian f32 state[N,3], then covariance[N,3,3], C order."""
        with Path(path).open("wb") as output:
            self.state.astype("<f4", copy=False).tofile(output)
            self.covariance.astype("<f4", copy=False).tofile(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instances", type=int, default=10_000)
    parser.add_argument("--turns", type=int, default=20)
    parser.add_argument("--warmup", type=int, default=5)
    parser.add_argument("--input", type=Path, help="shared raw little-endian f32 inputs")
    parser.add_argument("--dump", type=Path, help="write full final state and covariance outside timing")
    args = parser.parse_args()
    if args.instances < 1 or args.turns < 1 or args.warmup < 0:
        parser.error("instances and turns must be positive; warmup must be nonnegative")
    supplied = load_inputs(args.input, args.instances) if args.input else None
    ekf = BatchedEKF(args.instances, supplied)
    for _ in range(args.warmup):
        ekf.step()
    ekf.reset()
    started = time.perf_counter()
    for _ in range(args.turns):
        ekf.step()
    elapsed = time.perf_counter() - started
    if args.dump:
        ekf.dump(args.dump)
    print(json.dumps({
        "implementation": "NumPy stacked matrices",
        "numpy_version": np.__version__,
        "dtype": "float32",
        "checked": False,
        "instances": args.instances,
        "turns": args.turns,
        "warmup": args.warmup,
        "elapsed_s": elapsed,
        "filter_turns_per_second": args.instances * args.turns / elapsed,
        "checksum": ekf.checksum(),
    }, sort_keys=True))


if __name__ == "__main__":
    main()
