#!/usr/bin/env python3
"""Fixed-shape SoA Taichi EKF control.

This is the Taichi equivalent of the Mojo resident kernel: every matrix is
scalarized at source level, state is structure-of-arrays, and a checked turn
rejects the complete candidate before any state store.  The benchmark keeps
the same 500,000-filter workload, exact ``atan2`` calls, and per-turn timing
boundary used by the other language controls.
"""

import argparse
import math
import statistics
import time

import numpy as np
import taichi as ti


DT = np.float32(0.1)
DT2 = np.float32(0.01)
Q0 = np.float32(0.01)
Q1 = np.float32(0.0025)
R = np.float32(0.25)
LIMIT = np.float32(3.4028235e38)
TOL = np.float32(0.0001)


def build_inputs(instances: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    phase = np.float32(2.0 * math.pi) * np.arange(instances, dtype=np.float32)
    phase /= np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(
        phase * np.float32(3.0)
    )
    angular_velocity = np.float32(0.015) * (
        np.float32(1.0) + np.float32(0.1) * np.sin(phase * np.float32(2.0))
    )
    bearing = (
        np.float32(-0.55)
        + np.float32(0.01) * np.sin(phase * np.float32(7.0))
        + np.float32(0.005) * np.sin(phase * np.float32(11.0))
    )
    return velocity, angular_velocity, bearing


def make_runner(instances: int, backend: str, threads: int):
    if backend == "cpu":
        options: dict[str, object] = {
            "arch": ti.cpu,
            "log_level": ti.ERROR,
            "fast_math": False,
        }
        if threads > 0:
            options["cpu_max_num_threads"] = threads
        ti.init(**options)
    elif backend == "gpu":
        ti.init(arch=ti.gpu, log_level=ti.ERROR, fast_math=False)
    else:
        raise ValueError(f"unsupported backend: {backend}")

    x0 = ti.field(dtype=ti.f32, shape=instances)
    x1 = ti.field(dtype=ti.f32, shape=instances)
    x2 = ti.field(dtype=ti.f32, shape=instances)
    p00 = ti.field(dtype=ti.f32, shape=instances)
    p01 = ti.field(dtype=ti.f32, shape=instances)
    p02 = ti.field(dtype=ti.f32, shape=instances)
    p10 = ti.field(dtype=ti.f32, shape=instances)
    p11 = ti.field(dtype=ti.f32, shape=instances)
    p12 = ti.field(dtype=ti.f32, shape=instances)
    p20 = ti.field(dtype=ti.f32, shape=instances)
    p21 = ti.field(dtype=ti.f32, shape=instances)
    p22 = ti.field(dtype=ti.f32, shape=instances)
    velocity = ti.field(dtype=ti.f32, shape=instances)
    angular_velocity = ti.field(dtype=ti.f32, shape=instances)
    bearing = ti.field(dtype=ti.f32, shape=instances)
    faults = ti.field(dtype=ti.i32, shape=())

    @ti.kernel
    def reset():
        for i in range(instances):
            x0[i] = 55.0
            x1[i] = 25.0
            x2[i] = 0.4
            p00[i] = 100.0
            p01[i] = 0.0
            p02[i] = 0.0
            p10[i] = 0.0
            p11[i] = 100.0
            p12[i] = 0.0
            p20[i] = 0.0
            p21[i] = 0.0
            p22[i] = 0.15
        faults[None] = 0

    @ti.kernel
    def ekf_step(checked: ti.template()):
        for i in range(instances):
            theta = x2[i]
            st = ti.sin(theta)
            ct = ti.cos(theta)
            d = velocity[i] * DT
            predicted_x0 = x0[i] + d * ct
            predicted_x1 = x1[i] + d * st
            predicted_x2 = theta + angular_velocity[i] * DT
            f02 = -d * st
            f12 = d * ct
            c00 = p00[i]
            c01 = p01[i]
            c02 = p02[i]
            c10 = p10[i]
            c11 = p11[i]
            c12 = p12[i]
            c20 = p20[i]
            c21 = p21[i]
            c22 = p22[i]
            ap00 = c00 + f02 * c20
            ap01 = c01 + f02 * c21
            ap02 = c02 + f02 * c22
            ap10 = c10 + f12 * c20
            ap11 = c11 + f12 * c21
            ap12 = c12 + f12 * c22
            process00 = ct * ct * DT2 * Q0
            process01 = ct * st * DT2 * Q0
            process11 = st * st * DT2 * Q0
            pp00 = ap00 + ap02 * f02 + process00
            pp01 = ap01 + ap02 * f12 + process01
            pp02 = ap02
            pp10 = ap10 + ap12 * f02 + process01
            pp11 = ap11 + ap12 * f12 + process11
            pp12 = ap12
            pp20 = c20 + c22 * f02
            pp21 = c21 + c22 * f12
            pp22 = c22 + DT2 * Q1
            dx = 140.0 - predicted_x0
            dy = 12.0 - predicted_x1
            squared_range = dx * dx + dy * dy
            predicted_bearing = ti.atan2(dy, dx) - predicted_x2
            raw = bearing[i] - predicted_bearing
            innovation = ti.atan2(ti.sin(raw), ti.cos(raw))
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
            candidate_x0 = predicted_x0 + k0 * innovation
            candidate_x1 = predicted_x1 + k1 * innovation
            candidate_x2 = predicted_x2 + k2 * innovation
            a00 = 1.0 - k0 * h0
            a01 = -k0 * h1
            a02 = -k0 * h2
            a10 = -k1 * h0
            a11 = 1.0 - k1 * h1
            a12 = -k1 * h2
            a20 = -k2 * h0
            a21 = -k2 * h1
            a22 = 1.0 - k2 * h2
            b00 = a00 * pp00 + a01 * pp10 + a02 * pp20
            b01 = a00 * pp01 + a01 * pp11 + a02 * pp21
            b02 = a00 * pp02 + a01 * pp12 + a02 * pp22
            b10 = a10 * pp00 + a11 * pp10 + a12 * pp20
            b11 = a10 * pp01 + a11 * pp11 + a12 * pp21
            b12 = a10 * pp02 + a11 * pp12 + a12 * pp22
            b20 = a20 * pp00 + a21 * pp10 + a22 * pp20
            b21 = a20 * pp01 + a21 * pp11 + a22 * pp21
            b22 = a20 * pp02 + a21 * pp12 + a22 * pp22
            n00 = b00 * a00 + b01 * a01 + b02 * a02 + k0 * k0 * R
            n01 = b00 * a10 + b01 * a11 + b02 * a12 + k0 * k1 * R
            n02 = b00 * a20 + b01 * a21 + b02 * a22 + k0 * k2 * R
            n10 = b10 * a00 + b11 * a01 + b12 * a02 + k1 * k0 * R
            n11 = b10 * a10 + b11 * a11 + b12 * a12 + k1 * k1 * R
            n12 = b10 * a20 + b11 * a21 + b12 * a22 + k1 * k2 * R
            n20 = b20 * a00 + b21 * a01 + b22 * a02 + k2 * k0 * R
            n21 = b20 * a10 + b21 * a11 + b22 * a12 + k2 * k1 * R
            n22 = b20 * a20 + b21 * a21 + b22 * a22 + k2 * k2 * R
            if ti.static(checked):
                valid = (
                    candidate_x0 == candidate_x0
                    and ti.abs(candidate_x0) <= LIMIT
                    and candidate_x1 == candidate_x1
                    and ti.abs(candidate_x1) <= LIMIT
                    and candidate_x2 == candidate_x2
                    and ti.abs(candidate_x2) <= LIMIT
                    and n00 == n00
                    and ti.abs(n00) <= LIMIT
                    and n01 == n01
                    and ti.abs(n01) <= LIMIT
                    and n02 == n02
                    and ti.abs(n02) <= LIMIT
                    and n10 == n10
                    and ti.abs(n10) <= LIMIT
                    and n11 == n11
                    and ti.abs(n11) <= LIMIT
                    and n12 == n12
                    and ti.abs(n12) <= LIMIT
                    and n20 == n20
                    and ti.abs(n20) <= LIMIT
                    and n21 == n21
                    and ti.abs(n21) <= LIMIT
                    and n22 == n22
                    and ti.abs(n22) <= LIMIT
                    and n00 > 0.0
                    and n11 > 0.0
                    and n22 > 0.0
                    and ti.abs(n01 - n10) <= TOL
                    and ti.abs(n02 - n20) <= TOL
                    and ti.abs(n12 - n21) <= TOL
                )
                if not valid:
                    ti.atomic_add(faults[None], 1)
                    continue
            x0[i] = candidate_x0
            x1[i] = candidate_x1
            x2[i] = candidate_x2
            p00[i] = n00
            p01[i] = n01
            p02[i] = n02
            p10[i] = n10
            p11[i] = n11
            p12[i] = n12
            p20[i] = n20
            p21[i] = n21
            p22[i] = n22

    velocity_values, angular_values, bearing_values = build_inputs(instances)
    velocity.from_numpy(velocity_values)
    angular_velocity.from_numpy(angular_values)
    bearing.from_numpy(bearing_values)
    reset()
    ekf_step(True)
    ti.sync()
    reset()
    ti.sync()
    return ekf_step, reset, (x0, x1, x2, p00, p01, p02, p10, p11, p12, p20, p21, p22), faults


def measure(
    instances: int,
    backend: str,
    turns: int,
    samples: int,
    threads: int,
    checked: bool,
    sync_each_turn: bool,
) -> dict[str, object]:
    ekf_step, reset, state, faults = make_runner(instances, backend, threads)
    for _ in range(3):
        ekf_step(checked)
    ti.sync()
    durations: list[float] = []
    for _ in range(samples):
        reset()
        ti.sync()
        started = time.perf_counter()
        for _ in range(turns):
            ekf_step(checked)
            if sync_each_turn:
                ti.sync()
        if not sync_each_turn:
            ti.sync()
        durations.append(time.perf_counter() - started)
    values = [field.to_numpy() for field in state]
    checksum = sum(float(np.sum(value, dtype=np.float64)) for value in values)
    median = statistics.median(durations)
    return {
        "backend": backend,
        "threads": threads if backend == "cpu" and threads > 0 else "default",
        "synchronization": "each-turn" if sync_each_turn else "batch",
        "instances": instances,
        "turns": turns,
        "samples": samples,
        "validation": "checked" if checked else "unchecked",
        "faults": int(faults.to_numpy()),
        "median_ms_per_turn": median * 1000.0 / turns,
        "throughput": instances * turns / median,
        "checksum": checksum,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--backend", choices=("cpu", "gpu"), default="cpu")
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--threads", type=int, default=8)
    parser.add_argument("--unchecked", action="store_true")
    parser.add_argument("--sync-each-turn", action="store_true")
    args = parser.parse_args()
    result = measure(
        max(1, args.instances),
        args.backend,
        max(1, args.turns),
        max(1, args.samples),
        max(0, args.threads),
        not args.unchecked,
        args.sync_each_turn,
    )
    print("lane: Taichi fixed-shape SoA fused kernel")
    for key, value in result.items():
        print(f"{key}: {value}")


if __name__ == "__main__":
    main()
