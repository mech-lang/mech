#!/usr/bin/env python3
"""Steady-state array-of-EKFs benchmark for Taichi.

The workload is the same generic bearing-only EKF used by the parallel Mech
fixture and scalar-language controls.  Taichi fields hold one persistent state
and covariance per lane; the outer lane loop is compiled into one kernel.
Compilation and field initialization happen before the timed region.
"""

from __future__ import annotations

import argparse
import math
import statistics
import time

import numpy as np
import taichi as ti


DT = np.float32(0.1)
LANDMARK_X = np.float32(140.0)
LANDMARK_Y = np.float32(12.0)
MEASUREMENT_NOISE = np.float32(0.25)
Q00 = np.float32(0.01)
Q11 = np.float32(0.0025)


def build_inputs(instances: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    phase = np.float32(2.0 * math.pi) * np.arange(instances, dtype=np.float32)
    phase /= np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(phase * np.float32(3.0))
    angular_velocity = np.float32(0.015) * (
        np.float32(1.0) + np.float32(0.1) * np.sin(phase * np.float32(2.0))
    )
    bearing = (
        np.float32(-0.55)
        + np.float32(0.01) * np.sin(phase * np.float32(7.0))
        + np.float32(0.005) * np.sin(phase * np.float32(11.0))
    )
    return velocity, angular_velocity, bearing


def reference_step(
    state: np.ndarray,
    covariance: np.ndarray,
    velocity: np.float32,
    angular_velocity: np.float32,
    bearing: np.float32,
) -> tuple[np.ndarray, np.ndarray]:
    """One f32 reference turn used only for pre-timing validation."""
    theta = state[2]
    sin_theta = np.sin(theta, dtype=np.float32)
    cos_theta = np.cos(theta, dtype=np.float32)
    distance = velocity * DT
    f = np.array(
        [[1.0, 0.0, -distance * sin_theta],
         [0.0, 1.0, distance * cos_theta],
         [0.0, 0.0, 1.0]],
        dtype=np.float32,
    )
    g = np.array(
        [[cos_theta * DT, 0.0], [sin_theta * DT, 0.0], [0.0, DT]],
        dtype=np.float32,
    )
    q = np.array([[Q00, 0.0], [0.0, Q11]], dtype=np.float32)
    predicted_state = state + np.array(
        [distance * cos_theta, distance * sin_theta, angular_velocity * DT],
        dtype=np.float32,
    )
    predicted_covariance = (
        np.matmul(np.matmul(f, covariance), f.T)
        + np.matmul(np.matmul(g, q), g.T)
    ).astype(np.float32)
    dx = LANDMARK_X - predicted_state[0]
    dy = LANDMARK_Y - predicted_state[1]
    squared_range = dx * dx + dy * dy
    predicted_bearing = np.arctan2(dy, dx) - predicted_state[2]
    raw_innovation = bearing - predicted_bearing
    innovation = np.arctan2(np.sin(raw_innovation), np.cos(raw_innovation))
    h = np.array([dy / squared_range, -dx / squared_range, -1.0], dtype=np.float32)
    pht = np.matmul(predicted_covariance, h).astype(np.float32)
    variance = np.dot(h, pht) + MEASUREMENT_NOISE
    gain = pht / variance
    next_state = (predicted_state + gain * innovation).astype(np.float32)
    a = np.eye(3, dtype=np.float32) - np.outer(gain, h)
    next_covariance = (
        np.matmul(np.matmul(a, predicted_covariance), a.T)
        + np.outer(gain, gain) * MEASUREMENT_NOISE
    ).astype(np.float32)
    return next_state, next_covariance


def make_runner(instances: int, arch: str, threads: int):
    if arch == "cpu":
        options = {"arch": ti.cpu, "log_level": ti.ERROR}
        if threads > 0:
            options["cpu_max_num_threads"] = threads
        ti.init(**options)
    elif arch == "gpu":
        ti.init(arch=ti.gpu, log_level=ti.ERROR)
    else:
        raise ValueError(f"unsupported backend: {arch}")

    state = ti.Vector.field(3, dtype=ti.f32, shape=instances)
    covariance = ti.Matrix.field(3, 3, dtype=ti.f32, shape=instances)
    velocity = ti.field(dtype=ti.f32, shape=instances)
    angular_velocity = ti.field(dtype=ti.f32, shape=instances)
    bearing = ti.field(dtype=ti.f32, shape=instances)

    @ti.kernel
    def reset():
        for i in range(instances):
            state[i] = ti.Vector([55.0, 25.0, 0.4])
            covariance[i] = ti.Matrix(
                [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 0.15]]
            )

    @ti.kernel
    def ekf_step():
        # This is the only outer loop: one independent persistent EKF per lane.
        for i in range(instances):
            xi = state[i]
            pi = covariance[i]
            theta = xi[2]
            sin_theta = ti.sin(theta)
            cos_theta = ti.cos(theta)
            distance = velocity[i] * DT

            f = ti.Matrix(
                [
                    [1.0, 0.0, -distance * sin_theta],
                    [0.0, 1.0, distance * cos_theta],
                    [0.0, 0.0, 1.0],
                ]
            )
            g = ti.Matrix(
                [
                    [cos_theta * DT, 0.0],
                    [sin_theta * DT, 0.0],
                    [0.0, DT],
                ]
            )
            x_pred = xi + ti.Vector(
                [
                    distance * cos_theta,
                    distance * sin_theta,
                    angular_velocity[i] * DT,
                ]
            )
            q = ti.Matrix([[Q00, 0.0], [0.0, Q11]])
            p_pred = f @ pi @ f.transpose() + g @ q @ g.transpose()

            dx = LANDMARK_X - x_pred[0]
            dy = LANDMARK_Y - x_pred[1]
            squared_range = dx * dx + dy * dy
            predicted_bearing = ti.atan2(dy, dx) - x_pred[2]
            raw_innovation = bearing[i] - predicted_bearing
            innovation = ti.atan2(ti.sin(raw_innovation), ti.cos(raw_innovation))

            h = ti.Vector([dy / squared_range, -dx / squared_range, -1.0])
            pht = p_pred @ h
            innovation_variance = h.dot(pht) + MEASUREMENT_NOISE
            gain = pht / innovation_variance

            identity = ti.Matrix.identity(ti.f32, 3)
            a = identity - gain.outer_product(h)
            x_next = x_pred + gain * innovation
            p_next = (
                a @ p_pred @ a.transpose()
                + gain.outer_product(gain) * MEASUREMENT_NOISE
            )
            state[i] = x_next
            covariance[i] = p_next

    velocity_values, angular_values, bearing_values = build_inputs(instances)
    velocity.from_numpy(velocity_values)
    angular_velocity.from_numpy(angular_values)
    bearing.from_numpy(bearing_values)
    reset()
    # First invocation compiles the kernel.  It is deliberately excluded.
    ekf_step()
    ti.sync()
    expected_state, expected_covariance = reference_step(
        np.array([55.0, 25.0, 0.4], dtype=np.float32),
        np.diag(np.array([100.0, 100.0, 0.15], dtype=np.float32)),
        velocity_values[0],
        angular_values[0],
        bearing_values[0],
    )
    actual_state = state.to_numpy()[0]
    actual_covariance = covariance.to_numpy()[0]
    validation_error = float(
        max(
            np.max(np.abs(actual_state - expected_state)),
            np.max(np.abs(actual_covariance - expected_covariance)),
        )
    )
    reset()
    ti.sync()
    return ekf_step, reset, state, covariance, validation_error


