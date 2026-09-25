#!/usr/bin/env python3
"""Read-only audit of a completed 500000-filter, 40-turn, n=10 campaign.

Checks the evidence's internal consistency without loading executables or
rerunning benchmarks. Recorded hashes identify artifacts; this does not attest
that a binary was built from the recorded source. Checksums are diagnostics,
not a substitute for the recorded component-wise numerical preflight.
"""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
from pathlib import Path
import random
import re
import statistics
import sys

BACKENDS = ("evaluator", "scalar-jit", "simd-aot", "simd-jit-8w", "metal")
MODES = ("checked", "unchecked")
CASES = tuple((backend, mode) for backend in BACKENDS for mode in MODES)
GUARDS = ["finite-candidate!", "positive-covariance!", "symmetric-covariance!"]
INSTANCES, TURNS, WARMUP, SAMPLES, VALIDATION_INSTANCES = 500_000, 40, 5, 10, 4092


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def number(value: object, label: str) -> float:
    require(type(value) in (int, float) and math.isfinite(value), f"{label}: non-finite/non-numeric")
    return float(value)


def close(actual: object, expected: float, label: str) -> None:
    require(math.isclose(number(actual, label), expected, rel_tol=1e-12, abs_tol=1e-12),
            f"{label}: expected {expected!r}, got {actual!r}")


def sha256(value: object, label: str) -> str:
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value) is not None,
            f"{label}: invalid SHA-256")
    return value


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise AuditError(f"non-finite JSON literal: {value}")


def decode(text: str) -> dict:
    return json.loads(text, object_pairs_hook=unique_object, parse_constant=reject_constant)


def audit_provenance(provenance: dict) -> dict:
    binary_hash = sha256(provenance["binary_sha256"], "binary")
    require(isinstance(provenance["binary"], str) and bool(provenance["binary"]), "missing binary path")
    revision_record = provenance["git_head"]
    require(revision_record["returncode"] == 0, "Git revision capture failed")
    revision = revision_record["stdout"].strip()
    require(re.fullmatch(r"[0-9a-f]{40}", revision) is not None, "invalid source Git revision")
    hashes = provenance["source_files_sha256"]
    snapshots = provenance["changed_and_untracked_source_snapshots"]
    required = {"examples/embedded_ekf/ekf.mec", "benchmarks/iros-2026/mech_backend_pairs.rs",
                "benchmarks/iros-2026/measure_mech_backend_pairs.py", "Cargo.toml", "Cargo.lock"}
    require(required <= hashes.keys(), "required source/manifests missing from provenance")
    for name, digest in hashes.items():
        sha256(digest, f"source {name}")
        snapshot = snapshots[name]
        require(snapshot["sha256"] == digest, f"source {name}: conflicting hash records")
        computed = hashlib.sha256(snapshot["content_utf8"].encode("utf-8")).hexdigest()
        require(computed == digest, f"source {name}: embedded content does not match hash")
    live_snapshots = {name for name, item in snapshots.items() if not item.get("deleted", False)}
    require(live_snapshots == set(hashes), "source snapshot/hash inventory differs")
    diff_hash = sha256(provenance["tracked_diff_sha256"], "tracked diff")
    require(provenance["tracked_diff_returncode"] == 0, "tracked diff capture failed")
    require(hashlib.sha256(provenance["tracked_diff_content"].encode("utf-8")).hexdigest() == diff_hash,
            "tracked diff content does not match hash")
    return {"git_revision": revision, "binary_sha256": binary_hash,
            "source_hashes_verified": len(hashes), "tracked_diff_sha256": diff_hash}


