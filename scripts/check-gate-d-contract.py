#!/usr/bin/env python3
"""Validate frozen Gate D reports and their hard gates."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REPORT = ROOT / "benchmarks/runtime/gate-d/d2-resident-nbody.json"
DEFAULT_POINTER = ROOT / "tests/architecture/resident-activation/gate-d-regression.json"
D3_REPORT = ROOT / "benchmarks/runtime/gate-d/d3-resident-external.json"
D3_SEMANTIC_COMMIT = "cf61038766c3ec6c83fe6aeac5d0c41d579036f1"
D3_EVIDENCE_SHA256 = "90582fdc0d5773be84d83084205edb1188b175ca6c3f464124450daea5539b52"
D3_HARD_GATES = {
    "accepted_publication_store",
    "accepted_receipt_append",
    "candidate_allocations",
    "d2_pure_regression",
    "epoch_independent",
    "history_independent",
    "live_reads_exact",
    "no_commit_runtime",
    "no_legacy_journal",
    "no_premature_delivery",
    "no_rejected_delivery",
    "no_runtime_execution_transaction",
    "ordinary_outbox_append",
    "rejected_publication_store",
    "replay_exact",
    "replay_reads_zero",
    "source_bytecode_exact",
    "source_bytecode_ratio",
}

D3_THRESHOLDS = {
    "d2_pure_complete_turn_regression_max": 1.05,
    "d3_source_bytecode_ratio_max": 1.03,
    "history_ratio_max": 1.05,
    "high_epoch_ratio_max": 1.05,
}


def validate_d3_contract(report: dict) -> tuple[list[str], str]:
    errors: list[str] = []
    if report.get("schema_version") != 1 or report.get("gate") != "D":
        errors.append("Gate D3 report schema changed")
    if report.get("phase") != "D3-resident-external":
        errors.append("Gate D3 report phase changed")
    if report.get("thresholds") != D3_THRESHOLDS:
        errors.append("Gate D3 thresholds changed")
    gates = report.get("hard_gates", {})
    if not isinstance(gates, dict) or set(gates) != D3_HARD_GATES:
        errors.append("Gate D3 hard-gate names changed")
    elif any(type(value) is not bool for value in gates.values()):
        errors.append("Gate D3 hard-gate values must be booleans")
    decision = (
        "Pass"
        if isinstance(gates, dict)
        and set(gates) == D3_HARD_GATES
        and all(type(value) is bool and value for value in gates.values())
        else "Fail"
    )
    if report.get("decision") != decision:
        errors.append("Gate D3 decision does not match hard gates")
    return errors, decision


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--pointer", type=Path)
    parser.add_argument(
        "--expected-phase",
        choices=("D2-resident-nbody", "D3-resident-external"),
        default="D2-resident-nbody",
        help="bind validation to the contract phase selected by the invocation",
    )
    parser.add_argument(
        "--expected-semantic-commit", "--expected-commit", dest="expected_semantic_commit"
    )
    args = parser.parse_args(argv)
    report_path = args.report if args.report.is_absolute() else ROOT / args.report
    pointer_arg = args.pointer
    is_d3_report = report_path.resolve() == D3_REPORT.resolve()
    if pointer_arg is None and is_d3_report:
        pointer = {
            "semantic_commit": D3_SEMANTIC_COMMIT,
            "evidence_path": D3_REPORT.relative_to(ROOT).as_posix(),
            "evidence_sha256": D3_EVIDENCE_SHA256,
        }
        pointer_path = None
    else:
        pointer_path = (
            DEFAULT_POINTER
            if pointer_arg is None
            else pointer_arg if pointer_arg.is_absolute() else ROOT / pointer_arg
        )
    errors: list[str] = []
    try:
        report_bytes = report_path.read_bytes()
        report = json.loads(report_bytes)
        if pointer_path is not None:
            pointer = json.loads(pointer_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"Gate D contract failed: {error}")
        return 2
    expected_commit = args.expected_semantic_commit or pointer["semantic_commit"]
    if report.get("schema_version") != 1 or report.get("gate") != "D":
        errors.append("Gate D report schema changed")
    if report.get("phase") != args.expected_phase:
        errors.append(
            f"report phase {report.get('phase')} != expected phase {args.expected_phase}"
        )
    if report.get("semantic_commit") != expected_commit:
        errors.append(f"semantic commit {report.get('semantic_commit')} != {expected_commit}")
    digest = hashlib.sha256(report_bytes).hexdigest()
    if pointer.get("evidence_sha256") != digest:
        errors.append(f"evidence sha256 {digest} != {pointer.get('evidence_sha256')}")
    if pointer.get("evidence_path") != report_path.relative_to(ROOT).as_posix():
        errors.append("evidence path does not match the pinned report")
    if args.expected_phase == "D3-resident-external":
        contract_errors, decision = validate_d3_contract(report)
        errors.extend(contract_errors)
        if errors:
            print("Gate D contract failed:")
            print(*errors, sep="\n")
            return 1
        print(f"Gate D3 contract and evidence are internally valid: decision={decision}")
        return 0 if decision == "Pass" else 3

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
