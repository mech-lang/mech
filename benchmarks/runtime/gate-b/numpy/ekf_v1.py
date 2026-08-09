#!/usr/bin/env python3
"""Persistent NumPy control for the frozen Gate B EKF v1 workload."""

from __future__ import annotations

import argparse
import contextlib
import gc
import hashlib
import importlib.util
import io
import json
import math
import os
import platform
import struct
import sys
import time
import warnings
from pathlib import Path
from typing import Any

import numpy as np


ROOT = Path(__file__).resolve().parents[4]
TRACE_PATH = ROOT / "benchmarks/runtime/gate-b/ekf-input-v1.bin"
MANIFEST_PATH = ROOT / "benchmarks/runtime/gate-b/ekf-v1.json"
GENERATOR_PATH = ROOT / "scripts/generate-gate-b-ekf-trace.py"

EPISODE_LENGTH = 4_096
SCALED_INSTANCES = (1, 8, 64)
DT = 0.05
LANDMARK_X = 25.0
LANDMARK_Y = -10.0
ABSOLUTE_TOLERANCE = 1.0e-10
RELATIVE_TOLERANCE = 1.0e-10
QUANTIZATION = 1.0e-10
INITIAL_STATE = np.array([2.0, 1.0, 0.15], dtype=np.float64, order="F")
INITIAL_COVARIANCE = np.array(
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.05]],
    dtype=np.float64,
    order="F",
)
PROCESS_COVARIANCE = np.array(
    [[0.04, 0.0], [0.0, 0.0025]], dtype=np.float64, order="F"
)
MEASUREMENT_COVARIANCE = np.array(
    [[0.25, 0.0], [0.0, 0.0009]], dtype=np.float64, order="F"
)
IDENTITY = np.eye(3, dtype=np.float64, order="F")


def _load_trace() -> np.ndarray:
    values = np.fromfile(TRACE_PATH, dtype="<f8")
    if values.size != EPISODE_LENGTH * 4:
        raise RuntimeError("Gate B trace has the wrong number of f64 values")
    trace = np.array(values.reshape((EPISODE_LENGTH, 4)), dtype=np.float64, order="F")
    if not trace.flags.f_contiguous:
        raise RuntimeError("Gate B NumPy trace must be Fortran contiguous")
    return trace


def _load_manifest() -> dict[str, Any]:
    with MANIFEST_PATH.open("r", encoding="utf-8") as handle:
        return json.load(handle)


TRACE = _load_trace()
MANIFEST = _load_manifest()


