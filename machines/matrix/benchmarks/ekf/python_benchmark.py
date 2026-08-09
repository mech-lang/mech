#!/usr/bin/env python3
import argparse
import gc
import math
import statistics
import time

SAMPLE_COUNT = 9
INPUT_PERIOD = 4_096
TARGET_SAMPLE_SECONDS = 0.075
WARMUP_SECONDS = 0.25
DT = 0.1
LANDMARK_X = 140.0
LANDMARK_Y = 12.0
MEASUREMENT_NOISE = 0.25


def wrap_angle(angle):
    return math.atan2(math.sin(angle), math.cos(angle))


def input_samples():
    truth = [45.0, 15.0, 0.0]
    samples = []
    base_angular_velocity = math.tau / (INPUT_PERIOD * DT)
    for index in range(INPUT_PERIOD):
        phase = math.tau * index / INPUT_PERIOD
        linear_velocity = 1.0 + 0.05 * math.sin(phase * 3.0)
        angular_velocity = base_angular_velocity * (1.0 + 0.1 * math.cos(phase * 2.0))
        truth[0] += linear_velocity * math.cos(truth[2]) * DT
        truth[1] += linear_velocity * math.sin(truth[2]) * DT
        truth[2] = wrap_angle(truth[2] + angular_velocity * DT)
        noise = 0.01 * math.sin(phase * 7.0) + 0.005 * math.cos(phase * 11.0)
        bearing = wrap_angle(
            math.atan2(LANDMARK_Y - truth[1], LANDMARK_X - truth[0]) - truth[2] + noise
        )
        samples.append((linear_velocity, angular_velocity, bearing))
    return samples


def transpose(matrix):
    return [list(column) for column in zip(*matrix)]


def matmul(lhs, rhs):
    rhs_columns = transpose(rhs)
    return [[sum(a * b for a, b in zip(row, column)) for column in rhs_columns] for row in lhs]


def matrix_add(lhs, rhs):
    return [[a + b for a, b in zip(lhs_row, rhs_row)] for lhs_row, rhs_row in zip(lhs, rhs)]


def matrix_subtract(lhs, rhs):
    return [[a - b for a, b in zip(lhs_row, rhs_row)] for lhs_row, rhs_row in zip(lhs, rhs)]


def matrix_scale(matrix, scale):
    return [[value * scale for value in row] for row in matrix]


class PureEkf:
    def __init__(self):
        self.state = [55.0, 25.0, 0.4]
        self.covariance = [[100.0, 0.0, 0.0], [0.0, 100.0, 0.0], [0.0, 0.0, 0.15]]

    def turn(self, sample):
        linear_velocity, angular_velocity, bearing = sample
        theta = self.state[2]
        sin_theta = math.sin(theta)
        cos_theta = math.cos(theta)
        distance = linear_velocity * DT
        predicted_state = [
            self.state[0] + distance * cos_theta,
            self.state[1] + distance * sin_theta,
            self.state[2] + angular_velocity * DT,
        ]
        motion_jacobian = [
            [1.0, 0.0, -distance * sin_theta],
            [0.0, 1.0, distance * cos_theta],
            [0.0, 0.0, 1.0],
        ]
        control_jacobian = [
            [cos_theta * DT, 0.0],
            [sin_theta * DT, 0.0],
            [0.0, DT],
        ]
        process_noise = [[0.01, 0.0], [0.0, 0.0025]]
        predicted_covariance = matrix_add(
            matmul(matmul(motion_jacobian, self.covariance), transpose(motion_jacobian)),
            matmul(matmul(control_jacobian, process_noise), transpose(control_jacobian)),
        )

        delta_x = LANDMARK_X - predicted_state[0]
        delta_y = LANDMARK_Y - predicted_state[1]
        squared_range = delta_x * delta_x + delta_y * delta_y
        predicted_bearing = math.atan2(delta_y, delta_x) - predicted_state[2]
        innovation = wrap_angle(bearing - predicted_bearing)
        observation_jacobian = [[delta_y / squared_range, -delta_x / squared_range, -1.0]]
        innovation_variance = (
            matmul(matmul(observation_jacobian, predicted_covariance), transpose(observation_jacobian))[0][0]
            + MEASUREMENT_NOISE
        )
        gain = matrix_scale(
            matmul(predicted_covariance, transpose(observation_jacobian)),
            1.0 / innovation_variance,
        )

        self.state = [predicted_state[index] + gain[index][0] * innovation for index in range(3)]
        identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        correction = matrix_subtract(identity, matmul(gain, observation_jacobian))
        self.covariance = matrix_add(
            matmul(matmul(correction, predicted_covariance), transpose(correction)),
            matrix_scale(matmul(gain, transpose(gain)), MEASUREMENT_NOISE),
        )

    def check(self):
        return sum(self.state) + sum(self.covariance[index][index] for index in range(3))


