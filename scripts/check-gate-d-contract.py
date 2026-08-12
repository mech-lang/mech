#!/usr/bin/env python3
"""Validate the frozen D2 Gate D report and hard gates."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REPORT = ROOT / "benchmarks/runtime/gate-d/d2-resident-nbody.json"
DEFAULT_POINTER = ROOT / "tests/architecture/resident-activation/gate-d-regression.json"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--pointer", type=Path, default=DEFAULT_POINTER)
    parser.add_argument("--expected-semantic-commit")
    args = parser.parse_args()
    report_path = args.report if args.report.is_absolute() else ROOT / args.report
    pointer_path = args.pointer if args.pointer.is_absolute() else ROOT / args.pointer
    errors: list[str] = []
    try:
        report_bytes = report_path.read_bytes()
        report = json.loads(report_bytes)
        pointer = json.loads(pointer_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"Gate D contract failed: {error}")
        return 2
    expected_commit = args.expected_semantic_commit or pointer["semantic_commit"]
    if report.get("semantic_commit") != expected_commit:
        errors.append(f"semantic commit {report.get('semantic_commit')} != {expected_commit}")
    digest = hashlib.sha256(report_bytes).hexdigest()
    if pointer.get("evidence_sha256") != digest:
        errors.append(f"evidence sha256 {digest} != {pointer.get('evidence_sha256')}")
    if pointer.get("evidence_path") != report_path.relative_to(ROOT).as_posix():
        errors.append("evidence path does not match the pinned report")
    expected_thresholds = {
        "nbody_source_bytecode_ratio_max": 1.03,
        "nbody_resident_raw_ratio_max": 1.50,
        "nbody_legacy_gap_closure_min": 0.75,
        "nbody_history_ratio_max": 1.05,
        "nbody_high_epoch_ratio_max": 1.05,
        "ekf_source_bytecode_ratio_max": 1.03,
        "ekf_complete_d1_ratio_max": 1.20,
        "ekf_kernel_d1_ratio_max": 1.20,
    }
    if report.get("thresholds") != expected_thresholds:
        errors.append("Gate D thresholds changed")
    for workload in ("nbody", "ekf"):
        gates = report.get(workload, {}).get("hard_gates", {})
        decision = "Pass" if gates and all(gates.values()) else "Fail"
        if report.get(workload, {}).get("decision") != decision:
            errors.append(f"{workload} decision does not match hard gates")
    overall = (
        "Pass"
        if report.get("nbody", {}).get("decision") == "Pass"
        and report.get("ekf", {}).get("decision") == "Pass"
        else "Fail"
    )
    if report.get("decision") != overall:
        errors.append("overall decision does not match workload decisions")
    if errors:
        print("Gate D contract failed:")
        print(*errors, sep="\n")
        return 1
    print(f"Gate D contract and evidence are internally valid: decision={overall}")
    return 0 if overall == "Pass" else 3


if __name__ == "__main__":
    raise SystemExit(main())
