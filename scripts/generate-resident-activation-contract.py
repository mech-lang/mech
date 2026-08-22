#!/usr/bin/env python3
"""Generate and validate the permanent resident-activation contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_DIR = ROOT / "tests/architecture/resident-activation"
SOURCE_PATH = CONTRACT_DIR / "ekf-source-v1.mec"
SOURCE_DIGEST_PATH = CONTRACT_DIR / "ekf-source-v1.sha256"
WORKLOAD_PATH = CONTRACT_DIR / "ekf-workload-v1.json"
WORKLOAD_SCHEMA_PATH = CONTRACT_DIR / "ekf-workload-v1-schema.json"
ACTIVATION_CONTRACT_PATH = CONTRACT_DIR / "resident-activation-contract.json"
ACTIVATION_CONTRACT_SCHEMA_PATH = CONTRACT_DIR / "resident-activation-contract-schema.json"
FROZEN_TARGETS_PATH = ROOT / "tests/architecture/value-system/frozen-semantic-targets-v1.json"
GATE_B_CONTRACT_PATH = ROOT / "benchmarks/runtime/gate-b/ekf-v1.json"
GATE_B_TRACE_PATH = ROOT / "benchmarks/runtime/gate-b/ekf-input-v1.bin"

TRACE_SHA256 = "ab901e1d115aa92166dc2a6d45a28732e6a548363b829997aa410ae4c2d77c8b"
TRAJECTORY_SHA256 = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758"
EKF_SOURCE_SHA256 = "14dce066e441f78e928b56493dcd9b2ab4dad8b969e9c145e847e9f65b34aa96"
EPISODE_LENGTH = 4096
PERMANENT_TARGET_IDS = (
    "mutable-reference-runtime-storage",
    "uninitialized-storage",
)

EXPECTED_OPERATIONS = [
    {"ordinal": 0, "role": "resident-kernel", "operation": "ekf/trigonometric-state", "input_schemas": ["3x1"], "output_schema": "2x1", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 1, "role": "resident-kernel", "operation": "ekf/motion-jacobian", "input_schemas": ["3x1", "4x1", "2x1", "f64"], "output_schema": "3x3", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 2, "role": "resident-kernel", "operation": "ekf/control-jacobian", "input_schemas": ["2x1", "f64"], "output_schema": "3x2", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 3, "role": "resident-kernel", "operation": "ekf/predicted-state", "input_schemas": ["3x1", "4x1", "2x1", "f64"], "output_schema": "3x1", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 4, "role": "resident-kernel", "operation": "ekf/predicted-covariance", "input_schemas": ["3x3", "3x3", "3x2", "2x2"], "output_schema": "3x3", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 5, "role": "resident-kernel", "operation": "ekf/landmark-delta-and-range", "input_schemas": ["3x1", "2x1"], "output_schema": "3x1", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 6, "role": "resident-kernel", "operation": "ekf/predicted-measurement", "input_schemas": ["3x1", "3x1"], "output_schema": "2x1", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 7, "role": "resident-kernel", "operation": "ekf/measurement-jacobian", "input_schemas": ["3x1"], "output_schema": "2x3", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 8, "role": "resident-kernel", "operation": "ekf/innovation-covariance", "input_schemas": ["3x3", "2x3", "2x2"], "output_schema": "2x2", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 9, "role": "resident-kernel", "operation": "ekf/solve-2x2", "input_schemas": ["2x2"], "output_schema": "2x2", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 10, "role": "resident-kernel", "operation": "ekf/kalman-gain", "input_schemas": ["3x3", "2x3", "2x2"], "output_schema": "3x2", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 11, "role": "resident-kernel", "operation": "ekf/innovation-from-frame", "input_schemas": ["4x1", "2x1"], "output_schema": "2x1", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 12, "role": "resident-kernel", "operation": "ekf/corrected-state", "input_schemas": ["3x1", "3x2", "2x1"], "output_schema": "3x1", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 13, "role": "resident-kernel", "operation": "ekf/joseph-covariance-update", "input_schemas": ["3x3", "2x3", "3x2", "2x2"], "output_schema": "3x3", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 14, "role": "resident-kernel", "operation": "ekf/covariance-symmetrization", "input_schemas": ["3x3"], "output_schema": "3x3", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "KernelReported"},
    {"ordinal": 15, "role": "integrity-predicate", "operation": "ekf/candidate-finite", "input_schemas": ["3x1", "3x3"], "output_schema": "bool", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "ExactScalar"},
    {"ordinal": 16, "role": "integrity-predicate", "operation": "ekf/covariance-positive-diagonal", "input_schemas": ["3x3"], "output_schema": "bool", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "ExactScalar"},
    {"ordinal": 17, "role": "integrity-predicate", "operation": "ekf/covariance-symmetric", "input_schemas": ["3x3"], "output_schema": "bool", "contract": "Declared", "delivery": "Signal", "interaction": "Pure", "input_access": "Read", "output_access": "Write", "construction": {"kind": "FullWrite", "shape": "Declared"}, "alias": "NoAlias", "change_detection": "ExactScalar"},
]
EXPECTED_INPUT = {"name": "frame", "schema": "Matrix<f64,4,1>", "fields": ["v", "omega", "z_range", "z_bearing"], "storage": "turn-workspace", "interaction": "Observation", "replay": "CaptureAsInputFact"}
EXPECTED_STATE = [
    {"name": "state", "schema": "Matrix<f64,3,1>", "initial_payload": [2.0, 1.0, 0.15], "storage": "dual-version"},
    {"name": "covariance", "schema": "Matrix<f64,3,3>", "initial_payload_column_major": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05], "storage": "dual-version"},
]
EXPECTED_CONSTANTS = [
    {"name": "dt", "schema": "f64", "payload": 0.05},
    {"name": "landmark", "schema": "Matrix<f64,2,1>", "payload": [25.0, -10.0]},
    {"name": "process-covariance", "schema": "Matrix<f64,2,2>", "payload": [0.04, 0.0, 0.0, 0.0025]},
    {"name": "measurement-covariance", "schema": "Matrix<f64,2,2>", "payload": [0.25, 0.0, 0.0, 0.0009]},
]
EXPECTED_INTEGRITY_CONSTRAINTS = [
    {"name": "finite-candidate", "operation": "integrity/assert", "predicate_operation_ordinal": 15, "predicate_schema": "bool", "interaction": "Pure", "access": "Read", "delivery": "Signal", "input_count": 1, "output_count": 0},
    {"name": "positive-covariance", "operation": "integrity/assert", "predicate_operation_ordinal": 16, "predicate_schema": "bool", "interaction": "Pure", "access": "Read", "delivery": "Signal", "input_count": 1, "output_count": 0},
    {"name": "symmetric-covariance", "operation": "integrity/assert", "predicate_operation_ordinal": 17, "predicate_schema": "bool", "interaction": "Pure", "access": "Read", "delivery": "Signal", "input_count": 1, "output_count": 0},
]
EXPECTED_STATE_UPDATES = [
    {"name": "state", "source": "corrected-state", "write": "complete-full-write"},
    {"name": "covariance", "source": "symmetrized-covariance", "write": "complete-full-write"},
]
EXPECTED_OUTPUT = {"name": "estimate", "schema": "Matrix<f64,3,1>", "source": "state", "observer_policy": "synchronous-before-next-candidate"}
EXPECTED_D1_ACCEPTANCE_TARGETS = {
    "ordinary_source_compiles": True,
    "source_and_bytecode_artifacts_equal": True,
    "legacy_opaque_contracts": 0,
    "resident_activation_succeeds": True,
    "steady_state_allocations": 0,
    "persistent_candidate_bytes_per_instance": 96,
    "candidate_seed_bytes": 0,
    "published_buffer_copy_bytes": 0,
    "publication_store_count": 1,
    "commit_runtime_calls": 0,
    "legacy_journal_captures": 0,
    "trajectory_sha256": TRAJECTORY_SHA256,
}
EXPECTED_GATE_B_LINKAGE = {
    "contract": "benchmarks/runtime/gate-b/ekf-v1.json",
    "trace": "benchmarks/runtime/gate-b/ekf-input-v1.bin",
    "trace_sha256": TRACE_SHA256,
    "episode_length": EPISODE_LENGTH,
    "trajectory_sha256": TRAJECTORY_SHA256,
}
EXPECTED_PUBLICATION_CONTRACT = {
    "store_count": 1,
    "writer_ordering": "Release",
    "reader_ordering": "Acquire",
    "abort_preserves_published_epoch": True,
    "ordered_steps": [
        "reserve",
        "begin",
        "execute",
        "validate",
        "summary",
        "prepare",
        "publish",
        "append",
    ],
    "capacity_reserved_before_execution": True,
    "candidate_executes_before_receipt_preparation": True,
    "candidate_summary_before_receipt_preparation": True,
    "receipt_prepared_before_publish": True,
    "append_after_publish": "infallible",
}


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def render_json(value) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def schema_type_matches(value, expected: str) -> bool:
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


def json_schema_errors(value, schema, root_schema=None, path: str = "$") -> list[str]:
    """Validate the JSON-Schema vocabulary used by the three D0 schemas."""
    if schema is False:
        return [f"{path} is prohibited by the schema"]
    if schema is True or not schema:
        return []
    root_schema = schema if root_schema is None else root_schema
    if "$ref" in schema:
        reference = schema["$ref"]
        if not reference.startswith("#/"):
            return [f"{path} uses unsupported schema reference {reference}"]
        target = root_schema
        for component in reference[2:].split("/"):
            target = target[component.replace("~1", "/").replace("~0", "~")]
        return json_schema_errors(value, target, root_schema, path)
    if "allOf" in schema:
        errors = []
        for branch in schema["allOf"]:
            errors.extend(json_schema_errors(value, branch, root_schema, path))
        remainder = {key: child for key, child in schema.items() if key != "allOf"}
        if remainder:
            errors.extend(json_schema_errors(value, remainder, root_schema, path))
        return errors
    if "oneOf" in schema:
        branch_failures = [
            json_schema_errors(value, branch, root_schema, path)
            for branch in schema["oneOf"]
        ]
        matches = sum(not failures for failures in branch_failures)
        if matches != 1:
            return [f"{path} must match exactly one schema branch; matched {matches}"]
        return []

    errors = []
    expected_types = schema.get("type")
    if expected_types is not None:
        if isinstance(expected_types, str):
            expected_types = [expected_types]
        if not any(schema_type_matches(value, expected) for expected in expected_types):
            return [f"{path} has the wrong type"]
    if "const" in schema and value != schema["const"]:
        errors.append(f"{path} does not equal the schema constant")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path} is not one of the schema enum values")

    if isinstance(value, dict):
        properties = schema.get("properties", {})
        for required in schema.get("required", []):
            if required not in value:
                errors.append(f"{path} is missing required property {required}")
        if schema.get("additionalProperties") is False:
            for key in value.keys() - properties.keys():
                errors.append(f"{path}.{key} is not allowed")
        for key, child in value.items():
            if key in properties:
                errors.extend(json_schema_errors(child, properties[key], root_schema, f"{path}.{key}"))
    elif isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{path} has too few items")
        if len(value) > schema.get("maxItems", len(value)):
            errors.append(f"{path} has too many items")
        prefix_items = schema.get("prefixItems", [])
        for index, child in enumerate(value):
            child_schema = (
                prefix_items[index]
                if index < len(prefix_items)
                else schema.get("items", {})
            )
            errors.extend(json_schema_errors(child, child_schema, root_schema, f"{path}[{index}]"))
    elif isinstance(value, str):
        if len(value) < schema.get("minLength", 0):
            errors.append(f"{path} is too short")
    elif isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path} is below the schema minimum")
    return errors


def build_resident_activation_contract(root: Path = ROOT):
    """Build the permanent structural owner contract from the frozen target set."""
    frozen = read_json(root / FROZEN_TARGETS_PATH.relative_to(ROOT))
    copied_fields = (
        "id",
        "applies_to",
        "semantic_category",
        "representation",
        "key_semantics",
        "runtime_storage",
    )
    targets = [
        {field: target[field] for field in copied_fields}
        for target in sorted(frozen["targets"], key=lambda row: row["id"])
        if target["id"] in PERMANENT_TARGET_IDS
    ]
    return {
        "schema_version": 1,
        "contract": "resident-activation",
        "source_targets": "tests/architecture/value-system/frozen-semantic-targets-v1.json",
        "semantic_targets": targets,
        "activation_owners": [
            {
                "path": "src/engine/src/artifact/model.rs",
                "markers": ["pub struct ProgramArtifact"],
            },
            {
                "path": "src/engine/src/resident/general/mod.rs",
                "markers": ["pub fn activate(", "pub fn preflight_activation("],
            },
            {
                "path": "src/runtime/src/runtime/program/loading.rs",
                "markers": ["pub fn load_source_program(", "pub fn load_bytecode_program("],
            },
            {
                "path": "src/runtime/src/runtime/program/external/admission.rs",
                "markers": ["struct ResidentAdmissionProof"],
            },
        ],
        "obsolete_owners_absent": [
            "src/engine/src/program/instance.rs",
            "src/runtime/src/runtime/resident_program",
            "src/runtime/src/resident_external",
            "src/interpreter",
            "src/bin/interpreter2.rs",
        ],
    }


def source_operation_tokens(source: str) -> list[str]:
    tokens = []
    offset = 0
    while True:
        start = source.find("ekf/", offset)
        if start < 0:
            return tokens
        end = start
        while end < len(source) and (source[end].isalnum() or source[end] in "/-"):
            end += 1
        cursor = end
        while cursor < len(source) and source[cursor].isspace():
            cursor += 1
        if cursor < len(source) and source[cursor] == "(":
            tokens.append(source[start:end])
        offset = end


def validate_source_digest(
    source: bytes, workload_digest: str, digest_document: str
) -> list[str]:
    actual = sha256_bytes(source)
    failures = []
    if actual != EKF_SOURCE_SHA256:
        failures.append(
            f"EKF source bytes differ from frozen SHA-256 {EKF_SOURCE_SHA256}"
        )
    if workload_digest != EKF_SOURCE_SHA256:
        failures.append("workload source SHA-256 differs from the independently pinned digest")
    if digest_document != EKF_SOURCE_SHA256 + "\n":
        failures.append("ekf-source-v1.sha256 differs from the independently pinned digest")
    return failures


def validate_source(source: str, workload) -> list[str]:
    failures = []
    if workload.get("operations") != EXPECTED_OPERATIONS:
        failures.append("workload operations differ from the exact frozen 18-operation semantics")
    expected_names = [operation["operation"] for operation in EXPECTED_OPERATIONS]
    for name in expected_names:
        if source.count(name + "(") != 1:
            failures.append(f"source must contain {name} exactly once")
    exact_sections = {
        "gate_b": EXPECTED_GATE_B_LINKAGE,
        "input": EXPECTED_INPUT,
        "state": EXPECTED_STATE,
        "constants": EXPECTED_CONSTANTS,
        "integrity_constraints": EXPECTED_INTEGRITY_CONSTRAINTS,
        "state_updates": EXPECTED_STATE_UPDATES,
        "output": EXPECTED_OUTPUT,
        "d1_acceptance_targets": EXPECTED_D1_ACCEPTANCE_TARGETS,
    }
    for key, expected in exact_sections.items():
        if workload.get(key) != expected:
            failures.append(f"workload {key} differs from its exact frozen contract")
    unlisted = sorted(set(source_operation_tokens(source)) - set(expected_names))
    if unlisted:
        failures.append("source contains unlisted EKF operations: " + ", ".join(unlisted))
    if source.count("! :=") != 3:
        failures.append("source must contain exactly three integrity definitions")
    if source.count(
        "finite-candidate! := ekf/candidate-finite(corrected-state,\n  symmetrized-covariance)"
    ) != 1:
        failures.append("finite-candidate must validate both corrected state and symmetrized covariance")
    mutable_definitions = [line.strip() for line in source.splitlines() if line.lstrip().startswith("~")]
    if mutable_definitions != [
        "~state := [2.0; 1.0; 0.15]",
        "~covariance := [1.0, 0.0, 0.0; 0.0, 1.0, 0.0; 0.0, 0.0, 0.05]",
    ]:
        failures.append("source must contain exactly the frozen state and covariance definitions")
    assignments = [
        line.strip()
        for line in source.splitlines()
        if line.strip().startswith("state =") or line.strip().startswith("covariance =")
    ]
    if assignments != ["state = corrected-state", "covariance = symmetrized-covariance"]:
        failures.append("source must contain exactly the frozen state and covariance assignments")
    if [line.strip() for line in source.splitlines() if line.strip() == "estimate := state"] != ["estimate := state"]:
        failures.append("source must end with the frozen estimate binding")
    return failures


def validate_forbidden_tokens(root: Path = ROOT) -> list[str]:
    failures = []
    tokens = (
        "LegacyValue",
        "ValRef",
        "MutableReference",
        "ReactiveCellId",
        "commit_runtime",
        "RuntimeExecutionTransaction",
    )
    paths = (
        "tests/architecture/resident-activation/ekf-source-v1.mec",
        "tests/architecture/resident-activation/ekf-workload-v1.json",
        "tests/architecture/resident-activation/README.md",
        "src/engine/tests/resident_activation_contract.rs",
    )
    for relative in paths:
        path = root / relative
        if not path.exists():
            continue
        source = path.read_text(encoding="utf-8")
        if relative == "tests/architecture/resident-activation/ekf-workload-v1.json":
            source = source.replace('"commit_runtime_calls": 0', "")
        for token in tokens:
            if token in source:
                failures.append(f"{relative} contains forbidden resident dependency token {token}")
    return failures


def validate(root: Path = ROOT) -> list[str]:
    contract_dir = root / CONTRACT_DIR.relative_to(ROOT)
    source_path = contract_dir / "ekf-source-v1.mec"
    workload_path = contract_dir / "ekf-workload-v1.json"
    source_bytes = source_path.read_bytes()
    source = source_bytes.decode("utf-8")
    workload = read_json(workload_path)
    failures = []

    digest_path = contract_dir / "ekf-source-v1.sha256"
    digest_document = digest_path.read_text(encoding="utf-8") if digest_path.exists() else ""
    failures.extend(
        validate_source_digest(
            source_bytes,
            workload.get("source", {}).get("sha256", ""),
            digest_document,
        )
    )

    documents = (
        (contract_dir / "d0-boundary.json", contract_dir / "d0-boundary-schema.json"),
        (
            contract_dir / "resident-activation-contract.json",
            contract_dir / "resident-activation-contract-schema.json",
        ),
        (workload_path, contract_dir / "ekf-workload-v1-schema.json"),
    )
    for document_path, schema_path in documents:
        if not document_path.exists():
            failures.append(f"missing generated contract {document_path.relative_to(root)}")
            continue
        failures.extend(
            f"{document_path.relative_to(root)}: {error}"
            for error in json_schema_errors(read_json(document_path), read_json(schema_path))
        )

    boundary = read_json(contract_dir / "d0-boundary.json")
    if boundary.get("publication_contract") != EXPECTED_PUBLICATION_CONTRACT:
        failures.append("D0 publication contract differs from the exact reserve/execute/prepare/publish/append sequence")

    expected_activation_contract = render_json(build_resident_activation_contract(root))
    activation_contract_path = contract_dir / "resident-activation-contract.json"
    if (
        not activation_contract_path.exists()
        or activation_contract_path.read_text(encoding="utf-8")
        != expected_activation_contract
    ):
        failures.append(
            "resident-activation-contract.json is not the permanent structural projection"
        )
    permanent_target_ids = [
        target["id"]
        for target in build_resident_activation_contract(root)["semantic_targets"]
    ]
    if permanent_target_ids != list(PERMANENT_TARGET_IDS):
        failures.append(
            "permanent resident activation target set differs from its exact frozen target set"
        )

    trace_path = root / GATE_B_TRACE_PATH.relative_to(ROOT)
    if sha256_bytes(trace_path.read_bytes()) != TRACE_SHA256:
        failures.append("Gate B trace SHA-256 changed")
    gate_b = read_json(root / GATE_B_CONTRACT_PATH.relative_to(ROOT))
    if gate_b.get("episode_length") != EPISODE_LENGTH:
        failures.append("Gate B episode length changed")
    if gate_b.get("trace", {}).get("sha256") != TRACE_SHA256:
        failures.append("Gate B workload trace digest changed")
    if gate_b.get("reference", {}).get("quantized_trajectory_sha256") != TRAJECTORY_SHA256:
        failures.append("Gate B trajectory digest changed")
    if workload.get("gate_b") != EXPECTED_GATE_B_LINKAGE:
        failures.append("D0 workload does not exactly retain the Gate B linkage")

    failures.extend(validate_source(source, workload))
    failures.extend(validate_forbidden_tokens(root))
    return failures


def write_generated(root: Path = ROOT) -> None:
    contract_dir = root / CONTRACT_DIR.relative_to(ROOT)
    (contract_dir / "ekf-source-v1.sha256").write_text(
        EKF_SOURCE_SHA256 + "\n", encoding="utf-8"
    )
    (contract_dir / "resident-activation-contract.json").write_text(
        render_json(build_resident_activation_contract(root)), encoding="utf-8"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    if not arguments.check:
        write_generated()
    failures = validate()
    if failures:
        for failure in failures:
            print(f"resident activation generation failure: {failure}", file=sys.stderr)
        return 1
    if arguments.check:
        print("resident activation generated contract is current")
    else:
        print("generated resident activation contract")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
