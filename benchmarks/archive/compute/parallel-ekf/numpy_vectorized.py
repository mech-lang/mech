#!/usr/bin/env python3
"""Batched NumPy EKF control with fixed 3x3 products and optional checks."""

import math
import sys
import time

import numpy as np


DT = np.float32(0.1)
R = np.float32(0.25)
SYMMETRY_TOLERANCE = np.float32(0.0001)
Q0 = np.float32(0.01)
Q1 = np.float32(0.0025)


def mm33(a, b):
    a00, a01, a02, a10, a11, a12, a20, a21, a22 = a
    b00, b01, b02, b10, b11, b12, b20, b21, b22 = b
    return (
        a00 * b00 + a01 * b10 + a02 * b20,
        a00 * b01 + a01 * b11 + a02 * b21,
        a00 * b02 + a01 * b12 + a02 * b22,
        a10 * b00 + a11 * b10 + a12 * b20,
        a10 * b01 + a11 * b11 + a12 * b21,
        a10 * b02 + a11 * b12 + a12 * b22,
        a20 * b00 + a21 * b10 + a22 * b20,
        a20 * b01 + a21 * b11 + a22 * b21,
        a20 * b02 + a21 * b12 + a22 * b22,
    )


def transpose33(a):
    return (a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8])


def step(x0, x1, x2, covariance, velocity, angular_velocity, bearing, checked):
    sin_theta = np.sin(x2)
    cos_theta = np.cos(x2)
    distance = velocity * DT
    predicted_x0 = x0 + distance * cos_theta
    predicted_x1 = x1 + distance * sin_theta
    predicted_x2 = x2 + angular_velocity * DT

    f = (
        np.ones_like(x0), np.zeros_like(x0), -distance * sin_theta,
        np.zeros_like(x0), np.ones_like(x0), distance * cos_theta,
        np.zeros_like(x0), np.zeros_like(x0), np.ones_like(x0),
    )
    p = covariance
    predicted_p0 = mm33(mm33(f, p), transpose33(f))
    process00 = cos_theta * cos_theta * (DT * DT) * Q0
    process01 = cos_theta * sin_theta * (DT * DT) * Q0
    process11 = sin_theta * sin_theta * (DT * DT) * Q0
    process22 = (DT * DT) * Q1
    predicted_p = (
        predicted_p0[0] + process00,
        predicted_p0[1] + process01,
        predicted_p0[2],
        predicted_p0[3] + process01,
        predicted_p0[4] + process11,
        predicted_p0[5],
        predicted_p0[6],
        predicted_p0[7],
        predicted_p0[8] + process22,
    )

    dx = np.float32(140.0) - predicted_x0
    dy = np.float32(12.0) - predicted_x1
    squared_range = dx * dx + dy * dy
    predicted_bearing = np.arctan2(dy, dx) - predicted_x2
    raw_innovation = bearing - predicted_bearing
    innovation = np.arctan2(np.sin(raw_innovation), np.cos(raw_innovation))
    h0 = dy / squared_range
    h1 = -dx / squared_range
    h2 = np.full_like(x0, -1.0)

    pht0 = predicted_p[0] * h0 + predicted_p[1] * h1 + predicted_p[2] * h2
    pht1 = predicted_p[3] * h0 + predicted_p[4] * h1 + predicted_p[5] * h2
    pht2 = predicted_p[6] * h0 + predicted_p[7] * h1 + predicted_p[8] * h2
    variance = h0 * pht0 + h1 * pht1 + h2 * pht2 + R
    k0 = pht0 / variance
    k1 = pht1 / variance
    k2 = pht2 / variance
    a = (
        np.float32(1.0) - k0 * h0, -k0 * h1, -k0 * h2,
        -k1 * h0, np.float32(1.0) - k1 * h1, -k1 * h2,
        -k2 * h0, -k2 * h1, np.float32(1.0) - k2 * h2,
    )
    corrected_p = mm33(mm33(a, predicted_p), transpose33(a))
    candidate_p = (
        corrected_p[0] + k0 * k0 * R,
        corrected_p[1] + k0 * k1 * R,
        corrected_p[2] + k0 * k2 * R,
        corrected_p[3] + k1 * k0 * R,
        corrected_p[4] + k1 * k1 * R,
        corrected_p[5] + k1 * k2 * R,
        corrected_p[6] + k2 * k0 * R,
        corrected_p[7] + k2 * k1 * R,
        corrected_p[8] + k2 * k2 * R,
    )
    candidate_x0 = predicted_x0 + k0 * innovation
    candidate_x1 = predicted_x1 + k1 * innovation
    candidate_x2 = predicted_x2 + k2 * innovation

    if checked:
        valid = (
            np.isfinite(candidate_x0) & np.isfinite(candidate_x1) & np.isfinite(candidate_x2)
        )
        valid &= np.logical_and.reduce([np.isfinite(value) for value in candidate_p])
        valid &= (candidate_p[0] > 0) & (candidate_p[4] > 0) & (candidate_p[8] > 0)
        valid &= np.abs(candidate_p[1] - candidate_p[3]) <= SYMMETRY_TOLERANCE
        valid &= np.abs(candidate_p[2] - candidate_p[6]) <= SYMMETRY_TOLERANCE
        valid &= np.abs(candidate_p[5] - candidate_p[7]) <= SYMMETRY_TOLERANCE
        faults = int(np.count_nonzero(~valid))
        np.copyto(x0, candidate_x0, where=valid)
        np.copyto(x1, candidate_x1, where=valid)
        np.copyto(x2, candidate_x2, where=valid)
        for target, candidate in zip(covariance, candidate_p):
            np.copyto(target, candidate, where=valid)
        return faults

    x0[...] = candidate_x0
    x1[...] = candidate_x1
    x2[...] = candidate_x2
    for target, candidate in zip(covariance, candidate_p):
        target[...] = candidate
    return 0


