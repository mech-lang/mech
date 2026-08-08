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


def import_roots(source: str) -> set[str]:
    tree = ast.parse(source)
    roots = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            roots.update(alias.name.split(".", 1)[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            roots.add(node.module.split(".", 1)[0])
    return roots


def required_lane_keys() -> set[tuple[str, int]]:
    keys = {
        (lane, instances)
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
            ("rust-epoch-full-write", 1),
            ("mech-legacy-atomic-full-write", 1),
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
    ):
        if frozen not in runner:
            errors.append(f"Gate B runner attribution/fairness contract lost: {frozen}")
    for variable in THREAD_VARIABLES:
        if variable not in runner:
            errors.append(f"Gate B runner no longer fixes {variable}")

    allowed_production = {
        root / "src/runtime/src/lib.rs",
        root / "src/runtime/src/turn_record.rs",
    }
    for source in (root / "src").rglob("*.rs"):
        if "/benches/" in source.as_posix() or source in allowed_production:
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


def _lane_map(report: dict[str, Any], errors: list[str]) -> dict[tuple[str, int], dict[str, Any]]:
    lanes: dict[tuple[str, int], dict[str, Any]] = {}
    for lane in report.get("lanes", []):
        try:
            key = (str(lane["lane"]), int(lane["instances"]))
        except (KeyError, TypeError, ValueError):
            errors.append("Gate B report contains an invalid lane identity")
            continue
        if key in lanes:
            errors.append(f"Gate B report duplicates lane {key[0]}/{key[1]}")
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
    missing = required_lane_keys().difference(lanes)
    if missing:
        errors.append(
            "Gate B report is missing lanes: "
            + ", ".join(f"{lane}/{instances}" for lane, instances in sorted(missing))
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
        key = ("rust-epoch", instances)
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

    full_key = ("rust-epoch-full-write", 1)
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

    for lane_name in ("mech-legacy-atomic", "mech-legacy-atomic-full-write"):
        for key, lane in lanes.items():
            if key[0] != lane_name:
                continue
            structural = lane.get("structural", {})
            if structural.get("commit_runtime_call_count") != EPISODE_LENGTH:
                errors.append(f"legacy lane {key[0]}/{key[1]} did not commit every turn")
            if structural.get("legacy_journal_capture_count", 0) <= 0:
                errors.append(f"legacy lane {key[0]}/{key[1]} did not capture journal state")

    if ("mech-legacy-atomic", 1) in lanes and ("rust-epoch", 1) in lanes:
        legacy = lanes[("mech-legacy-atomic", 1)]["timing"]["median_ns_per_turn"]
        raw_epoch = lanes[("rust-epoch", 1)]["timing"]["median_ns_per_turn"]
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