def audit_measurement(record: dict, provenance: dict, preflight: bool) -> tuple[dict, dict]:
    backend, mode = record["backend"], record["mode"]
    label = f"{backend}/{mode} {'preflight' if preflight else 'sample'}"
    require((backend, mode) in CASES, f"{label}: unknown case")
    require(record["status"] == "ok" and record["returncode"] == 0, f"{label}: unsuccessful process")
    measurement = record["measurement"]
    require(decode(record["stdout"]) == measurement, f"{label}: stdout differs from saved measurement")
    instances = VALIDATION_INSTANCES if preflight else INSTANCES
    expected = {
        "schema_version": 1, "backend": backend, "mode": mode, "instances": instances,
        "turns": TURNS, "total_filter_turns": instances * TURNS,
        "attempted_turns": WARMUP + TURNS, "faults": 0, "warmup_turns": WARMUP,
        "warmup_in_measured_session": True, "measured_start_after_turn": WARMUP,
        "workers": 8 if backend == "simd-jit-8w" else 1,
        "publication": "one completed publication per turn", "state_components": 12 * instances,
        "source": "examples/embedded_ekf/ekf.mec",
        "removed_source_guards": [] if mode == "checked" else GUARDS,
    }
    for key, value in expected.items():
        require(measurement.get(key) == value, f"{label}: invalid {key}")
    command = [provenance["binary"], "--backend", backend, "--mode", mode,
               "--instances", str(instances), "--turns", str(TURNS)]
    if preflight:
        command.append("--validate")
    require(record["command"] == command, f"{label}: command does not match provenance/case")
    elapsed = measurement["elapsed_ns"]
    require(type(elapsed) is int and elapsed > 0, f"{label}: invalid elapsed_ns")
    close(measurement["throughput_million_filter_turns_per_second"],
          instances * TURNS * 1000.0 / elapsed, f"{label} throughput")
    checksum = number(measurement["checksum"], f"{label} checksum")
    states = {}
    for state in measurement["state_summaries"]:
        slot, count = state["slot"], state["components"]
        require(type(slot) is int and slot >= 0 and slot not in states, f"{label}: invalid/duplicate slot")
        require(type(count) is int and count > 0, f"{label}: invalid state width")
        states[slot] = {"components": count, "sum_f64": number(state["sum_f64"], f"{label} state {slot}")}
    require(sorted(state["components"] for state in states.values()) == [3 * instances, 9 * instances],
            f"{label}: state/covariance component counts differ")
    close(checksum, sum(state["sum_f64"] for state in states.values()), f"{label} summed state checksum")
    if backend == "simd-aot":
        library = record["loaded_library"]
        sha256(library["sha256"], f"{label} loaded AOT library")
        require(type(library["size_bytes"]) is int and library["size_bytes"] > 0,
                f"{label}: invalid AOT size")
        require(bool(measurement["library_path"]) and bool(library["path"]), f"{label}: missing AOT path")
        require(Path(measurement["library_path"]).name == Path(library["path"]).name,
                f"{label}: AOT path/hash record mismatch")
    else:
        require(measurement["library_path"] is None and "loaded_library" not in record,
                f"{label}: unexpected AOT library")
    if preflight:
        validation = measurement["validation"]
        for key, value in {"passed": True, "instances": VALIDATION_INSTANCES,
                           "turns": WARMUP + TURNS, "reference": "scalar checked",
                           "compared_components": 12 * VALIDATION_INSTANCES}.items():
            require(validation.get(key) == value, f"{label}: invalid validation {key}")
        for key, expected_tolerance in (("absolute_tolerance", 2e-4), ("relative_tolerance", 1e-5)):
            require(math.isclose(number(validation[key], key), expected_tolerance, rel_tol=1e-7),
                    f"{label}: wrong {key}")
        ratio = number(validation["maximum_tolerance_ratio"], f"{label} tolerance ratio")
        require(0 <= ratio <= 1, f"{label}: component validation exceeded tolerance")
        require(number(validation["maximum_absolute_error"], f"{label} maximum error") >= 0,
                f"{label}: negative error")
        rollback = validation["nan_rollback"]
        if mode == "checked":
            for key, value in {"passed": True, "faults": 1, "attempted_turns": 1,
                               "fault_lane": 4091, "fault_attempted_turn": 1,
                               "injected_nan_lane": 4091, "all_state_unchanged": True,
                               "stage": "initial published state"}.items():
                require(rollback.get(key) == value, f"{label}: invalid NaN rollback {key}")
            require(type(rollback["fault_constraint_id"]) is int and rollback["fault_constraint_id"] >= 0,
                    f"{label}: missing observed fault constraint")
            require(rollback["fault_constraint_name"] in GUARDS, f"{label}: unexpected fault name")
        else:
            require(rollback is None, f"{label}: unchecked case claims NaN rejection")
    else:
        require(measurement["validation"] is None, f"{label}: preflight included in measured records")
    return measurement, states