class Workspace:
    """All mutable state and matrix scratch for one instance scale."""

    def __init__(self, instances: int) -> None:
        if instances not in SCALED_INSTANCES:
            raise ValueError("instances must be one of 1, 8, or 64")
        self.instances = instances
        self.state = np.empty((3, instances), dtype=np.float64, order="F")
        self.covariance = np.empty(
            (3, 3, instances), dtype=np.float64, order="F"
        )
        self.predicted_state = np.empty(3, dtype=np.float64, order="F")
        self.corrected_state = np.empty(3, dtype=np.float64, order="F")
        self.motion_jacobian = np.empty((3, 3), dtype=np.float64, order="F")
        self.control_jacobian = np.empty((3, 2), dtype=np.float64, order="F")
        self.gp = np.empty((3, 3), dtype=np.float64, order="F")
        self.predicted_covariance = np.empty(
            (3, 3), dtype=np.float64, order="F"
        )
        self.vq = np.empty((3, 2), dtype=np.float64, order="F")
        self.process_covariance = np.empty((3, 3), dtype=np.float64, order="F")
        self.measurement_jacobian = np.empty(
            (2, 3), dtype=np.float64, order="F"
        )
        self.hp = np.empty((2, 3), dtype=np.float64, order="F")
        self.innovation_covariance = np.empty(
            (2, 2), dtype=np.float64, order="F"
        )
        self.inverse_innovation = np.empty((2, 2), dtype=np.float64, order="F")
        self.pht = np.empty((3, 2), dtype=np.float64, order="F")
        self.gain = np.empty((3, 2), dtype=np.float64, order="F")
        self.innovation = np.empty(2, dtype=np.float64, order="F")
        self.correction = np.empty(3, dtype=np.float64, order="F")
        self.kh = np.empty((3, 3), dtype=np.float64, order="F")
        self.joseph_a = np.empty((3, 3), dtype=np.float64, order="F")
        self.ap = np.empty((3, 3), dtype=np.float64, order="F")
        self.corrected_covariance = np.empty(
            (3, 3), dtype=np.float64, order="F"
        )
        self.kr = np.empty((3, 2), dtype=np.float64, order="F")
        self.measurement_covariance = np.empty(
            (3, 3), dtype=np.float64, order="F"
        )
        self.reset()

    def reset(self) -> None:
        self.state[:] = INITIAL_STATE[:, None]
        for instance in range(self.instances):
            self.covariance[:, :, instance] = INITIAL_COVARIANCE

    def step(self, instance: int, row: np.ndarray) -> None:
        state = self.state[:, instance]
        covariance = self.covariance[:, :, instance]
        velocity = float(row[0])
        angular_velocity = float(row[1])
        measured_range = float(row[2])
        measured_bearing = float(row[3])
        theta = float(state[2])
        cosine = math.cos(theta)
        sine = math.sin(theta)

        self.motion_jacobian[:] = IDENTITY
        self.motion_jacobian[0, 2] = -velocity * sine * DT
        self.motion_jacobian[1, 2] = velocity * cosine * DT
        self.control_jacobian.fill(0.0)
        self.control_jacobian[0, 0] = cosine * DT
        self.control_jacobian[1, 0] = sine * DT
        self.control_jacobian[2, 1] = DT
        self.predicted_state[0] = state[0] + velocity * cosine * DT
        self.predicted_state[1] = state[1] + velocity * sine * DT
        self.predicted_state[2] = state[2] + angular_velocity * DT

        np.matmul(self.motion_jacobian, covariance, out=self.gp)
        np.matmul(
            self.gp, self.motion_jacobian.T, out=self.predicted_covariance
        )
        np.matmul(self.control_jacobian, PROCESS_COVARIANCE, out=self.vq)
        np.matmul(
            self.vq, self.control_jacobian.T, out=self.process_covariance
        )
        np.add(
            self.predicted_covariance,
            self.process_covariance,
            out=self.predicted_covariance,
        )

        delta_x = LANDMARK_X - self.predicted_state[0]
        delta_y = LANDMARK_Y - self.predicted_state[1]
        q = delta_x * delta_x + delta_y * delta_y
        if q <= 1.0e-12:
            raise ArithmeticError("landmark distance")
        distance = math.sqrt(q)
        predicted_bearing = (
            math.atan2(delta_y, delta_x) - self.predicted_state[2]
        )
        self.measurement_jacobian[0, 0] = -delta_x / distance
        self.measurement_jacobian[1, 0] = delta_y / q
        self.measurement_jacobian[0, 1] = -delta_y / distance
        self.measurement_jacobian[1, 1] = -delta_x / q
        self.measurement_jacobian[0, 2] = 0.0
        self.measurement_jacobian[1, 2] = -1.0
        np.matmul(
            self.measurement_jacobian,
            self.predicted_covariance,
            out=self.hp,
        )
        np.matmul(
            self.hp,
            self.measurement_jacobian.T,
            out=self.innovation_covariance,
        )
        np.add(
            self.innovation_covariance,
            MEASUREMENT_COVARIANCE,
            out=self.innovation_covariance,
        )
        determinant = (
            self.innovation_covariance[0, 0]
            * self.innovation_covariance[1, 1]
            - self.innovation_covariance[0, 1]
            * self.innovation_covariance[1, 0]
        )
        if abs(determinant) <= 1.0e-12:
            raise ArithmeticError("innovation determinant")
        self.inverse_innovation[0, 0] = (
            self.innovation_covariance[1, 1] / determinant
        )
        self.inverse_innovation[1, 0] = (
            -self.innovation_covariance[1, 0] / determinant
        )
        self.inverse_innovation[0, 1] = (
            -self.innovation_covariance[0, 1] / determinant
        )
        self.inverse_innovation[1, 1] = (
            self.innovation_covariance[0, 0] / determinant
        )
        np.matmul(
            self.predicted_covariance,
            self.measurement_jacobian.T,
            out=self.pht,
        )
        np.matmul(self.pht, self.inverse_innovation, out=self.gain)
        self.innovation[0] = measured_range - distance
        self.innovation[1] = measured_bearing - predicted_bearing
        np.matmul(self.gain, self.innovation, out=self.correction)
        np.add(
            self.predicted_state, self.correction, out=self.corrected_state
        )

        np.matmul(self.gain, self.measurement_jacobian, out=self.kh)
        np.subtract(IDENTITY, self.kh, out=self.joseph_a)
        np.matmul(
            self.joseph_a, self.predicted_covariance, out=self.ap
        )
        np.matmul(
            self.ap, self.joseph_a.T, out=self.corrected_covariance
        )
        np.matmul(self.gain, MEASUREMENT_COVARIANCE, out=self.kr)
        np.matmul(
            self.kr, self.gain.T, out=self.measurement_covariance
        )
        np.add(
            self.corrected_covariance,
            self.measurement_covariance,
            out=self.corrected_covariance,
        )
        for column in range(3):
            for matrix_row in range(column):
                symmetric = 0.5 * (
                    self.corrected_covariance[matrix_row, column]
                    + self.corrected_covariance[column, matrix_row]
                )
                self.corrected_covariance[matrix_row, column] = symmetric
                self.corrected_covariance[column, matrix_row] = symmetric

        if not np.isfinite(self.corrected_state).all():
            raise ArithmeticError("non-finite state")
        if not np.isfinite(self.corrected_covariance).all():
            raise ArithmeticError("non-finite covariance")
        if not all(
            self.corrected_covariance[index, index] > 0.0
            for index in range(3)
        ):
            raise ArithmeticError("covariance diagonal")
        symmetry_error = 0.0
        for column in range(3):
            for matrix_row in range(3):
                symmetry_error = max(
                    symmetry_error,
                    abs(
                        self.corrected_covariance[matrix_row, column]
                        - self.corrected_covariance[column, matrix_row]
                    ),
                )
        if symmetry_error > 1.0e-10:
            raise ArithmeticError("covariance symmetry")

        state[:] = self.corrected_state
        covariance[:] = self.corrected_covariance

    def run_episode(self) -> None:
        for turn in range(EPISODE_LENGTH):
            row = TRACE[turn]
            for instance in range(self.instances):
                self.step(instance, row)