def reset(x0, x1, x2, covariance):
    x0.fill(np.float32(55.0))
    x1.fill(np.float32(25.0))
    x2.fill(np.float32(0.4))
    for index, target in enumerate(covariance):
        target.fill(np.float32(100.0) if index in (0, 4) else np.float32(0.15) if index == 8 else np.float32(0.0))


def main():
    instances = max(1, int(sys.argv[1]) if len(sys.argv) > 1 else 10_000)
    turns = max(1, int(sys.argv[2]) if len(sys.argv) > 2 else 5)
    checked = len(sys.argv) > 3 and sys.argv[3].lower() == "checked"
    phase = np.float32(2.0 * math.pi) * np.arange(instances, dtype=np.float32) / np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(phase * np.float32(3.0))
    angular_velocity = np.float32(0.015) * (np.float32(1.0) + np.float32(0.1) * np.sin(phase * np.float32(2.0)))
    bearing = np.float32(-0.55) + np.float32(0.01) * np.sin(phase * np.float32(7.0)) + np.float32(0.005) * np.sin(phase * np.float32(11.0))
    x0 = np.full(instances, np.float32(55.0))
    x1 = np.full(instances, np.float32(25.0))
    x2 = np.full(instances, np.float32(0.4))
    covariance = [np.full(instances, np.float32(100.0) if i in (0, 4) else np.float32(0.15) if i == 8 else np.float32(0.0)) for i in range(9)]

    for _ in range(5):
        for _ in range(turns):
            step(x0, x1, x2, covariance, velocity, angular_velocity, bearing, checked)
    reset(x0, x1, x2, covariance)
    started = time.perf_counter()
    faults = 0
    for _ in range(turns):
        faults += step(x0, x1, x2, covariance, velocity, angular_velocity, bearing, checked)
    elapsed = time.perf_counter() - started
    print("lane: NumPy vectorized fixed-shape")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {float(np.sum(x0, dtype=np.float64) + np.sum(x1, dtype=np.float64) + np.sum(x2, dtype=np.float64) + sum(np.sum(value, dtype=np.float64) for value in covariance)):.9f}")
    print(f"validation: {'checked' if checked else 'unchecked'}")
    print(f"faults: {faults}")


if __name__ == "__main__":
    main()
