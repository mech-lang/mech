#!/usr/bin/env python3
"""Preallocated fixed-shape pure-Python EKF timeline control."""

from __future__ import annotations

import argparse
import gc
import json
import math
import struct
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
TRACE_PATH = ROOT / "benchmarks/runtime/gate-b/ekf-input-v1.bin"
EPISODE_LENGTH = 4_096
DT = 0.05
LANDMARK = (25.0, -10.0)
PROCESS_COVARIANCE = (0.04, 0.0, 0.0, 0.0025)
MEASUREMENT_COVARIANCE = (0.25, 0.0, 0.0, 0.0009)
INITIAL_STATE = (2.0, 1.0, 0.15)
INITIAL_COVARIANCE = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05)
EXPECTED_STATE = (18.169827258925427, 4.339708695271022, 0.2557219366745068)
EXPECTED_COVARIANCE = (
    0.3270953723043491,
    0.1509754472729972,
    -0.022618166436367253,
    0.1509754472729972,
    0.07105284175378412,
    -0.010486015657880304,
    -0.022618166436367253,
    -0.010486015657880304,
    0.0016395600302299483,
)
TRACE = tuple(struct.iter_unpack("<4d", TRACE_PATH.read_bytes()))


def matmul(out, left, left_rows, left_columns, right, right_columns):
    for column in range(right_columns):
        for row in range(left_rows):
            total = 0.0
            for inner in range(left_columns):
                total += left[inner * left_rows + row] * right[column * left_columns + inner]
            out[column * left_rows + row] = total


def matmul_right_transpose(out, left, left_rows, inner_size, right, right_rows):
    for column in range(right_rows):
        for row in range(left_rows):
            total = 0.0
            for inner in range(inner_size):
                total += left[inner * left_rows + row] * right[inner * right_rows + column]
            out[column * left_rows + row] = total