WORKSPACES = {instances: Workspace(instances) for instances in SCALED_INSTANCES}


def _reference_module() -> Any:
    spec = importlib.util.spec_from_file_location(
        "gate_b_trace_generator", GENERATOR_PATH
    )
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load Gate B scalar oracle")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _close(actual: float, expected: float) -> bool:
    tolerance = ABSOLUTE_TOLERANCE + RELATIVE_TOLERANCE * abs(expected)
    return abs(actual - expected) <= tolerance


def _quantize(value: float) -> int:
    scaled = value / QUANTIZATION
    if scaled >= 0.0:
        return math.floor(scaled + 0.5)
    return math.ceil(scaled - 0.5)


def validate_per_turn(workspace: Workspace) -> str:
    oracle = _reference_module()
    workspace.reset()
    expected_state = oracle.INITIAL_STATE
    expected_covariance = oracle.INITIAL_COVARIANCE
    digest = hashlib.sha256()
    for turn in range(EPISODE_LENGTH):
        expected_state, expected_covariance, _, _, _ = oracle.ekf_step(
            expected_state,
            expected_covariance,
            tuple(float(value) for value in TRACE[turn]),
        )
        expected_values = expected_state + expected_covariance
        for instance in range(workspace.instances):
            workspace.step(instance, TRACE[turn])
            actual_values = tuple(
                float(value) for value in workspace.state[:, instance]
            )
            actual_values += tuple(
                float(value)
                for value in workspace.covariance[:, :, instance].reshape(
                    -1, order="F"
                )
            )
            for index, (actual, expected) in enumerate(
                zip(actual_values, expected_values)
            ):
                if not _close(actual, expected):
                    raise AssertionError(
                        f"Gate B NumPy mismatch at turn {turn + 1}, "
                        f"instance {instance}, value {index}: "
                        f"{actual} != {expected}"
                    )
                if instance == 0:
                    digest.update(struct.pack("<q", _quantize(actual)))
    result = digest.hexdigest()
    return result