def audit(document: dict) -> dict:
    require(document["status"] == "complete", "campaign is not complete; audit after collection finishes")
    for key, value in {"schema_version": 1, "instances": INSTANCES, "turns": TURNS,
                       "samples_per_case": SAMPLES, "preflight_only": False}.items():
        require(document.get(key) == value, f"unexpected campaign {key}")
    require(bool(document.get("finished_at")), "completed campaign lacks finished_at")
    provenance = audit_provenance(document["provenance"])
    records, preflight = document["records"], document["preflight_records"]
    require(len(records) == 100 and len(preflight) == 10, "expected 100 measured and 10 preflight records")
    require(Counter((r["backend"], r["mode"]) for r in records) == Counter({case: 10 for case in CASES}),
            "expected ten samples in every backend/mode case")
    require(Counter((r["backend"], r["mode"]) for r in preflight) == Counter(CASES),
            "expected one preflight for every backend/mode case")
    randomizer = random.Random(document["random_seed"])
    expected_schedule = []
    for _ in range(SAMPLES):
        order = list(CASES)
        randomizer.shuffle(order)
        expected_schedule.append([list(case) for case in order])
    require(document["schedule"] == expected_schedule, "recorded schedule differs from seeded shuffle")
    for record in preflight:
        audit_measurement(record, document["provenance"], True)
    grouped = defaultdict(list)
    state_groups = defaultdict(list)
    for index, record in enumerate(records):
        round_index, position = divmod(index, 10)
        case = (record["backend"], record["mode"])
        require(record["round"] == round_index + 1 and record["position"] == position + 1,
                "sample round/position is missing, repeated, or reordered")
        require(list(case) == expected_schedule[round_index][position], "sample deviates from recorded schedule")
        measurement, states = audit_measurement(record, document["provenance"], False)
        grouped[case].append(record)
        state_groups[case].append(states)
    warnings, maximum_checksum_span, maximum_state_sum_span = [], 0.0, 0.0
    aot_hashes = {}
    require(set(document["summary"]) == set(BACKENDS), "summary backend set differs")
    for case in CASES:
        backend, mode = case
        require(set(document["summary"][backend]) == set(MODES), f"{backend}: summary modes differ")
        samples = [r["measurement"]["throughput_million_filter_turns_per_second"] for r in grouped[case]]
        checksums = [r["measurement"]["checksum"] for r in grouped[case]]
        summary = document["summary"][backend][mode]
        median = statistics.median(samples)
        require(summary["n"] == SAMPLES, f"{case}: summary sample count differs")
        for key, value in {"median_million_filter_turns_per_second": median,
                           "mad_million_filter_turns_per_second": statistics.median(abs(x - median) for x in samples),
                           "minimum": min(samples), "maximum": max(samples)}.items():
            close(summary[key], value, f"{case} summary {key}")
        for key, values in {"samples_million_filter_turns_per_second": samples, "checksums": checksums,
                            "samples_elapsed_ns": [r["measurement"]["elapsed_ns"] for r in grouped[case]]}.items():
            require(summary[key] == values, f"{case}: summary {key} differs from raw records")
        checksum_span = max(checksums) - min(checksums)
        maximum_checksum_span = max(maximum_checksum_span, checksum_span)
        first_states = state_groups[case][0]
        require(all(set(states) == set(first_states) for states in state_groups[case]), f"{case}: state slots vary")
        for slot, baseline in first_states.items():
            require(all(states[slot]["components"] == baseline["components"] for states in state_groups[case]),
                    f"{case}: state widths vary")
            sums = [states[slot]["sum_f64"] for states in state_groups[case]]
            span = max(sums) - min(sums)
            maximum_state_sum_span = max(maximum_state_sum_span, span)
            if span:
                warnings.append({"case": list(case), "state_slot": slot, "nonrepeatable_sum_span": span})
        if checksum_span:
            warnings.append({"case": list(case), "nonrepeatable_checksum_span": checksum_span})
        if backend == "simd-aot":
            identities = {(r["loaded_library"]["sha256"], r["loaded_library"]["size_bytes"]) for r in grouped[case]}
            require(len(identities) == 1, f"{case}: loaded AOT library changed during measured samples")
            digest, size = identities.pop()
            aot_hashes[mode] = {"sha256": digest, "size_bytes": size}
    pair_diagnostics = {}
    for backend in BACKENDS:
        checked, unchecked = state_groups[(backend, "checked")], state_groups[(backend, "unchecked")]
        require(set(checked[0]) == set(unchecked[0]), f"{backend}: mode state slots differ")
        rows = []
        for slot in sorted(checked[0]):
            require(checked[0][slot]["components"] == unchecked[0][slot]["components"], f"{backend}: mode widths differ")
            left = statistics.median(states[slot]["sum_f64"] for states in checked)
            right = statistics.median(states[slot]["sum_f64"] for states in unchecked)
            rows.append({"slot": slot, "components": checked[0][slot]["components"],
                         "checked_median_sum": left, "unchecked_median_sum": right,
                         "absolute_difference": abs(right - left)})
        pair_diagnostics[backend] = rows
    return {"status": "passed", "measured_samples": 100, "cases": 10, "samples_per_case": 10,
            "preflight_samples": 10, "checked_nan_rollback_tests": 5,
            "preflight_components_compared_per_case": 12 * VALIDATION_INSTANCES,
            "checks": {"structure_and_schedule": True, "timing_and_workload": True,
                       "all_preflights": True, "summaries_recomputed": True,
                       "embedded_source_hashes": True, "per_mode_aot_identity": True},
            "provenance": provenance, "aot_libraries": aot_hashes,
            "maximum_within_case_checksum_mismatch": maximum_checksum_span,
            "maximum_within_case_state_sum_mismatch": maximum_state_sum_span,
            "warnings": warnings, "checked_unchecked_state_sum_diagnostics": pair_diagnostics,
            "diagnostic_scope": "State sums can hide compensating errors; component-wise preflight is the numerical correctness check. Recorded binary hashes are not live-file/build attestations."}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True, help="completed campaign JSON; never modified")
    args = parser.parse_args()
    try:
        result = audit(decode(args.input.read_text(encoding="utf-8")))
    except (AuditError, OSError, KeyError, TypeError, ValueError) as error:
        print(json.dumps({"status": "failed", "error": str(error)}, allow_nan=False), file=sys.stderr)
        raise SystemExit(1) from error
    print(json.dumps(result, indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
