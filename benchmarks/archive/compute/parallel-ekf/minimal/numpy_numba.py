#!/usr/bin/env python3
"""Compiled NumPy-compatible EKF control with a synchronous threaded turn loop."""

import sys
import time

import numpy as np
from numba import njit, prange


DT = np.float32(0.1)
R = np.float32(0.25)
TOL = np.float32(0.0001)


@njit(inline="always")
def mm(a, b):
    return (
        a[0] * b[0] + a[1] * b[3] + a[2] * b[6],
        a[0] * b[1] + a[1] * b[4] + a[2] * b[7],
        a[0] * b[2] + a[1] * b[5] + a[2] * b[8],
        a[3] * b[0] + a[4] * b[3] + a[5] * b[6],
        a[3] * b[1] + a[4] * b[4] + a[5] * b[7],
        a[3] * b[2] + a[4] * b[5] + a[5] * b[8],
        a[6] * b[0] + a[7] * b[3] + a[8] * b[6],
        a[6] * b[1] + a[7] * b[4] + a[8] * b[7],
        a[6] * b[2] + a[7] * b[5] + a[8] * b[8],
    )


@njit(inline="always")
def transpose(a):
    return (a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8])


@njit(parallel=True, fastmath=False)
def dispatch(x0, x1, x2, p, velocity, angular_velocity, bearing, turns, checked):
    n = x0.shape[0]
    faults = np.zeros(n, dtype=np.int32)
    for _ in range(turns):
        for i in prange(n):
            theta = x2[i]
            st = np.sin(theta)
            ct = np.cos(theta)
            d = velocity[i] * DT
            y0 = x0[i] + d * ct
            y1 = x1[i] + d * st
            y2 = theta + angular_velocity[i] * DT
            f = (np.float32(1), np.float32(0), -d * st,
                 np.float32(0), np.float32(1), d * ct,
                 np.float32(0), np.float32(0), np.float32(1))
            old = (p[0][i], p[1][i], p[2][i], p[3][i], p[4][i],
                   p[5][i], p[6][i], p[7][i], p[8][i])
            predicted = mm(mm(f, old), transpose(f))
            # G @ Q @ G.T for G = [[ct*dt, 0], [st*dt, 0], [0, dt]].
            # Keep this expanded so Numba can scalarize it without tuple
            # comprehensions or temporary matrix allocations.
            q0 = ct * ct * np.float32(0.0001)
            q1 = ct * st * np.float32(0.0001)
            q2 = st * st * np.float32(0.0001)
            q3 = np.float32(0.000025)
            process = (
                q0, q1, np.float32(0),
                q1, q2, np.float32(0),
                np.float32(0), np.float32(0), q3,
            )
            predicted = (
                predicted[0] + process[0], predicted[1] + process[1], predicted[2] + process[2],
                predicted[3] + process[3], predicted[4] + process[4], predicted[5] + process[5],
                predicted[6] + process[6], predicted[7] + process[7], predicted[8] + process[8],
            )
            dx = np.float32(140) - y0
            dy = np.float32(12) - y1
            radius = dx * dx + dy * dy
            raw = bearing[i] - (np.arctan2(dy, dx) - y2)
            innovation = np.arctan2(np.sin(raw), np.cos(raw))
            h0 = dy / radius
            h1 = -dx / radius
            h2 = np.float32(-1)
            q0 = predicted[0] * h0 + predicted[1] * h1 + predicted[2] * h2
            q1 = predicted[3] * h0 + predicted[4] * h1 + predicted[5] * h2
            q2 = predicted[6] * h0 + predicted[7] * h1 + predicted[8] * h2
            variance = h0 * q0 + h1 * q1 + h2 * q2 + R
            k0 = q0 / variance
            k1 = q1 / variance
            k2 = q2 / variance
            a = (np.float32(1) - k0 * h0, -k0 * h1, -k0 * h2,
                 -k1 * h0, np.float32(1) - k1 * h1, -k1 * h2,
                 -k2 * h0, -k2 * h1, np.float32(1) - k2 * h2)
            corrected = mm(mm(a, predicted), transpose(a))
            candidate = (
                corrected[0] + k0 * k0 * R, corrected[1] + k0 * k1 * R,
                corrected[2] + k0 * k2 * R, corrected[3] + k1 * k0 * R,
                corrected[4] + k1 * k1 * R, corrected[5] + k1 * k2 * R,
                corrected[6] + k2 * k0 * R, corrected[7] + k2 * k1 * R,
                corrected[8] + k2 * k2 * R,
            )
            nx0 = y0 + k0 * innovation
            nx1 = y1 + k1 * innovation
            nx2 = y2 + k2 * innovation
            valid = (np.isfinite(nx0) and np.isfinite(nx1) and np.isfinite(nx2))
            for j in range(9):
                valid = valid and np.isfinite(candidate[j])
            valid = valid and candidate[0] > 0 and candidate[4] > 0 and candidate[8] > 0
            valid = valid and abs(candidate[1] - candidate[3]) <= TOL
            valid = valid and abs(candidate[2] - candidate[6]) <= TOL
            valid = valid and abs(candidate[5] - candidate[7]) <= TOL
            if checked and not valid:
                faults[i] += 1
                continue
            x0[i] = nx0
            x1[i] = nx1
            x2[i] = nx2
            for j in range(9):
                p[j][i] = candidate[j]
    return faults


