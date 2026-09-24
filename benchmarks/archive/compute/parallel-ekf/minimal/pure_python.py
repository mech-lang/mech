#!/usr/bin/env python3
"""Pure-Python scalar EKF control.

This intentionally uses only the Python standard library. It is a lower-bound
control for the runtime comparison, not a claim about optimized Python
implementations that delegate arithmetic to NumPy, Numba, or native kernels.
"""

import math
import sys
import time


DT = 0.1
R = 0.25
SYMMETRY_TOLERANCE = 0.0001
FINITE_LIMIT = 3.402823466e38


def valid_candidate(x0, x1, x2, covariance):
    values = (x0, x1, x2, *covariance)
    finite = all(math.isfinite(value) and abs(value) <= FINITE_LIMIT for value in values)
    positive = covariance[0] > 0.0 and covariance[4] > 0.0 and covariance[8] > 0.0
    symmetric = (
        abs(covariance[1] - covariance[3]) <= SYMMETRY_TOLERANCE
        and abs(covariance[2] - covariance[6]) <= SYMMETRY_TOLERANCE
        and abs(covariance[5] - covariance[7]) <= SYMMETRY_TOLERANCE
    )
    return finite and positive and symmetric


def step(state, covariance, velocity, angular_velocity, bearing, checked):
    sx0 = state[0]
    sx1 = state[1]
    sx2 = state[2]
    sp00 = covariance[0]
    sp01 = covariance[1]
    sp02 = covariance[2]
    sp10 = covariance[3]
    sp11 = covariance[4]
    sp12 = covariance[5]
    sp20 = covariance[6]
    sp21 = covariance[7]
    sp22 = covariance[8]
    st = math.sin(sx2)
    ct = math.cos(sx2)
    distance = velocity * DT
    nx0 = sx0 + distance * ct
    nx1 = sx1 + distance * st
    nx2 = sx2 + angular_velocity * DT
    f02 = -distance * st
    f12 = distance * ct
    ap0 = sp00 + f02 * sp20
    ap1 = sp01 + f02 * sp21
    ap2 = sp02 + f02 * sp22
    aq0 = sp10 + f12 * sp20
    aq1 = sp11 + f12 * sp21
    aq2 = sp12 + f12 * sp22
    q00 = ct * ct * 0.0001
    q01 = ct * st * 0.0001
    q11 = st * st * 0.0001
    q22 = 0.000025
    np00 = ap0 + ap2 * f02 + q00
    np01 = ap1 + ap2 * f12 + q01
    np02 = ap2
    np10 = aq0 + aq2 * f02 + q01
    np11 = aq1 + aq2 * f12 + q11
    np12 = aq2
    np20 = sp20 + sp22 * f02
    np21 = sp21 + sp22 * f12
    np22 = sp22 + q22
    dx = 140.0 - nx0
    dy = 12.0 - nx1
    squared_range = dx * dx + dy * dy
    raw_innovation = bearing - (math.atan2(dy, dx) - nx2)
    innovation = math.atan2(math.sin(raw_innovation), math.cos(raw_innovation))
    h0 = dy / squared_range
    h1 = -dx / squared_range
    h2 = -1.0
    ph0 = np00 * h0 + np01 * h1 + np02 * h2
    ph1 = np10 * h0 + np11 * h1 + np12 * h2
    ph2 = np20 * h0 + np21 * h1 + np22 * h2
    innovation_variance = h0 * ph0 + h1 * ph1 + h2 * ph2 + R
    k0 = ph0 / innovation_variance
    k1 = ph1 / innovation_variance
    k2 = ph2 / innovation_variance
    b00 = 1.0 - k0 * h0
    b01 = -k0 * h1
    b02 = -k0 * h2
    b10 = -k1 * h0
    b11 = 1.0 - k1 * h1
    b12 = -k1 * h2
    b20 = -k2 * h0
    b21 = -k2 * h1
    b22 = 1.0 - k2 * h2
    c00 = b00 * np00 + b01 * np10 + b02 * np20
    c01 = b00 * np01 + b01 * np11 + b02 * np21
    c02 = b00 * np02 + b01 * np12 + b02 * np22
    c10 = b10 * np00 + b11 * np10 + b12 * np20
    c11 = b10 * np01 + b11 * np11 + b12 * np21
    c12 = b10 * np02 + b11 * np12 + b12 * np22
    c20 = b20 * np00 + b21 * np10 + b22 * np20
    c21 = b20 * np01 + b21 * np11 + b22 * np21
    c22 = b20 * np02 + b21 * np12 + b22 * np22
    cp00 = c00 * b00 + c01 * b01 + c02 * b02 + k0 * k0 * R
    cp01 = c00 * b10 + c01 * b11 + c02 * b12 + k0 * k1 * R
    cp02 = c00 * b20 + c01 * b21 + c02 * b22 + k0 * k2 * R
    cp10 = c10 * b00 + c11 * b01 + c12 * b02 + k1 * k0 * R
    cp11 = c10 * b10 + c11 * b11 + c12 * b12 + k1 * k1 * R
    cp12 = c10 * b20 + c11 * b21 + c12 * b22 + k1 * k2 * R
    cp20 = c20 * b00 + c21 * b01 + c22 * b02 + k2 * k0 * R
    cp21 = c20 * b10 + c21 * b11 + c22 * b12 + k2 * k1 * R
    cp22 = c20 * b20 + c21 * b21 + c22 * b22 + k2 * k2 * R
    cx0 = nx0 + k0 * innovation
    cx1 = nx1 + k1 * innovation
    cx2 = nx2 + k2 * innovation
    candidate = (cp00, cp01, cp02, cp10, cp11, cp12, cp20, cp21, cp22)
    if checked and not valid_candidate(cx0, cx1, cx2, candidate):
        return 1
    state[0] = cx0
    state[1] = cx1
    state[2] = cx2
    covariance[:] = candidate
    return 0