def measure(
    instances: int,
    arch: str,
    turns: int,
    samples: int,
    threads: int,
    sync_each_turn: bool,
) -> dict[str, object]:
    ekf_step, reset, state, covariance, validation_error = make_runner(
        instances, arch, threads
    )
    warmup_turns = max(5, min(turns, 25))
    for _ in range(warmup_turns):
        ekf_step()
    ti.sync()

    measurements: list[float] = []
    for _ in range(samples):
        # Resetting outside the timer keeps every sample at the same state.
        # The reset itself is not part of the steady-state loop measurement.
        reset()
        ti.sync()
        started = time.perf_counter()
        for _ in range(turns):
            ekf_step()
            if sync_each_turn:
                ti.sync()
        if not sync_each_turn:
            ti.sync()
        measurements.append(time.perf_counter() - started)

    state_values = state.to_numpy()
    covariance_values = covariance.to_numpy()
    checksum = float(
        np.sum(state_values, dtype=np.float64)
        + np.sum(covariance_values, dtype=np.float64)
    )
    med = statistics.median(measurements)
    return {
        "backend": arch,
        "threads": threads if arch == "cpu" and threads > 0 else "default",
        "synchronization": "each-turn" if sync_each_turn else "batch",
        "instances": instances,
        "turns": turns,
        "samples": samples,
        "median_ms_per_turn": med * 1000.0 / turns,
        "min_ms_per_turn": min(measurements) * 1000.0 / turns,
        "max_ms_per_turn": max(measurements) * 1000.0 / turns,
        "throughput": instances * turns / med,
        "checksum": checksum,
        "validation_max_abs_error": validation_error,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--backend", choices=("cpu", "gpu"), default="cpu")
    parser.add_argument("--instances", type=int, default=100_000)
    parser.add_argument("--turns", type=int, default=20)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument(
        "--sync-each-turn",
        action="store_true",
        help="synchronize the device after every timed kernel invocation",
    )
    parser.add_argument(
        "--threads",
        type=int,
        default=0,
        help="limit Taichi CPU worker threads (0 uses Taichi default)",
    )
    args = parser.parse_args()
    result = measure(
        max(1, args.instances),
        args.backend,
        max(1, args.turns),
        max(1, args.samples),
        max(0, args.threads),
        args.sync_each_turn,
    )
    print("lane: Taichi persistent array EKF")
    for key, value in result.items():
        if key in {
            "backend",
            "threads",
            "synchronization",
            "instances",
            "turns",
            "samples",
        }:
            print(f"{key}: {value}")
    print(f"median_ms_per_turn: {result['median_ms_per_turn']:.6f}")
    print(f"min_ms_per_turn: {result['min_ms_per_turn']:.6f}")
    print(f"max_ms_per_turn: {result['max_ms_per_turn']:.6f}")
    print(f"throughput: {result['throughput']:.3f}")
    print(f"checksum: {result['checksum']:.9f}")
    print(f"validation_max_abs_error: {result['validation_max_abs_error']:.3e}")


if __name__ == "__main__":
    main()