@njit(parallel=True, fastmath=False)
def dispatch_fused(x0, x1, x2, p, velocity, angular_velocity, bearing, turns, checked):
    """Run all turns inside each worker's lane range.

    This is the Numba equivalent of the fused Mech block: there is one
    parallel scheduling boundary for the whole measured block, while checked
    mode still rejects each invalid candidate before publishing that lane.
    """
    n = x0.shape[0]
    faults = np.zeros(n, dtype=np.int32)
    for i in prange(n):
        for _ in range(turns):
            theta = x2[i]
            st = np.sin(theta)
            ct = np.cos(theta)
            d = velocity[i] * DT
            y0 = x0[i] + d * ct
            y1 = x1[i] + d * st
            y2 = theta + angular_velocity[i] * DT
            f = (np.float32(1), np.float32(0), -d * st,
                 np.float32(0), np.float32(1), d * ct,
                 np.float32(0), np.float32(0), np.float32(1))
            old = (p[0][i], p[1][i], p[2][i], p[3][i], p[4][i],
                   p[5][i], p[6][i], p[7][i], p[8][i])
            predicted = mm(mm(f, old), transpose(f))
            q0 = ct * ct * np.float32(0.0001)
            q1 = ct * st * np.float32(0.0001)
            q2 = st * st * np.float32(0.0001)
            q3 = np.float32(0.000025)
            predicted = (
                predicted[0] + q0, predicted[1] + q1, predicted[2],
                predicted[3] + q1, predicted[4] + q2, predicted[5],
                predicted[6], predicted[7], predicted[8] + q3,
            )
            dx = np.float32(140) - y0
            dy = np.float32(12) - y1
            radius = dx * dx + dy * dy
            raw = bearing[i] - (np.arctan2(dy, dx) - y2)
            innovation = np.arctan2(np.sin(raw), np.cos(raw))
            h0 = dy / radius
            h1 = -dx / radius
            h2 = np.float32(-1)
            q0 = predicted[0] * h0 + predicted[1] * h1 + predicted[2] * h2
            q1 = predicted[3] * h0 + predicted[4] * h1 + predicted[5] * h2
            q2 = predicted[6] * h0 + predicted[7] * h1 + predicted[8] * h2
            variance = h0 * q0 + h1 * q1 + h2 * q2 + R
            k0 = q0 / variance
            k1 = q1 / variance
            k2 = q2 / variance
            a = (np.float32(1) - k0 * h0, -k0 * h1, -k0 * h2,
                 -k1 * h0, np.float32(1) - k1 * h1, -k1 * h2,
                 -k2 * h0, -k2 * h1, np.float32(1) - k2 * h2)
            corrected = mm(mm(a, predicted), transpose(a))
            candidate = (
                corrected[0] + k0 * k0 * R, corrected[1] + k0 * k1 * R,
                corrected[2] + k0 * k2 * R, corrected[3] + k1 * k0 * R,
                corrected[4] + k1 * k1 * R, corrected[5] + k1 * k2 * R,
                corrected[6] + k2 * k0 * R, corrected[7] + k2 * k1 * R,
                corrected[8] + k2 * k2 * R,
            )
            nx0 = y0 + k0 * innovation
            nx1 = y1 + k1 * innovation
            nx2 = y2 + k2 * innovation
            if checked:
                valid = np.isfinite(nx0) and np.isfinite(nx1) and np.isfinite(nx2)
                for j in range(9):
                    valid = valid and np.isfinite(candidate[j])
                valid = valid and candidate[0] > 0 and candidate[4] > 0 and candidate[8] > 0
                valid = valid and abs(candidate[1] - candidate[3]) <= TOL
                valid = valid and abs(candidate[2] - candidate[6]) <= TOL
                valid = valid and abs(candidate[5] - candidate[7]) <= TOL
                if not valid:
                    faults[i] += 1
                    continue
            x0[i] = nx0
            x1[i] = nx1
            x2[i] = nx2
            for j in range(9):
                p[j][i] = candidate[j]
    return faults


