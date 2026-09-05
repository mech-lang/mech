#!/usr/bin/env pypy3
"""Textbook-fidelity pure-Python EKF control for PyPy.

This intentionally favors correspondence with the mathematical algorithm over
throughput: vectors and matrices are ordinary Python lists, and each EKF step
uses the obvious generic matrix operations.  It is the PyPy baseline, not the
optimized flat/SoA control.
"""

from __future__ import annotations

import math
import sys
import time


DT = 0.1
Q = [[0.01, 0.0], [0.0, 0.0025]]
R = 0.25
I = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
LANDMARK = [140.0, 12.0]
TOLERANCE = 1.0e-4


def transpose(matrix):
    return [list(column) for column in zip(*matrix)]


def matmul(left, right):
    right_t = transpose(right)
    return [
        [sum(a * b for a, b in zip(row, column)) for column in right_t]
        for row in left
    ]


def matvec(matrix, vector):
    return [sum(value * component for value, component in zip(row, vector)) for row in matrix]


def add(left, right):
    return [[a + b for a, b in zip(row_a, row_b)] for row_a, row_b in zip(left, right)]


def subtract(left, right):
    return [[a - b for a, b in zip(row_a, row_b)] for row_a, row_b in zip(left, right)]


def outer(left, right):
    return [[a * b for b in right] for a in left]


def vector_add(left, right):
    return [a + b for a, b in zip(left, right)]


def vector_subtract(left, right):
    return [a - b for a, b in zip(left, right)]


def vector_scale(vector, scalar):
    return [value * scalar for value in vector]


def valid(state, covariance):
    if not all(math.isfinite(value) for value in state):
        return False
    if not all(math.isfinite(value) for row in covariance for value in row):
        return False
    if covariance[0][0] <= 0.0 or covariance[1][1] <= 0.0 or covariance[2][2] <= 0.0:
        return False
    return all(
        abs(covariance[row][column] - covariance[column][row]) <= TOLERANCE
        for row, column in ((0, 1), (0, 2), (1, 2))
    )


def step(state, covariance, velocity, angular_velocity, bearing, checked):
    theta = state[2]
    sine = math.sin(theta)
    cosine = math.cos(theta)
    distance = velocity * DT
    predicted_state = vector_add(
        state,
        [distance * cosine, distance * sine, angular_velocity * DT],
    )
    f = [[1.0, 0.0, -distance * sine], [0.0, 1.0, distance * cosine], [0.0, 0.0, 1.0]]
    g = [[cosine * DT, 0.0], [sine * DT, 0.0], [0.0, DT]]
    predicted_covariance = add(
        matmul(matmul(f, covariance), transpose(f)),
        matmul(matmul(g, Q), transpose(g)),
    )

    delta = vector_subtract(LANDMARK, predicted_state[:2])
    squared_range = sum(value * value for value in delta)
    predicted_bearing = math.atan2(delta[1], delta[0]) - predicted_state[2]
    raw_innovation = bearing - predicted_bearing
    innovation = math.atan2(math.sin(raw_innovation), math.cos(raw_innovation))
    h = [delta[1] / squared_range, -delta[0] / squared_range, -1.0]
    pht = matvec(predicted_covariance, h)
    variance = sum(a * b for a, b in zip(h, pht)) + R
    gain = vector_scale(pht, 1.0 / variance)
    candidate_state = vector_add(predicted_state, vector_scale(gain, innovation))
    a = subtract(I, outer(gain, h))
    candidate_covariance = add(
        matmul(matmul(a, predicted_covariance), transpose(a)),
        [[value * R for value in row] for row in outer(gain, gain)],
    )
    if checked and not valid(candidate_state, candidate_covariance):
        return True
    state[:] = candidate_state
    for row in range(3):
        covariance[row][:] = candidate_covariance[row]
    return False


def reset(states, covariances):
    for lane in range(len(states)):
        states[lane][:] = [55.0, 25.0, 0.4]
        covariances[lane][:] = [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 0.15]]


def dispatch(turns, states, covariances, velocity, angular_velocity, bearing, checked):
    faults = 0
    for _ in range(turns):
        for lane in range(len(states)):
            faults += step(
                states[lane],
                covariances[lane],
                velocity[lane],
                angular_velocity[lane],
                bearing[lane],
                checked,
            )
    return faults


def main():
    instances = max(1, int(sys.argv[1]) if len(sys.argv) > 1 else 10000)
    turns = max(1, int(sys.argv[2]) if len(sys.argv) > 2 else 5)
    mode = sys.argv[3].lower() if len(sys.argv) > 3 else "unchecked"
    if mode not in {"checked", "unchecked"}:
        raise SystemExit("mode must be checked or unchecked")
    checked = mode == "checked"
    phase_step = 2.0 * math.pi / instances
    velocity = []
    angular_velocity = []
    bearing = []
    for lane in range(instances):
        phase = phase_step * lane
        velocity.append(1.0 + 0.05 * math.sin(phase * 3.0))
        angular_velocity.append(0.015 * (1.0 + 0.1 * math.sin(phase * 2.0)))
        bearing.append(-0.55 + 0.01 * math.sin(phase * 7.0) + 0.005 * math.sin(phase * 11.0))
    states = [[55.0, 25.0, 0.4] for _ in range(instances)]
    covariances = [
        [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 0.15]]
        for _ in range(instances)
    ]
    dispatch(5, states, covariances, velocity, angular_velocity, bearing, checked)
    reset(states, covariances)
    started = time.perf_counter()
    faults = dispatch(turns, states, covariances, velocity, angular_velocity, bearing, checked)
    elapsed = time.perf_counter() - started
    checksum = sum(sum(state) for state in states)
    checksum += sum(sum(value for row in covariance for value in row) for covariance in covariances)
    print("lane: PyPy textbook-fidelity scalar outer loop")
    print(f"mode: {mode}")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {checksum:.9f}")
    print(f"faults: {faults}")


if __name__ == "__main__":
    main()