def make_inputs(instances):
    velocity = []
    angular_velocity = []
    bearing = []
    for index in range(instances):
        phase = 2.0 * math.pi * index / instances
        velocity.append(1.0 + 0.05 * math.sin(phase * 3.0))
        angular_velocity.append(0.015 * (1.0 + 0.1 * math.sin(phase * 2.0)))
        bearing.append(-0.55 + 0.01 * math.sin(phase * 7.0) + 0.005 * math.sin(phase * 11.0))
    return velocity, angular_velocity, bearing


def make_state(instances):
    return [[55.0, 25.0, 0.4] for _ in range(instances)], [[100.0, 0.0, 0.0,
        0.0, 100.0, 0.0, 0.0, 0.0, 0.15] for _ in range(instances)]


def run_turns(states, covariances, velocity, angular_velocity, bearing, turns, checked):
    faults = 0
    for _ in range(turns):
        for index in range(len(states)):
            faults += step(
                states[index],
                covariances[index],
                velocity[index],
                angular_velocity[index],
                bearing[index],
                checked,
            )
    return faults


def main():
    instances = max(1, int(sys.argv[1])) if len(sys.argv) > 1 else 10000
    turns = max(1, int(sys.argv[2])) if len(sys.argv) > 2 else 20
    checked = len(sys.argv) > 3 and sys.argv[3].lower() == "checked"
    velocity, angular_velocity, bearing = make_inputs(instances)
    states, covariances = make_state(instances)
    run_turns(states, covariances, velocity, angular_velocity, bearing, 5, checked)
    states, covariances = make_state(instances)
    started = time.perf_counter()
    faults = run_turns(states, covariances, velocity, angular_velocity, bearing, turns, checked)
    elapsed = time.perf_counter() - started
    checksum = sum(sum(state) for state in states) + sum(sum(covariance) for covariance in covariances)
    print("lane: pure Python scalar")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {checksum:.9f}")
    print(f"validation: {'checked' if checked else 'unchecked'}")
    print(f"faults: {faults}")


if __name__ == "__main__":
    main()
