#!/usr/bin/env python3
"""Enforce the frozen Gate B workload, fairness, and B0 evidence contract."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import math
import re
import struct
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
GATE_B_DIR = ROOT / "benchmarks/runtime/gate-b"
B2_EVIDENCE_FLOOR = "d5e41f6fd43c9d21c5858d80dab50e7ce64e9a10"
TRACE_SHA256 = "ab901e1d115aa92166dc2a6d45a28732e6a548363b829997aa410ae4c2d77c8b"
REFERENCE_HASH = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758"
EPISODE_LENGTH = 4_096
SCALED_INSTANCES = (1, 8, 64)
THREAD_VARIABLES = (
    "OMP_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "BLIS_NUM_THREADS",
    "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
    "NUMEXPR_NUM_THREADS",
    "RAYON_NUM_THREADS",
)


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def commit_descends_from(
    commit: str,
    floor: str = B2_EVIDENCE_FLOOR,
    root: Path = ROOT,
) -> bool:
    """Return whether commit exists and is a descendant of the frozen B2 floor."""
    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", floor, commit],
        cwd=root,
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return result.returncode == 0


def import_roots(source: str) -> set[str]:
    tree = ast.parse(source)
    roots = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            roots.update(alias.name.split(".", 1)[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            roots.add(node.module.split(".", 1)[0])
    return roots


LaneKey = tuple[str, int, int, int]
D1_ARTIFACT_LANE_KEYS = frozenset(
    {
        ("mech-resident-artifact-source", 1, 0, 1),
        ("mech-resident-artifact-source", 1, 1_000, 1),
        ("mech-resident-artifact-source", 1, 100_000, 1),
        ("mech-resident-artifact-source", 1, 0, 1_000_000_001),
        ("mech-resident-artifact-bytecode", 1, 0, 1),
        ("mech-resident-artifact-kernel-source", 1, 0, 1),
        ("mech-resident-artifact-kernel-bytecode", 1, 0, 1),
    }
)


def required_lane_keys() -> set[LaneKey]:
    keys = {
        (lane, instances, 0, 1)
        for lane in (
            "rust-kernel",
            "rust-epoch",
            "mech-legacy-atomic",
            "numpy-persistent",
        )
        for instances in SCALED_INSTANCES
    }
    keys.update(
        {
            ("rust-epoch-full-write", 1, 0, 1),
            ("mech-legacy-atomic-full-write", 1, 0, 1),
        }
    )
    return keys


def required_phase_lane_keys(phase: str) -> set[LaneKey]:
    if phase not in {"B0-controls", "B1-resident-kernel", "B2-resident-turn"}:
        raise ValueError(f"unsupported Gate B report phase {phase!r}")
    keys = required_lane_keys()
    if phase in {"B1-resident-kernel", "B2-resident-turn"}:
        keys.update(
            ("mech-resident-kernel", instances, 0, 1)
            for instances in SCALED_INSTANCES
        )
        keys.add(("mech-resident-kernel-full-write", 1, 0, 1))
    if phase == "B2-resident-turn":
        keys.update(
            {
                ("mech-resident-scheduled", 1, 0, 1),
                ("mech-resident-turn", 1, 0, 1),
                ("mech-resident-turn", 1, 1_000, 1),
                ("mech-resident-turn", 1, 100_000, 1),
                ("mech-resident-turn", 1, 0, 1_000_000_001),
                ("mech-resident-turn-full-write", 1, 0, 1),
            }
        )
    return keys


def call_occurs_before(source: str, call: str, boundary: str) -> bool:
    call_position = source.find(f"{call}(")
    boundary_position = source.find(boundary)
    return (
        call_position >= 0
        and boundary_position >= 0
        and call_position < boundary_position
    )


def _schema_type_matches(value: Any, expected: str) -> bool:
    if expected == "null":
        return value is None
    if expected == "boolean":
        return isinstance(value, bool)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if expected == "string":
        return isinstance(value, str)
    if expected == "array":
        return isinstance(value, list)
    if expected == "object":
        return isinstance(value, dict)
    return True


def json_schema_errors(
    value: Any,
    schema: dict[str, Any] | bool,
    root_schema: dict[str, Any] | None = None,
    path: str = "$",
) -> list[str]:
    """Validate the JSON Schema vocabulary used by the frozen result schema."""
    if schema is False:
        return [f"{path} is prohibited by the Gate B result schema"]
    if schema is True:
        return []
    root_schema = schema if root_schema is None else root_schema
    if "$ref" in schema:
        reference = schema["$ref"]
        if not reference.startswith("#/"):
            return [f"{path} uses unsupported schema reference {reference}"]
        target: Any = root_schema
        for component in reference[2:].split("/"):
            target = target[component.replace("~1", "/").replace("~0", "~")]
        return json_schema_errors(value, target, root_schema, path)

    errors: list[str] = []
    expected_types = schema.get("type")
    if expected_types is not None:
        if isinstance(expected_types, str):
            expected_types = [expected_types]
        if not any(_schema_type_matches(value, expected) for expected in expected_types):
            return [f"{path} has the wrong type for the Gate B result schema"]
    if "const" in schema and value != schema["const"]:
        errors.append(f"{path} does not equal the schema constant")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path} is not one of the schema enum values")

    if isinstance(value, dict):
        properties = schema.get("properties", {})
        for required in schema.get("required", []):
            if required not in value:
                errors.append(f"{path} is missing schema-required property {required}")
        if schema.get("additionalProperties") is False:
            for key in value.keys() - properties.keys():
                errors.append(f"{path}.{key} is not allowed by the result schema")
        for key, child in value.items():
            if key in properties:
                errors.extend(
                    json_schema_errors(
                        child, properties[key], root_schema, f"{path}.{key}"
                    )
                )
    elif isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{path} has fewer items than the result schema requires")
        prefix = schema.get("prefixItems", [])
        for index, child in enumerate(value):
            child_schema = prefix[index] if index < len(prefix) else schema.get("items", {})
            errors.extend(
                json_schema_errors(child, child_schema, root_schema, f"{path}[{index}]")
            )
    elif isinstance(value, str):
        if len(value) < schema.get("minLength", 0):
            errors.append(f"{path} is shorter than the result schema permits")
        pattern = schema.get("pattern")
        if pattern and re.search(pattern, value) is None:
            errors.append(f"{path} does not match the result schema pattern")
    elif isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path} is below the result schema minimum")
        if "exclusiveMinimum" in schema and value <= schema["exclusiveMinimum"]:
            errors.append(f"{path} is not above the result schema exclusive minimum")
    return errors


def static_contract_errors(root: Path = ROOT) -> list[str]:
    gate_b_dir = root / "benchmarks/runtime/gate-b"
    resident_recording = root / "src/runtime/src/resident_recording.rs"
    if not resident_recording.exists():
        resident_recording = root / "src/runtime/src/resident_gate_b.rs"
    program_activation = root / "src/engine/src/resident/general/mod.rs"
    program_execution = root / "src/engine/src/resident/general/execution.rs"
    if not program_execution.exists():
        program_activation = root / "src/engine/src/resident/program_activation.rs"
        program_execution = root / "src/engine/src/resident/program_execution.rs"
    required = (
        gate_b_dir / "README.md",
        gate_b_dir / "ekf-v1.json",
        gate_b_dir / "ekf-input-v1.bin",
        gate_b_dir / "ekf-input-v1.sha256",
        gate_b_dir / "result-schema.json",
        gate_b_dir / "numpy/ekf_v1.py",
        root / "scripts/generate-gate-b-ekf-trace.py",
        root / "scripts/run-gate-b-benchmarks.py",
        root / "src/runtime/benches/resident_ekf.rs",
        root / "src/runtime/benches/support/gate_b/raw_kernel.rs",
        root / "src/runtime/benches/support/gate_b/raw_epoch.rs",
        root / "src/runtime/benches/support/gate_b/full_write.rs",
        root / "src/runtime/benches/support/gate_b/legacy_atomic.rs",
        root / "src/runtime/benches/support/gate_b/resident_kernel.rs",
        root / "src/runtime/benches/support/gate_b/resident_turn.rs",
        root / "src/runtime/benches/support/gate_b/resident_artifact.rs",
        resident_recording,
        root / "src/engine/src/resident/artifact.rs",
        root / "src/engine/src/resident/activation.rs",
        root / "src/engine/src/resident/arena.rs",
        root / "src/engine/src/resident/workspace.rs",
        root / "src/engine/src/resident/candidate.rs",
        root / "src/engine/src/resident/full_write.rs",
        root / "src/engine/src/resident/kernel.rs",
        root / "src/engine/src/resident/efficacy/ekf.rs",
        program_activation,
        program_execution,
    )
    errors = [f"missing required Gate B fixture: {path.relative_to(root)}" for path in required if not path.is_file()]
    if errors:
        return errors

    trace = (gate_b_dir / "ekf-input-v1.bin").read_bytes()
    trace_hash = hashlib.sha256(trace).hexdigest()
    if len(trace) != EPISODE_LENGTH * 4 * 8:
        errors.append("EKF trace is not exactly 4,096 little-endian f64 rows")
    if trace_hash != TRACE_SHA256:
        errors.append(f"EKF trace SHA-256 changed: {trace_hash}")
    hash_fixture = read_text(gate_b_dir / "ekf-input-v1.sha256").split()[0]
    if hash_fixture != TRACE_SHA256:
        errors.append("EKF trace hash fixture does not match the frozen SHA-256")

    try:
        manifest = json.loads(read_text(gate_b_dir / "ekf-v1.json"))
        if manifest.get("workload") != "resident-ekf-v1":
            errors.append("EKF manifest workload version changed")
        if manifest.get("episode_length") != EPISODE_LENGTH:
            errors.append("EKF manifest episode length changed")
        if manifest.get("scaled_instances") != list(SCALED_INSTANCES):
            errors.append("EKF manifest instance scales changed")
        if manifest.get("trace", {}).get("sha256") != TRACE_SHA256:
            errors.append("EKF manifest trace SHA-256 changed")
        if len(manifest.get("trace", {}).get("first_eight_rows", [])) != 8:
            errors.append("EKF manifest no longer records the first eight rows")
        if len(manifest.get("trace", {}).get("last_eight_rows", [])) != 8:
            errors.append("EKF manifest no longer records the last eight rows")
        if (
            manifest.get("reference", {}).get("quantized_trajectory_sha256")
            != REFERENCE_HASH
        ):
            errors.append("EKF manifest reference trajectory hash changed")
        if manifest.get("representation") != {
            "endianness": "little",
            "matrix_storage": "column-major",
            "scalar": "f64",
            "trace_row": ["v", "omega", "z_range", "z_bearing"],
        }:
            errors.append("EKF representation contract changed")
        correctness = manifest.get("correctness", {})
        for field in ("absolute_tolerance", "relative_tolerance", "quantization"):
            if correctness.get(field) != 1.0e-10:
                errors.append(f"EKF correctness {field} changed")
    except (KeyError, TypeError, json.JSONDecodeError) as error:
        errors.append(f"invalid EKF manifest: {error}")

    try:
        json.loads(read_text(gate_b_dir / "result-schema.json"))
    except json.JSONDecodeError as error:
        errors.append(f"invalid Gate B result schema JSON: {error}")

    readme = read_text(gate_b_dir / "README.md")
    for frozen in (
        "437f6c6c636d9818729597342165dfc9af5eb4a7",
        "4,096-turn",
        "column-major",
        "T_mech-legacy-atomic - T_rust-epoch > 0",
        "Admission permits are reserved outside timing",
    ):
        if frozen not in readme:
            errors.append(f"Gate B README lost frozen contract text: {frozen}")

    raw_kernel = read_text(root / "src/runtime/benches/support/gate_b/raw_kernel.rs")
    raw_epoch = read_text(root / "src/runtime/benches/support/gate_b/raw_epoch.rs")
    full_write = read_text(root / "src/runtime/benches/support/gate_b/full_write.rs")
    legacy = read_text(root / "src/runtime/benches/support/gate_b/legacy_atomic.rs")
    benchmark = read_text(root / "src/runtime/benches/resident_ekf.rs")
    resident_root = root / "src/engine/src/resident"
    resident_sources = {
        source: read_text(source) for source in resident_root.rglob("*.rs")
    }
    forbidden_resident_identifiers = {
        "Value", "LegacyValue", "Ref", "ValRef", "ReactiveCellId", "ReactiveTurnJournal",
        "ValueStateJournal", "RuntimeExecutionTransaction",
        "transaction_state_values", "capture_runtime_operation_savepoint",
        "commit_runtime",
    }
    for source, text in resident_sources.items():
        identifiers = set(re.split(r"[^A-Za-z0-9_]+", text))
        forbidden = forbidden_resident_identifiers.intersection(identifiers)
        if source == program_activation or source == root / "src/engine/src/resident/general/execution.rs":
            forbidden.discard("Value")
        if forbidden:
            errors.append(
                f"resident source {source.relative_to(root)} uses legacy state: "
                + ", ".join(sorted(forbidden))
            )
    resident_text = "\n".join(resident_sources.values())
    forbidden_resident_source = (
        "slots: Box<[Versioned<Box<[f64]>>]>",
        "plan.slot(",
        "left_rows",
        "left_columns",
        "right_columns",
        "Vec<ResidentEkf>",
        "raw_kernel::step",
        "ekf_step",
    )
    for forbidden in forbidden_resident_source:
        if forbidden in resident_text:
            errors.append(f"resident timed implementation contains forbidden {forbidden}")
    full_write_path = root / "src/engine/src/resident/full_write.rs"
    for source, text in resident_sources.items():
        if source not in {full_write_path, program_activation} and "Box<[f64]>" in text:
            errors.append(
                "resident typed EKF storage is erased in "
                f"{source.relative_to(root)}"
            )
    candidate_source = resident_sources[root / "src/engine/src/resident/candidate.rs"]
    program_activation_source = resident_sources[program_activation]
    program_execution_source = resident_sources[program_execution]
    if candidate_source.count(".store(") != 1:
        errors.append("Gate B control must contain exactly one publication store site")
    if program_execution_source.count(".store(") != 2:
        errors.append(
            "D1 artifact execution must contain one prepared and one summary-free publication store site"
        )
    if program_activation_source.count(".store(") != 1:
        errors.append("D1 artifact reactivation must contain one epoch-preservation store site")
    if sum(text.count(".store(") for text in resident_sources.values()) != 4:
        errors.append("resident execution contains an unapproved publication store site")
    if "next_epoch: Option<InstanceEpoch>" not in candidate_source or "checked_next()" not in candidate_source:
        errors.append("resident candidate epochs do not use checked exhaustion")
    artifact_source = resident_sources[root / "src/engine/src/resident/artifact.rs"]
    if artifact_source.count("node(") < 15:
        errors.append("resident EKF artifact lost its explicit node manifest")
    activation_source = resident_sources[root / "src/engine/src/resident/activation.rs"]
    for required_topology in (
        "consumer_offsets",
        "consumer_nodes",
        "downstream_offsets",
        "downstream_nodes",
        "linear_node_order",
    ):
        if required_topology not in activation_source:
            errors.append(f"resident activation lost {required_topology}")
    resident_fixture = read_text(
        root / "src/runtime/benches/support/gate_b/resident_kernel.rs"
    )
    resident_turn_fixture = read_text(
        root / "src/runtime/benches/support/gate_b/resident_turn.rs"
    )
    resident_recorder = read_text(resident_recording)
    if "resident: ResidentEkfBatch" not in resident_fixture:
        errors.append("scaled resident lanes do not use one ResidentEkfBatch")
    complete_path = resident_turn_fixture + "\n" + resident_recorder
    forbidden_complete_identifiers = forbidden_resident_identifiers | {
        "RuntimeContext",
        "HashSet",
        "BinaryHeap",
    }
    complete_identifiers = set(re.split(r"[^A-Za-z0-9_]+", complete_path))
    forbidden_complete = forbidden_complete_identifiers.intersection(
        complete_identifiers
    )
    if forbidden_complete:
        errors.append(
            "resident complete path uses forbidden legacy or variable-work state: "
            + ", ".join(sorted(forbidden_complete))
        )
    if re.search(r"GateBFixedReceipt\s*[<&]", complete_path):
        errors.append("resident complete path retains a receipt payload reference")
    for frozen in (
        "pub fn prepare_commit<'instance>",
        "pub fn prepare_full_write_commit<'instance>",
        "let summary = turn.summary();",
        "self.prepare_accepted_append(permit, summary)",
        "PreparedResidentPublication::Ekf(turn)",
        "PreparedResidentPublication::FullWrite(turn)",
        "turn.abort();",
    ):
        if frozen not in resident_recorder:
            errors.append(f"resident candidate/receipt binding contract lost: {frozen}")
    for forbidden in (
        "pub fn prepare_accepted(",
        "pub fn new_full_write(",
        "pub fn new(\n        turn: PreparedResidentTurn",
    ):
        if forbidden in resident_recorder:
            errors.append(f"resident mismatched receipt construction remains public: {forbidden}")
    for frozen in (
        ".prepare_commit(permit, prepared)",
        ".prepare_full_write_commit(permit, prepared)",
    ):
        if frozen not in resident_turn_fixture:
            errors.append(f"resident benchmark bypasses bound commit preparation: {frozen}")
    commit_body = resident_recorder[
        resident_recorder.find("pub fn commit(self)") :
    ]
    if not call_occurs_before(commit_body, "self.turn.publish", "self.append.append"):
        errors.append("resident commit does not publish before retained append")
    if "fn append(self) -> LedgerSequence" not in resident_recorder:
        errors.append("resident post-publication append is not statically infallible")
    if "pub fn commit(self) -> LedgerSequence" not in resident_recorder:
        errors.append("resident complete commit exposes post-publication failure")
    if "OwnedTurnRecord<GateBFixedReceipt>" not in raw_epoch:
        errors.append("raw epoch does not use the fixed resident receipt type")
    if "OwnedTurnRecord<GateBFixedReceipt>" not in resident_recorder:
        errors.append("resident turn does not use the fixed raw-epoch receipt type")
    resident_full_write = resident_sources[full_write_path]
    if "self.execute_candidate(input)?.publish();" not in resident_full_write:
        errors.append("resident kernel full-write lane computes complete receipt summary")
    if not call_occurs_before(
        resident_full_write, "self.execute_candidate", "let summary = candidate.summary();"
    ):
        errors.append("resident complete full-write summary is not derived after execution")
    for frozen in (
        "EdgeTiming::SameTurn",
        "EdgeTiming::NextTurn",
        "same_turn_downstream",
        "next_turn_consumers",
        "timing == EdgeTiming::SameTurn",
    ):
        if frozen not in activation_source:
            errors.append(f"resident temporal topology contract lost: {frozen}")
    if "raw_kernel::step(" not in raw_epoch or "raw_kernel::step(" not in legacy:
        errors.append("raw epoch and legacy controls must call the shared raw EKF kernel")
    if raw_epoch.count("published_epoch.store(") != 1:
        errors.append("raw EKF epoch must contain exactly one publication store site")
    if full_write.count("published_epoch.store(") != 1:
        errors.append("raw full-write epoch must contain exactly one publication store site")
    if "candidate_seed_bytes: 0" not in raw_epoch or "candidate_seed_bytes: 0" not in full_write:
        errors.append("raw epoch controls must freeze zero candidate seed bytes")
    if "published_buffer_copy_bytes: 0" not in raw_epoch or "published_buffer_copy_bytes: 0" not in full_write:
        errors.append("raw epoch controls must freeze zero published-buffer copy bytes")
    if not call_occurs_before(raw_epoch, "reserve_retained", "pub fn run_episode"):
        errors.append("raw EKF admission must be reserved during fixture setup")
    if not call_occurs_before(full_write, "reserve_retained", "pub fn run_episode"):
        errors.append("raw full-write admission must be reserved during fixture setup")
    for frozen in (
        "RuntimeHostInput::new(vec![",
        "RuntimeHostInputDriver for GateBInputDriver",
        "apply_host_input_with_context",
        "transaction_state_values",
    ):
        if frozen not in legacy:
            errors.append(f"legacy ordinary-turn fixture lost: {frozen}")
    if legacy.count("RuntimeHostInputUpdate {") < 4:
        errors.append("legacy EKF host-input packet no longer contains four updates")
    for frozen in (
        "Instant::now()",
        "reset_allocations();",
        "GATE_B_SAMPLE ",
        "for _ in 0..iterations",
    ):
        if frozen not in benchmark:
            errors.append(f"Rust benchmark timing/probe contract lost: {frozen}")

    numpy_source = read_text(gate_b_dir / "numpy/ekf_v1.py")
    prohibited = import_roots(numpy_source).intersection(
        {"numba", "jax", "torch", "cupy", "tensorflow", "cython"}
    )
    if prohibited:
        errors.append(
            "persistent NumPy control imports prohibited accelerators: "
            + ", ".join(sorted(prohibited))
        )
    for frozen in (
        "time.perf_counter_ns()",
        'order="F"',
        "np.show_config()",
        '"type": "ready"',
        'command == "benchmark"',
    ):
        if frozen not in numpy_source:
            errors.append(f"persistent NumPy contract lost: {frozen}")

    runner = read_text(root / "scripts/run-gate-b-benchmarks.py")
    for frozen in (
        "refusing to attribute Gate B benchmark results to a dirty worktree",
        "git\", \"rev-parse\", \"HEAD",
        "git\", \"merge-base",
        "MECH_GATE_B_STRUCTURAL_ONLY",
        "legacy_denominator",
        f'B2_EVIDENCE_FLOOR = "{B2_EVIDENCE_FLOOR}"',
        'choices=("B2-resident-turn",)',
    ):
        if frozen not in runner:
            errors.append(f"Gate B runner attribution/fairness contract lost: {frozen}")
    for variable in THREAD_VARIABLES:
        if variable not in runner:
            errors.append(f"Gate B runner no longer fixes {variable}")

    allowed_production = {
        root / "src/runtime/src/lib.rs",
        root / "src/runtime/src/resident_gate_b.rs",
        root / "src/runtime/src/resident_recording.rs",
        root / "src/runtime/src/turn_record.rs",
    }
    for source in (root / "src").rglob("*.rs"):
        if (
            "/benches/" in source.as_posix()
            or "/tests/" in source.as_posix()
            or source in allowed_production
        ):
            continue
        text = read_text(source)
        if "GateBFixedReceipt" in text or "__gate_b_recording" in text:
            errors.append(
                "Gate B provisional receipt escaped its approved production boundary: "
                f"{source.relative_to(root)}"
            )
    production_lib = read_text(root / "src/runtime/src/lib.rs")
    production_receipt = read_text(root / "src/runtime/src/turn_record.rs")
    if 'feature = "runtime_bench_gate_b"' not in production_lib:
        errors.append("Gate B ledger facade is not Gate-B-feature gated")
    if 'feature = "runtime_bench_gate_b"' not in production_receipt:
        errors.append("Gate B fixed receipt is not Gate-B-feature gated")
    return errors


def _lane_map(report: dict[str, Any], errors: list[str]) -> dict[LaneKey, dict[str, Any]]:
    lanes: dict[LaneKey, dict[str, Any]] = {}
    for lane in report.get("lanes", []):
        try:
            key = (
                str(lane["lane"]),
                int(lane["instances"]),
                int(lane.get("retained_history", 0)),
                int(lane.get("next_epoch", 1)),
            )
        except (KeyError, TypeError, ValueError):
            errors.append("Gate B report contains an invalid lane identity")
            continue
        if key in lanes:
            errors.append(f"Gate B report duplicates lane {key}")
        lanes[key] = lane
    return lanes


def report_contract_errors(
    report: dict[str, Any], expected_commit: str | None = None
) -> list[str]:
    try:
        schema = json.loads(read_text(GATE_B_DIR / "result-schema.json"))
        errors = json_schema_errors(report, schema)
    except (KeyError, TypeError, json.JSONDecodeError) as error:
        errors = [f"cannot apply Gate B result schema: {error}"]
    missing_core: list[str] = []
    for field in (
        "schema_version",
        "gate",
        "phase",
        "git_commit",
        "git_branch",
        "machine",
        "toolchain",
        "thread_environment",
        "trace",
        "workload",
        "sample_protocol",
        "lanes",
        "derived",
        "stop_condition",
    ):
        if field not in report:
            missing_core.append(f"Gate B report is missing {field}")
    errors.extend(missing_core)
    if missing_core:
        return errors
    if report["schema_version"] != 1 or report["gate"] != "B":
        errors.append("Gate B report schema identity changed")
    if report["phase"] == "B0-controls" and report["git_branch"] != "test/resident-ekf-efficacy-contract":
        errors.append("B0 evidence is attributed to the wrong branch")
    commit = report["git_commit"]
    if not isinstance(commit, str) or len(commit) != 40 or any(
        character not in "0123456789abcdef" for character in commit
    ):
        errors.append("Gate B report git_commit is not a full lowercase SHA")
    if expected_commit is not None and commit != expected_commit:
        errors.append(f"Gate B report commit {commit} != expected {expected_commit}")
    if report["trace"].get("sha256") != TRACE_SHA256:
        errors.append("Gate B report uses the wrong trace SHA-256")
    workload = report["workload"]
    if workload.get("version") != "resident-ekf-v1":
        errors.append("Gate B report uses the wrong workload version")
    if workload.get("episode_length") != EPISODE_LENGTH:
        errors.append("Gate B report uses the wrong episode length")
    if workload.get("scaled_instances") != list(SCALED_INSTANCES):
        errors.append("Gate B report uses the wrong instance scales")
    for variable in THREAD_VARIABLES:
        if report["thread_environment"].get(variable) != "1":
            errors.append(f"Gate B report did not fix {variable}=1")
    protocol = report["sample_protocol"]
    if protocol.get("turns_per_sample") != EPISODE_LENGTH:
        errors.append("Gate B report sample protocol has the wrong turn count")
    if protocol.get("fixture_setup_included_in_timing") is not False:
        errors.append("Gate B report includes fixture setup in timing")
    if protocol.get("correctness_included_in_timing") is not False:
        errors.append("Gate B report includes correctness work in timing")

    lanes = _lane_map(report, errors)
    try:
        required_lanes = required_phase_lane_keys(report["phase"])
    except ValueError as error:
        errors.append(str(error))
        required_lanes = set()
    has_d1_lanes = any(
        key[0].startswith("mech-resident-artifact-") for key in lanes
    )
    if report.get("d1_decision") is not None or has_d1_lanes:
        required_lanes.update(D1_ARTIFACT_LANE_KEYS)
    missing = required_lanes.difference(lanes)
    if missing:
        errors.append(
            "Gate B report is missing lanes: "
            + ", ".join(
                f"{lane}/{instances}/history-{history}/epoch-{epoch}"
                for lane, instances, history, epoch in sorted(missing)
            )
        )
    unexpected = set(lanes).difference(required_lanes)
    if required_lanes and unexpected:
        errors.append(
            "Gate B report contains unexpected lanes: "
            + ", ".join(str(key) for key in sorted(unexpected))
        )
    for key, lane in lanes.items():
        if lane.get("sample_count", 0) < 10:
            errors.append(f"Gate B lane {key[0]}/{key[1]} has fewer than 10 samples")
        if lane.get("turns_per_sample") != EPISODE_LENGTH:
            errors.append(f"Gate B lane {key[0]}/{key[1]} has the wrong turn count")
        timing = lane.get("timing", {})
        median = timing.get("median_ns_per_turn", 0)
        p95 = timing.get("p95_ns_per_turn", 0)
        if not isinstance(median, (int, float)) or median <= 0:
            errors.append(f"Gate B lane {key[0]}/{key[1]} has no positive median")
        if not isinstance(p95, (int, float)) or p95 <= 0:
            errors.append(f"Gate B lane {key[0]}/{key[1]} has no positive p95")
        if isinstance(median, (int, float)) and isinstance(p95, (int, float)) and p95 < median:
            errors.append(f"Gate B lane {key[0]}/{key[1]} has p95 below its median")
        if lane.get("correctness") is not True:
            errors.append(f"Gate B lane {key[0]}/{key[1]} failed correctness")
        if key[0] != "numpy-persistent" and not key[0].endswith("full-write"):
            if lane.get("quantized_state_hash") != REFERENCE_HASH:
                errors.append(f"Gate B Rust EKF lane {key[0]}/{key[1]} changed its trajectory hash")

    for instances in SCALED_INSTANCES:
        key = ("rust-epoch", instances, 0, 1)
        if key not in lanes:
            continue
        lane = lanes[key]
        allocation = lane.get("allocation", {})
        structural = lane.get("structural", {})
        if allocation.get("episode_allocation_count") != 0 or allocation.get("episode_allocated_bytes") != 0:
            errors.append(f"raw epoch {instances} allocates during the timed episode")
        if structural.get("candidate_seed_bytes") != 0:
            errors.append(f"raw epoch {instances} seeds its candidate buffer")
        if structural.get("published_buffer_copy_bytes") != 0:
            errors.append(f"raw epoch {instances} copies its published buffer")
        if structural.get("publication_store_count") != 1:
            errors.append(f"raw epoch {instances} does not use one publication store")
        if structural.get("candidate_written_bytes") != instances * 96:
            errors.append(f"raw epoch {instances} reports the wrong written-byte count")
        if structural.get("receipt_bytes") != 64:
            errors.append(f"raw epoch {instances} reports the wrong receipt size")
        if report["phase"] == "B2-resident-turn":
            for field, expected_value in {
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_appended": EPISODE_LENGTH,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
            }.items():
                if structural.get(field) != expected_value:
                    errors.append(f"raw epoch {instances} reports wrong {field}")

    if report["phase"] in {"B1-resident-kernel", "B2-resident-turn"}:
        if (
            report["phase"] == "B1-resident-kernel"
            and report["git_branch"] != "feat/engine-resident-ekf-substrate"
        ):
            errors.append("B1 evidence is attributed to the wrong branch")
        if (
            report["phase"] == "B2-resident-turn"
            and report["git_branch"] != "perf/runtime-resident-ekf-efficacy"
        ):
            if expected_commit is None:
                errors.append(
                    "B2 descendant refresh evidence requires an exact expected commit"
                )
            elif not commit_descends_from(commit):
                errors.append(
                    f"B2 descendant refresh commit {commit} does not descend from "
                    f"the frozen evidence floor {B2_EVIDENCE_FLOOR}"
                )
        for instances in SCALED_INSTANCES:
            key = ("mech-resident-kernel", instances, 0, 1)
            if key not in lanes:
                continue
            lane = lanes[key]
            allocation = lane.get("allocation", {})
            structural = lane.get("structural", {})
            if allocation.get("episode_allocation_count") != 0 or allocation.get("episode_allocated_bytes") != 0:
                errors.append(f"resident kernel {instances} allocates during the timed episode")
            if structural.get("candidate_seed_bytes") != 0:
                errors.append(f"resident kernel {instances} seeds candidate storage")
            if structural.get("published_buffer_copy_bytes") != 0:
                errors.append(f"resident kernel {instances} copies published storage")
            if structural.get("publication_store_count") != 1:
                errors.append(f"resident kernel {instances} does not use one publication store")
            if structural.get("candidate_written_bytes") != instances * 96:
                errors.append(
                    f"resident kernel {instances} reports the wrong written-byte count"
                )
            if structural.get("receipt_bytes") != 0:
                errors.append(f"resident kernel {instances} constructs a receipt in B1")
            if structural.get("commit_runtime_call_count") != 0:
                errors.append(f"resident kernel {instances} calls the runtime commit path")
            if structural.get("legacy_journal_capture_count") != 0:
                errors.append(f"resident kernel {instances} captures a legacy journal")

        resident_full_key = ("mech-resident-kernel-full-write", 1, 0, 1)
        if resident_full_key in lanes:
            lane = lanes[resident_full_key]
            allocation = lane.get("allocation", {})
            structural = lane.get("structural", {})
            if allocation.get("episode_allocation_count") != 0 or allocation.get("episode_allocated_bytes") != 0:
                errors.append("resident full-write allocates during the timed episode")
            for field in (
                "candidate_seed_bytes",
                "published_buffer_copy_bytes",
                "receipt_bytes",
                "commit_runtime_call_count",
                "legacy_journal_capture_count",
            ):
                if structural.get(field) != 0:
                    errors.append(f"resident full-write reports nonzero {field}")
            if structural.get("candidate_written_bytes") != 64 * 64 * 8:
                errors.append("resident full-write reports the wrong written-byte count")
            if structural.get("publication_store_count") != 1:
                errors.append("resident full-write does not use one publication store")
            if not structural.get("abort_output_hash"):
                errors.append("resident full-write has no forced-abort output hash")
            raw_full = lanes.get(("rust-epoch-full-write", 1, 0, 1))
            if raw_full is not None:
                if lane.get("quantized_state_hash") != raw_full.get("quantized_state_hash"):
                    errors.append("resident full-write terminal hash differs from raw epoch")
                if structural.get("abort_output_hash") != raw_full.get("structural", {}).get(
                    "abort_output_hash"
                ):
                    errors.append("resident full-write forced-abort hash differs from raw epoch")

    full_key = ("rust-epoch-full-write", 1, 0, 1)
    if full_key in lanes:
        lane = lanes[full_key]
        allocation = lane.get("allocation", {})
        structural = lane.get("structural", {})
        if allocation.get("episode_allocation_count") != 0 or allocation.get("episode_allocated_bytes") != 0:
            errors.append("raw full-write epoch allocates during the timed episode")
        for field in ("candidate_seed_bytes", "published_buffer_copy_bytes"):
            if structural.get(field) != 0:
                errors.append(f"raw full-write epoch reports nonzero {field}")
        if structural.get("candidate_written_bytes") != 64 * 64 * 8:
            errors.append("raw full-write epoch reports the wrong written-byte count")
        if structural.get("publication_store_count") != 1:
            errors.append("raw full-write epoch does not use one publication store")
        if not structural.get("abort_output_hash"):
            errors.append("raw full-write epoch has no forced-abort output hash")
        if report["phase"] == "B2-resident-turn":
            for field, expected_value in {
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_appended": EPISODE_LENGTH,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
            }.items():
                if structural.get(field) != expected_value:
                    errors.append(f"raw full-write epoch reports wrong {field}")

    for lane_name in ("mech-legacy-atomic", "mech-legacy-atomic-full-write"):
        for key, lane in lanes.items():
            if key[0] != lane_name:
                continue
            structural = lane.get("structural", {})
            if structural.get("commit_runtime_call_count") != EPISODE_LENGTH:
                errors.append(f"legacy lane {key[0]}/{key[1]} did not commit every turn")
            if structural.get("legacy_journal_capture_count", 0) <= 0:
                errors.append(f"legacy lane {key[0]}/{key[1]} did not capture journal state")

    legacy_key = ("mech-legacy-atomic", 1, 0, 1)
    raw_epoch_key = ("rust-epoch", 1, 0, 1)
    if legacy_key in lanes and raw_epoch_key in lanes:
        legacy = lanes[legacy_key]["timing"]["median_ns_per_turn"]
        raw_epoch = lanes[raw_epoch_key]["timing"]["median_ns_per_turn"]
        denominator = legacy - raw_epoch
        derived = report["derived"]
        reported = derived.get("legacy_denominator_ns_per_turn")
        if not isinstance(reported, (int, float)) or not math.isclose(
            float(reported), float(denominator), rel_tol=1.0e-12, abs_tol=1.0e-9
        ):
            errors.append("Gate B report legacy denominator is not derived from primary medians")
        if denominator <= 0.0:
            errors.append("Gate B B0 stop condition failed: legacy denominator is non-positive")
        if derived.get("positive") is not (denominator > 0.0):
            errors.append("Gate B derived positive flag disagrees with the denominator")
        if report["stop_condition"].get("passed") is not (denominator > 0.0):
            errors.append("Gate B stop-condition result disagrees with the denominator")

    if report["phase"] in {"B1-resident-kernel", "B2-resident-turn"}:
        progression = report.get("b1_progression")
        if not isinstance(progression, dict):
            errors.append("B1 report is missing b1_progression")
        elif all(
            key in lanes
            for key in (
                ("mech-resident-kernel", 1, 0, 1),
                ("rust-kernel", 1, 0, 1),
                ("rust-epoch", 1, 0, 1),
            )
        ):
            resident = lanes[("mech-resident-kernel", 1, 0, 1)]["timing"]["median_ns_per_turn"]
            rust_kernel = lanes[("rust-kernel", 1, 0, 1)]["timing"]["median_ns_per_turn"]
            rust_epoch = lanes[("rust-epoch", 1, 0, 1)]["timing"]["median_ns_per_turn"]
            expected = {
                "resident_kernel_ns_per_turn": resident,
                "rust_kernel_ns_per_turn": rust_kernel,
                "rust_epoch_ns_per_turn": rust_epoch,
                "resident_kernel_ratio": resident / rust_kernel,
                "resident_kernel_vs_raw_epoch": resident / rust_epoch,
                "limit_multiplier": 1.05,
                "limit_ns_per_turn": 1.05 * rust_epoch,
            }
            for field, value in expected.items():
                reported = progression.get(field)
                if not isinstance(reported, (int, float)) or not math.isclose(
                    float(reported), float(value), rel_tol=1.0e-12, abs_tol=1.0e-9
                ):
                    errors.append(f"B1 progression {field} is inconsistent with lane medians")
            passed = resident <= 1.05 * rust_epoch
            if progression.get("passed") is not passed:
                errors.append("B1 progression passed flag disagrees with the unchanged limit")

    if report["phase"] == "B2-resident-turn":
        turn_keys = [
            ("mech-resident-turn", 1, 0, 1),
            ("mech-resident-turn", 1, 1_000, 1),
            ("mech-resident-turn", 1, 100_000, 1),
            ("mech-resident-turn", 1, 0, 1_000_000_001),
        ]
        artifact_complete_keys = [
            ("mech-resident-artifact-source", 1, 0, 1),
            ("mech-resident-artifact-source", 1, 1_000, 1),
            ("mech-resident-artifact-source", 1, 100_000, 1),
            ("mech-resident-artifact-source", 1, 0, 1_000_000_001),
            ("mech-resident-artifact-bytecode", 1, 0, 1),
        ]
        artifact_kernel_keys = [
            ("mech-resident-artifact-kernel-source", 1, 0, 1),
            ("mech-resident-artifact-kernel-bytecode", 1, 0, 1),
        ]
        complete_keys = [
            ("mech-resident-scheduled", 1, 0, 1),
            *turn_keys,
            ("mech-resident-turn-full-write", 1, 0, 1),
        ]
        for key in complete_keys:
            if key not in lanes:
                continue
            lane = lanes[key]
            allocation = lane.get("allocation", {})
            structural = lane.get("structural", {})
            if (
                allocation.get("episode_allocation_count") != 0
                or allocation.get("episode_allocated_bytes") != 0
            ):
                errors.append(f"B2 resident lane {key} allocates during the timed episode")
            if structural.get("publication_store_count") != 1:
                errors.append(f"B2 resident lane {key} does not use one publication store")
            if structural.get("candidate_seed_bytes") != 0:
                errors.append(f"B2 resident lane {key} seeds candidate storage")
            if structural.get("published_buffer_copy_bytes") != 0:
                errors.append(f"B2 resident lane {key} copies published storage")

        for key in artifact_complete_keys:
            if key not in lanes:
                continue
            structural = lanes[key].get("structural", {})
            expected_structural = {
                "candidate_written_bytes": 96,
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_retained_before_timing": key[2],
                "records_appended": EPISODE_LENGTH,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
                "commit_runtime_call_count": 0,
                "legacy_journal_capture_count": 0,
            }
            for field, expected_value in expected_structural.items():
                if structural.get(field) != expected_value:
                    errors.append(f"D1 artifact lane {key} reports wrong {field}")
            if not structural.get("abort_output_hash"):
                errors.append(f"D1 artifact lane {key} has no forced-abort output hash")
            dirty = structural.get("dirty_node_count")
            if not isinstance(dirty, int) or not 5 <= dirty <= 20:
                errors.append(f"D1 artifact lane {key} reports invalid actual dirty count")

        for key in artifact_kernel_keys:
            if key not in lanes:
                continue
            structural = lanes[key].get("structural", {})
            for field, expected_value in {
                "candidate_written_bytes": 96,
                "record_preparation_count": 0,
                "record_append_count": 0,
                "records_appended": 0,
                "post_publication_append_infallible": False,
                "commit_runtime_call_count": 0,
                "legacy_journal_capture_count": 0,
            }.items():
                if structural.get(field) != expected_value:
                    errors.append(f"D1 artifact kernel lane {key} reports wrong {field}")

        for key in turn_keys:
            if key not in lanes:
                continue
            structural = lanes[key].get("structural", {})
            expected_structural = {
                "candidate_written_bytes": 96,
                "receipt_bytes": 64,
                "dirty_node_count": 15,
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_retained_before_timing": key[2],
                "records_appended": EPISODE_LENGTH,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
            }
            for field, expected_value in expected_structural.items():
                if structural.get(field) != expected_value:
                    errors.append(
                        f"B2 resident lane {key} reports wrong {field}"
                    )

        full_turn_key = ("mech-resident-turn-full-write", 1, 0, 1)
        if full_turn_key in lanes:
            full = lanes[full_turn_key]
            structural = full.get("structural", {})
            expected_structural = {
                "candidate_written_bytes": 32_768,
                "receipt_bytes": 64,
                "dirty_node_count": 1,
                "record_preparation_count": 1,
                "record_append_count": 1,
                "records_retained_before_timing": 0,
                "records_appended": EPISODE_LENGTH,
                "ledger_records_inspected": 0,
                "post_publication_append_infallible": True,
            }
            for field, expected_value in expected_structural.items():
                if structural.get(field) != expected_value:
                    errors.append(f"B2 full-write lane reports wrong {field}")
            if not structural.get("abort_output_hash"):
                errors.append("B2 full-write lane has no forced-rejection output hash")

        decision_keys = {
            ("mech-legacy-atomic", 1, 0, 1),
            ("rust-epoch", 1, 0, 1),
            ("numpy-persistent", 1, 0, 1),
            ("mech-resident-kernel", 1, 0, 1),
            ("mech-resident-scheduled", 1, 0, 1),
            ("mech-resident-turn", 1, 0, 1),
            ("mech-resident-turn", 1, 1_000, 1),
            ("mech-resident-turn", 1, 100_000, 1),
            ("mech-resident-turn", 1, 0, 1_000_000_001),
            full_turn_key,
        }
        b2 = report.get("b2_decision")
        if not isinstance(b2, dict):
            errors.append("B2 report is missing b2_decision")
        elif decision_keys.issubset(lanes):
            median = lambda key: float(lanes[key]["timing"]["median_ns_per_turn"])
            legacy = median(("mech-legacy-atomic", 1, 0, 1))
            raw_epoch = median(("rust-epoch", 1, 0, 1))
            numpy = median(("numpy-persistent", 1, 0, 1))
            kernel = median(("mech-resident-kernel", 1, 0, 1))
            scheduled = median(("mech-resident-scheduled", 1, 0, 1))
            turn_key = ("mech-resident-turn", 1, 0, 1)
            turn = median(turn_key)
            turn_p95 = float(lanes[turn_key]["timing"]["p95_ns_per_turn"])
            history_ratio = median(("mech-resident-turn", 1, 100_000, 1)) / turn
            history_1k_ratio = median(("mech-resident-turn", 1, 1_000, 1)) / turn
            high_epoch_ratio = median(
                ("mech-resident-turn", 1, 0, 1_000_000_001)
            ) / turn
            metrics = {
                "legacy_gap_closure": (legacy - turn) / (legacy - raw_epoch),
                "raw_epoch_ratio": turn / raw_epoch,
                "executor_tax_ns": turn - kernel,
                "scheduler_tax_ns": scheduled - kernel,
                "recording_tax_ns": turn - scheduled,
                "numpy_ratio": turn / numpy,
                "tail_ratio": turn_p95 / turn,
                "history_1k_over_history_0_median_ratio": history_1k_ratio,
                "history_100k_over_history_0_median_ratio": history_ratio,
                "high_epoch_over_low_epoch_median_ratio": high_epoch_ratio,
            }
            for field, expected_value in metrics.items():
                reported = b2.get(field)
                if not isinstance(reported, (int, float)) or not math.isclose(
                    float(reported), expected_value, rel_tol=1.0e-12, abs_tol=1.0e-9
                ):
                    errors.append(f"B2 decision {field} is inconsistent with lane evidence")

            resident_decision_lanes = [lanes[key] for key in complete_keys]
            primary_structural = lanes[turn_key]["structural"]
            full_structural = lanes[full_turn_key]["structural"]
            hard_gates = {
                "correctness": all(
                    lane.get("correctness") is True
                    and lane.get("quantized_state_hash")
                    == lane.get("reference_quantized_state_hash")
                    for lane in resident_decision_lanes
                ),
                "zero_allocation": all(
                    lane.get("allocation", {}).get("episode_allocation_count") == 0
                    for lane in resident_decision_lanes
                ),
                "constant_publication": (
                    primary_structural.get("publication_store_count") == 1
                    and full_structural.get("publication_store_count") == 1
                ),
                "no_full_clone": (
                    primary_structural.get("candidate_seed_bytes") == 0
                    and primary_structural.get("published_buffer_copy_bytes") == 0
                    and full_structural.get("candidate_seed_bytes") == 0
                    and full_structural.get("candidate_written_bytes") == 32_768
                    and full_structural.get("published_buffer_copy_bytes") == 0
                ),
                "history_independent": (
                    history_1k_ratio <= 1.05
                    and history_ratio <= 1.05
                    and high_epoch_ratio <= 1.05
                    and all(
                        lanes[key]["structural"].get("ledger_records_inspected") == 0
                        for key in turn_keys
                    )
                ),
                "legacy_gap_closure": metrics["legacy_gap_closure"] >= 0.80,
                "raw_epoch_ratio": metrics["raw_epoch_ratio"] <= 1.25,
                "executor_tax": metrics["executor_tax_ns"]
                <= (1.25 * raw_epoch - kernel),
                "tail_stability": metrics["tail_ratio"] <= 1.50,
                "post_publication_append_infallible": (
                    primary_structural.get("post_publication_append_infallible") is True
                    and full_structural.get("post_publication_append_infallible") is True
                ),
            }
            if b2.get("hard_gates") != hard_gates:
                errors.append("B2 decision hard gates are inconsistent with lane evidence")
            numpy_target = turn <= 1.10 * numpy
            if b2.get("numpy_target") is not numpy_target:
                errors.append("B2 decision NumPy target is inconsistent with lane evidence")
            hard_pass = all(hard_gates.values())
            expected_decision = (
                "Pass"
                if hard_pass and numpy_target
                else "ConditionalPass"
                if hard_pass
                else "Fail"
            )
            expected_attribution = (
                "kernel selection, numerical backend, or data layout"
                if expected_decision == "ConditionalPass"
                else None
            )
            if b2.get("decision") != expected_decision:
                errors.append("B2 final decision is inconsistent with recomputed gates")
            if b2.get("conditional_attribution") != expected_attribution:
                errors.append("B2 conditional attribution is inconsistent with its decision")

        d1_keys = {*artifact_complete_keys, *artifact_kernel_keys}
        d1 = report.get("d1_decision")
        has_d1_evidence = d1 is not None or any(key in lanes for key in d1_keys)
        if has_d1_evidence and not isinstance(d1, dict):
            errors.append("D1 report is missing independent d1_decision")
        elif isinstance(d1, dict) and not d1_keys.issubset(lanes):
            errors.append("D1 report is missing one or more required artifact lanes")
        elif isinstance(d1, dict) and d1_keys.issubset(lanes) and decision_keys.issubset(lanes):
            median = lambda key: float(lanes[key]["timing"]["median_ns_per_turn"])
            source_key = ("mech-resident-artifact-source", 1, 0, 1)
            bytecode_key = ("mech-resident-artifact-bytecode", 1, 0, 1)
            gate_b_key = ("mech-resident-turn", 1, 0, 1)
            source = median(source_key)
            bytecode = median(bytecode_key)
            gate_b = median(gate_b_key)
            raw_epoch = median(("rust-epoch", 1, 0, 1))
            legacy = median(("mech-legacy-atomic", 1, 0, 1))
            history_1k_ratio = (
                median(("mech-resident-artifact-source", 1, 1_000, 1)) / source
            )
            history_100k_ratio = (
                median(("mech-resident-artifact-source", 1, 100_000, 1)) / source
            )
            high_epoch_ratio = (
                median(("mech-resident-artifact-source", 1, 0, 1_000_000_001))
                / source
            )
            metrics = {
                "legacy_gap_closure": (legacy - source) / (legacy - raw_epoch),
                "raw_epoch_ratio": source / raw_epoch,
                "source_bytecode_ratio": max(source, bytecode) / min(source, bytecode),
                "artifact_complete_turn_ratio": source / gate_b,
                "executor_tax_ns": source - gate_b,
                "history_1k_over_history_0_median_ratio": history_1k_ratio,
                "history_100k_over_history_0_median_ratio": history_100k_ratio,
                "high_epoch_over_low_epoch_median_ratio": high_epoch_ratio,
                "kernel_source_ns_per_turn": median(
                    ("mech-resident-artifact-kernel-source", 1, 0, 1)
                ),
                "kernel_bytecode_ns_per_turn": median(
                    ("mech-resident-artifact-kernel-bytecode", 1, 0, 1)
                ),
            }
            for field, expected_value in metrics.items():
                reported = d1.get(field)
                if not isinstance(reported, (int, float)) or not math.isclose(
                    float(reported), expected_value, rel_tol=1.0e-12, abs_tol=1.0e-9
                ):
                    errors.append(f"D1 decision {field} is inconsistent with lane evidence")
            complete_lanes = [lanes[key] for key in artifact_complete_keys]
            structural = lanes[source_key]["structural"]
            structural_fields = (
                "candidate_seed_bytes",
                "candidate_written_bytes",
                "published_buffer_copy_bytes",
                "publication_store_count",
                "record_preparation_count",
                "record_append_count",
                "records_appended",
                "ledger_records_inspected",
                "post_publication_append_infallible",
                "commit_runtime_call_count",
                "legacy_journal_capture_count",
            )
            structural_equivalent = all(
                lane["structural"].get(field) == structural.get(field)
                for lane in complete_lanes
                for field in structural_fields
            )
            hard_gates = {
                "correctness": all(
                    lane.get("correctness") is True
                    and lane.get("quantized_state_hash")
                    == lane.get("reference_quantized_state_hash")
                    for lane in complete_lanes
                ),
                "source_bytecode_equivalence": metrics["source_bytecode_ratio"] <= 1.03,
                "complete_turn_control_ratio": metrics["artifact_complete_turn_ratio"] <= 1.20,
                "raw_epoch_ratio": metrics["raw_epoch_ratio"] <= 1.50,
                "legacy_gap_closure": metrics["legacy_gap_closure"] >= 0.75,
                "history_independent": history_1k_ratio <= 1.05 and history_100k_ratio <= 1.05,
                "epoch_magnitude_independent": high_epoch_ratio <= 1.05,
                "zero_allocation": all(
                    lane.get("allocation", {}).get("episode_allocation_count") == 0
                    for lane in complete_lanes
                ),
                "candidate_contract": (
                    structural.get("candidate_seed_bytes") == 0
                    and structural.get("candidate_written_bytes") == 96
                    and structural.get("published_buffer_copy_bytes") == 0
                    and structural.get("publication_store_count") == 1
                ),
                "recording_contract": (
                    structural_equivalent
                    and structural.get("record_preparation_count") == 1
                    and structural.get("record_append_count") == 1
                    and structural.get("records_appended") == EPISODE_LENGTH
                    and structural.get("ledger_records_inspected") == 0
                    and structural.get("post_publication_append_infallible") is True
                ),
                "legacy_boundaries_unused": (
                    structural.get("commit_runtime_call_count") == 0
                    and structural.get("legacy_journal_capture_count") == 0
                ),
            }
            if d1.get("hard_gates") != hard_gates:
                errors.append("D1 decision hard gates are inconsistent with lane evidence")
            expected_decision = "Pass" if all(hard_gates.values()) else "Fail"
            if d1.get("decision") != expected_decision:
                errors.append("D1 final decision is inconsistent with recomputed gates")
    return errors


def generator_check_error(root: Path = ROOT) -> str | None:
    process = subprocess.run(
        [sys.executable, str(root / "scripts/generate-gate-b-ekf-trace.py"), "--check"],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if process.returncode == 0:
        return None
    return "Gate B trace generator check failed:\n" + process.stdout.strip()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--contract-only",
        action="store_true",
        help="check committed fixtures and implementation without an evidence report",
    )
    parser.add_argument(
        "--report",
        type=Path,
        default=GATE_B_DIR / "b0-controls.json",
        help="Gate B evidence report to validate",
    )
    parser.add_argument(
        "--expected-commit",
        help="require the report's git_commit to equal this exact SHA",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    errors = static_contract_errors()
    generator_error = generator_check_error()
    if generator_error:
        errors.append(generator_error)
    if not args.contract_only:
        report_path = args.report if args.report.is_absolute() else ROOT / args.report
        if not report_path.is_file():
            errors.append(f"missing Gate B evidence report: {report_path}")
        else:
            try:
                report = json.loads(read_text(report_path))
                errors.extend(report_contract_errors(report, args.expected_commit))
            except json.JSONDecodeError as error:
                errors.append(f"invalid Gate B evidence JSON: {error}")
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    scope = "contract" if args.contract_only else "contract and evidence"
    print(f"Gate B {scope} checks passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