class NumpyEkf:
    def __init__(self):
        import numpy as np

        self.np = np
        self.state = np.array([55.0, 25.0, 0.4], dtype=np.float64)
        self.covariance = np.diag([100.0, 100.0, 0.15])
        self.identity = np.identity(3, dtype=np.float64)
        self.process_noise = np.diag([0.01, 0.0025])

    def turn(self, sample):
        np = self.np
        linear_velocity, angular_velocity, bearing = sample
        theta = self.state[2]
        sin_theta = math.sin(theta)
        cos_theta = math.cos(theta)
        distance = linear_velocity * DT
        predicted_state = self.state + np.array(
            [distance * cos_theta, distance * sin_theta, angular_velocity * DT]
        )
        motion_jacobian = np.array(
            [
                [1.0, 0.0, -distance * sin_theta],
                [0.0, 1.0, distance * cos_theta],
                [0.0, 0.0, 1.0],
            ]
        )
        control_jacobian = np.array(
            [[cos_theta * DT, 0.0], [sin_theta * DT, 0.0], [0.0, DT]]
        )
        predicted_covariance = (
            motion_jacobian @ self.covariance @ motion_jacobian.T
            + control_jacobian @ self.process_noise @ control_jacobian.T
        )

        delta_x = LANDMARK_X - predicted_state[0]
        delta_y = LANDMARK_Y - predicted_state[1]
        squared_range = delta_x * delta_x + delta_y * delta_y
        predicted_bearing = math.atan2(delta_y, delta_x) - predicted_state[2]
        innovation = wrap_angle(bearing - predicted_bearing)
        observation_jacobian = np.array(
            [[delta_y / squared_range, -delta_x / squared_range, -1.0]]
        )
        innovation_variance = (
            observation_jacobian @ predicted_covariance @ observation_jacobian.T
        )[0, 0] + MEASUREMENT_NOISE
        gain = predicted_covariance @ observation_jacobian.T / innovation_variance

        self.state = predicted_state + gain[:, 0] * innovation
        correction = self.identity - gain @ observation_jacobian
        self.covariance = (
            correction @ predicted_covariance @ correction.T
            + gain @ gain.T * MEASUREMENT_NOISE
        )

    def check(self):
        return float(self.state.sum() + self.covariance.trace())


def validate(samples):
    pure = PureEkf()
    numpy = NumpyEkf()
    for index in range(256):
        sample = samples[index % len(samples)]
        pure.turn(sample)
        numpy.turn(sample)
    state_error = max(abs(pure.state[index] - numpy.state[index]) for index in range(3))
    covariance_error = max(
        abs(pure.covariance[row][column] - numpy.covariance[row, column])
        for row in range(3)
        for column in range(3)
    )
    assert state_error < 1e-9 and covariance_error < 1e-9, (state_error, covariance_error)


def measure(operation):
    start = time.perf_counter()
    warmup_iterations = 0
    while warmup_iterations < 2 or time.perf_counter() - start < WARMUP_SECONDS:
        operation()
        warmup_iterations += 1

    start = time.perf_counter()
    operation()
    per_iteration = max(time.perf_counter() - start, 1e-9)
    batch_iterations = max(1, min(100_000, math.ceil(TARGET_SAMPLE_SECONDS / per_iteration)))

    was_enabled = gc.isenabled()
    gc.disable()
    try:
        timings = []
        for _ in range(SAMPLE_COUNT):
            start = time.perf_counter()
            for _ in range(batch_iterations):
                operation()
            timings.append((time.perf_counter() - start) * 1_000.0 / batch_iterations)
    finally:
        if was_enabled:
            gc.enable()
    timings.sort()
    return statistics.median(timings), timings[0], timings[-1], batch_iterations


def benchmark(filter_type, samples):
    ekf = filter_type()
    index = 0

    def operation():
        nonlocal index
        ekf.turn(samples[index])
        index = (index + 1) % len(samples)

    result = measure(operation)
    assert math.isfinite(ekf.check())
    return result, ekf.check()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("runtime", choices=("python", "numpy"))
    args = parser.parse_args()
    samples = input_samples()
    validate(samples)
    filter_type = PureEkf if args.runtime == "python" else NumpyEkf
    result, check = benchmark(filter_type, samples)
    median, minimum, maximum, iterations = result
    print("runtime,operation,median_ms,min_ms,max_ms,batch_iterations,check")
    print(
        f"{args.runtime}-loop,ekf,{median:.9f},{minimum:.9f},{maximum:.9f},{iterations},{check:.12f}"
    )


if __name__ == "__main__":
    main()
