#!/usr/bin/env python3
"""Single authoritative contract for F0 formulas, evidence, and closeout."""

from __future__ import annotations

import argparse
import importlib.util
import json
import math
import subprocess
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from f0_evidence import (  # noqa: E402
    EVIDENCE_MANIFEST,
    PRODUCT_TREE_MANIFEST,
    PROTOCOL_VERSION,
    ROOT,
    TOOLCHAIN_MANIFEST,
    EvidenceError,
    directory_evidence,
    load_json,
    same_provenance,
    sha256_file,
    validate_git_oid,
    validate_sha256,
)


D2_PHASE = "D2-resident-nbody"
D3_PHASE = "D3-resident-external"
D2_THRESHOLDS = {
    "nbody_source_bytecode_ratio_max": 1.03,
    "nbody_resident_raw_ratio_max": 1.50,
    "nbody_legacy_gap_closure_min": 0.75,
    "nbody_history_ratio_max": 1.05,
    "nbody_high_epoch_ratio_max": 1.05,
    "ekf_source_bytecode_ratio_max": 1.03,
    "ekf_complete_d1_ratio_max": 1.20,
    "ekf_kernel_d1_ratio_max": 1.20,
}
D3_THRESHOLDS = {
    "d2_pure_complete_turn_regression_max": 1.05,
    "d3_source_bytecode_ratio_max": 1.03,
    "history_ratio_max": 1.05,
    "high_epoch_ratio_max": 1.05,
}
GATE_D_SAMPLE_PROTOCOL = {
    "samples": 10,
    "turns_per_sample": 4_096,
    "profile": "release",
}
D2_NBODY_HARD_GATES = {
    "candidate_bytes",
    "candidate_materialized_bytes",
    "candidate_seed_bytes",
    "epoch_magnitude_independent",
    "history_independent",
    "legacy_gap_closure",
    "no_commit_runtime",
    "no_legacy_journal",
    "post_publication_append_infallible",
    "publication_stores",
    "published_copy_bytes",
    "record_appends",
    "record_preparations",
    "resident_raw_ratio",
    "source_bytecode_ratio",
    "trajectory_exact",
    "zero_allocation",
}
D2_EKF_HARD_GATES = {
    "candidate_bytes",
    "candidate_seed_bytes",
    "complete_d1_ratio",
    "kernel_d1_ratio",
    "publication_stores",
    "source_bytecode_ratio",
    "trajectory_exact",
    "zero_allocation",
}
D2_ADVISORY_PERFORMANCE_GATES = {
    "nbody": {"legacy_gap_closure", "resident_raw_ratio", "source_bytecode_ratio"},
    "ekf": {"complete_d1_ratio", "kernel_d1_ratio", "source_bytecode_ratio"},
}
D3_HARD_GATES = {
    "accepted_publication_store",
    "accepted_receipt_append",
    "candidate_allocations",
    "d2_authenticated",
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
D3_ADVISORY_PERFORMANCE_GATES = {"d2_pure_regression", "source_bytecode_ratio"}
GATE_B_ADVISORY_PERFORMANCE_GATES = {
    "b2_decision": {
        "executor_tax",
        "legacy_gap_closure",
        "raw_epoch_ratio",
        "tail_stability",
    },
    "d1_decision": {
        "complete_turn_control_ratio",
        "legacy_gap_closure",
        "raw_epoch_ratio",
        "source_bytecode_equivalence",
    },
}


def qualified_gate_decision(
    gates: dict[str, bool], advisory_performance_gates: set[str]
) -> str:
    return (
        "Pass"
        if all(value for name, value in gates.items() if name not in advisory_performance_gates)
        else "Fail"
    )


def d2_qualification(report: dict[str, Any]) -> tuple[str, dict[str, list[str]]]:
    failures: dict[str, list[str]] = {}
    required_pass = True
    for workload in ("nbody", "ekf"):
        gates = report.get(workload, {}).get("hard_gates", {})
        advisory = D2_ADVISORY_PERFORMANCE_GATES[workload]
        if not isinstance(gates, dict):
            failures[workload] = []
            required_pass = False
            continue
        failures[workload] = sorted(
            name for name in advisory if gates.get(name) is False
        )
        if qualified_gate_decision(gates, advisory) != "Pass":
            required_pass = False
    return ("Pass" if required_pass else "Fail"), failures


def d3_qualification(report: dict[str, Any]) -> tuple[str, list[str]]:
    gates = report.get("hard_gates", {})
    if not isinstance(gates, dict):
        return "Fail", []
    failures = sorted(
        name for name in D3_ADVISORY_PERFORMANCE_GATES if gates.get(name) is False
    )
    return qualified_gate_decision(gates, D3_ADVISORY_PERFORMANCE_GATES), failures


def gate_b_qualification(report: dict[str, Any]) -> tuple[str, dict[str, list[str]]]:
    failures: dict[str, list[str]] = {}
    required_pass = True
    for section, advisory in GATE_B_ADVISORY_PERFORMANCE_GATES.items():
        gates = report.get(section, {}).get("hard_gates", {})
        if not isinstance(gates, dict) or not gates:
            failures[section] = []
            required_pass = False
            continue
        failures[section] = sorted(
            name for name in advisory if gates.get(name) is False
        )
        if qualified_gate_decision(gates, advisory) != "Pass":
            required_pass = False
    return ("Pass" if required_pass else "Fail"), failures


def _gate_b_checker():
    path = ROOT / "scripts/check-gate-b-contract.py"
    spec = importlib.util.spec_from_file_location("f0_gate_b_contract", path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def gate_b_contract_errors(
    report: dict[str, Any], *, require_canonical: bool = True, require_pass: bool = True
) -> list[str]:
    errors = [
        f"Gate B2 {error}"
        for error in _gate_b_checker().report_contract_errors(
            report, report.get("git_commit")
        )
    ]
    if report.get("phase") != "B2-resident-turn":
        errors.append("Gate B2 phase changed")
    if require_canonical and report.get("canonical") is not True:
        errors.append("Gate B2 report is not canonical")
    qualification, advisory_failures = gate_b_qualification(report)
    if require_canonical or "qualification_decision" in report:
        if report.get("qualification_decision") != qualification:
            errors.append("Gate B2 qualification decision changed")
        if report.get("advisory_performance_failures") != advisory_failures:
            errors.append("Gate B2 advisory performance findings changed")
    if require_pass and qualification != "Pass":
        errors.append("Gate B2 report did not pass every release-blocking gate")
    return errors


def same_number(left: Any, right: Any) -> bool:
    return (
        isinstance(left, (int, float))
        and not isinstance(left, bool)
        and isinstance(right, (int, float))
        and not isinstance(right, bool)
        and math.isfinite(float(left))
        and math.isfinite(float(right))
        and math.isclose(float(left), float(right), rel_tol=1e-12, abs_tol=1e-12)
    )


def gate_b_lane(report: dict[str, Any], name: str) -> dict[str, Any]:
    matches = [
        lane
        for lane in report["lanes"]
        if lane["lane"] == name
        and lane["instances"] == 1
        and lane.get("retained_history", 0) == 0
        and lane.get("next_epoch", 1) == 1
    ]
    if len(matches) != 1:
        raise ValueError(f"expected one Gate B {name} lane, found {len(matches)}")
    return matches[0]


def _d2_expected(report: dict[str, Any], gate_b_report: dict[str, Any]) -> tuple[dict, dict, dict, dict]:
    nbody = report["nbody"]
    lanes = nbody["lanes"]
    structural = nbody["structural"]
    correctness = nbody["correctness"]
    raw = lanes["nbody-raw-rust"]["median_ns"]
    legacy = lanes["nbody-legacy-mech"]["median_ns"]
    source = lanes["nbody-resident-source"]["median_ns"]
    bytecode = lanes["nbody-resident-bytecode"]["median_ns"]
    kernel = lanes["nbody-resident-kernel-source"]["median_ns"]
    denominator = legacy - raw
    if min(raw, source, bytecode) <= 0 or denominator <= 0:
        raise ValueError("Gate D2 timing denominator is not positive")
    nbody_ratios = {
        "source_bytecode": max(source, bytecode) / min(source, bytecode),
        "resident_raw": source / raw,
        "kernel_raw": kernel / raw,
        "complete_turn_tax_ns": source - kernel,
        "legacy_gap_closure": (legacy - source) / denominator,
        "history_1k": lanes["nbody-resident-source-history-1k"]["median_ns"] / source,
        "history_100k": lanes["nbody-resident-source-history-100k"]["median_ns"] / source,
        "high_epoch": lanes["nbody-resident-source-high-epoch"]["median_ns"] / source,
    }
    nbody_gates = {
        "trajectory_exact": all(
            correctness[key]
            for key in (
                "source_bytecode_trajectory_equal",
                "raw_rust_trajectory_equal",
                "legacy_trajectory_equal",
            )
        ),
        "source_bytecode_ratio": nbody_ratios["source_bytecode"]
        <= D2_THRESHOLDS["nbody_source_bytecode_ratio_max"],
        "resident_raw_ratio": nbody_ratios["resident_raw"]
        <= D2_THRESHOLDS["nbody_resident_raw_ratio_max"],
        "legacy_gap_closure": nbody_ratios["legacy_gap_closure"]
        >= D2_THRESHOLDS["nbody_legacy_gap_closure_min"],
        "zero_allocation": lanes["nbody-resident-source"]["allocation_counts"] == [0]
        and lanes["nbody-resident-bytecode"]["allocation_counts"] == [0],
        "candidate_bytes": structural["candidate_bytes"] == 480,
        "candidate_seed_bytes": structural["candidate_seed_bytes"] == 480,
        "candidate_materialized_bytes": structural["candidate_materialized_bytes"] == 480,
        "published_copy_bytes": structural["published_buffer_copy_bytes"] == 0,
        "publication_stores": structural["publication_store_count"] == 1,
        "record_preparations": structural["record_preparation_count"] == 1,
        "record_appends": structural["record_append_count"] == 1,
        "post_publication_append_infallible": structural[
            "post_publication_append_infallible"
        ]
        is True,
        "history_independent": nbody_ratios["history_1k"]
        <= D2_THRESHOLDS["nbody_history_ratio_max"]
        and nbody_ratios["history_100k"]
        <= D2_THRESHOLDS["nbody_history_ratio_max"],
        "epoch_magnitude_independent": nbody_ratios["high_epoch"]
        <= D2_THRESHOLDS["nbody_high_epoch_ratio_max"],
        "no_commit_runtime": structural["commit_runtime_call_count"] == 0,
        "no_legacy_journal": structural["legacy_journal_capture_count"] == 0,
    }

    ekf = report["ekf"]
    ekf_lanes = ekf["lanes"]
    fresh_source = gate_b_lane(gate_b_report, "mech-resident-artifact-source")
    fresh_bytecode = gate_b_lane(gate_b_report, "mech-resident-artifact-bytecode")
    fresh_kernel_source = gate_b_lane(
        gate_b_report, "mech-resident-artifact-kernel-source"
    )
    fresh_kernel_bytecode = gate_b_lane(
        gate_b_report, "mech-resident-artifact-kernel-bytecode"
    )
    fresh_control = gate_b_lane(gate_b_report, "mech-resident-turn")
    source_ns = fresh_source["timing"]["median_ns_per_turn"]
    bytecode_ns = fresh_bytecode["timing"]["median_ns_per_turn"]
    authenticated_lanes = {
        "d2_source": source_ns,
        "d2_bytecode": bytecode_ns,
        "d2_kernel_source": fresh_kernel_source["timing"]["median_ns_per_turn"],
        "d2_kernel_bytecode": fresh_kernel_bytecode["timing"]["median_ns_per_turn"],
        "gate_b_control": fresh_control["timing"]["median_ns_per_turn"],
        "fresh_source": source_ns,
    }
    for name, value in authenticated_lanes.items():
        if not same_number(ekf_lanes.get(name), value):
            raise ValueError(f"Gate D2 EKF lane {name} changed from fresh Gate B")
    if min(
        source_ns,
        bytecode_ns,
        ekf_lanes["d1_complete"],
        ekf_lanes["d1_kernel"],
        ekf_lanes["historical_d2_source"],
    ) <= 0:
        raise ValueError("Gate D2 EKF timing denominator is not positive")
    ekf_ratios = {
        "source_bytecode": max(source_ns, bytecode_ns) / min(source_ns, bytecode_ns),
        "complete_d1": source_ns / ekf_lanes["d1_complete"],
        "kernel_d1": ekf_lanes["d2_kernel_source"] / ekf_lanes["d1_kernel"],
        "fresh_over_historical_d2": source_ns / ekf_lanes["historical_d2_source"],
    }
    ekf_gates = {
        "trajectory_exact": fresh_source["correctness"] is True
        and fresh_bytecode["correctness"] is True,
        "source_bytecode_ratio": ekf_ratios["source_bytecode"]
        <= D2_THRESHOLDS["ekf_source_bytecode_ratio_max"],
        "complete_d1_ratio": ekf_ratios["complete_d1"]
        <= D2_THRESHOLDS["ekf_complete_d1_ratio_max"],
        "kernel_d1_ratio": ekf_ratios["kernel_d1"]
        <= D2_THRESHOLDS["ekf_kernel_d1_ratio_max"],
        "zero_allocation": fresh_source["allocation"]["episode_allocation_count"] == 0,
        "candidate_bytes": fresh_source["structural"]["candidate_written_bytes"] == 96,
        "candidate_seed_bytes": fresh_source["structural"]["candidate_seed_bytes"] == 0,
        "publication_stores": fresh_source["structural"]["publication_store_count"] == 1,
    }
    return nbody_gates, nbody_ratios, ekf_gates, ekf_ratios


def _d3_expected(report: dict[str, Any]) -> dict[str, bool]:
    fixtures = report["fixtures"]
    structural = report["structural"]
    ratios = report["controls"]["ratios"]
    all_exact = True
    all_reads_exact = True
    all_receipts_exact = True
    all_publications_exact = True
    all_outbox_exact = True
    source_bytecode_ratios = []
    replay_exact = True
    replay_reads_zero = True
    for name in ("effect", "transactional"):
        fixture = fixtures[name]
        source = fixture["lanes"]["source"]
        bytecode = fixture["lanes"]["bytecode"]
        source_time = source["timing"]["median_ns"]
        bytecode_time = bytecode["timing"]["median_ns"]
        if min(source_time, bytecode_time) <= 0:
            raise ValueError("Gate D3 timing denominator is not positive")
        lane_ratio = max(source_time, bytecode_time) / min(source_time, bytecode_time)
        if not same_number(fixture.get("source_bytecode_ratio"), lane_ratio):
            raise ValueError(f"Gate D3 {name} source/bytecode ratio changed")
        source_bytecode_ratios.append(lane_ratio)
        exact = all(
            source[field] == bytecode[field]
            for field in (
                "state_hash",
                "receipt_hash",
                "effect_batch_hash",
                "effect_id_hash",
                "idempotency_key_hash",
            )
        )
        if fixture.get("source_bytecode_exact") is not exact:
            raise ValueError(f"Gate D3 {name} source/bytecode exactness changed")
        all_exact &= exact
        all_reads_exact &= source["provider_reads_per_turn"] == 1
        all_receipts_exact &= source["receipt_appends_per_turn"] == 1
        all_publications_exact &= source["publication_stores_per_turn"] == 1
        all_outbox_exact &= source["ordinary_outbox_appends_per_turn"] == (
            1 if name == "effect" else 0
        )
        replay = fixture["replay"]
        replay_reads_zero &= replay["provider_reads"] == 0
        for field in (
            "state_hash_exact",
            "receipt_hash_exact",
            "effect_batch_hash_exact",
            "effect_id_hash_exact",
            "idempotency_key_hash_exact",
        ):
            value_field = field.removesuffix("_exact")
            exact = replay[value_field] == source[value_field]
            if replay.get(field) is not exact:
                raise ValueError(f"Gate D3 {name} replay exactness changed for {value_field}")
            replay_exact &= exact
    return {
        "d2_authenticated": report["d2_authentication"].get(
            "qualification_decision", report["d2_authentication"].get("decision")
        )
        == "Pass",
        "d2_pure_regression": report["d2_pure"]["regression_ratio"]
        <= D3_THRESHOLDS["d2_pure_complete_turn_regression_max"],
        "source_bytecode_ratio": max(source_bytecode_ratios)
        <= D3_THRESHOLDS["d3_source_bytecode_ratio_max"],
        "source_bytecode_exact": all_exact,
        "candidate_allocations": structural["candidate_allocations"] == 0,
        "accepted_publication_store": structural["publication_stores_per_accepted_turn"]
        == 1
        and all_publications_exact,
        "rejected_publication_store": structural["publication_stores_per_rejected_turn"]
        == 0
        and structural["post_candidate_rejections"] == 1
        and structural["rejected_receipt_appends"] == 1
        and structural["rejected_outbox_batch_appends"] == 0
        and structural["rejected_provider_preparation_attempts"] == 1,
        "accepted_receipt_append": all_receipts_exact,
        "ordinary_outbox_append": all_outbox_exact,
        "no_premature_delivery": structural["effects_delivered_before_publication"] == 0,
        "no_rejected_delivery": structural["effects_delivered_for_rejected_turns"] == 0
        and structural["rejected_delivery_count"] == 0,
        "live_reads_exact": all_reads_exact,
        "replay_reads_zero": replay_reads_zero,
        "replay_exact": replay_exact,
        "history_independent": ratios["history_1k"]
        <= D3_THRESHOLDS["history_ratio_max"]
        and ratios["history_100k"] <= D3_THRESHOLDS["history_ratio_max"],
        "epoch_independent": ratios["high_epoch"]
        <= D3_THRESHOLDS["high_epoch_ratio_max"],
        "no_commit_runtime": structural["commit_runtime_calls"] == 0,
        "no_legacy_journal": structural["legacy_journal_captures"] == 0,
        "no_runtime_execution_transaction": structural[
            "runtime_execution_transaction_constructions"
        ]
        == 0,
    }


def gate_d_contract_errors(
    report: dict[str, Any],
    phase: str,
    *,
    gate_b_report: dict[str, Any] | None = None,
    require_canonical: bool = True,
    require_pass: bool = True,
) -> list[str]:
    errors: list[str] = []
    if report.get("schema_version") != 2 or report.get("gate") != "D":
        errors.append(f"{phase} schema identity changed")
    if report.get("phase") != phase:
        errors.append(f"{phase} phase changed")
    if report.get("sample_protocol") != GATE_D_SAMPLE_PROTOCOL:
        errors.append(f"{phase} sample protocol changed")
    if require_canonical and report.get("canonical") is not True:
        errors.append(f"{phase} report is not canonical")
    if phase == D2_PHASE:
        if report.get("thresholds") != D2_THRESHOLDS:
            errors.append("Gate D2 thresholds changed")
        nbody_lanes = report.get("nbody", {}).get("lanes", {})
        if not isinstance(nbody_lanes, dict) or any(
            not isinstance(lane, dict)
            or lane.get("samples") != GATE_D_SAMPLE_PROTOCOL["samples"]
            or lane.get("turns_per_sample")
            != GATE_D_SAMPLE_PROTOCOL["turns_per_sample"]
            for lane in nbody_lanes.values()
        ):
            errors.append("Gate D2 lane sample counts changed")
        cold = report.get("cold_path", {})
        if not isinstance(cold, dict) or len(cold) != 4 or any(
            not isinstance(lane, dict)
            or lane.get("samples") != GATE_D_SAMPLE_PROTOCOL["samples"]
            for lane in cold.values()
        ):
            errors.append("Gate D2 cold-path sample counts changed")
        if gate_b_report is None:
            errors.append("Gate D2 fresh Gate B evidence is absent")
            expected = ({}, {}, {}, {})
        else:
            try:
                expected = _d2_expected(report, gate_b_report)
            except (KeyError, TypeError, ValueError, ZeroDivisionError) as error:
                errors.append(f"Gate D2 retained measurements are malformed: {error}")
                expected = ({}, {}, {}, {})
        expected_gates = {"nbody": expected[0], "ekf": expected[2]}
        expected_ratios = {"nbody": expected[1], "ekf": expected[3]}
        decisions = []
        for workload, expected_names in (
            ("nbody", D2_NBODY_HARD_GATES),
            ("ekf", D2_EKF_HARD_GATES),
        ):
            section = report.get(workload, {})
            gates = section.get("hard_gates")
            if not isinstance(gates, dict) or set(gates) != expected_names:
                errors.append(f"Gate D2 {workload} hard-gate set changed")
                decision = "Fail"
            else:
                if gates != expected_gates[workload]:
                    errors.append(
                        f"Gate D2 {workload} hard gates do not match retained measurements"
                    )
                ratios = report.get(workload, {}).get("ratios", {})
                for name, calculated in expected_ratios[workload].items():
                    if not same_number(ratios.get(name), calculated):
                        errors.append(f"Gate D2 {workload} ratio {name} changed")
                decision = "Pass" if gates == expected_gates[workload] and all(gates.values()) else "Fail"
            if section.get("decision") != decision:
                errors.append(f"Gate D2 {workload} decision does not match hard gates")
            decisions.append(decision)
        overall = "Pass" if decisions == ["Pass", "Pass"] else "Fail"
        if report.get("decision") != overall:
            errors.append("Gate D2 decision does not match workload hard gates")
        qualification, advisory_failures = d2_qualification(report)
        if require_canonical or "qualification_decision" in report:
            if report.get("qualification_decision") != qualification:
                errors.append("Gate D2 qualification decision changed")
            if report.get("advisory_performance_failures") != advisory_failures:
                errors.append("Gate D2 advisory performance findings changed")
        if require_pass and qualification != "Pass":
            errors.append("Gate D2 did not pass every release-blocking gate")
        return errors
    if phase == D3_PHASE:
        if report.get("thresholds") != D3_THRESHOLDS:
            errors.append("Gate D3 thresholds changed")
        fixtures = report.get("fixtures", {})
        for fixture_name in ("effect", "transactional"):
            for artifact_name in ("source", "bytecode"):
                timing = (
                    fixtures.get(fixture_name, {})
                    .get("lanes", {})
                    .get(artifact_name, {})
                    .get("timing", {})
                )
                if timing.get("samples") != GATE_D_SAMPLE_PROTOCOL["samples"]:
                    errors.append(
                        f"Gate D3 {fixture_name}/{artifact_name} sample count changed"
                    )
        gates = report.get("hard_gates")
        try:
            expected_gates = _d3_expected(report)
        except (KeyError, TypeError, ValueError, ZeroDivisionError) as error:
            errors.append(f"Gate D3 retained measurements are malformed: {error}")
            expected_gates = {}
        if not isinstance(gates, dict) or set(gates) != D3_HARD_GATES:
            errors.append("Gate D3 hard-gate set changed")
            decision = "Fail"
        else:
            if gates != expected_gates:
                errors.append("Gate D3 hard gates do not match retained measurements")
            decision = "Pass" if gates == expected_gates and all(gates.values()) else "Fail"
        if report.get("decision") != decision:
            errors.append("Gate D3 decision does not match hard gates")
        qualification, advisory_failures = d3_qualification(report)
        if require_canonical or "qualification_decision" in report:
            if report.get("qualification_decision") != qualification:
                errors.append("Gate D3 qualification decision changed")
            if report.get("advisory_performance_failures") != advisory_failures:
                errors.append("Gate D3 advisory performance findings changed")
        if require_pass and qualification != "Pass":
            errors.append("Gate D3 did not pass every release-blocking gate")
        return errors
    return [f"unsupported F0 phase {phase}"]


def phase_contract_errors(
    report: dict[str, Any], key: str, *, gate_b_report: dict[str, Any] | None = None
) -> list[str]:
    if key == "gate_b2":
        return gate_b_contract_errors(report)
    if key == "gate_d2":
        return gate_d_contract_errors(report, D2_PHASE, gate_b_report=gate_b_report)
    if key == "gate_d3":
        return gate_d_contract_errors(report, D3_PHASE)
    return [f"unknown F0 report key {key}"]


RAW_LAYOUT = {
    "benchmark_output": "b2-criterion.log",
    "criterion_samples": "b2-criterion-samples",
    "numpy_output": "b2-numpy.json",
    "structural_output": "b2-structural.log",
}


def load_runner():
    path = ROOT / "scripts/run-gate-b-benchmarks.py"
    spec = importlib.util.spec_from_file_location("f0_authenticated_gate_b_runner", path)
    if spec is None or spec.loader is None:
        raise EvidenceError("cannot load the sealed Gate B runner")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def compare_value(actual: Any, expected: Any, label: str, errors: list[str]) -> None:
    if isinstance(actual, bool) or isinstance(expected, bool):
        if actual is not expected:
            errors.append(f"{label} changed from authenticated Gate B raw evidence")
        return
    if isinstance(actual, (int, float)) and isinstance(expected, (int, float)):
        if not (
            math.isfinite(float(actual))
            and math.isfinite(float(expected))
            and math.isclose(float(actual), float(expected), rel_tol=1e-12, abs_tol=1e-12)
        ):
            errors.append(f"{label} changed from authenticated Gate B raw evidence")
        return
    if isinstance(actual, dict) and isinstance(expected, dict):
        if set(actual) != set(expected):
            errors.append(f"{label} fields changed from authenticated Gate B raw evidence")
            return
        for key in sorted(actual):
            compare_value(actual[key], expected[key], f"{label}.{key}", errors)
        return
    if isinstance(actual, list) and isinstance(expected, list):
        if len(actual) != len(expected):
            errors.append(f"{label} length changed from authenticated Gate B raw evidence")
            return
        for index, (left, right) in enumerate(zip(actual, expected)):
            compare_value(left, right, f"{label}[{index}]", errors)
        return
    if actual != expected:
        errors.append(f"{label} changed from authenticated Gate B raw evidence")


def sample_protocol_errors(
    report: dict[str, Any],
    lanes: list[dict[str, Any]],
    numpy_results: list[dict[str, Any]],
    protocol: dict[str, Any],
) -> list[str]:
    """Bind the report and both retained sample sources to the frozen protocol."""
    errors: list[str] = []
    if report.get("sample_protocol") != protocol:
        errors.append("Gate B retained sample protocol changed")
    for lane in lanes:
        if lane.get("sample_count") != protocol["criterion_sample_size"]:
            errors.append(
                "Gate B retained raw lane "
                f"{lane.get('lane')}/{lane.get('instances')} does not have exactly "
                f"{protocol['criterion_sample_size']} samples"
            )
    numpy_instances = []
    for result in numpy_results:
        instances = result.get("instances")
        if not isinstance(instances, int) or isinstance(instances, bool):
            errors.append("Gate B retained raw NumPy lane identity is not integral")
            continue
        numpy_instances.append(instances)
        if (
            result.get("type") != "benchmark-result"
            or result.get("lane") != "numpy-persistent"
            or not isinstance(result.get("turns"), int)
            or isinstance(result.get("turns"), bool)
            or result.get("turns") != protocol["turns_per_sample"]
        ):
            errors.append(f"Gate B retained raw NumPy lane {instances} changed")
        if len(result.get("samples_ns", [])) != protocol["numpy_sample_size"]:
            errors.append(
                "Gate B retained raw NumPy lane "
                f"{result.get('instances')} does not have exactly "
                f"{protocol['numpy_sample_size']} samples"
            )
    if sorted(numpy_instances) != [1, 8, 64]:
        errors.append("Gate B retained raw NumPy lane set changed")
    return errors


def gate_b_raw_evidence_errors(
    report: dict[str, Any], evidence_root: Path, logical_prefix: str
) -> list[str]:
    """Verify exact chain paths/hashes and rebuild every Gate B summary field."""
    errors: list[str] = []
    raw = report.get("raw_evidence")
    if not isinstance(raw, dict) or set(raw) != set(RAW_LAYOUT):
        return ["Gate B raw evidence set is incomplete"]
    physical: dict[str, Path] = {}
    for name, relative in RAW_LAYOUT.items():
        reference = raw.get(name)
        expected_logical = f"{logical_prefix}/{relative}"
        if not isinstance(reference, dict):
            errors.append(f"Gate B {name} reference is malformed")
            continue
        if reference.get("path") != expected_logical:
            errors.append(
                f"Gate B {name} is not bound to its registered chain path"
            )
        path = evidence_root / relative
        physical[name] = path
        try:
            if name == "criterion_samples":
                actual = directory_evidence(path, expected_logical)
                if actual.get("files") != reference.get("files"):
                    errors.append("Gate B Criterion sample manifest changed")
                if actual.get("tree_sha256") != reference.get("tree_sha256"):
                    errors.append("Gate B Criterion sample tree changed")
            elif sha256_file(path) != reference.get("sha256"):
                errors.append(f"Gate B {name} digest changed")
        except (EvidenceError, OSError, TypeError) as error:
            errors.append(f"Gate B {name} cannot be authenticated: {error}")
    if errors or set(physical) != set(RAW_LAYOUT):
        return errors
    try:
        runner = load_runner()
        benchmark = physical["benchmark_output"].read_text(encoding="utf-8")
        structural = physical["structural_output"].read_text(encoding="utf-8")
        numpy = load_json(physical["numpy_output"])
        timed_probes = runner.parse_probe_samples(benchmark)
        structural_probes = runner.parse_probe_samples(structural)
        expected_timed, expected_structural = runner.expected_b2_probe_keys()
        if set(timed_probes) != expected_timed:
            errors.append("Gate B retained timed probe set changed")
        if set(structural_probes) != expected_structural:
            errors.append("Gate B retained structural probe set changed")
        if errors:
            return errors
        probes = runner.merge_structural_probes(timed_probes, structural_probes)
        reference_hash = load_json(ROOT / "benchmarks/runtime/gate-b/ekf-v1.json")[
            "reference"
        ]["quantized_trajectory_sha256"]
        lanes = runner.assemble_lanes(
            runner.criterion_samples_from_root(physical["criterion_samples"]),
            probes,
            numpy["results"],
            reference_hash,
        )
        errors.extend(
            sample_protocol_errors(
                report, lanes, numpy.get("results", []), runner.SAMPLE_PROTOCOL
            )
        )
        expected = {
            "lanes": lanes,
            "derived": runner.legacy_denominator(lanes),
            "b1_progression": runner.b1_progression(lanes),
            "b2_decision": runner.b2_decision(lanes),
            "d1_decision": runner.d1_decision(lanes),
        }
        for field, value in expected.items():
            compare_value(report.get(field), value, f"Gate B {field}", errors)
    except (EvidenceError, KeyError, OSError, TypeError, ValueError) as error:
        errors.append(f"Gate B raw reconstruction failed: {error}")
    return errors


RECORDED_CHAINS = ("chain-1", "chain-2", "chain-3")
PHASES = ("B2", "D2", "D3")
REPLICATION_RULE = {
    "recorded_chains": list(RECORDED_CHAINS),
    "phase_order": list(PHASES),
    "canonical_chain": "chain-1",
    "retain_every_chain": True,
    "replace_failed_chain": False,
}
MANIFEST_KEYS = {
    "schema_version",
    "protocol_version",
    "product_subject",
    "protocol",
    "environment",
    "replication_rule",
    "evidence",
    "closeout",
}
PROVENANCE_FIELDS = (
    "protocol_version",
    "runtime_subject_commit",
    "runtime_subject_tree",
    "qualification_protocol_commit",
    "evidence_generation_commit",
    "qualification_environment_id",
    "chain_id",
    "session_id",
    "workflow_run_id",
    "workflow_run_attempt",
)


def repository_path(value: Any, label: str, errors: list[str], root: Path) -> Path | None:
    if not isinstance(value, str) or not value:
        errors.append(f"{label} path is missing")
        return None
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        errors.append(f"{label} path is not repository-relative")
        return None
    path = root / relative
    try:
        path.resolve().relative_to(root.resolve())
    except ValueError:
        errors.append(f"{label} path escapes the repository")
        return None
    return path


def git_tree(commit: str, root: Path) -> str | None:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", f"{commit}^{{tree}}"], cwd=root, text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def reference(
    value: Any, label: str, errors: list[str], root: Path
) -> tuple[dict[str, Any], Path] | None:
    if not isinstance(value, dict) or set(value) != {"path", "sha256"}:
        errors.append(f"{label} reference is malformed")
        return None
    validate_sha256(value.get("sha256"), f"{label} sha256", errors)
    path = repository_path(value.get("path"), label, errors, root)
    if path is None:
        return None
    try:
        if sha256_file(path) != value.get("sha256"):
            errors.append(f"{label} bytes changed")
    except OSError as error:
        errors.append(f"{label} cannot be read: {error}")
        return None
    return value, path


def report_reference(
    value: Any, phase: str, errors: list[str], root: Path
) -> tuple[dict[str, Any], Path, dict[str, Any]] | None:
    loaded = reference(value, f"F0 {phase} report", errors, root)
    if loaded is None:
        return None
    ref, path = loaded
    try:
        report = load_json(path)
    except EvidenceError as error:
        errors.append(str(error))
        return None
    return ref, path, report


def protocol_errors(manifest: dict[str, Any], root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    if set(manifest) != MANIFEST_KEYS:
        errors.append("F0 manifest fields changed")
    if manifest.get("schema_version") != 1:
        errors.append("F0 manifest schema changed")
    if manifest.get("protocol_version") != PROTOCOL_VERSION:
        errors.append("F0 protocol version changed")

    try:
        product = manifest["product_subject"]
        frozen = load_json(root / PRODUCT_TREE_MANIFEST.relative_to(ROOT))
    except (KeyError, EvidenceError) as error:
        errors.append(f"F0 product subject is unavailable: {error}")
        product, frozen = {}, {}
    if not isinstance(product, dict) or set(product) != {"commit", "tree", "guard"}:
        errors.append("F0 product subject is malformed")
    else:
        validate_git_oid(product.get("commit"), "F0 product commit", errors)
        validate_git_oid(product.get("tree"), "F0 product tree", errors)
        if product.get("commit") != frozen.get("baseline_commit"):
            errors.append("F0 product commit differs from the product-tree guard")
        if product.get("tree") != frozen.get("baseline_tree"):
            errors.append("F0 product tree differs from the product-tree guard")
        if product.get("guard") != PRODUCT_TREE_MANIFEST.relative_to(ROOT).as_posix():
            errors.append("F0 product-tree guard path changed")

    protocol = manifest.get("protocol")
    if not isinstance(protocol, dict) or set(protocol) != {"commit", "tree", "contract"}:
        errors.append("F0 reviewed protocol reference is malformed")
    else:
        commit, tree = protocol.get("commit"), protocol.get("tree")
        if (commit is None) != (tree is None):
            errors.append("F0 reviewed protocol commit and tree must be recorded together")
        if commit is not None:
            validate_git_oid(commit, "F0 reviewed protocol commit", errors)
            validate_git_oid(tree, "F0 reviewed protocol tree", errors)
            if git_tree(commit, root) != tree:
                errors.append("F0 reviewed protocol tree does not match its commit")
        if protocol.get("contract") != "scripts/f0_contract.py":
            errors.append("F0 authoritative phase contract path changed")

    environment = manifest.get("environment")
    expected_lock = TOOLCHAIN_MANIFEST.relative_to(ROOT).as_posix()
    if not isinstance(environment, dict) or set(environment) != {
        "toolchain_lock",
        "toolchain_lock_sha256",
        "qualification_environment_id",
    }:
        errors.append("F0 environment reference is malformed")
    else:
        if environment.get("toolchain_lock") != expected_lock:
            errors.append("F0 measurement toolchain path changed")
        validate_sha256(
            environment.get("toolchain_lock_sha256"),
            "F0 measurement toolchain sha256",
            errors,
        )
        try:
            if sha256_file(root / expected_lock) != environment.get(
                "toolchain_lock_sha256"
            ):
                errors.append("F0 measurement toolchain bytes changed")
        except OSError as error:
            errors.append(f"F0 measurement toolchain cannot be read: {error}")
        identity = environment.get("qualification_environment_id")
        if identity is not None:
            validate_sha256(identity, "F0 qualification environment ID", errors)

    if manifest.get("replication_rule") != REPLICATION_RULE:
        errors.append("F0 replication rule changed")
    if manifest.get("evidence") is None and manifest.get("closeout") is not None:
        errors.append("F0 closeout cannot precede controlled evidence")
    return errors


def ledger_errors(
    ledger: dict[str, Any], evidence: dict[str, Any], manifest: dict[str, Any]
) -> list[str]:
    errors: list[str] = []
    expected = {
        "protocol_version": PROTOCOL_VERSION,
        "runtime_subject_commit": manifest["product_subject"]["commit"],
        "runtime_subject_tree": manifest["product_subject"]["tree"],
        "qualification_protocol_commit": manifest["protocol"]["commit"],
        "evidence_generation_commit": evidence.get("generation_commit"),
        "qualification_environment_id": manifest["environment"].get(
            "qualification_environment_id"
        ),
    }
    if ledger.get("status") != "Pass":
        errors.append("F0 session did not pass")
    for field, value in expected.items():
        if ledger.get(field) != value:
            errors.append(f"F0 session {field} changed")
    preconditioning = ledger.get("preconditioning")
    if not isinstance(preconditioning, dict) or preconditioning.get("status") != "Pass":
        errors.append("F0 untimed preconditioning did not pass")
    elif "reports" in preconditioning or "chain_id" in preconditioning:
        errors.append("F0 preconditioning became an evidence chain")
    chains = ledger.get("chains")
    if not isinstance(chains, list) or [row.get("chain_id") for row in chains] != list(
        RECORDED_CHAINS
    ):
        errors.append("F0 session recorded-chain order changed")
        return errors
    for chain in chains:
        if chain.get("status") != "Pass":
            errors.append(f"F0 {chain.get('chain_id')} did not pass")
        steps = chain.get("steps")
        if not isinstance(steps, list) or [row.get("phase") for row in steps] != list(PHASES):
            errors.append(f"F0 {chain.get('chain_id')} phase order changed")
        elif any(row.get("returncode") != 0 for row in steps):
            errors.append(f"F0 {chain.get('chain_id')} contains a failed phase")
    return errors


def d3_binding_errors(
    d2_ref: dict[str, Any], d2: dict[str, Any], d3: dict[str, Any]
) -> list[str]:
    authentication = d3.get("d2_authentication")
    if not isinstance(authentication, dict):
        return ["Gate D3 D2 authentication is absent"]
    expected = {
        "evidence_sha256": d2_ref.get("sha256"),
        "decision": d2.get("decision"),
        "qualification_decision": "Pass",
        "runtime_subject_tree": d2.get("provenance", {}).get("runtime_subject_tree"),
        "qualification_environment_id": d2.get("provenance", {}).get(
            "qualification_environment_id"
        ),
        "protocol_version": d2.get("provenance", {}).get("protocol_version"),
        "chain_id": d2.get("provenance", {}).get("chain_id"),
    }
    return [
        f"Gate D3 authenticated a different D2 {field}"
        for field, value in expected.items()
        if authentication.get(field) != value
    ]


def evidence_errors(manifest: dict[str, Any], root: Path = ROOT) -> list[str]:
    evidence = manifest.get("evidence")
    if evidence is None:
        return []
    errors: list[str] = []
    if not isinstance(evidence, dict) or set(evidence) != {
        "generation_commit",
        "generation_tree",
        "session",
        "chains",
    }:
        return ["F0 controlled evidence record is malformed"]
    validate_git_oid(evidence.get("generation_commit"), "F0 generation commit", errors)
    validate_git_oid(evidence.get("generation_tree"), "F0 generation tree", errors)
    if git_tree(evidence.get("generation_commit", ""), root) != evidence.get(
        "generation_tree"
    ):
        errors.append("F0 generation tree does not match its commit")
    if manifest.get("protocol", {}).get("commit") is None:
        errors.append("F0 evidence has no reviewed protocol commit")
    if manifest.get("environment", {}).get("qualification_environment_id") is None:
        errors.append("F0 evidence has no qualification environment ID")

    session_loaded = reference(evidence.get("session"), "F0 session ledger", errors, root)
    if session_loaded is not None:
        try:
            errors.extend(ledger_errors(load_json(session_loaded[1]), evidence, manifest))
        except EvidenceError as error:
            errors.append(str(error))

    chains = evidence.get("chains")
    if not isinstance(chains, list) or [row.get("id") for row in chains] != list(
        RECORDED_CHAINS
    ):
        errors.append("F0 evidence chain set or order changed")
        return errors
    session_id: str | None = None
    for chain in chains:
        chain_id = chain.get("id")
        logical_root = f"benchmarks/runtime/f0-evidence/{chain_id}"
        if chain.get("root") != logical_root:
            errors.append(f"F0 {chain_id} evidence root changed")
        reports = chain.get("reports")
        if not isinstance(reports, dict) or list(reports) != list(PHASES):
            errors.append(f"F0 {chain_id} report set or order changed")
            continue
        loaded = {
            phase: report_reference(reports.get(phase), phase, errors, root)
            for phase in PHASES
        }
        if any(value is None for value in loaded.values()):
            continue
        b2_ref, _, b2 = loaded["B2"]  # type: ignore[misc]
        d2_ref, _, d2 = loaded["D2"]  # type: ignore[misc]
        _, _, d3 = loaded["D3"]  # type: ignore[misc]
        errors.extend(gate_b_contract_errors(b2))
        errors.extend(gate_b_raw_evidence_errors(b2, root / logical_root, logical_root))
        errors.extend(gate_d_contract_errors(d2, D2_PHASE, gate_b_report=b2))
        errors.extend(gate_d_contract_errors(d3, D3_PHASE))
        errors.extend(d3_binding_errors(d2_ref, d2, d3))
        if d2.get("ekf", {}).get("evidence", {}).get(
            "fresh_gate_b_report_sha256"
        ) != b2_ref.get("sha256"):
            errors.append(f"F0 {chain_id} D2 authenticated different Gate B bytes")
        for report in (d2, d3):
            errors.extend(same_provenance(b2, report, PROVENANCE_FIELDS))
        chain_session = b2.get("provenance", {}).get("session_id")
        if session_id is None:
            session_id = chain_session
        elif chain_session != session_id:
            errors.append("F0 recorded chains do not share one session")
        if b2.get("provenance", {}).get("chain_id") != chain_id:
            errors.append(f"F0 {chain_id} provenance changed")
    return errors


def closeout_errors(manifest: dict[str, Any]) -> list[str]:
    closeout = manifest.get("closeout")
    if closeout is None:
        return []
    if manifest.get("evidence") is None:
        return ["F0 closeout cannot precede controlled evidence"]
    if not isinstance(closeout, dict) or set(closeout) != {"selected_ci", "full_ci"}:
        return ["F0 closeout record is malformed"]
    errors: list[str] = []
    for name in ("selected_ci", "full_ci"):
        run = closeout.get(name)
        if not isinstance(run, dict) or set(run) != {
            "run_id",
            "head_sha",
            "conclusion",
            "url",
        }:
            errors.append(f"F0 {name} closeout is malformed")
            continue
        if not isinstance(run.get("run_id"), int) or isinstance(run.get("run_id"), bool):
            errors.append(f"F0 {name} run ID is invalid")
        validate_git_oid(run.get("head_sha"), f"F0 {name} head", errors)
        if run.get("conclusion") != "success":
            errors.append(f"F0 {name} did not succeed")
        if not isinstance(run.get("url"), str) or not run["url"].startswith(
            "https://github.com/mech-lang/mech/actions/runs/"
        ):
            errors.append(f"F0 {name} URL is invalid")
    selected, full = closeout.get("selected_ci"), closeout.get("full_ci")
    if isinstance(selected, dict) and isinstance(full, dict) and selected.get(
        "head_sha"
    ) != full.get("head_sha"):
        errors.append("F0 selected and full CI did not validate one exact head")
    return errors


def validate(manifest: dict[str, Any], root: Path = ROOT) -> list[str]:
    errors = protocol_errors(manifest, root)
    if not errors:
        errors.extend(evidence_errors(manifest, root))
        errors.extend(closeout_errors(manifest))
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=EVIDENCE_MANIFEST)
    args = parser.parse_args(argv)
    path = args.manifest if args.manifest.is_absolute() else ROOT / args.manifest
    try:
        errors = validate(load_json(path))
    except EvidenceError as error:
        errors = [str(error)]
    if errors:
        print("F0 evidence contract failed:", file=sys.stderr)
        print(*(f"  {error}" for error in errors), sep="\n", file=sys.stderr)
        return 1
    print("F0 evidence contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
