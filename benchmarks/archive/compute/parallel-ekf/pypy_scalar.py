#!/usr/bin/env pypy3
"""Pure-Python scalar EKF control for PyPy.

The timed region is the same persistent outer loop as the other scalar
controls: one EKF turn per lane, with no allocation or input construction in
the timed section.  State and covariance live in float32 arrays; arithmetic is
performed by the PyPy JIT as scalar operations and is stored back to float32.
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


def reset(state: array, covariance: array) -> None:
    for lane in range(len(state) // 3):
        state_index = lane * 3
        state[state_index] = 55.0
        state[state_index + 1] = 25.0
        state[state_index + 2] = 0.4
        covariance_index = lane * 9
        for offset in range(9):
            covariance[covariance_index + offset] = 0.0
        covariance[covariance_index] = 100.0
        covariance[covariance_index + 4] = 100.0
        covariance[covariance_index + 8] = 0.15


def finite_candidate(
    x0: float,
    x1: float,
    x2: float,
    covariance: tuple[float, ...],
) -> bool:
    if not (math.isfinite(x0) and math.isfinite(x1) and math.isfinite(x2)):
        return False
    for value in covariance:
        if not math.isfinite(value):
            return False
    if covariance[0] <= 0.0 or covariance[4] <= 0.0 or covariance[8] <= 0.0:
        return False
    return (
        abs(covariance[1] - covariance[3]) <= TOLERANCE
        and abs(covariance[2] - covariance[6]) <= TOLERANCE
        and abs(covariance[5] - covariance[7]) <= TOLERANCE
    )


def step(
    lane: int,
    state: array,
    covariance: array,
    velocity: array,
    angular_velocity: array,
    bearing: array,
    checked: bool,
) -> str | None:
    state_index = lane * 3
    covariance_index = lane * 9
    theta = state[state_index + 2]
    sine = math.sin(theta)
    cosine = math.cos(theta)
    distance = velocity[lane] * DT
    x0 = state[state_index] + distance * cosine
    x1 = state[state_index + 1] + distance * sine
    x2 = theta + angular_velocity[lane] * DT
    fs = -distance * sine
    fc = distance * cosine

    p00 = covariance[covariance_index]
    p01 = covariance[covariance_index + 1]
    p02 = covariance[covariance_index + 2]
    p10 = covariance[covariance_index + 3]
    p11 = covariance[covariance_index + 4]
    p12 = covariance[covariance_index + 5]
    p20 = covariance[covariance_index + 6]
    p21 = covariance[covariance_index + 7]
    p22 = covariance[covariance_index + 8]

    l00 = p00 + fs * p20
    l01 = p01 + fs * p21
    l02 = p02 + fs * p22
    l10 = p10 + fc * p20
    l11 = p11 + fc * p21
    l12 = p12 + fc * p22
    l20 = p20
    l21 = p21
    l22 = p22

    dt2 = DT * DT
    pp00 = l00 + l02 * fs + cosine * cosine * dt2 * Q0
    pp01 = l01 + l02 * fc + cosine * sine * dt2 * Q0
    pp02 = l02
    pp10 = l10 + l12 * fs + sine * cosine * dt2 * Q0
    pp11 = l11 + l12 * fc + sine * sine * dt2 * Q0
    pp12 = l12
    pp20 = l20 + l22 * fs
    pp21 = l21 + l22 * fc
    pp22 = l22 + dt2 * Q1

    dx = 140.0 - x0
    dy = 12.0 - x1
    squared_range = dx * dx + dy * dy
    predicted_bearing = math.atan2(dy, dx) - x2
    raw_innovation = bearing[lane] - predicted_bearing
    innovation = math.atan2(math.sin(raw_innovation), math.cos(raw_innovation))
    h0 = dy / squared_range
    h1 = -dx / squared_range
    h2 = -1.0
    pht0 = pp00 * h0 + pp01 * h1 + pp02 * h2
    pht1 = pp10 * h0 + pp11 * h1 + pp12 * h2
    pht2 = pp20 * h0 + pp21 * h1 + pp22 * h2
    variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + R
    k0 = pht0 / variance
    k1 = pht1 / variance
    k2 = pht2 / variance
    next_x0 = x0 + k0 * innovation
    next_x1 = x1 + k1 * innovation
    next_x2 = x2 + k2 * innovation

    a00 = 1.0 - k0 * h0
    a01 = -k0 * h1
    a02 = -k0 * h2
    a10 = -k1 * h0
    a11 = 1.0 - k1 * h1
    a12 = -k1 * h2
    a20 = -k2 * h0
    a21 = -k2 * h1
    a22 = 1.0 - k2 * h2

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
    next_covariance = (
        next_p00,
        next_p01,
        next_p02,
        next_p10,
        next_p11,
        next_p12,
        next_p20,
        next_p21,
        next_p22,
    )
    if checked and not finite_candidate(next_x0, next_x1, next_x2, next_covariance):
        return "finite-candidate!"

    state[state_index] = next_x0
    state[state_index + 1] = next_x1
    state[state_index + 2] = next_x2
    for offset, value in enumerate(next_covariance):
        covariance[covariance_index + offset] = value
    return None


def dispatch(
    turns: int,
    state: array,
    covariance: array,
    velocity: array,
    angular_velocity: array,
    bearing: array,
    checked: bool,
) -> tuple[int, str]:
    faults = 0
    latest_fault = ""
    instances = len(velocity)
    for _ in range(turns):
        for lane in range(instances):
            fault = step(lane, state, covariance, velocity, angular_velocity, bearing, checked)
            if fault is not None:
                faults += 1
                latest_fault = fault
    return faults, latest_fault


def main() -> None:
    instances = max(1, int(sys.argv[1]) if len(sys.argv) > 1 else 10_000)
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

    state = array("f", [0.0]) * (instances * 3)
    covariance = array("f", [0.0]) * (instances * 9)
    reset(state, covariance)
    dispatch(5, state, covariance, velocity, angular_velocity, bearing, checked)
    reset(state, covariance)
    dispatch(1, state, covariance, velocity, angular_velocity, bearing, checked)
    reset(state, covariance)

    started = time.perf_counter()
    faults, latest_fault = dispatch(
        turns, state, covariance, velocity, angular_velocity, bearing, checked
    )
    elapsed = time.perf_counter() - started
    checksum = sum(float(value) for value in state) + sum(float(value) for value in covariance)
    print("lane: PyPy pure-Python scalar outer loop")
    print(f"mode: {mode}")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {checksum:.9f}")
    print(f"faults: {faults}")
    print(f"latest_fault: {latest_fault}")


if __name__ == "__main__":
    main()
