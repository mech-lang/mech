#!/usr/bin/env python3
import argparse
import gc
import math
import statistics
import time

SAMPLE_COUNT = 9
TARGET_SAMPLE_SECONDS = 0.075
WARMUP_SECONDS = 0.25


def parse_shape(value):
    dimensions = value.lower().replace(",", "x").split("x")
    if len(dimensions) != 3:
        raise argparse.ArgumentTypeError("shapes must use rows x inner x columns")
    shape = tuple(int(dimension) for dimension in dimensions)
    if any(dimension <= 0 for dimension in shape):
        raise argparse.ArgumentTypeError("shape dimensions must be positive")
    return shape


def measure(operation):
    start = time.perf_counter()
    warmup_iterations = 0
    while warmup_iterations < 2 or time.perf_counter() - start < WARMUP_SECONDS:
        operation()
        warmup_iterations += 1

    start = time.perf_counter()
    operation()
    per_iteration = max(time.perf_counter() - start, 1e-9)
    batch_iterations = max(
        1, min(100_000, math.ceil(TARGET_SAMPLE_SECONDS / per_iteration))
    )

    was_enabled = gc.isenabled()
    gc.disable()
    try:
        samples = []
        for _ in range(SAMPLE_COUNT):
            start = time.perf_counter()
            for _ in range(batch_iterations):
                operation()
            samples.append(
                (time.perf_counter() - start) * 1_000.0 / batch_iterations
            )
    finally:
        if was_enabled:
            gc.enable()
    samples.sort()
    return statistics.median(samples), samples[0], samples[-1], batch_iterations


def matrix_value(row, column, salt):
    return ((row * 17 + column * 13 + salt * 19) % 101 + 1) / 101.0


def benchmark(shape):
    import numpy as np

    rows, inner, columns = shape
    lhs = np.asfortranarray(
        np.fromfunction(
            lambda row, column: (
                (row * 17 + column * 13 + 19) % 101 + 1
            )
            / 101.0,
            (rows, inner),
        )
    )
    rhs = np.asfortranarray(
        np.fromfunction(
            lambda row, column: (
                (row * 17 + column * 13 + 38) % 101 + 1
            )
            / 101.0,
            (inner, columns),
        )
    )
    output = np.empty((rows, columns), dtype=np.float64, order="F")
    matmul_result = measure(lambda: np.matmul(lhs, rhs, out=output))
    expected = sum(lhs[0, index] * rhs[index, 0] for index in range(inner))
    assert abs(output[0, 0] - expected) < 1e-8
    results = [("numpy-matmul", matmul_result, float(output[0, 0]))]
    if inner == 1:
        outer_result = measure(lambda: np.multiply(lhs, rhs, out=output))
        assert abs(output[0, 0] - expected) < 1e-8
        results.append(("numpy-outer", outer_result, float(output[0, 0])))
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("shapes", nargs="+", type=parse_shape)
    args = parser.parse_args()
    print(
        "runtime,rows,inner,columns,median_ms,min_ms,max_ms,"
        "batch_iterations,check"
    )
    for shape in args.shapes:
        rows, inner, columns = shape
        for runtime, result, check in benchmark(shape):
            median, minimum, maximum, iterations = result
            print(
                f"{runtime},{rows},{inner},{columns},{median:.9f},"
                f"{minimum:.9f},{maximum:.9f},{iterations},{check:.12f}"
            )


if __name__ == "__main__":
    main()
