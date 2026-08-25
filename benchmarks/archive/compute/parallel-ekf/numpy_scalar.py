#!/usr/bin/env python3
import math
import sys
import time

import numpy as np


class Scratch:
    def __init__(self):
        self.f = np.empty((3, 3), dtype=np.float32)
        self.g = np.empty((3, 2), dtype=np.float32)
        self.left = np.empty((3, 3), dtype=np.float32)
        self.predicted_p = np.empty((3, 3), dtype=np.float32)
        self.process_left = np.empty((3, 2), dtype=np.float32)
        self.process_p = np.empty((3, 3), dtype=np.float32)
        self.pht = np.empty(3, dtype=np.float32)
        self.a = np.empty((3, 3), dtype=np.float32)
        self.ap = np.empty((3, 3), dtype=np.float32)
        self.corrected_p = np.empty((3, 3), dtype=np.float32)


Q = np.array([[0.01, 0.0], [0.0, 0.0025]], dtype=np.float32)


def step(state, covariance, velocity, angular_velocity, bearing, s):
    dt = np.float32(0.1)
    sin_theta = np.sin(state[2])
    cos_theta = np.cos(state[2])
    distance = velocity * dt
    predicted_state = np.array(
        [
            state[0] + distance * cos_theta,
            state[1] + distance * sin_theta,
            state[2] + angular_velocity * dt,
        ],
        dtype=np.float32,
    )
    s.f[:] = ((1.0, 0.0, -distance * sin_theta), (0.0, 1.0, distance * cos_theta), (0.0, 0.0, 1.0))
    s.g[:] = ((cos_theta * dt, 0.0), (sin_theta * dt, 0.0), (0.0, dt))
    np.matmul(s.f, covariance, out=s.left)
    np.matmul(s.left, s.f.T, out=s.predicted_p)
    np.matmul(s.g, Q, out=s.process_left)
    np.matmul(s.process_left, s.g.T, out=s.process_p)
    s.predicted_p += s.process_p

    delta_x = np.float32(140.0) - predicted_state[0]
    delta_y = np.float32(12.0) - predicted_state[1]
    squared_range = delta_x * delta_x + delta_y * delta_y
    predicted_bearing = np.arctan2(delta_y, delta_x) - predicted_state[2]
    raw_innovation = bearing - predicted_bearing
    innovation = np.arctan2(np.sin(raw_innovation), np.cos(raw_innovation))
    h = np.array([delta_y / squared_range, -delta_x / squared_range, -1.0], dtype=np.float32)
    np.matmul(s.predicted_p, h, out=s.pht)
    innovation_variance = np.dot(h, s.pht) + np.float32(0.25)
    gain = s.pht / innovation_variance
    state[:] = predicted_state + gain * innovation
    s.a[:] = np.eye(3, dtype=np.float32) - np.outer(gain, h)
    np.matmul(s.a, s.predicted_p, out=s.ap)
    np.matmul(s.ap, s.a.T, out=s.corrected_p)
    covariance[:] = s.corrected_p + np.outer(gain, gain) * np.float32(0.25)


def main():
    instances = max(1, int(sys.argv[1]) if len(sys.argv) > 1 else 10_000)
    turns = max(1, int(sys.argv[2]) if len(sys.argv) > 2 else 5)
    phase = np.float32(2.0 * math.pi) * np.arange(instances, dtype=np.float32) / np.float32(instances)
    velocity = np.float32(1.0) + np.float32(0.05) * np.sin(phase * np.float32(3.0))
    angular_velocity = np.float32(0.015) * (np.float32(1.0) + np.float32(0.1) * np.sin(phase * np.float32(2.0)))
    bearing = np.float32(-0.55) + np.float32(0.01) * np.sin(phase * np.float32(7.0)) + np.float32(0.005) * np.sin(phase * np.float32(11.0))
    state = np.tile(np.array([55.0, 25.0, 0.4], dtype=np.float32), (instances, 1))
    covariance = np.tile(np.diag(np.array([100.0, 100.0, 0.15], dtype=np.float32)), (instances, 1, 1))
    scratch = Scratch()
    for _ in range(5):
        for lane in range(instances):
            step(state[lane], covariance[lane], velocity[lane], angular_velocity[lane], bearing[lane], scratch)
    state[:] = np.array([55.0, 25.0, 0.4], dtype=np.float32)
    covariance[:] = np.diag(np.array([100.0, 100.0, 0.15], dtype=np.float32))
    started = time.perf_counter()
    for _ in range(turns):
        for lane in range(instances):
            step(state[lane], covariance[lane], velocity[lane], angular_velocity[lane], bearing[lane], scratch)
    elapsed = time.perf_counter() - started
    print("lane: NumPy scalar outer loop")
    print(f"instances: {instances}")
    print(f"turns: {turns}")
    print(f"elapsed_s: {elapsed:.9f}")
    print(f"throughput: {instances * turns / elapsed:.3f}")
    print(f"checksum: {float(np.sum(state, dtype=np.float64) + np.sum(covariance, dtype=np.float64)):.9f}")


if __name__ == "__main__":
    main()