class Workspace:
    def __init__(self):
        self.state = list(INITIAL_STATE)
        self.covariance = list(INITIAL_COVARIANCE)
        self.motion_jacobian = [0.0] * 9
        self.control_jacobian = [0.0] * 6
        self.predicted_state = [0.0] * 3
        self.gp = [0.0] * 9
        self.predicted_covariance = [0.0] * 9
        self.vq = [0.0] * 6
        self.process_covariance = [0.0] * 9
        self.measurement_jacobian = [0.0] * 6
        self.hp = [0.0] * 6
        self.innovation_covariance = [0.0] * 4
        self.inverse_innovation = [0.0] * 4
        self.pht = [0.0] * 6
        self.gain = [0.0] * 6
        self.innovation = [0.0] * 2
        self.correction = [0.0] * 3
        self.kh = [0.0] * 9
        self.joseph_a = [0.0] * 9
        self.ap = [0.0] * 9
        self.corrected_covariance = [0.0] * 9
        self.kr = [0.0] * 6
        self.measurement_covariance = [0.0] * 9

    def reset(self):
        self.state[:] = INITIAL_STATE
        self.covariance[:] = INITIAL_COVARIANCE

    def step(self, inputs):
        velocity, angular_velocity, measured_range, measured_bearing = inputs
        state = self.state
        covariance = self.covariance
        cosine = math.cos(state[2])
        sine = math.sin(state[2])
        g = self.motion_jacobian
        g[0], g[1], g[2] = 1.0, 0.0, 0.0
        g[3], g[4], g[5] = 0.0, 1.0, 0.0
        g[6] = -velocity * sine * DT
        g[7] = velocity * cosine * DT
        g[8] = 1.0
        v = self.control_jacobian
        v[0], v[1], v[2] = cosine * DT, sine * DT, 0.0
        v[3], v[4], v[5] = 0.0, 0.0, DT
        predicted_state = self.predicted_state
        predicted_state[0] = state[0] + velocity * cosine * DT
        predicted_state[1] = state[1] + velocity * sine * DT
        predicted_state[2] = state[2] + angular_velocity * DT

        matmul(self.gp, g, 3, 3, covariance, 3)
        matmul_right_transpose(self.predicted_covariance, self.gp, 3, 3, g, 3)
        matmul(self.vq, v, 3, 2, PROCESS_COVARIANCE, 2)
        matmul_right_transpose(self.process_covariance, self.vq, 3, 2, v, 3)
        for index in range(9):
            self.predicted_covariance[index] += self.process_covariance[index]

        delta_x = LANDMARK[0] - predicted_state[0]
        delta_y = LANDMARK[1] - predicted_state[1]
        q = delta_x * delta_x + delta_y * delta_y
        if q <= 1.0e-12:
            raise ArithmeticError("landmark distance")
        distance = math.sqrt(q)
        predicted_bearing = math.atan2(delta_y, delta_x) - predicted_state[2]
        h = self.measurement_jacobian
        h[0], h[1] = -delta_x / distance, delta_y / q
        h[2], h[3] = -delta_y / distance, -delta_x / q
        h[4], h[5] = 0.0, -1.0
        matmul(self.hp, h, 2, 3, self.predicted_covariance, 3)
        matmul_right_transpose(self.innovation_covariance, self.hp, 2, 3, h, 2)
        for index in range(4):
            self.innovation_covariance[index] += MEASUREMENT_COVARIANCE[index]
        s = self.innovation_covariance
        determinant = s[0] * s[3] - s[2] * s[1]
        if abs(determinant) <= 1.0e-12:
            raise ArithmeticError("innovation determinant")
        inverse = self.inverse_innovation
        inverse[0], inverse[1] = s[3] / determinant, -s[1] / determinant
        inverse[2], inverse[3] = -s[2] / determinant, s[0] / determinant
        matmul_right_transpose(self.pht, self.predicted_covariance, 3, 3, h, 2)
        matmul(self.gain, self.pht, 3, 2, inverse, 2)
        self.innovation[0] = measured_range - distance
        self.innovation[1] = measured_bearing - predicted_bearing
        matmul(self.correction, self.gain, 3, 2, self.innovation, 1)
        for index in range(3):
            state[index] = predicted_state[index] + self.correction[index]

        matmul(self.kh, self.gain, 3, 2, h, 3)
        for index in range(9):
            self.joseph_a[index] = (1.0 if index in (0, 4, 8) else 0.0) - self.kh[index]
        matmul(self.ap, self.joseph_a, 3, 3, self.predicted_covariance, 3)
        matmul_right_transpose(self.corrected_covariance, self.ap, 3, 3, self.joseph_a, 3)
        matmul(self.kr, self.gain, 3, 2, MEASUREMENT_COVARIANCE, 2)
        matmul_right_transpose(self.measurement_covariance, self.kr, 3, 2, self.gain, 3)
        for index in range(9):
            self.corrected_covariance[index] += self.measurement_covariance[index]
        for column in range(3):
            for row in range(column):
                left = column * 3 + row
                right = row * 3 + column
                symmetric = 0.5 * (self.corrected_covariance[left] + self.corrected_covariance[right])
                self.corrected_covariance[left] = symmetric
                self.corrected_covariance[right] = symmetric
        covariance[:] = self.corrected_covariance
        if not all(math.isfinite(value) for value in state):
            raise ArithmeticError("non-finite state")
        if not all(math.isfinite(value) for value in covariance):
            raise ArithmeticError("non-finite state")
        if not all(covariance[index] > 0.0 for index in (0, 4, 8)):
            raise ArithmeticError("covariance diagonal")

    def run_episode(self):
        for inputs in TRACE:
            self.step(inputs)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", type=int, default=60)
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("--samples must be positive")
    workspace = Workspace()
    workspace.run_episode()
    for actual, expected in zip(workspace.state, EXPECTED_STATE):
        if abs(actual - expected) > 1.0e-9:
            raise AssertionError(f"fixed Python final state mismatch: {actual} != {expected}")
    for actual, expected in zip(workspace.covariance, EXPECTED_COVARIANCE):
        if abs(actual - expected) > 1.0e-9:
            raise AssertionError(
                f"fixed Python final covariance mismatch: {actual} != {expected}"
            )
    for sample in range(args.samples):
        workspace.reset()
        gc_before = gc.get_count()
        started = time.perf_counter_ns()
        workspace.run_episode()
        elapsed_ns = time.perf_counter_ns() - started
        print(json.dumps({
            "lane": "python-fixed-preallocated",
            "sample": sample,
            "turns": EPISODE_LENGTH,
            "elapsed_ns": elapsed_ns,
            "gc_ns": None,
            "gc_count_before": gc_before,
            "gc_count_after": gc.get_count(),
        }, separators=(",", ":")))


if __name__ == "__main__":
    main()
