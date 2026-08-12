#!/usr/bin/env python3
"""Run and summarize the controlled D2 Gate D evidence lanes."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import statistics
import subprocess
from datetime import datetime, timezone
from pathlib import Path


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
            "--",
            "--gate-d-benchmark",
        ],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    )
    return process.stdout


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
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--raw-input", type=Path)
    parser.add_argument("--semantic-commit")
    args = parser.parse_args()

    semantic_commit = args.semantic_commit or command("git", "rev-parse", "HEAD")
    raw = load_raw(args.raw_input)
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
        "sample_protocol": {"samples": 10, "turns_per_sample": TURNS, "profile": "release"},
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
