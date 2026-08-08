#!/usr/bin/env python3
"""Generate and verify the frozen Gate B EKF v1 trace and oracle."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "benchmarks/runtime/gate-b"
TRACE_PATH = OUTPUT_DIR / "ekf-input-v1.bin"
HASH_PATH = OUTPUT_DIR / "ekf-input-v1.sha256"
MANIFEST_PATH = OUTPUT_DIR / "ekf-v1.json"

EPISODE_LENGTH = 4_096
DT = 0.05
LANDMARK = (25.0, -10.0)
INITIAL_STATE = (2.0, 1.0, 0.15)
INITIAL_COVARIANCE = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05)
Q = (0.04, 0.0, 0.0, 0.0025)
R = (0.25, 0.0, 0.0, 0.0009)
ABSOLUTE_TOLERANCE = 1.0e-10
RELATIVE_TOLERANCE = 1.0e-10
QUANTIZATION = 1.0e-10
FROZEN_TRACE_SHA256 = "ab901e1d115aa92166dc2a6d45a28732e6a548363b829997aa410ae4c2d77c8b"
FROZEN_HASH_FIXTURE_SHA256 = (
    "1ad00b948e57311b6e59d48e6675ced4c7278bbfef54eb90f2ff519f42be7461"
)
FROZEN_MANIFEST_SHA256 = (
    "30e28916800cdb826e6289d2da102feec64a4d208f967f55066494a683fc627b"
)


def matrix_multiply(
    left: tuple[float, ...],
    left_rows: int,
    left_columns: int,
    right: tuple[float, ...],
    right_columns: int,
) -> tuple[float, ...]:
    result = [0.0] * (left_rows * right_columns)
    for column in range(right_columns):
        for row in range(left_rows):
            total = 0.0
            for inner in range(left_columns):
                total += (
                    left[inner * left_rows + row]
                    * right[column * left_columns + inner]
                )
            result[column * left_rows + row] = total
    return tuple(result)


def transpose(
    matrix: tuple[float, ...], rows: int, columns: int
) -> tuple[float, ...]:
    result = [0.0] * len(matrix)
    for column in range(columns):
        for row in range(rows):
            result[row * columns + column] = matrix[column * rows + row]
    return tuple(result)


def add(
    left: tuple[float, ...], right: tuple[float, ...]
) -> tuple[float, ...]:
    return tuple(a + b for a, b in zip(left, right))


def ekf_step(
    state: tuple[float, float, float],
    covariance: tuple[float, ...],
    inputs: tuple[float, float, float, float],
) -> tuple[
    tuple[float, float, float], tuple[float, ...], float, float, float
]:
    px, py, theta = state
    velocity, angular_velocity, measured_range, measured_bearing = inputs
    cosine = math.cos(theta)
    sine = math.sin(theta)

    motion_jacobian = (
        1.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        -velocity * sine * DT,
        velocity * cosine * DT,
        1.0,
    )
    control_jacobian = (
        cosine * DT,
        sine * DT,
        0.0,
        0.0,
        0.0,
        DT,
    )
    predicted_state = (
        px + velocity * cosine * DT,
        py + velocity * sine * DT,
        theta + angular_velocity * DT,
    )

    gp = matrix_multiply(motion_jacobian, 3, 3, covariance, 3)
    predicted_covariance = matrix_multiply(
        gp, 3, 3, transpose(motion_jacobian, 3, 3), 3
    )
    vq = matrix_multiply(control_jacobian, 3, 2, Q, 2)
    process_covariance = matrix_multiply(
        vq, 3, 2, transpose(control_jacobian, 3, 2), 3
    )
    predicted_covariance = add(predicted_covariance, process_covariance)

    delta_x = LANDMARK[0] - predicted_state[0]
    delta_y = LANDMARK[1] - predicted_state[1]
    q = delta_x * delta_x + delta_y * delta_y
    distance = math.sqrt(q)
    predicted_measurement = (
        distance,
        math.atan2(delta_y, delta_x) - predicted_state[2],
    )
    measurement_jacobian = (
        -delta_x / distance,
        delta_y / q,
        -delta_y / distance,
        -delta_x / q,
        0.0,
        -1.0,
    )
    hp = matrix_multiply(measurement_jacobian, 2, 3, predicted_covariance, 3)
    innovation_covariance = add(
        matrix_multiply(
            hp,
            2,
            3,
            transpose(measurement_jacobian, 2, 3),
            2,
        ),
        R,
    )
    determinant = (
        innovation_covariance[0] * innovation_covariance[3]
        - innovation_covariance[2] * innovation_covariance[1]
    )
    inverse_innovation = (
        innovation_covariance[3] / determinant,
        -innovation_covariance[1] / determinant,
        -innovation_covariance[2] / determinant,
        innovation_covariance[0] / determinant,
    )
    pht = matrix_multiply(
        predicted_covariance,
        3,
        3,
        transpose(measurement_jacobian, 2, 3),
        2,
    )
    gain = matrix_multiply(pht, 3, 2, inverse_innovation, 2)
    innovation = (
        measured_range - predicted_measurement[0],
        measured_bearing - predicted_measurement[1],
    )
    correction = matrix_multiply(gain, 3, 2, innovation, 1)
    corrected_state = (
        predicted_state[0] + correction[0],
        predicted_state[1] + correction[1],
        predicted_state[2] + correction[2],
    )

    kh = matrix_multiply(gain, 3, 2, measurement_jacobian, 3)
    identity = (1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
    joseph_a = tuple(i - value for i, value in zip(identity, kh))
    ap = matrix_multiply(joseph_a, 3, 3, predicted_covariance, 3)
    joseph_covariance = matrix_multiply(
        ap, 3, 3, transpose(joseph_a, 3, 3), 3
    )
    kr = matrix_multiply(gain, 3, 2, R, 2)
    measurement_covariance = matrix_multiply(
        kr, 3, 2, transpose(gain, 3, 2), 3
    )
    corrected_covariance = add(joseph_covariance, measurement_covariance)
    corrected_covariance_transpose = transpose(corrected_covariance, 3, 3)
    corrected_covariance = tuple(
        0.5 * (value + transposed)
        for value, transposed in zip(
            corrected_covariance, corrected_covariance_transpose
        )
    )

    symmetry_error = max(
        abs(corrected_covariance[column * 3 + row]
            - corrected_covariance[row * 3 + column])
        for column in range(3)
        for row in range(3)
    )
    values = corrected_state + corrected_covariance
    if not all(math.isfinite(value) for value in values):
        raise ValueError("EKF v1 generated a non-finite value")
    if q <= 1.0e-12:
        raise ValueError("EKF v1 generated a degenerate landmark distance")
    if abs(determinant) <= 1.0e-12:
        raise ValueError("EKF v1 generated a singular innovation covariance")
    if not all(corrected_covariance[index] > 0.0 for index in (0, 4, 8)):
        raise ValueError("EKF v1 generated a non-positive covariance diagonal")
    if symmetry_error > 1.0e-10:
        raise ValueError("EKF v1 generated an asymmetric covariance")
    return (
        corrected_state,
        corrected_covariance,
        q,
        determinant,
        innovation[1],
    )


def trace_rows() -> list[tuple[float, float, float, float]]:
    truth_x, truth_y, truth_theta = 2.15, 0.9, 0.17
    rows: list[tuple[float, float, float, float]] = []
    for turn in range(EPISODE_LENGTH):
        phase = float(turn)
        velocity = (
            0.08
            + 0.018 * math.sin(phase * 0.017)
            + 0.004 * math.cos(phase * 0.043)
        )
        angular_velocity = (
            0.011 * math.sin(phase * 0.013)
            - 0.003 * math.cos(phase * 0.029)
        )
        cosine = math.cos(truth_theta)
        sine = math.sin(truth_theta)
        truth_x += velocity * cosine * DT
        truth_y += velocity * sine * DT
        truth_theta += angular_velocity * DT
        delta_x = LANDMARK[0] - truth_x
        delta_y = LANDMARK[1] - truth_y
        measured_range = math.sqrt(delta_x * delta_x + delta_y * delta_y)
        measured_range += 0.035 * math.sin(phase * 0.031)
        measured_range += 0.009 * math.cos(phase * 0.071)
        measured_bearing = math.atan2(delta_y, delta_x) - truth_theta
        measured_bearing += 0.0025 * math.sin(phase * 0.037)
        measured_bearing -= 0.0007 * math.cos(phase * 0.019)
        rows.append(
            (velocity, angular_velocity, measured_range, measured_bearing)
        )
    return rows


def quantize(value: float) -> int:
    scaled = value / QUANTIZATION
    if scaled >= 0.0:
        return math.floor(scaled + 0.5)
    return math.ceil(scaled - 0.5)


def generated_files() -> dict[Path, bytes]:
    rows = trace_rows()
    trace = b"".join(struct.pack("<4d", *row) for row in rows)
    trace_hash = hashlib.sha256(trace).hexdigest()

    state = INITIAL_STATE
    covariance = INITIAL_COVARIANCE
    trajectory_hasher = hashlib.sha256()
    maximum_bearing_innovation = 0.0
    minimum_q = math.inf
    minimum_abs_determinant = math.inf
    for row in rows:
        state, covariance, q, determinant, bearing_innovation = ekf_step(
            state, covariance, row
        )
        minimum_q = min(minimum_q, q)
        minimum_abs_determinant = min(
            minimum_abs_determinant, abs(determinant)
        )
        maximum_bearing_innovation = max(
            maximum_bearing_innovation, abs(bearing_innovation)
        )
        for value in state + covariance:
            trajectory_hasher.update(struct.pack("<q", quantize(value)))
    if maximum_bearing_innovation >= math.pi:
        raise ValueError("EKF v1 bearing innovation crosses the +/-pi boundary")

    manifest = {
        "schema_version": 1,
        "workload": "resident-ekf-v1",
        "generator": "scripts/generate-gate-b-ekf-trace.py",
        "episode_length": EPISODE_LENGTH,
        "scaled_instances": [1, 8, 64],
        "representation": {
            "scalar": "f64",
            "endianness": "little",
            "matrix_storage": "column-major",
            "trace_row": ["v", "omega", "z_range", "z_bearing"],
        },
        "constants": {
            "dt": DT,
            "landmark": list(LANDMARK),
            "initial_state": list(INITIAL_STATE),
            "initial_covariance_column_major": list(INITIAL_COVARIANCE),
            "process_covariance_column_major": list(Q),
            "measurement_covariance_column_major": list(R),
        },
        "integrity": {
            "minimum_q_exclusive": 1.0e-12,
            "minimum_abs_determinant_exclusive": 1.0e-12,
            "positive_covariance_diagonal": True,
            "maximum_symmetry_error_inclusive": 1.0e-10,
            "all_values_finite": True,
        },
        "correctness": {
            "absolute_tolerance": ABSOLUTE_TOLERANCE,
            "relative_tolerance": RELATIVE_TOLERANCE,
            "quantization": QUANTIZATION,
            "hash": "sha256-signed-i64-le-every-turn",
        },
        "trace": {
            "file": "ekf-input-v1.bin",
            "bytes": len(trace),
            "sha256": trace_hash,
            "first_eight_rows": [list(row) for row in rows[:8]],
            "last_eight_rows": [list(row) for row in rows[-8:]],
            "maximum_absolute_bearing_innovation": maximum_bearing_innovation,
        },
        "reference": {
            "final_state": list(state),
            "final_covariance_column_major": list(covariance),
            "quantized_trajectory_sha256": trajectory_hasher.hexdigest(),
            "minimum_q": minimum_q,
            "minimum_abs_determinant": minimum_abs_determinant,
        },
    }
    manifest_bytes = (
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")
    hash_bytes = f"{trace_hash}  ekf-input-v1.bin\n".encode("ascii")
    return {
        TRACE_PATH: trace,
        HASH_PATH: hash_bytes,
        MANIFEST_PATH: manifest_bytes,
    }


def committed_fixture_errors() -> list[str]:
    """Verify the frozen bytes without replaying platform-dependent libm calls."""
    expected_hashes = {
        TRACE_PATH: FROZEN_TRACE_SHA256,
        HASH_PATH: FROZEN_HASH_FIXTURE_SHA256,
        MANIFEST_PATH: FROZEN_MANIFEST_SHA256,
    }
    errors: list[str] = []
    for path, expected_hash in expected_hashes.items():
        if not path.is_file():
            errors.append(f"missing frozen fixture: {path.relative_to(ROOT)}")
            continue
        actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual_hash != expected_hash:
            errors.append(
                f"frozen fixture changed: {path.relative_to(ROOT)} "
                f"({actual_hash} != {expected_hash})"
            )
    return errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify committed fixtures without changing them",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.check:
        failures = committed_fixture_errors()
        if failures:
            print("\n".join(failures), file=sys.stderr)
            return 1
        print("Gate B EKF v1 frozen fixtures are intact")
        return 0

    generated = generated_files()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for path, contents in generated.items():
        path.write_bytes(contents)
        print(path.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
