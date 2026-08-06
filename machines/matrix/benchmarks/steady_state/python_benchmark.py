#!/usr/bin/env python3
import argparse
import gc
import math
import statistics
import time

SAMPLE_COUNT = 9
TARGET_SAMPLE_SECONDS = 0.075
WARMUP_SECONDS = 0.25


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
        samples = []
        for _ in range(SAMPLE_COUNT):
            start = time.perf_counter()
            for _ in range(batch_iterations):
                operation()
            samples.append((time.perf_counter() - start) * 1_000.0 / batch_iterations)
    finally:
        if was_enabled:
            gc.enable()
    samples.sort()
    return statistics.median(samples), samples[0], samples[-1], batch_iterations


def matrix_value(row, column, salt):
    return ((row * 17 + column * 13 + salt * 19) % 101 + 1) / 101.0


def flat_multiply_inputs(size):
    lhs = [matrix_value(row, column, 1) for row in range(size) for column in range(size)]
    rhs = [matrix_value(row, column, 2) for row in range(size) for column in range(size)]
    return lhs, rhs


def flat_solve_inputs(size):
    matrix = []
    for row in range(size):
        for column in range(size):
            matrix.append(
                size + 4.0
                if row == column
                else ((row * 7 + column * 11) % 19) * 0.01 - 0.09
            )
    rhs = [(row % 17 + 1) / 17.0 for row in range(size)]
    return matrix, rhs


def pure_matmul(size):
    lhs, rhs = flat_multiply_inputs(size)
    output = [0.0] * (size * size)

    def operation():
        for index in range(size * size):
            output[index] = 0.0
        for row in range(size):
            row_offset = row * size
            for inner in range(size):
                lhs_value = lhs[row_offset + inner]
                rhs_offset = inner * size
                for column in range(size):
                    output[row_offset + column] += lhs_value * rhs[rhs_offset + column]

    result = measure(operation)
    expected = sum(lhs[column] * rhs[column * size] for column in range(size))
    assert abs(output[0] - expected) < 1e-8
    return result, output[0]


def pure_transpose(size):
    input_matrix, _ = flat_multiply_inputs(size)
    scaled = [0.0] * (size * size)
    output = [0.0] * (size * size)
    pulse = 1.0

    def operation():
        nonlocal pulse
        pulse = 1.000_001 if pulse == 1.0 else 1.0
        for index in range(size * size):
            scaled[index] = input_matrix[index] * pulse
        for row in range(size):
            row_offset = row * size
            for column in range(size):
                output[column * size + row] = scaled[row_offset + column]

    result = measure(operation)
    check_index = min(1, size - 1)
    assert abs(output[check_index] - input_matrix[check_index * size] * pulse) < 1e-12
    return result, output[check_index]


def pure_solve(size):
    matrix, rhs = flat_solve_inputs(size)
    work = [0.0] * (size * size)
    work_rhs = [0.0] * size
    output = [0.0] * size

    def operation():
        work[:] = matrix
        work_rhs[:] = rhs
        for pivot_column in range(size):
            pivot_row = pivot_column
            pivot_value = abs(work[pivot_column * size + pivot_column])
            for row in range(pivot_column + 1, size):
                candidate = abs(work[row * size + pivot_column])
                if candidate > pivot_value:
                    pivot_row = row
                    pivot_value = candidate
            if pivot_row != pivot_column:
                pivot_offset = pivot_column * size
                swap_offset = pivot_row * size
                for column in range(pivot_column, size):
                    work[pivot_offset + column], work[swap_offset + column] = (
                        work[swap_offset + column],
                        work[pivot_offset + column],
                    )
                work_rhs[pivot_column], work_rhs[pivot_row] = work_rhs[pivot_row], work_rhs[pivot_column]
            pivot_offset = pivot_column * size
            pivot = work[pivot_offset + pivot_column]
            for row in range(pivot_column + 1, size):
                row_offset = row * size
                factor = work[row_offset + pivot_column] / pivot
                work[row_offset + pivot_column] = 0.0
                for column in range(pivot_column + 1, size):
                    work[row_offset + column] -= factor * work[pivot_offset + column]
                work_rhs[row] -= factor * work_rhs[pivot_column]
        for row in range(size - 1, -1, -1):
            row_offset = row * size
            total = work_rhs[row]
            for column in range(row + 1, size):
                total -= work[row_offset + column] * output[column]
            output[row] = total / work[row_offset + row]

    result = measure(operation)
    residual = max(
        abs(sum(matrix[row * size + column] * output[column] for column in range(size)) - rhs[row])
        for row in range(size)
    )
    assert residual < 1e-8, residual
    return result, output[0]