def benchmark(instances: int, samples: int) -> dict[str, Any]:
    if samples < 1:
        raise ValueError("samples must be positive")
    workspace = WORKSPACES[instances]
    diagnostic_hash = validate_per_turn(workspace)
    workspace.reset()
    workspace.run_episode()
    durations: list[int] = []
    gc_durations: list[int] = []
    gc_started_ns = 0
    gc_total_ns = 0

    def gc_event(phase: str, _info: dict[str, int]) -> None:
        nonlocal gc_started_ns, gc_total_ns
        if phase == "start":
            gc_started_ns = time.perf_counter_ns()
        elif gc_started_ns:
            gc_total_ns += time.perf_counter_ns() - gc_started_ns
            gc_started_ns = 0

    gc.callbacks.append(gc_event)
    try:
        for _ in range(samples):
            workspace.reset()
            gc_before = gc_total_ns
            started = time.perf_counter_ns()
            workspace.run_episode()
            durations.append(time.perf_counter_ns() - started)
            gc_durations.append(gc_total_ns - gc_before)
    finally:
        gc.callbacks.remove(gc_event)
    return {
        "type": "benchmark-result",
        "lane": "numpy-persistent",
        "instances": instances,
        "turns": EPISODE_LENGTH,
        "samples_ns": durations,
        "gc_samples_ns": gc_durations,
        "allocation_count": None,
        "allocated_bytes": None,
        "correctness": True,
        "quantized_state_hash": diagnostic_hash,
        "reference_quantized_state_hash": MANIFEST["reference"][
            "quantized_trajectory_sha256"
        ],
    }


def describe() -> dict[str, Any]:
    config_output = io.StringIO()
    with warnings.catch_warnings(), contextlib.redirect_stdout(config_output):
        warnings.simplefilter("ignore", UserWarning)
        np.show_config()
    numpy_config = config_output.getvalue().strip()
    config = getattr(np.__config__, "CONFIG", {})
    dependencies = config.get("Build Dependencies", {})
    provider_names = {
        dependency.get("name", "unknown")
        for kind, dependency in dependencies.items()
        if kind.lower() in {"blas", "lapack"}
    }
    if provider_names:
        blas_lapack_provider = ", ".join(sorted(provider_names))
    else:
        lowered_config = numpy_config.lower()
        blas_lapack_provider = next(
            (
                provider
                for provider in ("Accelerate", "OpenBLAS", "MKL", "BLIS")
                if provider.lower() in lowered_config
            ),
            "unknown",
        )
    return {
        "type": "ready",
        "protocol": "gate-b-numpy-v1",
        "pid": os.getpid(),
        "python": platform.python_version(),
        "numpy": np.__version__,
        "numpy_config": numpy_config,
        "blas_lapack_provider": blas_lapack_provider,
        "trace_sha256": MANIFEST["trace"]["sha256"],
        "workload": MANIFEST["workload"],
        "episode_length": EPISODE_LENGTH,
        "instances": list(SCALED_INSTANCES),
        "fortran_contiguous": {
            "trace": bool(TRACE.flags.f_contiguous),
            "state": all(
                workspace.state.flags.f_contiguous
                for workspace in WORKSPACES.values()
            ),
            "covariance": all(
                workspace.covariance.flags.f_contiguous
                for workspace in WORKSPACES.values()
            ),
        },
    }


def emit(payload: dict[str, Any]) -> None:
    print(json.dumps(payload, sort_keys=True), flush=True)


def serve() -> int:
    emit(describe())
    for line in sys.stdin:
        try:
            request = json.loads(line)
            command = request.get("command")
            if command == "describe":
                emit(describe())
            elif command == "benchmark":
                emit(
                    benchmark(
                        int(request["instances"]), int(request["samples"])
                    )
                )
            elif command == "quit":
                emit({"type": "bye"})
                return 0
            else:
                raise ValueError(f"unknown command: {command!r}")
        except Exception as error:  # Worker errors must remain structured.
            emit(
                {
                    "type": "error",
                    "error": type(error).__name__,
                    "message": str(error),
                }
            )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="validate one EKF outside the benchmark protocol",
    )
    args = parser.parse_args()
    if args.self_test:
        emit(
            {
                "type": "self-test",
                "correctness": True,
                "quantized_state_hash": validate_per_turn(WORKSPACES[1]),
                "reference_quantized_state_hash": MANIFEST["reference"][
                    "quantized_trajectory_sha256"
                ],
            }
        )
        return 0
    return serve()


if __name__ == "__main__":
    raise SystemExit(main())
