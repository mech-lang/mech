#!/usr/bin/env pypy3
"""Optimized pure-Python/PyPy scalar EKF control.

This is still the same scalar recurrence, not a native extension or NumPy
wrapper.  The persistent state uses structure-of-arrays storage so the PyPy
JIT sees monomorphic scalar loads in the lane loop.  Matrix products are
expanded once, and checked publication does not allocate a candidate tuple.
"""

from __future__ import annotations

from array import array
import math
import sys
import time


DT = 0.1
Q0 = 0.01
Q1 = 0.0025
R = 0.25
TOLERANCE = 1.0e-4


def reset(state, covariance):
    for lane in range(len(state[0])):
        state[0][lane] = 55.0
        state[1][lane] = 25.0
        state[2][lane] = 0.4
        for component in covariance:
            component[lane] = 0.0
        covariance[0][lane] = 100.0
        covariance[4][lane] = 100.0
        covariance[8][lane] = 0.15


def step(lane, state, covariance, velocity, angular_velocity, bearing, checked):
    x0s, x1s, x2s = state
    p00s, p01s, p02s, p10s, p11s, p12s, p20s, p21s, p22s = covariance
    theta = x2s[lane]
    sine = math.sin(theta)
    cosine = math.cos(theta)
    distance = velocity[lane] * DT
    x0 = x0s[lane] + distance * cosine
    x1 = x1s[lane] + distance * sine
    x2 = theta + angular_velocity[lane] * DT
    fs = -distance * sine
    fc = distance * cosine

    p00 = p00s[lane]
    p01 = p01s[lane]
    p02 = p02s[lane]
    p10 = p10s[lane]
    p11 = p11s[lane]
    p12 = p12s[lane]
    p20 = p20s[lane]
    p21 = p21s[lane]
    p22 = p22s[lane]
    l00 = p00 + fs * p20
    l01 = p01 + fs * p21
    l02 = p02 + fs * p22
    l10 = p10 + fc * p20
    l11 = p11 + fc * p21
    l12 = p12 + fc * p22
    dt2 = DT * DT
    pp00 = l00 + l02 * fs + cosine * cosine * dt2 * Q0
    pp01 = l01 + l02 * fc + cosine * sine * dt2 * Q0
    pp02 = l02
    pp10 = l10 + l12 * fs + sine * cosine * dt2 * Q0
    pp11 = l11 + l12 * fc + sine * sine * dt2 * Q0
    pp12 = l12
    pp20 = p20 + p22 * fs
    pp21 = p21 + p22 * fc
    pp22 = p22 + dt2 * Q1

    dx = 140.0 - x0
    dy = 12.0 - x1
    squared_range = dx * dx + dy * dy
    predicted_bearing = math.atan2(dy, dx) - x2
    raw_innovation = bearing[lane] - predicted_bearing
    innovation = math.atan2(math.sin(raw_innovation), math.cos(raw_innovation))
    h0 = dy / squared_range
    h1 = -dx / squared_range
    h2 = -1.0
    pht0 = pp00 * h0 + pp01 * h1 - pp02
    pht1 = pp10 * h0 + pp11 * h1 - pp12
    pht2 = pp20 * h0 + pp21 * h1 - pp22
    variance = h0 * pht0 + h1 * pht1 - pht2 + R
    k0 = pht0 / variance
    k1 = pht1 / variance
    k2 = pht2 / variance
    next_x0 = x0 + k0 * innovation
    next_x1 = x1 + k1 * innovation
    next_x2 = x2 + k2 * innovation

    a00 = 1.0 - k0 * h0
    a01 = -k0 * h1
    a02 = k0
    a10 = -k1 * h0
    a11 = 1.0 - k1 * h1
    a12 = k1
    a20 = -k2 * h0
    a21 = -k2 * h1
    a22 = 1.0 + k2
    ap00 = a00 * pp00 + a01 * pp10 + a02 * pp20
    ap01 = a00 * pp01 + a01 * pp11 + a02 * pp21
    ap02 = a00 * pp02 + a01 * pp12 + a02 * pp22
    ap10 = a10 * pp00 + a11 * pp10 + a12 * pp20
    ap11 = a10 * pp01 + a11 * pp11 + a12 * pp21
    ap12 = a10 * pp02 + a11 * pp12 + a12 * pp22
    ap20 = a20 * pp00 + a21 * pp10 + a22 * pp20
    ap21 = a20 * pp01 + a21 * pp11 + a22 * pp21
    ap22 = a20 * pp02 + a21 * pp12 + a22 * pp22
    next_p00 = ap00 * a00 + ap01 * a01 + ap02 * a02 + k0 * k0 * R
    next_p01 = ap00 * a10 + ap01 * a11 + ap02 * a12 + k0 * k1 * R
    next_p02 = ap00 * a20 + ap01 * a21 + ap02 * a22 + k0 * k2 * R
    next_p10 = ap10 * a00 + ap11 * a01 + ap12 * a02 + k1 * k0 * R
    next_p11 = ap10 * a10 + ap11 * a11 + ap12 * a12 + k1 * k1 * R
    next_p12 = ap10 * a20 + ap11 * a21 + ap12 * a22 + k1 * k2 * R
    next_p20 = ap20 * a00 + ap21 * a01 + ap22 * a02 + k2 * k0 * R
    next_p21 = ap20 * a10 + ap21 * a11 + ap22 * a12 + k2 * k1 * R
    next_p22 = ap20 * a20 + ap21 * a21 + ap22 * a22 + k2 * k2 * R

    if checked:
        valid = (
            math.isfinite(next_x0)
            and math.isfinite(next_x1)
            and math.isfinite(next_x2)
            and math.isfinite(next_p00)
            and math.isfinite(next_p01)
            and math.isfinite(next_p02)
            and math.isfinite(next_p10)
            and math.isfinite(next_p11)
            and math.isfinite(next_p12)
            and math.isfinite(next_p20)
            and math.isfinite(next_p21)
            and math.isfinite(next_p22)
            and next_p00 > 0.0
            and next_p11 > 0.0
            and next_p22 > 0.0
            and abs(next_p01 - next_p10) <= TOLERANCE
            and abs(next_p02 - next_p20) <= TOLERANCE
            and abs(next_p12 - next_p21) <= TOLERANCE
        )
        if not valid:
            return True

    x0s[lane] = next_x0
    x1s[lane] = next_x1
    x2s[lane] = next_x2
    p00s[lane] = next_p00
    p01s[lane] = next_p01
    p02s[lane] = next_p02
    p10s[lane] = next_p10
    p11s[lane] = next_p11
    p12s[lane] = next_p12
    p20s[lane] = next_p20
    p21s[lane] = next_p21
    p22s[lane] = next_p22
    return False


