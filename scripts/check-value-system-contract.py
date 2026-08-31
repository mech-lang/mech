#!/usr/bin/env python3
"""Validate permanent canonical value-system encoding and efficacy contracts."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_ROOT = ROOT / "tests/architecture/value-system"
DEFAULT_CANONICAL_ENCODING = CONTRACT_ROOT / "canonical-encoding-v1.json"
DEFAULT_CANONICAL_ENCODING_SCHEMA = CONTRACT_ROOT / "canonical-encoding-v1-schema.json"
DEFAULT_CANONICAL_VECTORS = CONTRACT_ROOT / "canonical-encoding-v1-vectors.json"
DEFAULT_CANONICAL_VECTORS_SCHEMA = CONTRACT_ROOT / "canonical-encoding-v1-vectors-schema.json"
DEFAULT_GATE_B = CONTRACT_ROOT / "gate-b-regression.json"
CANONICAL_REFERENCE_PATH = ROOT / "scripts/tests/canonical_encoding_v1_reference.py"

EXPECTED_HASH_CONTRACTS_V1 = {
    "NominalKey": {
        "algorithm": "SHA-256",
        "bytes": 32,
        "domain_separator_utf8": "mech-nominal-v1\0",
        "input": [
            "domain-separator",
            "U8-nominal-kind-tag",
            "U32-segment-count",
            "each-segment-as-Utf8",
        ],
    },
    "SchemaKey": {
        "algorithm": "SHA-256",
        "bytes": 32,
        "domain_separator_utf8": "mech-schema-v1\0",
        "input": ["domain-separator", "canonical-schema-bytes"],
    },
    "KeyHash": {
        "algorithm": "SHA-256",
        "bytes": 32,
        "domain_separator_utf8": "mech-key-v1\0",
        "input": [
            "domain-separator",
            "SchemaKey",
            "canonical-shape-bytes",
            "canonical-key-payload-bytes",
        ],
    },
    "ValueHash": {
        "algorithm": "SHA-256",
        "bytes": 32,
        "domain_separator_utf8": "mech-value-v1\0",
        "durability": "durable-snapshot-identity",
        "input": [
            "domain-separator",
            "SchemaKey",
            "canonical-shape-bytes",
            "canonical-payload-bytes",
        ],
    },
}

def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ValueError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module

@dataclass(frozen=True)
class Failure:
    contract_id: str
    subject: str
    path: str
    line: int | None
    column: int | None
    expected: str
    actual: str
    suggestion: str
    enum_name: str
    variant: str

    def render(self) -> str:
        return (
            f"[{self.contract_id}] enum={self.enum_name} variant={self.variant} "
            f"subject={self.subject} path={self.path} "
            f"line={self.line if self.line is not None else '-'} "
            f"column={self.column if self.column is not None else '-'}; "
            f"expected={self.expected}; actual={self.actual}; "
            f"update={self.suggestion}"
        )

def failure(
    contract_id: str,
    subject: str,
    path: str,
    expected: str,
    actual: str,
    suggestion: str,
    line: int | None = None,
    column: int | None = None,
    enum_name: str = "-",
    variant: str = "-",
) -> Failure:
    return Failure(
        contract_id,
        subject,
        path,
        line,
        column,
        expected,
        actual,
        suggestion,
        enum_name,
        variant,
    )

def json_type_matches(value: Any, expected: str) -> bool:
    mapping = {
        "array": list,
        "boolean": bool,
        "integer": int,
        "null": type(None),
        "number": (int, float),
        "object": dict,
        "string": str,
    }
    if expected not in mapping:
        raise ValueError(f"unsupported JSON Schema type {expected!r}")
    if expected in {"integer", "number"} and isinstance(value, bool):
        return False
    return isinstance(value, mapping[expected])

def resolve_ref(root_schema: dict[str, Any], reference: str) -> dict[str, Any]:
    if not reference.startswith("#/"):
        raise ValueError(f"only local JSON Schema refs are supported: {reference}")
    value: Any = root_schema
    for component in reference[2:].split("/"):
        value = value[component.replace("~1", "/").replace("~0", "~")]
    if not isinstance(value, dict):
        raise ValueError(f"JSON Schema ref is not an object: {reference}")
    return value

def schema_errors(
    value: Any,
    schema: dict[str, Any],
    root_schema: dict[str, Any] | None = None,
    path: str = "$",
) -> list[str]:
    root_schema = schema if root_schema is None else root_schema
    if "$ref" in schema:
        return schema_errors(value, resolve_ref(root_schema, schema["$ref"]), root_schema, path)
    errors: list[str] = []
    expected_type = schema.get("type")
    if expected_type is not None:
        candidates = expected_type if isinstance(expected_type, list) else [expected_type]
        if not any(json_type_matches(value, candidate) for candidate in candidates):
            return [f"{path}: expected type {candidates}, got {type(value).__name__}"]
    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: expected constant {schema['const']!r}, got {value!r}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: value {value!r} is not in {schema['enum']!r}")
    if isinstance(value, str):
        if len(value) < schema.get("minLength", 0):
            errors.append(f"{path}: string is shorter than minLength")
        if "pattern" in schema and re.search(schema["pattern"], value) is None:
            errors.append(f"{path}: string does not match {schema['pattern']!r}")
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path}: number is below minimum {schema['minimum']}")
    if isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{path}: array is shorter than minItems")
        if schema.get("uniqueItems") is True:
            rendered = [json.dumps(item, sort_keys=True) for item in value]
            if len(rendered) != len(set(rendered)):
                errors.append(f"{path}: array items are not unique")
        if "items" in schema:
            for index, item in enumerate(value):
                errors.extend(
                    schema_errors(item, schema["items"], root_schema, f"{path}[{index}]")
                )
    if isinstance(value, dict):
        required = schema.get("required", [])
        for key in required:
            if key not in value:
                errors.append(f"{path}: missing required property {key!r}")
        properties = schema.get("properties", {})
        additional = schema.get("additionalProperties", True)
        for key, item in value.items():
            child_path = f"{path}.{key}"
            if key in properties:
                errors.extend(
                    schema_errors(item, properties[key], root_schema, child_path)
                )
            elif additional is False:
                errors.append(f"{path}: additional property {key!r} is forbidden")
            elif isinstance(additional, dict):
                errors.extend(schema_errors(item, additional, root_schema, child_path))
    return errors

def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value

def select_lane(
    report: dict[str, Any],
    name: str,
    *,
    history: int = 0,
    next_epoch: int = 1,
) -> dict[str, Any] | None:
    matches = [
        lane
        for lane in report["lanes"]
        if lane["lane"] == name
        and lane["instances"] == 1
        and lane.get("retained_history", 0) == history
        and lane.get("next_epoch", 1) == next_epoch
    ]
    return matches[0] if len(matches) == 1 else None

def gate_b_failures(
    root: Path,
    contract: dict[str, Any],
    contract_path: Path,
) -> list[Failure]:
    failures: list[Failure] = []
    evidence_path = root / contract["evidence_path"]
    try:
        evidence_bytes = evidence_path.read_bytes()
        report = json.loads(evidence_bytes)
    except (OSError, json.JSONDecodeError) as error:
        return [
            failure(
                "C0-GATE-B-REGRESSION",
                "evidence",
                contract["evidence_path"],
                "readable exact B2 JSON evidence",
                str(error),
                str(contract_path),
            )
        ]
    expected_hash = contract["evidence_sha256"]
    actual_hash = hashlib.sha256(evidence_bytes).hexdigest()
    checks: list[tuple[str, Any, Any]] = [
        ("evidence sha256", actual_hash, expected_hash),
        ("evidence commit", report.get("git_commit"), contract["evidence_commit"]),
        ("phase", report.get("phase"), "B2-resident-turn"),
        ("decision", report.get("b2_decision", {}).get("decision"), "Pass"),
    ]
    thresholds = contract["thresholds"]
    decision = report.get("b2_decision", {})
    numeric_checks = [
        ("raw_epoch_ratio", decision.get("raw_epoch_ratio"), "<=", thresholds["raw_epoch_ratio_max"]),
        ("legacy_gap_closure", decision.get("legacy_gap_closure"), ">=", thresholds["legacy_gap_closure_min"]),
        ("history_1k", decision.get("history_1k_over_history_0_median_ratio"), "<=", thresholds["history_1k_ratio_max"]),
        ("history_100k", decision.get("history_100k_over_history_0_median_ratio"), "<=", thresholds["history_100k_ratio_max"]),
        ("high_epoch", decision.get("high_epoch_over_low_epoch_median_ratio"), "<=", thresholds["high_epoch_ratio_max"]),
    ]
    for subject, actual, expected in checks:
        if actual != expected:
            failures.append(
                failure(
                    "C0-GATE-B-REGRESSION",
                    subject,
                    contract["evidence_path"],
                    repr(expected),
                    repr(actual),
                    str(contract_path),
                )
            )
    for subject, actual, operator, limit in numeric_checks:
        valid = isinstance(actual, (int, float)) and not isinstance(actual, bool)
        valid = valid and ((actual <= limit) if operator == "<=" else (actual >= limit))
        if not valid:
            failures.append(
                failure(
                    "C0-GATE-B-REGRESSION",
                    subject,
                    contract["evidence_path"],
                    f"{operator} {limit}",
                    repr(actual),
                    str(contract_path),
                )
            )
    turn = select_lane(report, "mech-resident-turn")
    full = select_lane(report, "mech-resident-turn-full-write")
    if turn is None or full is None:
        failures.append(
            failure(
                "C0-GATE-B-REGRESSION",
                "resident lanes",
                contract["evidence_path"],
                "one primary and one full-write complete resident lane",
                "missing or duplicate",
                str(contract_path),
            )
        )
        return failures
    resident_lanes = [
        lane
        for lane in report["lanes"]
        if lane["lane"].startswith("mech-resident-")
    ]
    if any(
        lane["allocation"].get("episode_allocation_count")
        != thresholds["steady_state_allocation_count"]
        for lane in resident_lanes
    ):
        failures.append(
            failure(
                "C0-GATE-B-REGRESSION",
                "steady-state allocations",
                contract["evidence_path"],
                str(thresholds["steady_state_allocation_count"]),
                "one or more resident lanes allocate",
                str(contract_path),
            )
        )
    structural_checks = (
        ("primary publication", turn["structural"].get("publication_store_count"), thresholds["publication_store_count"]),
        ("full publication", full["structural"].get("publication_store_count"), thresholds["publication_store_count"]),
        ("full candidate seed bytes", full["structural"].get("candidate_seed_bytes"), thresholds["full_write_candidate_seed_bytes"]),
        ("primary published copy bytes", turn["structural"].get("published_buffer_copy_bytes"), thresholds["published_buffer_copy_bytes"]),
        ("full published copy bytes", full["structural"].get("published_buffer_copy_bytes"), thresholds["published_buffer_copy_bytes"]),
        ("primary append infallible", turn["structural"].get("post_publication_append_infallible"), thresholds["post_publication_append_infallible"]),
        ("full append infallible", full["structural"].get("post_publication_append_infallible"), thresholds["post_publication_append_infallible"]),
    )
    for subject, actual, expected in structural_checks:
        if actual != expected:
            failures.append(
                failure(
                    "C0-GATE-B-REGRESSION",
                    subject,
                    contract["evidence_path"],
                    repr(expected),
                    repr(actual),
                    str(contract_path),
                )
            )
    required_lanes = set(contract["validation_policy"]["required_evidence_lanes"])
    report_lanes = {lane.get("lane") for lane in report["lanes"]}
    if not required_lanes.issubset(report_lanes):
        failures.append(
            failure(
                "C0-GATE-B-REGRESSION",
                "controlled-session lanes",
                contract["evidence_path"],
                repr(sorted(required_lanes)),
                repr(sorted(required_lanes - report_lanes)),
                str(contract_path),
            )
        )
    return failures

def canonical_encoding_failures(
    canonical: dict[str, Any], canonical_path: Path
) -> list[Failure]:
    schema_tags = {
        "Bool": 1,
        "UnsignedInteger": 2,
        "SignedInteger": 3,
        "FloatingPoint": 4,
        "Complex": 5,
        "Rational": 6,
        "String": 7,
        "Id": 8,
        "Index": 9,
        "Atom": 10,
        "Enum": 11,
        "Option": 12,
        "Tuple": 13,
        "Record": 14,
        "Matrix": 15,
        "Table": 16,
        "Set": 17,
        "Map": 18,
        "ReifiedType": 19,
    }
    dimension_tags = {
        "Constant": 1,
        "Parameter": 2,
        "Add": 3,
        "Multiply": 4,
        "Min": 5,
        "Max": 6,
    }
    kind_tags = {
        "Wildcard": 1,
        "Never": 2,
        "Hole": 3,
        "Named": 4,
        "Id": 5,
        "Index": 6,
        "Atom": 7,
        "Enum": 8,
        "Matrix": 9,
        "Option": 10,
        "Tuple": 11,
        "Record": 12,
        "Table": 13,
        "Set": 14,
        "Map": 15,
        "Reference": 16,
        "TypeOf": 17,
    }
    schema_checks = (
        (
            "complete hash contracts",
            canonical.get("hashes"),
            EXPECTED_HASH_CONTRACTS_V1,
        ),
        (
            "SchemaKey hash contract",
            canonical.get("hashes", {}).get("SchemaKey"),
            {
                "algorithm": "SHA-256",
                "bytes": 32,
                "domain_separator_utf8": "mech-schema-v1\0",
                "input": ["domain-separator", "canonical-schema-bytes"],
            },
        ),
        ("ValueHash hash", canonical.get("hashes", {}).get("ValueHash", {}).get("algorithm"), "SHA-256"),
        (
            "NominalKey domain separator",
            canonical.get("hashes", {}).get("NominalKey", {}).get("domain_separator_utf8"),
            "mech-nominal-v1\0",
        ),
        (
            "nominal path segment derivation",
            canonical.get("nominal_identity", {}).get("segment_derivation"),
            {
                "collision_rule": "same-complete-CanonicalNominalPath-from-distinct-Cargo-package-IDs-in-one-ProgramArtifact-is-AmbiguousNominalDeclarationV1",
                "crate_aliases": "excluded",
                "module_source": "defining-module-canonical-namespace-relative-to-package-root",
                "package_name_source": "resolved-package-manifest-declared-name-exact-Utf8",
                "package_version_and_dependency_source": "excluded",
                "reexports": "resolve-to-the-defining-declaration",
                "segments": [
                    "resolved-package-name",
                    "zero-or-more-defining-module-namespace-segments",
                    "declaration-name",
                ],
            },
        ),
        (
            "golden vector frozen digest",
            canonical.get("golden_vectors"),
            {
                "canonical_json_sha256": "0be9531a4514ef359bedc3172f6ea327c28bcaab0fa493d9725d430799343a9f",
                "encoding_name": "MechSnapshotEncodingV1",
                "schema_version": 3,
            },
        ),
        ("SchemaKey bytes", canonical.get("hashes", {}).get("SchemaKey", {}).get("bytes"), 32),
        ("ValueHash bytes", canonical.get("hashes", {}).get("ValueHash", {}).get("bytes"), 32),
        ("byte order", canonical.get("primitive_encoding", {}).get("byte_order"), "little-endian"),
        ("Index semantic type", canonical.get("primitive_encoding", {}).get("index", {}).get("semantic_type"), "u64"),
        ("lengths", canonical.get("primitive_encoding", {}).get("lengths_and_cardinalities"), "u64"),
        ("rank", canonical.get("primitive_encoding", {}).get("rank"), "u32"),
        ("enum ordinal", canonical.get("primitive_encoding", {}).get("enum_variant_ordinal"), "u32"),
        (
            "schema root frame",
            canonical.get("schema_encoding", {}).get("root"),
            [
                "U8-version-0x01",
                "U32-dimension-parameter-count",
                "dimension-parameter-frames",
                "Node-root-schema-body",
            ],
        ),
        (
            "schema variant tags",
            canonical.get("schema_encoding", {}).get("tags"),
            schema_tags,
        ),
        (
            "dimension expression tags",
            canonical.get("dimension_expression_encoding", {}).get("tags"),
            dimension_tags,
        ),
        (
            "dimension constant overflow",
            canonical.get("dimension_expression_encoding", {}).get(
                "constant_overflow"
            ),
            "checked-u64-overflow-is-invalid",
        ),
        (
            "shape frame",
            canonical.get("shape_encoding", {}).get("frame"),
            [
                "U8-version-0x01",
                "U32-resolved-parameter-count",
                "U64-resolved-values-in-schema-parameter-order",
            ],
        ),
        (
            "shape parameter order",
            canonical.get("shape_encoding", {}).get("parameter_order"),
            "canonical-retained-schema-parameter-order",
        ),
        (
            "malformed canonical input errors",
            canonical.get("validation_errors"),
            {
                "AggregateArityMismatchV1": "tuple-payload-or-key-arity-does-not-equal-schema-arity",
                "AggregateFieldMismatchV1": "record-or-table-field-name-set-does-not-equal-schema-name-set",
                "DuplicateSchemaNameV1": "record-field-table-column-or-enum-variant-name-is-duplicated",
                "EnumOrdinalOutOfRangeV1": "enum-ordinal-is-not-an-in-range-integer",
                "EnumPayloadMismatchV1": "enum-payload-presence-or-fields-do-not-match-selected-variant",
                "InvalidSchemaWidthV1": "primitive-width-is-not-one-of-the-exact-V1-widths",
                "MapEntryArityMismatchV1": "map-entry-does-not-contain-exactly-key-and-value",
                "PayloadCardinalityMismatchV1": "option-matrix-table-set-or-map-payload-cardinality-does-not-match-schema",
                "ShapeBoundViolationV1": "shape-value-is-outside-its-resolved-lower-or-upper-bound",
                "ShapeParameterCountMismatchV1": "shape-value-count-does-not-equal-canonical-retained-parameter-count",
            },
        ),
        (
            "KindExpr tags",
            canonical.get("kind_expression_encoding", {}).get("tags"),
            kind_tags,
        ),
        (
            "dimension parameter framing",
            canonical.get("dimension_parameters", {}).get("frame"),
            [
                "U8-lifetime-tag",
                "Node-lower-bound",
                "U8-upper-present",
                "Node-upper-bound-only-when-present",
            ],
        ),
        (
            "record and table names",
            canonical.get("schema_encoding", {}).get("record_and_table_names"),
            "unique-valid-Utf8-in-declaration-order-not-sorted",
        ),
        (
            "recursive schema rejection",
            canonical.get("schema_encoding", {}).get("recursive_error"),
            "RecursiveSchemaUnsupportedV1",
        ),
        (
            "schema keyability derivation",
            canonical.get("schema_encoding", {}).get("keyability", {}).get("storage"),
            "derived-from-schema-body-not-encoded-as-independent-flag",
        ),
        (
            "recursive child framing",
            canonical.get("common_framing", {}).get("recursive_children"),
            "every-schema-kind-and-dimension-child-is-Node-framed",
        ),
    )
    key_checks = (
        ("KeyHash algorithm", canonical.get("hashes", {}).get("KeyHash", {}).get("algorithm"), "SHA-256"),
        ("KeyHash bytes", canonical.get("hashes", {}).get("KeyHash", {}).get("bytes"), 32),
        (
            "KeyHash domain",
            canonical.get("hashes", {}).get("KeyHash", {}).get("domain_separator_utf8"),
            "mech-key-v1\0",
        ),
        (
            "KeyHash framing",
            canonical.get("hashes", {}).get("KeyHash", {}).get("input"),
            [
                "domain-separator",
                "SchemaKey",
                "canonical-shape-bytes",
                "canonical-key-payload-bytes",
            ],
        ),
        (
            "schema keyability",
            canonical.get("schema_encoding", {}).get("keyability"),
            {
                "always_keyable": [
                    "Bool",
                    "UnsignedInteger",
                    "SignedInteger",
                    "FloatingPoint",
                    "Rational",
                    "String",
                    "Id",
                    "Index",
                    "Atom",
                ],
                "recursively_keyable": ["Enum", "Option", "Tuple", "Record"],
                "not_keyable": [
                    "Complex",
                    "Matrix",
                    "Table",
                    "Set",
                    "Map",
                    "ReifiedType",
                ],
                "storage": "derived-from-schema-body-not-encoded-as-independent-flag",
            },
        ),
        (
            "floating-point key normalization",
            canonical.get("key_encoding", {}).get("float_normalization"),
            {
                "F32_NaN": "0x7fc00000",
                "F32_negative_zero": "0x00000000",
                "F64_NaN": "0x7ff8000000000000",
                "F64_negative_zero": "0x0000000000000000",
                "non_NaN_non_negative_zero": "retain-original-bits",
                "order": "IEEE-754-totalOrder-after-normalization",
                "snapshot_payload": "exact-bits-without-key-normalization",
            },
        ),
        (
            "Rational64 key semantics",
            canonical.get("key_encoding", {}).get("rational64"),
            {
                "comparison": "i128(n1)*i128(d2)-compared-with-i128(n2)*i128(d1)",
                "denominator": "positive-u64",
                "error": "NonCanonicalRationalV1",
                "numerator": "signed-i64",
                "reduction": "greatest-common-divisor-is-1",
                "zero": "exactly-0-over-1",
            },
        ),
        (
            "Rational64 key order",
            canonical.get("key_order", {}).get("rational64"),
            "compare-i128-n1-times-d2-with-i128-n2-times-d1",
        ),
        (
            "map and set normalization",
            canonical.get("key_encoding", {}).get("collection_normalization"),
            [
                "convert-every-key-to-canonical-key-payload",
                "compare-by-schema-KeyOrder",
                "detect-equality-by-KeyEquality",
                "reject-duplicates-as-DuplicateCanonicalKeyV1",
                "sort-set-elements-and-map-entries-by-KeyOrder",
                "encode-canonical-key-payload-not-original-float-bits",
            ],
        ),
        (
            "duplicate canonical key error",
            canonical.get("key_encoding", {}).get("duplicate_error"),
            "DuplicateCanonicalKeyV1",
        ),
        (
            "not-keyable schema error",
            canonical.get("key_encoding", {}).get("schema_not_keyable_error"),
            "SchemaNotKeyableV1",
        ),
    )
    payload_checks = (
        (
            "ValueHash domain",
            canonical.get("hashes", {}).get("ValueHash", {}).get("domain_separator_utf8"),
            "mech-value-v1\0",
        ),
        (
            "ValueHash framing",
            canonical.get("hashes", {}).get("ValueHash", {}).get("input"),
            [
                "domain-separator",
                "SchemaKey",
                "canonical-shape-bytes",
                "canonical-payload-bytes",
            ],
        ),
        (
            "schema-directed payload encoding",
            canonical.get("payload_encoding"),
            {
                "Atom": "empty-payload-NominalKey-is-in-Schema",
                "Bool": "U8-exactly-0-or-1",
                "Complex": "real-floating-component-then-imaginary-floating-component",
                "Enum": "U32-variant-ordinal-then-declared-variant-payload-when-present",
                "FloatingPoint": "exact-IEEE-bits-at-declared-width-little-endian",
                "Id": "U64-little-endian",
                "Index": "U64-little-endian",
                "Map": "U64-entry-count-then-canonical-key-payload-and-value-snapshot-payload-sorted-by-KeyOrder",
                "Matrix": "element-payloads-in-logical-lexicographic-index-order-last-dimension-fastest-shape-not-repeated",
                "Option": "U8-0-absent-or-U8-1-then-element-payload",
                "Rational64": "i64-numerator-little-endian-then-u64-denominator-little-endian",
                "Record": "child-payloads-in-schema-field-order-without-field-names",
                "ReifiedType": "U8-1-then-Node-closed-KindExpr-or-U8-2-then-32-byte-SchemaKey",
                "Set": "U64-element-count-then-canonical-key-payloads-sorted-by-KeyOrder",
                "SignedInteger": "declared-fixed-width-twos-complement-little-endian",
                "String": "U64-byte-length-then-exact-valid-UTF8-bytes",
                "Table": "schema-column-order-then-each-row-in-semantic-row-order-row-count-not-repeated",
                "Tuple": "child-payloads-in-positional-order",
                "UnsignedInteger": "declared-fixed-width-little-endian",
            },
        ),
    )
    dimension_checks = (
        (
            "checked dimension overflow",
            canonical.get("dimension_expression_encoding", {}).get("constant_overflow"),
            "checked-u64-overflow-is-invalid",
        ),
        (
            "dimension errors",
            canonical.get("dimension_expression_encoding", {}).get("errors"),
            {
                "empty_min_max": "EmptyMinMaxV1",
                "overflow": "DimensionOverflowV1",
                "unknown_parameter": "UnknownDimensionParameterV1",
            },
        ),
        (
            "recursive dimension canonicalization",
            canonical.get("dimension_expression_encoding", {}).get("canonicalization"),
            {
                "Add": [
                    "flatten-same-tag",
                    "recursively-canonicalize",
                    "fold-constants",
                    "remove-zero",
                    "sort-encoded-operands-lexicographically",
                    "zero-operands-is-Constant-0",
                ],
                "Multiply": [
                    "flatten-same-tag",
                    "recursively-canonicalize",
                    "fold-constants",
                    "remove-one",
                    "zero-factor-is-Constant-0",
                    "sort-encoded-operands-lexicographically",
                    "zero-operands-is-Constant-1",
                ],
                "Min": [
                    "flatten-same-tag",
                    "recursively-canonicalize",
                    "remove-duplicates",
                    "sort-encoded-operands-lexicographically",
                    "zero-operands-invalid",
                ],
                "Max": [
                    "flatten-same-tag",
                    "recursively-canonicalize",
                    "remove-duplicates",
                    "sort-encoded-operands-lexicographically",
                    "zero-operands-invalid",
                ],
                "one_operand": "canonicalizes-to-the-operand",
            },
        ),
        (
            "dimension parameter errors",
            canonical.get("dimension_parameters", {}).get("errors"),
            {
                "cycle": "CyclicDimensionParameterBoundsV1",
                "forward_reference": "ForwardDimensionParameterReferenceV1",
                "unknown_parameter": "UnknownDimensionParameterV1",
            },
        ),
        (
            "dimension parameter canonicalization",
            canonical.get("dimension_parameters", {}).get("canonicalization"),
            [
                "parameter-names-not-encoded",
                "remove-unused-parameters",
                "rewrite-references-to-zero-based-U32-ordinals",
                "retain-explicit-parameters-in-source-declaration-order",
                "append-inferred-parameters-in-first-preorder-occurrence-order",
                "include-parameters-referenced-by-reachable-bounds",
                "bounds-may-reference-only-earlier-retained-parameters",
                "parameter-bound-dependencies-must-be-acyclic",
            ],
        ),
    )
    failures: list[Failure] = []
    for contract_id, checks in (
        ("C0-CANONICAL-SCHEMA-ENCODING", schema_checks),
        ("C0-KEY-SEMANTICS", key_checks),
        ("C0-CANONICAL-PAYLOAD-ENCODING", payload_checks),
        ("C0-DIMENSION-NORMALIZATION", dimension_checks),
    ):
        failures.extend(
            failure(
                contract_id,
                subject,
                str(canonical_path),
                repr(expected),
                repr(actual),
                str(canonical_path),
            )
            for subject, actual, expected in checks
            if actual != expected
        )
    return failures

def canonical_json_sha256(value: Any) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()

def canonical_vector_failures(
    vectors: dict[str, Any],
    vectors_path: Path,
    canonical: dict[str, Any] | None = None,
) -> list[Failure]:
    groups = {
        "value_vectors": (
            {
                "bool",
                "f64",
                "fixed-matrix-f64-2x3",
                "activation-vector-f64-n-0-to-1024",
                "tuple-bool-u64",
                "record-x-y-f64",
                "option-string",
                "enum-two-variants-one-payload",
                "reified-type-schema",
                "set-f64",
                "map-string-u64",
            },
            CANONICAL_REFERENCE.reproduce_value,
            "C0-CANONICAL-PAYLOAD-ENCODING",
        ),
        "key_vectors": (
            {
                "f64-signed-zero-equivalence",
                "f64-nan-equivalence",
                "duplicate-canonical-set-keys",
                "rational64-order",
                "complex-not-keyable",
            },
            CANONICAL_REFERENCE.reproduce_key,
            "C0-KEY-SEMANTICS",
        ),
        "dimension_vectors": (
            {
                "nested-add",
                "nested-multiply",
                "nested-min",
                "nested-max",
                "add-overflow",
                "multiply-overflow",
                "unknown-parameter",
            },
            CANONICAL_REFERENCE.reproduce_dimension,
            "C0-DIMENSION-NORMALIZATION",
        ),
        "invalid_value_vectors": (
            {
                "tuple-too-short",
                "tuple-too-long",
                "record-missing-field",
                "record-extra-field",
                "option-present-missing-value",
                "enum-ordinal-out-of-range",
                "enum-missing-payload",
                "enum-unexpected-payload",
                "matrix-element-count-mismatch",
                "table-column-set-mismatch",
                "table-row-count-mismatch",
                "set-cardinality-mismatch",
                "map-entry-arity-mismatch",
                "map-cardinality-mismatch",
                "shape-parameter-count-mismatch",
                "shape-lower-bound-violation",
                "shape-upper-bound-violation",
            },
            CANONICAL_REFERENCE.reproduce_invalid_value,
            "C0-CANONICAL-INVALID-VALUE",
        ),
        "invalid_schema_vectors": (
            {
                "duplicate-record-field",
                "duplicate-table-column",
                "duplicate-enum-variant",
                "invalid-unsigned-integer-width",
                "invalid-signed-integer-width",
                "invalid-floating-point-width",
                "invalid-complex-width",
                "invalid-rational-width",
            },
            CANONICAL_REFERENCE.reproduce_invalid_schema,
            "C0-CANONICAL-INVALID-SCHEMA",
        ),
    }
    failures: list[Failure] = []
    canonical = canonical or load_json(DEFAULT_CANONICAL_ENCODING)
    expected_digest = canonical.get("golden_vectors", {}).get(
        "canonical_json_sha256"
    )
    actual_digest = canonical_json_sha256(vectors)
    if actual_digest != expected_digest:
        failures.append(
            failure(
                "C0-CANONICAL-VECTOR-FREEZE",
                "golden vector inputs and outputs",
                str(vectors_path),
                repr(expected_digest),
                repr(actual_digest),
                str(DEFAULT_CANONICAL_ENCODING),
            )
        )
    if vectors.get("schema_version") != 3:
        failures.append(
            failure(
                "C0-CANONICAL-PAYLOAD-ENCODING",
                "golden vector schema version",
                str(vectors_path),
                "3",
                repr(vectors.get("schema_version")),
                str(vectors_path),
            )
        )
    for group, (expected_ids, reproduce, contract_id) in groups.items():
        rows = vectors.get(group, [])
        actual_ids = [vector.get("id") for vector in rows]
        if set(actual_ids) != expected_ids or len(actual_ids) != len(expected_ids):
            failures.append(
                failure(
                    contract_id,
                    f"{group} inventory",
                    str(vectors_path),
                    repr(sorted(expected_ids)),
                    repr(actual_ids),
                    str(vectors_path),
                )
            )
        for vector in rows:
            try:
                actual = reproduce(vector)
            except (KeyError, TypeError, ValueError, OverflowError) as error:
                actual = {"encoder_error": str(error)}
            expected = vector.get("expected")
            if actual != expected:
                failures.append(
                    failure(
                        contract_id,
                        vector.get("id", "golden vector"),
                        str(vectors_path),
                        repr(expected),
                        repr(actual),
                        str(vectors_path),
                    )
                )
    return failures

def sorted_failures(failures: Iterable[Failure]) -> list[Failure]:
    return sorted(
        set(failures),
        key=lambda item: (
            item.contract_id,
            item.enum_name,
            item.variant,
            item.path,
            -1 if item.line is None else item.line,
            -1 if item.column is None else item.column,
            item.subject,
            item.actual,
        ),
    )

CANONICAL_REFERENCE = load_module(
    "canonical_encoding_v1_reference", CANONICAL_REFERENCE_PATH
)


def permanent_boundary_failures(root: Path) -> list[Failure]:
    failures: list[Failure] = []
    schema_root = root / "src/core/src/schema"
    dependency = re.compile(r"\b(?:mech_runtime|mech_engine|crate::runtime|crate::engine)\b")
    for path in schema_root.rglob("*.rs"):
        source = path.read_text(encoding="utf-8")
        match = dependency.search(source)
        if match is not None:
            failures.append(
                failure(
                    "C0-SCHEMA-DEPENDENCY",
                    match.group(0),
                    path.relative_to(root).as_posix(),
                    "schema module independent of runtime and engine crates",
                    "forbidden dependency",
                    "src/core/src/schema/",
                    source.count("\n", 0, match.start()) + 1,
                )
            )
    for relative, markers in {
        "src/core/src/kind_expr.rs": (
            "validate_kind_structure(kind)?;",
            "KindNameCategory::RecordField",
            "KindNameCategory::TableColumn",
        ),
        "src/core/src/kind_scheme.rs": ("validate_kind_structure(kind)?;",),
        "src/core/src/schema/shape.rs": (".checked_mul(extent)",),
    }.items():
        source = (root / relative).read_text(encoding="utf-8")
        for marker in markers:
            if marker not in source:
                failures.append(
                    failure(
                        "C1-VALIDATED-CONSTRUCTION-ROUTE",
                        marker,
                        relative,
                        "canonical semantic construction retains every validated entry route",
                        "validation marker absent",
                        "src/core/tests/semantic_serde_contract.rs",
                    )
                )
    canonical_test = root / "src/core/tests/canonical_schema_vectors.rs"
    source = canonical_test.read_text(encoding="utf-8")
    for marker in (".finalize()", ".instantiate_shape("):
        if marker not in source:
            failures.append(
                failure(
                    "C1-FINALIZED-CONSTRUCTION",
                    marker,
                    canonical_test.relative_to(root).as_posix(),
                    "canonical conformance uses validated construction APIs",
                    "validation marker absent",
                    "src/core/tests/semantic_serde_contract.rs",
                )
            )
    return failures


def audit(
    root: Path,
    canonical_path: Path,
    canonical_schema_path: Path,
    vectors_path: Path,
    vectors_schema_path: Path,
    gate_b_path: Path,
) -> list[Failure]:
    canonical = load_json(canonical_path)
    canonical_schema = load_json(canonical_schema_path)
    vectors = load_json(vectors_path)
    vectors_schema = load_json(vectors_schema_path)
    failures: list[Failure] = []
    for contract_id, value, schema, path in (
        ("C0-CANONICAL-ENCODING-SCHEMA", canonical, canonical_schema, canonical_path),
        ("C0-CANONICAL-VECTORS-SCHEMA", vectors, vectors_schema, vectors_path),
    ):
        for message in schema_errors(value, schema):
            failures.append(
                failure(
                    contract_id,
                    "json-schema",
                    str(path),
                    "document valid against committed schema",
                    message,
                    str(path),
                )
            )
    if failures:
        return sorted_failures(failures)
    failures.extend(canonical_encoding_failures(canonical, canonical_path))
    failures.extend(canonical_vector_failures(vectors, vectors_path, canonical))
    failures.extend(gate_b_failures(root, load_json(gate_b_path), gate_b_path))
    failures.extend(permanent_boundary_failures(root))
    return sorted_failures(failures)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--canonical-encoding", type=Path, default=DEFAULT_CANONICAL_ENCODING)
    parser.add_argument("--canonical-encoding-schema", type=Path, default=DEFAULT_CANONICAL_ENCODING_SCHEMA)
    parser.add_argument("--canonical-vectors", type=Path, default=DEFAULT_CANONICAL_VECTORS)
    parser.add_argument("--canonical-vectors-schema", type=Path, default=DEFAULT_CANONICAL_VECTORS_SCHEMA)
    parser.add_argument("--gate-b", type=Path, default=DEFAULT_GATE_B)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        failures = audit(
            args.root.resolve(),
            args.canonical_encoding,
            args.canonical_encoding_schema,
            args.canonical_vectors,
            args.canonical_vectors_schema,
            args.gate_b,
        )
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"value-system contract checker failed internally: {error}", file=sys.stderr)
        return 2
    if not failures:
        print("permanent value-system contract passed")
        return 0
    print("permanent value-system contract failed:", file=sys.stderr)
    for item in failures:
        print(f"  {item.render()}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