def main():
    n = max(1, int(sys.argv[1])) if len(sys.argv) > 1 else 10000
    turns = max(1, int(sys.argv[2])) if len(sys.argv) > 2 else 20
    checked = len(sys.argv) > 3 and sys.argv[3].lower() == "checked"
    mode = sys.argv[4].lower() if len(sys.argv) > 4 else "per-turn"
    fused = mode in ("fused", "batched", "unchecked-batched")
    phase = np.float32(2 * np.pi) * np.arange(n, dtype=np.float32) / np.float32(n)
    velocity = np.float32(1) + np.float32(0.05) * np.sin(phase * np.float32(3))
    angular_velocity = np.float32(0.015) * (np.float32(1) + np.float32(0.1) * np.sin(phase * np.float32(2)))
    bearing = np.float32(-0.55) + np.float32(0.01) * np.sin(phase * np.float32(7)) + np.float32(0.005) * np.sin(phase * np.float32(11))
    x0 = np.full(n, np.float32(55))
    x1 = np.full(n, np.float32(25))
    x2 = np.full(n, np.float32(0.4))
    p = [np.full(n, np.float32(100) if j in (0, 4) else np.float32(0.15) if j == 8 else np.float32(0)) for j in range(9)]
    run = dispatch_fused if fused else dispatch
    run(x0, x1, x2, p, velocity, angular_velocity, bearing, 5, checked)
    x0.fill(55)
    x1.fill(25)
    x2.fill(0.4)
    for j, value in enumerate((100, 0, 0, 0, 100, 0, 0, 0, 0.15)):
        p[j].fill(value)
    started = time.perf_counter()
    faults = run(x0, x1, x2, p, velocity, angular_velocity, bearing, turns, checked)
    elapsed = time.perf_counter() - started
    checksum = float(x0.astype(np.float64).sum() + x1.astype(np.float64).sum() + x2.astype(np.float64).sum() + sum(a.astype(np.float64).sum() for a in p))
    print("lane: NumPy/Numba parallel JIT")
    print(f"instances: {n}")
    print(f"turns: {turns}")
    print(f"threads: {__import__('numba').get_num_threads()}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {n * turns / elapsed:.3f}")
    print(f"checksum: {checksum:.9f}")
    print(f"validation: {'checked' if checked else 'unchecked'}")
    print(f"synchronization: {'once after fused block' if fused else 'per-turn'}")
    print(f"faults: {int(faults.sum())}")


if __name__ == "__main__":
    main()
