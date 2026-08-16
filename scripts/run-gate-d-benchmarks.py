#!/usr/bin/env python3
"""Run and summarize the controlled Gate D evidence lanes."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import statistics
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))
from d2_historical_evidence import D2_HEAD, run_historical_d2_fixture


ROOT = Path(__file__).resolve().parents[1]
D1_HEAD = "7ff20887ea2d267b790917608c4bc8826b031762"
TURNS = 4_096
THRESHOLDS = {
    "nbody_source_bytecode_ratio_max": 1.03,
    "nbody_resident_raw_ratio_max": 1.50,
    "nbody_legacy_gap_closure_min": 0.75,
    "nbody_history_ratio_max": 1.05,
    "nbody_high_epoch_ratio_max": 1.05,
    "ekf_source_bytecode_ratio_max": 1.03,
    "ekf_complete_d1_ratio_max": 1.20,
    "ekf_kernel_d1_ratio_max": 1.20,
}
GATE_D_SAMPLE_PROTOCOL = {
    "samples": 10,
    "turns_per_sample": TURNS,
    "profile": "release",
}
HISTORICAL_REPLAY_MARKER = "GATE_D_HISTORICAL_REPLAY"


def command(*args: str) -> str:
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def parse_fields(line: str) -> dict[str, str]:
    return dict(field.split("=", 1) for field in line.split()[1:])


def summarize(values: list[float]) -> dict[str, float | int]:
    ordered = sorted(values)
    p95 = ordered[max(0, math.ceil(len(ordered) * 0.95) - 1)]
    return {
        "samples": len(ordered),
        "median_ns": statistics.median(ordered),
        "p95_ns": p95,
        "min_ns": ordered[0],
        "max_ns": ordered[-1],
    }


def d2_protocol_rows(
    raw: str,
) -> tuple[dict[str, list[dict[str, str]]], dict[str, list[dict[str, str]]], int]:
    lane_rows: dict[str, list[dict[str, str]]] = {}
    cold_rows: dict[str, list[dict[str, str]]] = {}
    structural_count = 0
    for line in raw.splitlines():
        if line.startswith("GATE_D_SAMPLE "):
            fields = parse_fields(line)
            lane_rows.setdefault(fields["lane"], []).append(fields)
        elif line.startswith("GATE_D_COLD "):
            fields = parse_fields(line)
            cold_rows.setdefault(fields["phase"], []).append(fields)
        elif line.startswith("GATE_D_STRUCTURAL "):
            structural_count += 1
    return lane_rows, cold_rows, structural_count


def validate_d2_sample_protocol(fresh: str, historical: str) -> None:
    fresh_lanes, fresh_cold, fresh_structural = d2_protocol_rows(fresh)
    historical_lanes, historical_cold, historical_structural = d2_protocol_rows(
        historical
    )
    required_fresh_lanes = {
        "nbody-raw-rust",
        "nbody-resident-source",
        "nbody-resident-bytecode",
        "nbody-resident-kernel-source",
        "nbody-resident-kernel-bytecode",
        "nbody-resident-source-history-1k",
        "nbody-resident-source-history-100k",
        "nbody-resident-source-high-epoch",
    }
    required_historical_lanes = required_fresh_lanes | {"nbody-legacy-mech"}
    required_cold = {
        "source-compilation-and-initial-encoding",
        "bytecode-encoding",
        "bytecode-decoding",
        "artifact-admission-and-activation",
    }
    if set(fresh_lanes) != required_fresh_lanes or set(fresh_cold) != required_cold:
        raise ValueError("fresh Gate D2 sample lane set changed")
    if set(historical_lanes) != required_historical_lanes or set(
        historical_cold
    ) != required_cold:
        raise ValueError("historical Gate D2 sample lane set changed")
    if fresh_structural != 1 or historical_structural != 1:
        raise ValueError("each Gate D2 replay must contain one structural record")
    expected_samples = list(range(GATE_D_SAMPLE_PROTOCOL["samples"]))
    for source, rows_by_lane in (
        ("fresh", fresh_lanes),
        ("historical", historical_lanes),
    ):
        for lane, rows in rows_by_lane.items():
            if sorted(int(row["sample"]) for row in rows) != expected_samples:
                raise ValueError(f"Gate D2 {source} {lane} sample identities changed")
            if any(int(row["turns"]) != TURNS for row in rows):
                raise ValueError(f"Gate D2 {source} {lane} turn count changed")
    for source, rows_by_phase in (
        ("fresh", fresh_cold),
        ("historical", historical_cold),
    ):
        for phase, rows in rows_by_phase.items():
            if sorted(int(row["sample"]) for row in rows) != expected_samples:
                raise ValueError(
                    f"Gate D2 {source} {phase} cold sample identities changed"
                )


def d2_measurement_raw(fresh: str, historical: str) -> str:
    legacy = [
        line
        for line in historical.splitlines()
        if line.startswith("GATE_D_SAMPLE ")
        and parse_fields(line).get("lane") == "nbody-legacy-mech"
    ]
    return fresh.rstrip() + "\n" + "\n".join(legacy) + "\n"


def load_raw(path: Path | None) -> str:
    if path is not None:
        return path.read_text(encoding="utf-8")
    process = subprocess.run(
        [
            "cargo",
            "+nightly-2026-03-03",
            "run",
            "--release",
            "--offline",
            "--manifest-path",
            "tests/fixtures/d2-contract-generator/Cargo.toml",
            "--target-dir",
            "target/gate-d2-generator",
            "--",
            "--gate-d-benchmark",
        ],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    )
    return process.stdout


def load_d3_raw(path: Path | None) -> str:
    if path is not None:
        return path.read_text(encoding="utf-8")
    process = subprocess.run(
        [
            "cargo",
            "+nightly-2026-03-03",
            "test",
            "--locked",
            "--release",
            "-p",
            "mech-runtime",
            "--no-default-features",
            "--features",
            "source_default,resident-routing-source,runtime_bench_gate_d3",
            "--test",
            "resident_external_gate_d3",
            "--",
            "--nocapture",
        ],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    )
    return process.stdout


def parse_json_lines(raw: str, prefix: str) -> list[dict]:
    return [
        json.loads(line.removeprefix(prefix))
        for line in raw.splitlines()
        if line.startswith(prefix)
    ]


def exact_field(rows: list[dict], field: str):
    values = {json.dumps(row[field], sort_keys=True) for row in rows}
    if len(values) != 1:
        raise ValueError(f"D3 {field} is not exact across samples: {sorted(values)}")
    return rows[0][field]


def exact_json_integer(value: object, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError(f"{label} must be an exact JSON integer")
    return value


def validate_d3_control_protocol(controls: list[dict]) -> None:
    required_lanes = {"history-0", "history-1k", "history-100k", "high-epoch"}
    if {row.get("lane") for row in controls} != required_lanes:
        raise ValueError("D3 control lane set changed")
    expected_samples = list(range(3))
    for lane in sorted(required_lanes):
        rows = [row for row in controls if row.get("lane") == lane]
        try:
            samples = sorted(
                exact_json_integer(row["sample"], f"D3 {lane} sample")
                for row in rows
            )
            turns = [
                exact_json_integer(row["turns"], f"D3 {lane} turns")
                for row in rows
            ]
        except KeyError as error:
            raise ValueError(f"D3 {lane} control sample is malformed") from error
        if samples != expected_samples:
            raise ValueError(f"D3 {lane} control sample identities changed")
        if any(turn != TURNS for turn in turns):
            raise ValueError(f"D3 {lane} control turn count changed")


def d3_report(raw: str, semantic_commit: str) -> dict:
    samples = parse_json_lines(raw, "GATE_D3_SAMPLE ")
    replays = parse_json_lines(raw, "GATE_D3_REPLAY ")
    controls = parse_json_lines(raw, "GATE_D3_CONTROL ")
    structural_rows = parse_json_lines(raw, "GATE_D3_STRUCTURAL ")
    if len(samples) != 40 or len(replays) != 2 or len(controls) != 12:
        raise ValueError(
            "incomplete D3 output: "
            f"samples={len(samples)} replays={len(replays)} controls={len(controls)}"
        )
    validate_d3_control_protocol(controls)
    if len(structural_rows) != 1:
        raise ValueError(f"expected one D3 structural record, found {len(structural_rows)}")
    structural = structural_rows[0]

    fixtures: dict[str, dict] = {}
    source_bytecode_ratios = []
    all_exact = True
    all_reads_exact = True
    all_receipts_exact = True
    all_publications_exact = True
    all_outbox_exact = True
    for fixture in ("effect", "transactional"):
        artifact_rows = {
            artifact: [
                row
                for row in samples
                if row["fixture"] == fixture and row["artifact"] == artifact
            ]
            for artifact in ("source", "bytecode")
        }
        if any(
            len(rows) != GATE_D_SAMPLE_PROTOCOL["samples"]
            for rows in artifact_rows.values()
        ):
            raise ValueError(f"D3 {fixture} does not have ten source/bytecode samples")
        for artifact, rows in artifact_rows.items():
            try:
                samples_for_artifact = sorted(
                    exact_json_integer(
                        row["sample"], f"D3 {fixture}/{artifact} sample"
                    )
                    for row in rows
                )
                turns_for_artifact = [
                    exact_json_integer(
                        row["turns"], f"D3 {fixture}/{artifact} turns"
                    )
                    for row in rows
                ]
            except KeyError as error:
                raise ValueError(
                    f"D3 {fixture}/{artifact} sample is malformed"
                ) from error
            if samples_for_artifact != list(
                range(GATE_D_SAMPLE_PROTOCOL["samples"])
            ):
                raise ValueError(
                    f"D3 {fixture}/{artifact} sample identities changed"
                )
            if any(turn != TURNS for turn in turns_for_artifact):
                raise ValueError(f"D3 {fixture}/{artifact} turn count changed")
        lanes = {}
        for artifact, rows in artifact_rows.items():
            lanes[artifact] = {
                "timing": summarize([row["elapsed_ns"] / row["turns"] for row in rows]),
                "state_hash": exact_field(rows, "state_hash"),
                "receipt_hash": exact_field(rows, "receipt_hash"),
                "effect_batch_hash": exact_field(rows, "effect_batch_hash"),
                "effect_id_hash": exact_field(rows, "effect_id_hash"),
                "idempotency_key_hash": exact_field(rows, "idempotency_key_hash"),
                "provider_reads_per_turn": exact_field(rows, "provider_reads") / TURNS,
                "receipt_appends_per_turn": exact_field(rows, "receipt_appends") / TURNS,
                "ordinary_outbox_appends_per_turn": exact_field(
                    rows, "ordinary_outbox_appends"
                )
                / TURNS,
                "publication_stores_per_turn": exact_field(rows, "publication_stores")
                / TURNS,
                "candidate_allocations": exact_field(rows, "candidate_allocations"),
            }
        source = lanes["source"]
        bytecode = lanes["bytecode"]
        ratio = max(source["timing"]["median_ns"], bytecode["timing"]["median_ns"]) / min(
            source["timing"]["median_ns"], bytecode["timing"]["median_ns"]
        )
        source_bytecode_ratios.append(ratio)
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
        all_exact &= exact
        all_reads_exact &= source["provider_reads_per_turn"] == 1
        all_receipts_exact &= source["receipt_appends_per_turn"] == 1
        all_publications_exact &= source["publication_stores_per_turn"] == 1
        all_outbox_exact &= source["ordinary_outbox_appends_per_turn"] == (
            1 if fixture == "effect" else 0
        )
        replay = next(row for row in replays if row["fixture"] == fixture)
        fixtures[fixture] = {
            "lanes": lanes,
            "source_bytecode_ratio": ratio,
            "source_bytecode_exact": exact,
            "replay": {
                **replay,
                "state_hash_exact": replay["state_hash"] == source["state_hash"],
                "receipt_hash_exact": replay["receipt_hash"] == source["receipt_hash"],
                "effect_batch_hash_exact": replay["effect_batch_hash"]
                == source["effect_batch_hash"],
                "effect_id_hash_exact": replay["effect_id_hash"] == source["effect_id_hash"],
                "idempotency_key_hash_exact": replay["idempotency_key_hash"]
                == source["idempotency_key_hash"],
            },
        }

    control_timings = {
        lane: summarize(
            [row["elapsed_ns"] / row["turns"] for row in controls if row["lane"] == lane]
        )
        for lane in ("history-0", "history-1k", "history-100k", "high-epoch")
    }
    control_base = control_timings["history-0"]["median_ns"]
    control_ratios = {
        "history_1k": control_timings["history-1k"]["median_ns"] / control_base,
        "history_100k": control_timings["history-100k"]["median_ns"] / control_base,
        "high_epoch": control_timings["high-epoch"]["median_ns"] / control_base,
    }

    d2_frozen = json.loads(
        command("git", "show", f"{D2_HEAD}:benchmarks/runtime/gate-b/b2-resident-turn.json")
    )
    d2_current = json.loads(
        (ROOT / "benchmarks/runtime/gate-b/b2-resident-turn.json").read_text(encoding="utf-8")
    )
    d2_frozen_source = select_lane(d2_frozen, "mech-resident-artifact-source")
    d2_current_source = select_lane(d2_current, "mech-resident-artifact-source")
    d2_regression = (
        d2_current_source["timing"]["median_ns_per_turn"]
        / d2_frozen_source["timing"]["median_ns_per_turn"]
    )
    thresholds = {
        "d2_pure_complete_turn_regression_max": 1.05,
        "d3_source_bytecode_ratio_max": 1.03,
        "history_ratio_max": 1.05,
        "high_epoch_ratio_max": 1.05,
    }
    replay_exact = all(
        fixture["replay"][field]
        for fixture in fixtures.values()
        for field in (
            "state_hash_exact",
            "receipt_hash_exact",
            "effect_batch_hash_exact",
            "effect_id_hash_exact",
            "idempotency_key_hash_exact",
        )
    )
    gates = {
        "d2_pure_regression": d2_regression <= thresholds["d2_pure_complete_turn_regression_max"],
        "source_bytecode_ratio": max(source_bytecode_ratios)
        <= thresholds["d3_source_bytecode_ratio_max"],
        "source_bytecode_exact": all_exact,
        "candidate_allocations": structural["candidate_allocations"] == 0,
        "accepted_publication_store": structural["publication_stores_per_accepted_turn"] == 1
        and all_publications_exact,
        "rejected_publication_store": structural["publication_stores_per_rejected_turn"] == 0
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
        "replay_reads_zero": all(fixture["replay"]["provider_reads"] == 0 for fixture in fixtures.values()),
        "replay_exact": replay_exact,
        "history_independent": control_ratios["history_1k"] <= thresholds["history_ratio_max"]
        and control_ratios["history_100k"] <= thresholds["history_ratio_max"],
        "epoch_independent": control_ratios["high_epoch"] <= thresholds["high_epoch_ratio_max"],
        "no_commit_runtime": structural["commit_runtime_calls"] == 0,
        "no_legacy_journal": structural["legacy_journal_captures"] == 0,
        "no_runtime_execution_transaction": structural[
            "runtime_execution_transaction_constructions"
        ]
        == 0,
    }
    return {
        "schema_version": 1,
        "gate": "D",
        "phase": "D3-resident-external",
        "semantic_commit": semantic_commit,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "machine": {
            "platform": platform.platform(),
            "architecture": platform.machine(),
            "processor": platform.processor(),
        },
        "sample_protocol": GATE_D_SAMPLE_PROTOCOL,
        "thresholds": thresholds,
        "d2_pure": {
            "frozen_semantic_head": D2_HEAD,
            "frozen_ns_per_turn": d2_frozen_source["timing"]["median_ns_per_turn"],
            "current_ns_per_turn": d2_current_source["timing"]["median_ns_per_turn"],
            "regression_ratio": d2_regression,
            "nbody_report": "benchmarks/runtime/gate-d/d2-resident-nbody.json",
        },
        "fixtures": fixtures,
        "controls": {"timings": control_timings, "ratios": control_ratios},
        "structural": structural,
        "hard_gates": gates,
        "decision": "Pass" if all(gates.values()) else "Fail",
    }


def select_lane(report: dict, name: str) -> dict:
    matches = [
        lane
        for lane in report["lanes"]
        if lane["lane"] == name
        and lane["instances"] == 1
        and lane.get("retained_history", 0) == 0
        and lane.get("next_epoch", 1) == 1
    ]
    if len(matches) != 1:
        raise ValueError(f"expected one {name} lane, found {len(matches)}")
    return matches[0]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--phase",
        choices=("D2-resident-nbody", "D3-resident-external", "D3-external-turn"),
        default="D2-resident-nbody",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--raw-input", type=Path)
    parser.add_argument("--raw-output", type=Path)
    parser.add_argument("--semantic-commit")
    args = parser.parse_args()

    semantic_commit = args.semantic_commit or command("git", "rev-parse", "HEAD")
    if args.phase in {"D3-resident-external", "D3-external-turn"}:
        report = d3_report(load_d3_raw(args.raw_input), semantic_commit)
        rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
        output = args.output if args.output.is_absolute() else ROOT / args.output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
        display = output.relative_to(ROOT) if output.is_relative_to(ROOT) else output
        print(f"wrote {display}: {report['decision']}")
        return 0 if report["decision"] == "Pass" else 3
    if args.raw_input is None:
        fresh_raw = load_raw(None)
        historical_raw = run_historical_d2_fixture(
            "--gate-d-benchmark", release=True
        )
        evidence_raw = (
            fresh_raw.rstrip()
            + f"\n{HISTORICAL_REPLAY_MARKER}\n"
            + historical_raw.lstrip()
        )
    else:
        evidence_raw = load_raw(args.raw_input)
        try:
            fresh_raw, historical_raw = evidence_raw.split(
                f"\n{HISTORICAL_REPLAY_MARKER}\n", 1
            )
        except ValueError as error:
            raise ValueError(
                "Gate D2 raw input has no historical replay boundary"
            ) from error
    validate_d2_sample_protocol(fresh_raw, historical_raw)
    raw = d2_measurement_raw(fresh_raw, historical_raw)
    if args.raw_output is not None:
        raw_output = (
            args.raw_output
            if args.raw_output.is_absolute()
            else ROOT / args.raw_output
        )
        raw_output.parent.mkdir(parents=True, exist_ok=True)
        raw_output.write_text(evidence_raw, encoding="utf-8")
    lane_samples: dict[str, list[float]] = {}
    lane_allocations: dict[str, set[int]] = {}
    cold_samples: dict[str, list[float]] = {}
    structural: dict[str, str] | None = None
    for line in raw.splitlines():
        if line.startswith("GATE_D_SAMPLE "):
            fields = parse_fields(line)
            lane_samples.setdefault(fields["lane"], []).append(
                int(fields["elapsed_ns"]) / int(fields["turns"])
            )
            lane_allocations.setdefault(fields["lane"], set()).add(
                int(fields["allocation_count"])
            )
        elif line.startswith("GATE_D_STRUCTURAL "):
            structural = parse_fields(line)
        elif line.startswith("GATE_D_COLD "):
            fields = parse_fields(line)
            cold_samples.setdefault(fields["phase"], []).append(int(fields["elapsed_ns"]))
    required = {
        "nbody-raw-rust",
        "nbody-legacy-mech",
        "nbody-resident-source",
        "nbody-resident-bytecode",
        "nbody-resident-kernel-source",
        "nbody-resident-kernel-bytecode",
        "nbody-resident-source-history-1k",
        "nbody-resident-source-history-100k",
        "nbody-resident-source-high-epoch",
    }
    if required - lane_samples.keys() or structural is None:
        raise ValueError(f"incomplete Gate D raw output: {sorted(required - lane_samples.keys())}")

    lanes = {
        name: {
            **summarize(values),
            "turns_per_sample": TURNS,
            "allocation_counts": sorted(lane_allocations[name]),
        }
        for name, values in sorted(lane_samples.items())
    }
    raw_median = lanes["nbody-raw-rust"]["median_ns"]
    legacy_median = lanes["nbody-legacy-mech"]["median_ns"]
    source_median = lanes["nbody-resident-source"]["median_ns"]
    bytecode_median = lanes["nbody-resident-bytecode"]["median_ns"]
    kernel_source_median = lanes["nbody-resident-kernel-source"]["median_ns"]
    source_bytecode_ratio = max(source_median, bytecode_median) / min(
        source_median, bytecode_median
    )
    resident_raw_ratio = source_median / raw_median
    legacy_denominator = legacy_median - raw_median
    legacy_gap_closure = (legacy_median - source_median) / legacy_denominator
    history_1k_ratio = lanes["nbody-resident-source-history-1k"]["median_ns"] / source_median
    history_100k_ratio = lanes["nbody-resident-source-history-100k"]["median_ns"] / source_median
    high_epoch_ratio = lanes["nbody-resident-source-high-epoch"]["median_ns"] / source_median

    execution = json.loads(
        (ROOT / "tests/architecture/resident-activation/d2-nbody-execution-v1.json").read_text()
    )
    layout = json.loads(
        (ROOT / "tests/architecture/resident-activation/d2-nbody-layout-v1.json").read_text()
    )
    integer_structural = {
        key: int(value)
        for key, value in structural.items()
        if value not in {"true", "false"}
    }
    boolean_structural = {
        key: value == "true"
        for key, value in structural.items()
        if value in {"true", "false"}
    }
    nbody_gates = {
        "trajectory_exact": all(
            execution[key]
            for key in (
                "source_bytecode_trajectory_equal",
                "raw_rust_trajectory_equal",
                "legacy_trajectory_equal",
            )
        ),
        "source_bytecode_ratio": source_bytecode_ratio <= THRESHOLDS["nbody_source_bytecode_ratio_max"],
        "resident_raw_ratio": resident_raw_ratio <= THRESHOLDS["nbody_resident_raw_ratio_max"],
        "legacy_gap_closure": legacy_gap_closure >= THRESHOLDS["nbody_legacy_gap_closure_min"],
        "zero_allocation": lane_allocations["nbody-resident-source"] == {0}
        and lane_allocations["nbody-resident-bytecode"] == {0},
        "candidate_bytes": integer_structural["candidate_bytes"] == 480 == layout["candidate_bytes"],
        "candidate_seed_bytes": integer_structural["candidate_seed_bytes"] == 480,
        "candidate_materialized_bytes": integer_structural["candidate_materialized_bytes"] == 480,
        "published_copy_bytes": integer_structural["published_buffer_copy_bytes"] == 0,
        "publication_stores": integer_structural["publication_store_count"] == 1,
        "record_preparations": integer_structural["record_preparation_count"] == 1,
        "record_appends": integer_structural["record_append_count"] == 1,
        "post_publication_append_infallible": boolean_structural["post_publication_append_infallible"],
        "history_independent": history_1k_ratio <= THRESHOLDS["nbody_history_ratio_max"]
        and history_100k_ratio <= THRESHOLDS["nbody_history_ratio_max"],
        "epoch_magnitude_independent": high_epoch_ratio <= THRESHOLDS["nbody_high_epoch_ratio_max"],
        "no_commit_runtime": integer_structural["commit_runtime_call_count"] == 0,
        "no_legacy_journal": integer_structural["legacy_journal_capture_count"] == 0,
    }

    d1_report = json.loads(
        command("git", "show", f"{D1_HEAD}:benchmarks/runtime/gate-b/b2-resident-turn.json")
    )
    d2_report = json.loads(
        (ROOT / "benchmarks/runtime/gate-b/b2-resident-turn.json").read_text()
    )
    d1_complete = select_lane(d1_report, "mech-resident-artifact-source")["timing"]["median_ns_per_turn"]
    d1_kernel = select_lane(d1_report, "mech-resident-artifact-kernel-source")["timing"]["median_ns_per_turn"]
    d2_source = select_lane(d2_report, "mech-resident-artifact-source")["timing"]["median_ns_per_turn"]
    d2_bytecode = select_lane(d2_report, "mech-resident-artifact-bytecode")["timing"]["median_ns_per_turn"]
    d2_kernel_source = select_lane(d2_report, "mech-resident-artifact-kernel-source")["timing"]["median_ns_per_turn"]
    d2_kernel_bytecode = select_lane(d2_report, "mech-resident-artifact-kernel-bytecode")["timing"]["median_ns_per_turn"]
    gate_b_control = select_lane(d2_report, "mech-resident-turn")["timing"]["median_ns_per_turn"]
    ekf_source_bytecode_ratio = max(d2_source, d2_bytecode) / min(d2_source, d2_bytecode)
    ekf_complete_d1_ratio = d2_source / d1_complete
    ekf_kernel_d1_ratio = d2_kernel_source / d1_kernel
    ekf_gates = {
        "trajectory_exact": select_lane(d2_report, "mech-resident-artifact-source")["correctness"]
        and select_lane(d2_report, "mech-resident-artifact-bytecode")["correctness"],
        "source_bytecode_ratio": ekf_source_bytecode_ratio <= THRESHOLDS["ekf_source_bytecode_ratio_max"],
        "complete_d1_ratio": ekf_complete_d1_ratio <= THRESHOLDS["ekf_complete_d1_ratio_max"],
        "kernel_d1_ratio": ekf_kernel_d1_ratio <= THRESHOLDS["ekf_kernel_d1_ratio_max"],
        "zero_allocation": select_lane(d2_report, "mech-resident-artifact-source")["allocation"]["episode_allocation_count"] == 0,
        "candidate_bytes": select_lane(d2_report, "mech-resident-artifact-source")["structural"]["candidate_written_bytes"] == 96,
        "candidate_seed_bytes": select_lane(d2_report, "mech-resident-artifact-source")["structural"]["candidate_seed_bytes"] == 0,
        "publication_stores": select_lane(d2_report, "mech-resident-artifact-source")["structural"]["publication_store_count"] == 1,
    }

    report = {
        "schema_version": 1,
        "gate": "D",
        "phase": "D2-resident-nbody",
        "semantic_commit": semantic_commit,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "machine": {
            "platform": platform.platform(),
            "architecture": platform.machine(),
            "processor": platform.processor(),
        },
        "sample_protocol": GATE_D_SAMPLE_PROTOCOL,
        "thresholds": THRESHOLDS,
        "nbody": {
            "lanes": lanes,
            "structural": {**integer_structural, **boolean_structural},
            "correctness": execution,
            "ratios": {
                "source_bytecode": source_bytecode_ratio,
                "resident_raw": resident_raw_ratio,
                "kernel_raw": kernel_source_median / raw_median,
                "complete_turn_tax_ns": source_median - kernel_source_median,
                "legacy_gap_closure": legacy_gap_closure,
                "history_1k": history_1k_ratio,
                "history_100k": history_100k_ratio,
                "high_epoch": high_epoch_ratio,
            },
            "hard_gates": nbody_gates,
            "decision": "Pass" if all(nbody_gates.values()) else "Fail",
        },
        "ekf": {
            "evidence": {
                "d1_head": D1_HEAD,
                "d1_report_commit": d1_report["git_commit"],
                "d2_report_commit": d2_report["git_commit"],
            },
            "lanes": {
                "d1_complete": d1_complete,
                "d1_kernel": d1_kernel,
                "d2_source": d2_source,
                "d2_bytecode": d2_bytecode,
                "d2_kernel_source": d2_kernel_source,
                "d2_kernel_bytecode": d2_kernel_bytecode,
                "gate_b_control": gate_b_control,
            },
            "ratios": {
                "source_bytecode": ekf_source_bytecode_ratio,
                "complete_d1": ekf_complete_d1_ratio,
                "kernel_d1": ekf_kernel_d1_ratio,
            },
            "hard_gates": ekf_gates,
            "decision": "Pass" if all(ekf_gates.values()) else "Fail",
        },
        "cold_path": {phase: summarize(values) for phase, values in sorted(cold_samples.items())},
    }
    report["decision"] = (
        "Pass" if report["nbody"]["decision"] == report["ekf"]["decision"] == "Pass" else "Fail"
    )
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    output = args.output if args.output.is_absolute() else ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(rendered, encoding="utf-8")
    print(f"wrote {output.relative_to(ROOT)}: {report['decision']}")
    return 0 if report["decision"] == "Pass" else 3


if __name__ == "__main__":
    raise SystemExit(main())