def numpy_cases(size):
    import numpy as np

    lhs = np.asfortranarray(
        np.fromfunction(lambda row, column: ((row * 17 + column * 13 + 19) % 101 + 1) / 101.0, (size, size))
    )
    rhs_matrix = np.asfortranarray(
        np.fromfunction(lambda row, column: ((row * 17 + column * 13 + 38) % 101 + 1) / 101.0, (size, size))
    )
    output_matrix = np.empty((size, size), dtype=np.float64, order="F")
    matmul_result = measure(lambda: np.matmul(lhs, rhs_matrix, out=output_matrix))
    assert abs(output_matrix[0, 0] - lhs[0, :].dot(rhs_matrix[:, 0])) < 1e-8

    transpose_output = np.empty((size, size), dtype=np.float64, order="F")
    transpose_scaled = np.empty((size, size), dtype=np.float64, order="F")
    pulse = 1.0

    def transpose_operation():
        nonlocal pulse
        pulse = 1.000_001 if pulse == 1.0 else 1.0
        np.multiply(lhs, pulse, out=transpose_scaled)
        np.copyto(transpose_output, transpose_scaled.T)

    transpose_result = measure(transpose_operation)
    check_index = min(1, size - 1)
    assert abs(transpose_output[0, check_index] - lhs[check_index, 0] * pulse) < 1e-12

    row, column = np.indices((size, size))
    solve_matrix = np.asfortranarray(
        np.where(row == column, size + 4.0, ((row * 7 + column * 11) % 19) * 0.01 - 0.09)
    )
    solve_rhs = np.asfortranarray((np.arange(size) % 17 + 1) / 17.0)
    solve_output = np.empty(size, dtype=np.float64)

    def solve_operation():
        solve_output[:] = np.linalg.solve(solve_matrix, solve_rhs)

    solve_result = measure(solve_operation)
    residual = np.max(np.abs(solve_matrix @ solve_output - solve_rhs))
    assert residual < 1e-8, residual
    return (
        (matmul_result, float(output_matrix[0, 0])),
        (transpose_result, float(transpose_output[0, check_index])),
        (solve_result, float(solve_output[0])),
    )


def emit(runtime, operation, size, result, check):
    median, minimum, maximum, iterations = result
    print(f"{runtime},{operation},{size},{median:.9f},{minimum:.9f},{maximum:.9f},{iterations},{check:.12f}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("runtime", choices=("python", "numpy"))
    parser.add_argument("sizes", nargs="+", type=int)
    args = parser.parse_args()
    print("runtime,operation,size,median_ms,min_ms,max_ms,batch_iterations,check")
    for size in args.sizes:
        if args.runtime == "python":
            matmul, matmul_check = pure_matmul(size)
            transpose, transpose_check = pure_transpose(size)
            solve, solve_check = pure_solve(size)
        else:
            (matmul, matmul_check), (transpose, transpose_check), (solve, solve_check) = numpy_cases(size)
        emit(args.runtime, "matmul", size, matmul, matmul_check)
        emit(args.runtime, "transpose", size, transpose, transpose_check)
        emit(args.runtime, "solve", size, solve, solve_check)


if __name__ == "__main__":
    main()