def dispatch(turns, state, covariance, velocity, angular_velocity, bearing, checked):
    faults = 0
    for _ in range(turns):
        for lane in range(len(velocity)):
            faults += step(lane, state, covariance, velocity, angular_velocity, bearing, checked)
    return faults


def main():
    instances = max(1, int(sys.argv[1]) if len(sys.argv) > 1 else 10000)
    turns = max(1, int(sys.argv[2]) if len(sys.argv) > 2 else 5)
    mode = sys.argv[3].lower() if len(sys.argv) > 3 else "unchecked"
    if mode not in {"checked", "unchecked"}:
        raise SystemExit("mode must be checked or unchecked")
    checked = mode == "checked"
    velocity = array("f", [0.0]) * instances
    angular_velocity = array("f", [0.0]) * instances
    bearing = array("f", [0.0]) * instances
    phase_step = 2.0 * math.pi / instances
    for lane in range(instances):
        phase = phase_step * lane
        velocity[lane] = 1.0 + 0.05 * math.sin(phase * 3.0)
        angular_velocity[lane] = 0.015 * (1.0 + 0.1 * math.sin(phase * 2.0))
        bearing[lane] = -0.55 + 0.01 * math.sin(phase * 7.0) + 0.005 * math.sin(phase * 11.0)
    state = tuple(array("f", [0.0]) * instances for _ in range(3))
    covariance = tuple(array("f", [0.0]) * instances for _ in range(9))
    reset(state, covariance)
    dispatch(5, state, covariance, velocity, angular_velocity, bearing, checked)
    reset(state, covariance)
    started = time.perf_counter()
    faults = dispatch(turns, state, covariance, velocity, angular_velocity, bearing, checked)
    elapsed = time.perf_counter() - started
    checksum = sum(map(float, state[0])) + sum(map(float, state[1])) + sum(map(float, state[2]))
    checksum += sum(sum(map(float, component)) for component in covariance)
    print("lane: PyPy optimized pure-Python scalar outer loop")
    print(f"mode: {mode}")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {checksum:.9f}")
    print(f"faults: {faults}")


if __name__ == "__main__":
    main()
