#!/usr/bin/env python3
"""Check cross-run scalar and WebGPU EKF browser evidence."""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path


def load(results: Path, name: str) -> dict[str, object]:
    with (results / name).open(encoding="utf-8") as source:
        return json.load(source)


def close_enough(left: float, right: float) -> tuple[float, float]:
    error = abs(left - right)
    tolerance = 1e-6 + 5e-5 * max(abs(left), abs(right))
    return error, tolerance


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", type=Path)
    args = parser.parse_args()

    cpu_single = load(args.results, "ekf-cpu-single.json")
    cpu_single_no_edit = load(args.results, "ekf-cpu-single-no-edit.json")
    cpu_no_edit = load(args.results, "ekf-cpu-no-edit.json")
    cpu = load(args.results, "ekf-cpu.json")
    wgpu_no_edit = load(args.results, "ekf-wgpu-no-edit.json")
    wgpu = load(args.results, "ekf-wgpu.json")

    expected_backends = (
        (cpu_single, "cpu-scalar"),
        (cpu_single_no_edit, "cpu-scalar"),
        (cpu_no_edit, "cpu-scalar"),
        (cpu, "cpu-scalar"),
        (wgpu_no_edit, "wgpu"),
        (wgpu, "wgpu"),
    )
    if any(result["backend"] != expected for result, expected in expected_backends):
        raise SystemExit("an EKF checkpoint reported the wrong compute backend")
    if cpu["updates"] != wgpu["updates"]:
        raise SystemExit("scalar and WebGPU checkpoints used different completed turns")
    if len(cpu["output"]) != 15 or len(wgpu["output"]) != 15:
        raise SystemExit("the EKF checkpoint must contain state, covariance, and prediction")
    if any(len(result["continuity_output"]) != 15 for result, _ in expected_backends):
        raise SystemExit("the EKF continuity checkpoint must contain one complete lane")

    continuity_oracles = (
        ("CPU-1000 edited", cpu_no_edit["continuity_output"], cpu["continuity_output"]),
        (
            "CPU-1 edited",
            cpu_single_no_edit["continuity_output"],
            cpu_single["continuity_output"],
        ),
        (
            "WebGPU-1000 edited",
            wgpu_no_edit["continuity_output"],
            wgpu["continuity_output"],
        ),
    )
    continuity_failures = []
    for label, reference, observed in continuity_oracles:
        for index, (left, right) in enumerate(zip(reference, observed, strict=True)):
            error, tolerance = close_enough(left, right)
            if not math.isfinite(error) or error > tolerance:
                continuity_failures.append(
                    (label, index, left, right, error, tolerance)
                )
    if continuity_failures:
        label, index, left, right, error, tolerance = max(
            continuity_failures, key=lambda item: item[4] / item[5]
        )
        raise SystemExit(
            f"{label} EKF restarted or diverged after compatible REPL replacement at "
            f"component {index}: no_edit={left}, observed={right}, "
            f"error={error}, tolerance={tolerance}"
        )

    for index, (left, right) in enumerate(
        zip(
            cpu_no_edit["continuity_output"],
            wgpu_no_edit["continuity_output"],
            strict=True,
        )
    ):
        error, tolerance = close_enough(left, right)
        if not math.isfinite(error) or error > tolerance:
            raise SystemExit(
                f"CPU/WebGPU turn-121 parity failed at component {index}: "
                f"cpu={left}, wgpu={right}, error={error}, tolerance={tolerance}"
            )

    print(
        "EKF_WEBGPU_PARITY",
        json.dumps(
            {
                "cpu_single_no_edit": cpu_single_no_edit,
                "cpu_single": cpu_single,
                "cpu_no_edit": cpu_no_edit,
                "cpu": cpu,
                "wgpu_no_edit": wgpu_no_edit,
                "wgpu": wgpu,
            },
            sort_keys=True,
        ),
    )


if __name__ == "__main__":
    main()
