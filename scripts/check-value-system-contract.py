#!/usr/bin/env python3
"""Validate the Gate C0 value semantics, inventory, and migration boundary."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_ROOT = ROOT / "tests/architecture/value-system"
DEFAULT_INVENTORY = CONTRACT_ROOT / "current-inventory.json"
DEFAULT_INVENTORY_SCHEMA = CONTRACT_ROOT / "current-inventory-schema.json"
DEFAULT_MIGRATION = CONTRACT_ROOT / "migration.json"
DEFAULT_MIGRATION_SCHEMA = CONTRACT_ROOT / "migration-schema.json"
DEFAULT_LEGACY_BASELINE = CONTRACT_ROOT / "legacy-growth-baseline.json"
DEFAULT_LEGACY_BASELINE_SCHEMA = CONTRACT_ROOT / "legacy-growth-baseline-schema.json"
DEFAULT_CANONICAL_ENCODING = CONTRACT_ROOT / "canonical-encoding-v1.json"
DEFAULT_CANONICAL_ENCODING_SCHEMA = CONTRACT_ROOT / "canonical-encoding-v1-schema.json"
DEFAULT_CANONICAL_VECTORS = CONTRACT_ROOT / "canonical-encoding-v1-vectors.json"
DEFAULT_CANONICAL_VECTORS_SCHEMA = (
    CONTRACT_ROOT / "canonical-encoding-v1-vectors-schema.json"
)
DEFAULT_FROZEN_TARGETS = CONTRACT_ROOT / "frozen-semantic-targets-v1.json"
DEFAULT_FROZEN_TARGETS_SCHEMA = (
    CONTRACT_ROOT / "frozen-semantic-targets-v1-schema.json"
)
DEFAULT_GATE_B = CONTRACT_ROOT / "gate-b-regression.json"
GATE_A_MANIFEST = ROOT / "tests/architecture/value-execution/legacy-boundary.json"
GENERATOR_PATH = ROOT / "scripts/generate-value-system-inventory.py"
GATE_A_CHECKER_PATH = ROOT / "scripts/check-value-execution-boundary.py"
CANONICAL_REFERENCE_PATH = ROOT / "scripts/tests/canonical_encoding_v1_reference.py"
EXPECTED_LEGACY_SCANNER_SHA256 = (
    "9624eb89c01085cc5e412506b30671f442ba53b057c228b3fbcd113cc77ad834"
)
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


GENERATOR = load_module("value_system_inventory_generator", GENERATOR_PATH)
CANONICAL_REFERENCE = load_module(
    "canonical_encoding_v1_reference", CANONICAL_REFERENCE_PATH
)


ROLES = {
    "semantic-payload",
    "mutable-storage",
    "type-wrapper",
    "constant",
    "machine-argument",
    "machine-output",
    "host-input",
    "host-output",
    "serialization",
    "diagnostic",
    "reactive-identity",
    "journal-discovery",
    "selection-ir",
    "compiler-type-data",
    "temporal-payload",
    "compiler-shape-hole",
    "generic-dispatch",
    "reified-type",
    "binding-contract",
}
DESTINATIONS = {
    "immutable-snapshot",
    "reified-type-snapshot",
    "kind-expression",
    "schema",
    "binding-contract",
    "selection-ir",
    "execution-control",
    "runtime-slot-state",
    "compiler-shape-hole",
    "compiler-construction-ir",
    "legacy-dispatch",
    "rejected-legacy-form",
}
IMPLEMENTATION_GATES = {"C1", "C2", "C3", "C4", "D", "final-cutover"}
AMBIGUOUS_TARGET = re.compile(
    r"-or-|\beither\b|\bone-of\b|\b(?:tbd|unknown|unclassified|maybe)\b",
    re.IGNORECASE,
)
TARGET_PROJECTION_FIELDS = (
    "id",
    "applies_to",
    "semantic_category",
    "representation",
    "implementation_gate",
    "key_semantics",
    "runtime_storage",
)


def expected_target_status(target: dict[str, Any]) -> dict[str, bool]:
    return {
        "inventoried": True,
        "semantics_frozen": True,
        "implemented": target["implementation_gate"] == "C1",
        "artifact_migrated": False,
        "ports_migrated": False,
        "resident_storage_migrated": False,
        "legacy_removed": False,
    }


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


def features(cfg: str | None) -> set[str]:
    if cfg is None:
        return set()
    return set(re.findall(r'feature\s*=\s*"([^"]+)"', cfg))


ENUM_FIELDS = {
    "Value": "value_variants",
    "ValueKind": "value_kind_variants",
    "Kind": "kind_variants",
}


def family_assignments(families: list[dict[str, Any]], field: str) -> dict[str, list[str]]:
    assigned: dict[str, list[str]] = {}
    for family in families:
        for variant in family["current"][field]:
            assigned.setdefault(variant, []).append(family["id"])
    return assigned


def coverage_failures(
    live: dict[str, Any], migration: dict[str, Any], migration_path: Path
) -> list[Failure]:
    failures: list[Failure] = []
    families = migration["families"]
    contract_ids = {
        "Value": "C0-VALUE-COVERAGE",
        "ValueKind": "C0-VALUE-KIND-COVERAGE",
        "Kind": "C0-KIND-COVERAGE",
    }
    for enum_name, field in ENUM_FIELDS.items():
        contract_id = contract_ids[enum_name]
        live_names = {row["name"] for row in live["enums"][enum_name]["variants"]}
        assigned = family_assignments(families, field)
        for variant in sorted(live_names - set(assigned)):
            failures.append(
                failure(
                    contract_id,
                    variant,
                    live["enums"][enum_name]["source"],
                    "exactly one migration family",
                    "no family assignment",
                    f"{migration_path}:families",
                    enum_name=enum_name,
                    variant=variant,
                )
            )
        for variant in sorted(set(assigned) - live_names):
            failures.append(
                failure(
                    contract_id,
                    variant,
                    str(migration_path),
                    "variant present in live enum",
                    f"assigned by {assigned[variant]}",
                    f"{migration_path}:families",
                    enum_name=enum_name,
                    variant=variant,
                )
            )
        for variant, owners in sorted(assigned.items()):
            if len(owners) > 1:
                failures.append(
                    failure(
                        "C0-DUPLICATE-FAMILY",
                        variant,
                        str(migration_path),
                        "one unpartitioned family assignment",
                        f"assigned by {owners}",
                        f"{migration_path}:families",
                        enum_name=enum_name,
                        variant=variant,
                    )
                )
    return failures


def family_contract_failures(
    live: dict[str, Any], migration: dict[str, Any], migration_path: Path
) -> list[Failure]:
    failures: list[Failure] = []
    live_values = {
        row["name"]: row for row in live["enums"]["Value"]["variants"]
    }
    target_owners: dict[str, list[str]] = {}
    family_ids: list[str] = []
    for family in migration["families"]:
        identifier = family["id"]
        family_ids.append(identifier)
        targets = family.get("targets", [])
        target_ids = [target.get("id") for target in targets if isinstance(target, dict)]
        if len(target_ids) != len(set(target_ids)):
            failures.append(
                failure(
                    "C0-TARGET-MEMBERSHIP",
                    identifier,
                    str(migration_path),
                    "unique target IDs within the family",
                    repr(target_ids),
                    f"{migration_path}:families[{identifier}].targets",
                )
            )
        for target in targets:
            target_id = target["id"]
            target_owners.setdefault(target_id, []).append(identifier)
            category = target.get("semantic_category")
            if category not in DESTINATIONS:
                failures.append(
                    failure(
                        "C0-TARGET-DESTINATION",
                        target_id,
                        str(migration_path),
                        f"semantic_category in {sorted(DESTINATIONS)}",
                        repr(category),
                        f"{migration_path}:families[{identifier}].targets",
                    )
                )
            gate = target.get("implementation_gate")
            if gate not in IMPLEMENTATION_GATES:
                failures.append(
                    failure(
                        "C0-TARGET-GATE",
                        target_id,
                        str(migration_path),
                        repr(sorted(IMPLEMENTATION_GATES)),
                        repr(gate),
                        f"{migration_path}:families[{identifier}].targets",
                    )
                )
            expected_status = expected_target_status(target)
            if target.get("status") != expected_status:
                failures.append(
                    failure(
                        "C0-TARGET-STATUS",
                        target_id,
                        str(migration_path),
                        repr(expected_status),
                        repr(target.get("status")),
                        f"{migration_path}:families[{identifier}].targets",
                    )
                )
            match = AMBIGUOUS_TARGET.search(json.dumps(target, sort_keys=True))
            if match is not None:
                failures.append(
                    failure(
                        "C0-AMBIGUOUS-TARGET",
                        target_id,
                        str(migration_path),
                        "one final semantic outcome per structured target",
                        f"contains {match.group(0)!r}",
                        f"{migration_path}:families[{identifier}].targets",
                    )
                )
        rows = [live_values[name] for name in family["current"]["value_variants"] if name in live_values]
        expected_features = set().union(*(features(row["cfg"]) for row in rows)) if rows else set()
        actual_features = set(family["current"]["features"])
        if actual_features != expected_features:
            failures.append(
                failure(
                    "C0-FEATURE-DRIFT",
                    identifier,
                    "src/core/src/value.rs",
                    repr(sorted(expected_features)),
                    repr(sorted(actual_features)),
                    f"{migration_path}:families[{identifier}].current.features",
                )
            )
        expected_storage = {row["payload_type"] for row in rows if row["payload_type"] is not None}
        actual_storage = set(family["current"]["storage"])
        if actual_storage != expected_storage:
            failures.append(
                failure(
                    "C0-STORAGE-DRIFT",
                    identifier,
                    "src/core/src/value.rs",
                    repr(sorted(expected_storage)),
                    repr(sorted(actual_storage)),
                    f"{migration_path}:families[{identifier}].current.storage",
                )
            )
    for identifier in sorted(set(family_ids)):
        count = family_ids.count(identifier)
        if count > 1:
            failures.append(
                failure(
                    "C0-DUPLICATE-FAMILY",
                    identifier,
                    str(migration_path),
                    "unique family ID",
                    f"declared {count} times",
                    f"{migration_path}:families",
                )
            )
    for target, owners in sorted(target_owners.items()):
        if len(owners) > 1:
            failures.append(
                failure(
                    "C0-TARGET-MEMBERSHIP",
                    target,
                    str(migration_path),
                    "one owning family",
                    repr(owners),
                    f"{migration_path}:families",
                )
            )
    return failures


def target_index(migration: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], dict[tuple[str, str], str]]:
    targets: dict[str, dict[str, Any]] = {}
    owners: dict[tuple[str, str], str] = {}
    for family in migration["families"]:
        for enum_name, field in ENUM_FIELDS.items():
            for variant in family["current"][field]:
                owners[(enum_name, variant)] = family["id"]
        for target in family["targets"]:
            targets[target["id"]] = {**target, "family": family["id"]}
    return targets, owners


def target_applicability_failures(
    migration: dict[str, Any], migration_path: Path
) -> list[Failure]:
    failures: list[Failure] = []
    targets, _owners = target_index(migration)
    for family in migration["families"]:
        family_id = family["id"]
        members = {
            (enum_name, variant)
            for enum_name, field in ENUM_FIELDS.items()
            for variant in family["current"][field]
        }
        covered: set[tuple[str, str]] = set()
        for target in family["targets"]:
            target_id = target["id"]
            applies = target.get("applies_to", [])
            flattened = [
                (row.get("enum"), variant)
                for row in applies
                for variant in row.get("variants", [])
            ]
            if not flattened:
                failures.append(
                    failure(
                        "C0-TARGET-APPLICABILITY",
                        target_id,
                        str(migration_path),
                        "at least one exact enum variant in the owning family",
                        "empty applies_to",
                        f"{migration_path}:families[{family_id}].targets[{target_id}].applies_to",
                    )
                )
            for enum_name, variant in flattened:
                if (enum_name, variant) not in members:
                    failures.append(
                        failure(
                            "C0-TARGET-APPLICABILITY",
                            target_id,
                            str(migration_path),
                            f"variant owned by family {family_id}",
                            "variant outside owning family",
                            f"{migration_path}:families[{family_id}].targets[{target_id}].applies_to",
                            enum_name=str(enum_name),
                            variant=str(variant),
                        )
                    )
                else:
                    covered.add((str(enum_name), str(variant)))
        for enum_name, variant in sorted(members - covered):
            failures.append(
                failure(
                    "C0-TARGET-APPLICABILITY",
                    family_id,
                    str(migration_path),
                    "at least one target explicitly applies to the family variant",
                    "no applicable target",
                    f"{migration_path}:families[{family_id}].targets[].applies_to",
                    enum_name=enum_name,
                    variant=variant,
                )
            )
    for classification in migration["use_classifications"]:
        enum_name = classification["enum"]
        variant = classification["variant"]
        target_id = classification["target"]
        target = targets.get(target_id, {})
        applicable = {
            (row["enum"], item)
            for row in target.get("applies_to", [])
            for item in row.get("variants", [])
        }
        if (enum_name, variant) not in applicable:
            first = classification["sites"][0]
            failures.append(
                failure(
                    "C0-TARGET-APPLICABILITY",
                    target_id,
                    classification["path"],
                    f"target explicitly applies to {enum_name}::{variant}",
                    "classification selected an inapplicable target",
                    f"{migration_path}:families[].targets[{target_id}].applies_to",
                    int(first["line"]),
                    int(first["column"]),
                    enum_name,
                    variant,
                )
            )
    return failures


def target_projection(migration: dict[str, Any]) -> list[dict[str, Any]]:
    return sorted(
        (
            {field: target[field] for field in TARGET_PROJECTION_FIELDS}
            for family in migration["families"]
            for target in family["targets"]
        ),
        key=lambda target: target["id"],
    )


def frozen_target_failures(
    migration: dict[str, Any],
    frozen: dict[str, Any],
    migration_path: Path,
    frozen_path: Path,
) -> list[Failure]:
    expected = {target["id"]: target for target in frozen["targets"]}
    actual = {target["id"]: target for target in target_projection(migration)}
    failures: list[Failure] = []
    for target_id in sorted(set(expected) | set(actual)):
        if expected.get(target_id) == actual.get(target_id):
            continue
        target = actual.get(target_id, expected.get(target_id, {}))
        applies = target.get("applies_to", [])
        enum_name = str(applies[0]["enum"]) if applies else "-"
        variants = applies[0].get("variants", []) if applies else []
        failures.append(
            failure(
                "C0-FROZEN-TARGET-DRIFT",
                target_id,
                str(migration_path),
                repr(expected.get(target_id)),
                repr(actual.get(target_id)),
                str(frozen_path),
                enum_name=enum_name,
                variant=str(variants[0]) if variants else "-",
            )
        )
    return failures


def occurrence_target_projection(migration: dict[str, Any]) -> list[dict[str, Any]]:
    return sorted(
        (
            {
                "enum": row["enum"],
                "variant": row["variant"],
                "path": row["path"],
                "sites": sorted(
                    (
                        {"line": int(site["line"]), "column": int(site["column"])}
                        for site in row["sites"]
                    ),
                    key=lambda site: (site["line"], site["column"]),
                ),
                "target": row["target"],
            }
            for row in migration["use_classifications"]
        ),
        key=lambda row: (
            row["enum"],
            row["variant"],
            row["path"],
            row["target"],
            tuple((site["line"], site["column"]) for site in row["sites"]),
        ),
    )


def frozen_occurrence_target_failures(
    migration: dict[str, Any],
    frozen: dict[str, Any],
    migration_path: Path,
    frozen_path: Path,
) -> list[Failure]:
    def flatten(rows: list[dict[str, Any]]) -> dict[tuple[str, str, str, int, int], str]:
        return {
            (
                row["enum"],
                row["variant"],
                row["path"],
                int(site["line"]),
                int(site["column"]),
            ): row["target"]
            for row in rows
            for site in row["sites"]
        }

    expected = flatten(frozen["occurrence_targets"])
    actual = flatten(occurrence_target_projection(migration))
    failures: list[Failure] = []
    for key in sorted(set(expected) | set(actual)):
        if expected.get(key) == actual.get(key):
            continue
        enum_name, variant, path, line, column = key
        failures.append(
            failure(
                "C0-FROZEN-OCCURRENCE-TARGET",
                "reviewed occurrence destination",
                path,
                repr(expected.get(key)),
                repr(actual.get(key)),
                f"{frozen_path}:occurrence_targets",
                line,
                column,
                enum_name,
                variant,
            )
        )
    return failures


def occurrence_classification_failures(
    live: dict[str, Any], migration: dict[str, Any], migration_path: Path
) -> list[Failure]:
    failures: list[Failure] = []
    expected = {
        (row["enum"], row["variant"], row["path"], row["line"], row["column"])
        for row in live["variant_uses"]
    }
    actual: dict[tuple[str, str, str, int, int], list[dict[str, Any]]] = {}
    targets, variant_owners = target_index(migration)
    for classification in migration["use_classifications"]:
        enum_name = classification["enum"]
        variant = classification["variant"]
        path = classification["path"]
        roles = set(classification["roles"])
        target_id = classification["target"]
        if not roles or not roles.issubset(ROLES):
            failures.append(
                failure(
                    "C0-OCCURRENCE-ROLE",
                    target_id,
                    path,
                    f"non-empty subset of {sorted(ROLES)}",
                    repr(sorted(roles)),
                    f"{migration_path}:use_classifications",
                    enum_name=enum_name,
                    variant=variant,
                )
            )
        target = targets.get(target_id)
        owner = variant_owners.get((enum_name, variant))
        if target is None or owner is None or target.get("family") != owner:
            failures.append(
                failure(
                    "C0-TARGET-MEMBERSHIP",
                    target_id,
                    path,
                    f"target owned by family for {enum_name}::{variant}",
                    "missing target or target belongs to another family",
                    f"{migration_path}:families and use_classifications",
                    enum_name=enum_name,
                    variant=variant,
                )
            )
        for site in classification["sites"]:
            key = (enum_name, variant, path, site["line"], site["column"])
            actual.setdefault(key, []).append(classification)
    for enum_name, variant, path, line, column in sorted(expected - set(actual)):
        failures.append(
            failure(
                "C0-OCCURRENCE-CLASSIFICATION",
                "qualified variant use",
                path,
                "exactly one reviewed occurrence classification",
                "unclassified production occurrence",
                f"{migration_path}:use_classifications",
                line,
                column,
                enum_name,
                variant,
            )
        )
    for enum_name, variant, path, line, column in sorted(set(actual) - expected):
        failures.append(
            failure(
                "C0-OCCURRENCE-CLASSIFICATION",
                "qualified variant use",
                path,
                "live production occurrence",
                "stale or nonexistent classification",
                f"{migration_path}:use_classifications",
                line,
                column,
                enum_name,
                variant,
            )
        )
    for (enum_name, variant, path, line, column), classifications in sorted(actual.items()):
        if len(classifications) > 1:
            failures.append(
                failure(
                    "C0-OCCURRENCE-CLASSIFICATION",
                    "qualified variant use",
                    path,
                    "exactly one reviewed occurrence classification",
                    f"classified {len(classifications)} times",
                    f"{migration_path}:use_classifications",
                    line,
                    column,
                    enum_name,
                    variant,
                )
            )
    return failures


def classified_targets(migration: dict[str, Any], enum_name: str, variant: str) -> list[str]:
    return [
        row["target"]
        for row in migration["use_classifications"]
        if row["enum"] == enum_name and row["variant"] == variant
        for _site in row["sites"]
    ]


def frozen_semantics_failures(migration: dict[str, Any], migration_path: Path) -> list[Failure]:
    failures: list[Failure] = []
    exact_targets = {
        ("Value", "Empty"): {
            "source-empty-expression",
            "option-absence",
            "execution-no-result",
            "uninitialized-storage",
            "unspecified-extent",
            "generic-dispatch",
        },
        ("ValueKind", "Empty"): {"value-kind-hole"},
        ("Kind", "Empty"): {"kind-hole"},
        ("Value", "MatrixValue"): {
            "matrix-construction-ir",
            "homogeneous-matrix-snapshot",
            "legacy-matrix-value-adapter",
        },
        ("Value", "EmptyKind"): {"legacy-typed-empty-adapter"},
    }
    for (enum_name, variant), expected in exact_targets.items():
        actual = set(classified_targets(migration, enum_name, variant))
        if actual != expected:
            failures.append(
                failure(
                    "C0-FROZEN-SEMANTICS",
                    "exact occurrence target set",
                    str(migration_path),
                    repr(sorted(expected)),
                    repr(sorted(actual)),
                    f"{migration_path}:use_classifications",
                    enum_name=enum_name,
                    variant=variant,
                )
            )
    targets, owners = target_index(migration)
    decisions = (
        ("Value", "Kind", "reified-type-snapshot", "reified-type-snapshot"),
        ("Kind", "Reference", "reference-binding-contract", "binding-contract"),
        ("ValueKind", "Any", "kind-wildcard", "kind-expression"),
    )
    for enum_name, variant, target_id, category in decisions:
        family_id = owners.get((enum_name, variant))
        target = targets.get(target_id)
        classified = set(classified_targets(migration, enum_name, variant))
        if (
            family_id is None
            or target is None
            or target.get("family") != family_id
            or target.get("semantic_category") != category
            or classified != {target_id}
        ):
            failures.append(
                failure(
                    "C0-FROZEN-SEMANTICS",
                    target_id,
                    str(migration_path),
                    f"{enum_name}::{variant} owns {category} target {target_id}",
                    f"target={target!r}; classified={sorted(classified)!r}",
                    f"{migration_path}:families",
                    enum_name=enum_name,
                    variant=variant,
                )
            )
    any_targets = {
        row["target"]
        for row in migration["use_classifications"]
        if row["enum"] == "ValueKind" and row["variant"] == "Any"
    }
    if any(targets.get(identifier, {}).get("semantic_category") == "schema" for identifier in any_targets):
        failures.append(
            failure(
                "C0-FROZEN-SEMANTICS",
                "wildcard is not a schema",
                str(migration_path),
                "only kind-expression targets",
                repr(sorted(any_targets)),
                f"{migration_path}:use_classifications",
                enum_name="ValueKind",
                variant="Any",
            )
        )
    adapter = targets.get("legacy-typed-empty-adapter", {})
    if adapter.get("implementation_gate") != "C2":
        failures.append(
            failure(
                "C0-FROZEN-SEMANTICS",
                "legacy-typed-empty-adapter",
                str(migration_path),
                "C2",
                repr(adapter.get("implementation_gate")),
                f"{migration_path}:families",
                enum_name="Value",
                variant="EmptyKind",
            )
        )
    return failures


def matrix_value_classification_failures(
    migration: dict[str, Any], migration_path: Path
) -> list[Failure]:
    rejected = [
        (row["path"], site)
        for row in migration["use_classifications"]
        if row["enum"] == "Value"
        and row["variant"] == "MatrixValue"
        and row["target"] == "heterogeneous-matrix-rejected"
        for site in row["sites"]
    ]
    return [
        failure(
            "C0-MATRIX-VALUE-CLASSIFICATION",
            "live MatrixValue destination",
            path,
            "legacy-matrix-value-adapter or a proved homogeneous/construction target",
            "heterogeneous-matrix-rejected",
            f"{migration_path}:use_classifications",
            int(site["line"]),
            int(site["column"]),
            "Value",
            "MatrixValue",
        )
        for path, site in rejected
    ]


def type_contract_source_failures(
    root: Path, live: dict[str, Any], inventory_path: Path
) -> list[Failure]:
    try:
        expected = GENERATOR.type_contract_sources(root)
    except GENERATOR.TypeContractError as error:
        return [
            failure(
                "C0-KIND-SCHEME-SEPARATION",
                "type contract source shape",
                str(root),
                "reviewed declaration forms, field types, and separated semantic/runtime layers",
                str(error),
                f"{inventory_path}:type_contract_sources",
            )
        ]
    actual = live.get("type_contract_sources")
    failures: list[Failure] = []
    if actual != expected:
        failures.append(
            failure(
                "C0-KIND-SCHEME-SEPARATION",
                "type contract source decomposition",
                str(inventory_path),
                repr(expected),
                repr(actual),
                f"{inventory_path}:type_contract_sources",
            )
        )
        return failures
    exact_targets = GENERATOR.TYPE_CONTRACT_TARGETS
    for group, records in actual.items():
        target, gate = exact_targets[group]
        for record in records:
            if (
                record.get("target") != target
                or record.get("implementation_gate") != gate
            ):
                failures.append(
                    failure(
                        "C0-KIND-SCHEME-SEPARATION",
                        record.get("symbol", group),
                        record.get("path", str(inventory_path)),
                        f"target={target!r}; implementation_gate={gate!r}",
                        f"target={record.get('target')!r}; implementation_gate={record.get('implementation_gate')!r}",
                        f"{inventory_path}:type_contract_sources.{group}",
                    )
                )
    return failures


def auxiliary_fixture_failures(
    inventory: dict[str, Any], live: dict[str, Any], inventory_path: Path
) -> list[Failure]:
    failures: list[Failure] = []
    for field, subject in (
        ("auxiliary_rust_fixtures", "proven trybuild fixture roots"),
        ("auxiliary_cargo_fixtures", "proven auxiliary Cargo target files"),
        ("enumerated_rust_files", "complete enumerated Rust source set"),
        ("audited_rust_files", "complete audited Rust source set"),
    ):
        expected = live.get(field, [])
        actual = inventory.get(field, [])
        if actual != expected:
            failures.append(
                failure(
                    "C0-AUXILIARY-FIXTURE",
                    subject,
                    str(inventory_path),
                    repr(expected),
                    repr(actual),
                    f"{inventory_path}:{field}",
                )
            )
    return failures


def workspace_source_coverage_failures(
    inventory: dict[str, Any], live: dict[str, Any], inventory_path: Path
) -> list[Failure]:
    expected = live.get("workspace_packages", [])
    actual = inventory.get("workspace_packages", [])
    if actual == expected:
        return []
    return [
        failure(
            "C0-WORKSPACE-SOURCE-COVERAGE",
            "Cargo workspace package source inventory",
            str(inventory_path),
            repr(expected),
            repr(actual),
            f"{inventory_path}:workspace_packages",
        )
    ]


def source_disposition_failures(
    inventory: dict[str, Any], inventory_path: Path
) -> list[Failure]:
    enumerated = set(inventory.get("enumerated_rust_files", []))
    audited = set(inventory.get("audited_rust_files", []))
    cargo = {
        path
        for package in inventory.get("auxiliary_cargo_fixtures", [])
        for target in package.get("targets", [])
        for path in target.get("reachable_rust_files", [])
    } - audited
    trybuild = {
        path
        for record in inventory.get("auxiliary_rust_fixtures", [])
        for path in record.get("reachable_rust_files", [])
    } - audited
    missing = enumerated - (audited | cargo | trybuild)
    extra = (audited | cargo | trybuild) - enumerated
    overlap = cargo & trybuild
    if not missing and not extra and not overlap:
        return []
    return [
        failure(
            "C0-AUDITED-SOURCE-SET",
            "complete disjoint Rust source disposition",
            str(inventory_path),
            "enumerated = audited union effective Cargo auxiliary union effective trybuild",
            repr(
                {
                    "missing": sorted(missing),
                    "extra": sorted(extra),
                    "auxiliary_overlap": sorted(overlap),
                }
            ),
            f"{inventory_path}:audited_rust_files",
        )
    ]


def qualification_failures(root: Path, live: dict[str, Any]) -> list[Failure]:
    failures: list[Failure] = []
    variants_by_enum = {
        enum_name: {row["name"] for row in record["variants"]}
        for enum_name, record in live["enums"].items()
    }
    for path in GENERATOR.production_files(root):
        source = path.read_text(encoding="utf-8")
        searchable = GENERATOR.mask_non_code(GENERATOR.production_source(source))
        tokens = GENERATOR.rust_tokens(source, searchable)
        relative = path.relative_to(root).as_posix()
        for violation in GENERATOR.qualification_violations(
            relative,
            tokens,
            variants_by_enum,
            GENERATOR.crate_root_bindings(root, path, tokens),
        ):
            contract_id = {
                "raw-audited-alias": "C0-RAW-AUDITED-ALIAS",
                "semantic-kind-alias": "C0-SEMANTIC-KIND-ALIAS",
                "ref-alias": "C0-REF-ALIAS",
                "type-alias-cycle": "C0-TYPE-ALIAS-CYCLE",
                "type-alias-ambiguous": "C0-TYPE-ALIAS-AMBIGUOUS",
                "kind-qualifier-ambiguous": "C0-KIND-QUALIFIER-AMBIGUOUS",
            }.get(str(violation["kind"]), "C0-VARIANT-QUALIFICATION")
            failures.append(
                failure(
                    contract_id,
                    str(violation["kind"]),
                    str(violation["path"]),
                    "production variants qualified with their exact enum name",
                    str(violation["kind"]),
                    "qualify the variant use; do not import or alias audited enums",
                    int(violation["line"]),
                    int(violation["column"]),
                    str(violation["enum"]),
                    str(violation.get("variant", "-")),
                )
            )
    return failures


def high_risk_failures(
    baseline: dict[str, Any], live: dict[str, Any], baseline_path: Path
) -> list[Failure]:
    failures: list[Failure] = []
    baseline_uses = baseline["high_risk_api_uses"]
    live_uses = live["high_risk_api_uses"]
    for identifier, current_rows in live_uses.items():
        approved_sites = Counter(
            (row["path"], site["fingerprint"])
            for row in baseline_uses.get(identifier, [])
            for site in row["sites"]
        )
        current_sites = Counter(
            (row["path"], site["fingerprint"])
            for row in current_rows
            for site in row["sites"]
        )
        additions = current_sites - approved_sites
        for path, fingerprint in sorted(additions):
            remaining = additions[(path, fingerprint)]
            for row in current_rows:
                if row["path"] != path:
                    continue
                for site in row["sites"]:
                    if site["fingerprint"] != fingerprint or remaining == 0:
                        continue
                    failures.append(
                        failure(
                            "C0-LEGACY-GROWTH",
                            identifier,
                            path,
                            "occurrence fingerprint already present in immutable baseline",
                            "new or substituted legacy occurrence",
                            f"{baseline_path}:high_risk_api_uses.{identifier}",
                            site["line"],
                            site["column"],
                        )
                    )
                    remaining -= 1
                if remaining == 0:
                    break
    return failures


def legacy_alias_baseline_failures(
    baseline: dict[str, Any], live: dict[str, Any], baseline_path: Path
) -> list[Failure]:
    def frozen(rows: list[dict[str, Any]]) -> Counter[str]:
        return Counter(
            json.dumps(
                {
                    key: value
                    for key, value in row.items()
                    if key not in {"line", "column"}
                },
                sort_keys=True,
                separators=(",", ":"),
            )
            for row in rows
        )

    approved = frozen(baseline.get("legacy_aliases", []))
    current = frozen(live.get("legacy_aliases", []))
    return [
        failure(
            "C0-IMMUTABLE-LEGACY-BASELINE",
            json.loads(record)["name"],
            json.loads(record)["path"],
            "exact archived Ref alias or removal",
            record,
            f"{baseline_path}:legacy_aliases",
            None,
            None,
        )
        for record, count in sorted((current - approved).items())
        for _ in range(count)
    ]


def compatibility_alias_failures(
    baseline: dict[str, Any], live: dict[str, Any], baseline_path: Path
) -> list[Failure]:
    approved = baseline.get("required_compatibility_aliases", [])
    current = live.get("required_compatibility_aliases", [])
    if current == approved:
        return []
    return [
        failure(
            "C0-PUBLIC-COMPAT-ALIAS",
            "required public compatibility aliases",
            str(baseline_path),
            repr(approved),
            repr(current),
            f"{baseline_path}:required_compatibility_aliases",
        )
    ]


def raw_approved_alias_failures(live: dict[str, Any]) -> list[Failure]:
    return [
        failure(
            "C0-RAW-APPROVED-ALIAS",
            str(record["name"]),
            str(record["path"]),
            "canonical approved-alias spelling",
            str(record["raw_name"]),
            "scripts/value_system_legacy_scanner_v1.py",
            int(record["line"]),
            int(record["column"]),
        )
        for record in live.get("raw_approved_aliases", [])
    ]


def immutable_baseline_failures(
    root: Path,
    baseline: dict[str, Any],
    baseline_path: Path,
    *,
    verify_git: bool,
) -> list[Failure]:
    scanner_contract = baseline.get("scanner_contract")
    expected_contract = GENERATOR.LEGACY_SCANNER_CONTRACT
    implementation_digest = GENERATOR.legacy_scanner_implementation_sha256()
    if (
        scanner_contract != expected_contract
        or expected_contract.get("implementation_sha256")
        != EXPECTED_LEGACY_SCANNER_SHA256
        or implementation_digest != expected_contract.get("implementation_sha256")
    ):
        return [
            failure(
                "C0-LEGACY-SCANNER-DRIFT",
                "legacy-growth scanner implementation",
                str(GENERATOR.LEGACY_SCANNER.__file__),
                repr(expected_contract),
                repr(
                    {
                        "baseline_contract": scanner_contract,
                        "implementation_sha256": implementation_digest,
                    }
                ),
                f"{baseline_path}:scanner_contract",
            )
        ]
    if not verify_git:
        return []
    reference = baseline["reference_commit"]
    try:
        expected = GENERATOR.legacy_baseline(
            GENERATOR.archived_inventory(root, reference), reference
        )
    except GENERATOR.TypeContractError as error:
        return [
            failure(
                "C0-IMMUTABLE-LEGACY-BASELINE",
                reference,
                ".git",
                "regeneratable frozen B2 source archive",
                str(error),
                str(baseline_path),
            )
        ]
    expected_bytes = GENERATOR.render(expected)
    actual_bytes = baseline_path.read_text(encoding="utf-8")
    if actual_bytes != expected_bytes:
        return [
            failure(
                "C0-IMMUTABLE-LEGACY-BASELINE",
                reference,
                str(baseline_path),
                "byte-identical baseline regenerated from frozen git ref",
                "committed baseline differs",
                str(baseline_path),
            )
        ]
    return []


def gate_a_failures(root: Path, inventory: dict[str, Any]) -> list[Failure]:
    if not GATE_A_MANIFEST.is_file() or not GATE_A_CHECKER_PATH.is_file():
        return [
            failure(
                "C0-GATE-A-INTEGRATION",
                "legacy-boundary",
                str(GATE_A_MANIFEST),
                "authoritative Gate A manifest and checker",
                "missing",
                "tests/architecture/value-execution/legacy-boundary.json",
            )
        ]
    manifest = load_json(GATE_A_MANIFEST)
    high_risk_patterns = {
        record["id"]: record["pattern"]
        for record in inventory["identity_mechanisms"] + inventory["journal_mechanisms"]
    }
    failures: list[Failure] = []
    for boundary in manifest["boundaries"]:
        identifier = boundary["id"]
        if identifier in high_risk_patterns and boundary["pattern"] != high_risk_patterns[identifier]:
            failures.append(
                failure(
                    "C0-GATE-A-INTEGRATION",
                    identifier,
                    str(GATE_A_MANIFEST),
                    repr(boundary["pattern"]),
                    repr(high_risk_patterns[identifier]),
                    "scripts/generate-value-system-inventory.py:HIGH_RISK_PATTERNS",
                )
            )
    checker = load_module("gate_a_value_execution_boundary", GATE_A_CHECKER_PATH)
    for message in checker.audit(root, GATE_A_MANIFEST):
        failures.append(
            failure(
                "C0-GATE-A-INTEGRATION",
                "legacy-boundary",
                str(GATE_A_MANIFEST),
                "no Gate A legacy growth",
                message,
                str(GATE_A_MANIFEST),
            )
        )
    return failures


def production_corpus(root: Path) -> Iterable[tuple[Path, str]]:
    try:
        paths = GENERATOR.production_files(root)
    except GENERATOR.CargoMetadataError:
        # Boundary-only synthetic fixtures do not model a Cargo workspace.
        # The authoritative inventory path never uses this fallback.
        paths = GENERATOR.rust_files_under(root)
    for path in paths:
        source = path.read_text(encoding="utf-8")
        yield path, GENERATOR.production_source(source)


def analyzed_production_corpus(
    root: Path,
) -> list[tuple[Path, str, str, list[Any]]]:
    """Read, mask, and tokenize each production source exactly once per audit."""
    analyzed: list[tuple[Path, str, str, list[Any]]] = []
    for path, source in production_corpus(root):
        searchable = GENERATOR.mask_non_code(source)
        analyzed.append(
            (path, source, searchable, GENERATOR.rust_tokens(source, searchable))
        )
    return analyzed


def local_conversion_aliases(tokens: list[Any]) -> dict[str, set[str]]:
    return GENERATOR.LEGACY_SCANNER.transparent_conversion_aliases(tokens)


def blanket_conversion(tokens: list[Any]) -> tuple[int, str] | None:
    aliases = local_conversion_aliases(tokens)
    trait_aliases = GENERATOR.LEGACY_SCANNER.imported_trait_aliases(
        tokens, {"From", "Into"}
    )

    def categories(items: list[Any]) -> set[str]:
        result: set[str] = set()
        for item in items:
            result.update(
                aliases.get(GENERATOR.canonical_identifier(item.value), set())
            )
        for index in range(len(items) - 2):
            if (
                GENERATOR.canonical_identifier(items[index].value) == "snapshot"
                and items[index + 1].value == "::"
                and GENERATOR.canonical_identifier(items[index + 2].value) == "Value"
            ):
                result.add("snapshot")
        return result

    for opening, token in enumerate(tokens):
        if token.value != "impl":
            continue
        index = opening + 1
        if index < len(tokens) and tokens[index].value == "<":
            generic_end = GENERATOR.balanced_token_end(tokens, index, "<", ">")
            if generic_end is None:
                continue
            index = generic_end + 1
        header_end = index
        while header_end < len(tokens) and tokens[header_end].value not in {"{", ";"}:
            header_end += 1
        trait = next(
            (
                position
                for position in range(index, header_end)
                if GENERATOR.canonical_identifier(tokens[position].value)
                in trait_aliases
            ),
            None,
        )
        if trait is None or trait + 1 >= len(tokens) or tokens[trait + 1].value != "<":
            continue
        argument_end = GENERATOR.balanced_token_end(tokens, trait + 1, "<", ">")
        if argument_end is None:
            continue
        for_index = argument_end + 1
        while for_index < len(tokens) and tokens[for_index].value != "for":
            if tokens[for_index].value in {"{", ";"}:
                break
            for_index += 1
        if for_index >= len(tokens) or tokens[for_index].value != "for":
            continue
        self_end = for_index + 1
        while self_end < len(tokens) and tokens[self_end].value not in {"where", "{", ";"}:
            self_end += 1
        linked = categories(tokens[trait + 2 : argument_end]) | categories(
            tokens[for_index + 1 : self_end]
        )
        if linked == {"legacy", "snapshot"}:
            return token.line, " ".join(
                item.value for item in tokens[opening:self_end]
            )
    return None


def module_file_or_descendant(root: Path, resolved: Path, name: str) -> bool:
    directory = (root / f"src/core/src/{name}").resolve()
    file_path = (root / f"src/core/src/{name}.rs").resolve()
    return resolved == file_path or directory in resolved.parents


FINALIZED_SEMANTIC_SERDE_TYPES = {
    "src/core/src/nominal.rs": {"CanonicalNominalPath"},
    "src/core/src/dimension.rs": {"DimensionParameter"},
    "src/core/src/kind_scheme.rs": {"KindScheme"},
    "src/core/src/schema/mod.rs": {"Schema"},
    "src/core/src/schema/shape.rs": {"ShapeInstance"},
}
OPEN_SEMANTIC_SERDE_TYPES = {
    "src/core/src/dimension.rs": {
        "DimensionExpr",
        "DimensionParameterDeclaration",
    },
    "src/core/src/kind_expr.rs": {"KindExpr"},
    "src/core/src/kind_scheme.rs": {
        "KindParameter",
        "InputKindScheme",
        "KindConstraint",
    },
    "src/core/src/schema/mod.rs": {"SchemaDraft", "SchemaBody"},
}
NON_SERDE_SEMANTIC_TYPES = {
    "src/core/src/schema/table.rs": {"SchemaHandle"},
}
FINALIZED_SEMANTIC_TYPE_NAMES = set().union(
    *FINALIZED_SEMANTIC_SERDE_TYPES.values(),
    *NON_SERDE_SEMANTIC_TYPES.values(),
)
STANDARD_INTERIOR_MUTABILITY_IDENTIFIERS = {
    "UnsafeCell",
    "SyncUnsafeCell",
    "Cell",
    "RefCell",
    "OnceCell",
    "LazyCell",
    "Mutex",
    "RwLock",
    "Once",
    "OnceLock",
    "LazyLock",
    "Barrier",
    "Condvar",
    "AtomicBool",
    "AtomicI8",
    "AtomicI16",
    "AtomicI32",
    "AtomicI64",
    "AtomicI128",
    "AtomicIsize",
    "AtomicU8",
    "AtomicU16",
    "AtomicU32",
    "AtomicU64",
    "AtomicU128",
    "AtomicUsize",
    "AtomicPtr",
}
SEMANTIC_FORBIDDEN_IDENTIFIERS = STANDARD_INTERIOR_MUTABILITY_IDENTIFIERS | {
    "Kind",
    "Value",
    "ValueKind",
    "Ref",
    "ValRef",
    "MutableReference",
    "ReactiveCellId",
    "ValueStateJournal",
    "ReactiveTurnJournal",
    "RuntimeExecutionTransaction",
    "StateArena",
    "nalgebra",
    "DMatrix",
}


def rust_type_identifiers(tokens: Iterable[Any]) -> set[str]:
    return {
        GENERATOR.canonical_identifier(token.value)
        for token in tokens
        if re.fullmatch(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", token.value)
    }


def field_type_dependencies(tokens: list[Any]) -> set[str]:
    dependencies: set[str] = set()
    for field in GENERATOR.LEGACY_SCANNER.split_top_level_tokens(tokens):
        colon = next(
            (index for index, token in enumerate(field) if token.value == ":"),
            None,
        )
        field_type = field[colon + 1 :] if colon is not None else field
        dependencies.update(rust_type_identifiers(field_type))
    return dependencies


def declared_type_dependencies(tokens: list[Any]) -> dict[str, set[str]]:
    declarations: dict[str, set[str]] = {}
    for alias in GENERATOR.LEGACY_SCANNER.type_alias_declarations(tokens):
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", alias.name) is None:
            continue
        declarations.setdefault(alias.name, set()).update(
            rust_type_identifiers(alias.rhs) - set(alias.parameters)
        )

    offset = 0
    while offset + 1 < len(tokens):
        declaration_kind = GENERATOR.canonical_identifier(tokens[offset].value)
        if declaration_kind not in {"struct", "enum", "union"}:
            offset += 1
            continue
        name_token = tokens[offset + 1]
        if re.fullmatch(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", name_token.value) is None:
            offset += 1
            continue
        name = GENERATOR.canonical_identifier(name_token.value)
        opening = next(
            (
                index
                for index in range(offset + 2, len(tokens))
                if tokens[index].value in {"{", "(", ";"}
            ),
            None,
        )
        if opening is None or tokens[opening].value == ";":
            offset += 2
            continue
        delimiter = tokens[opening].value
        closing = GENERATOR.balanced_token_end(
            tokens, opening, delimiter, "}" if delimiter == "{" else ")"
        )
        if closing is None:
            offset += 2
            continue
        body = list(tokens[opening + 1 : closing])
        dependencies: set[str] = set()
        if declaration_kind in {"struct", "union"}:
            dependencies = field_type_dependencies(body)
        else:
            for variant in GENERATOR.LEGACY_SCANNER.split_top_level_tokens(body):
                payload = next(
                    (
                        index
                        for index, token in enumerate(variant[1:], start=1)
                        if token.value in {"(", "{"}
                    ),
                    None,
                )
                if payload is None:
                    continue
                payload_delimiter = variant[payload].value
                payload_end = GENERATOR.balanced_token_end(
                    variant,
                    payload,
                    payload_delimiter,
                    "}" if payload_delimiter == "{" else ")",
                )
                if payload_end is None:
                    continue
                payload_tokens = list(variant[payload + 1 : payload_end])
                dependencies.update(
                    field_type_dependencies(payload_tokens)
                    if payload_delimiter == "{"
                    else rust_type_identifiers(payload_tokens)
                )
        declarations.setdefault(name, set()).update(dependencies)
        offset = closing + 1
    return declarations


def finalized_semantic_type_aliases(
    corpus: list[tuple[Path, str, str, list[Any]]],
) -> set[str]:
    """Resolve aliases and renamed imports that denote a finalized C1 type."""
    declarations: dict[str, set[str]] = {}
    for _path, _source, _searchable, tokens in corpus:
        for alias in GENERATOR.LEGACY_SCANNER.type_alias_declarations(tokens):
            declarations.setdefault(alias.name, set()).update(
                rust_type_identifiers(alias.rhs) - set(alias.parameters)
            )
        for binding in GENERATOR.LEGACY_SCANNER.use_bindings(tokens):
            if binding.path and not binding.glob:
                declarations.setdefault(binding.local, set()).add(binding.path[-1])

    aliases = set(FINALIZED_SEMANTIC_TYPE_NAMES)
    changed = True
    while changed:
        changed = False
        for name, dependencies in declarations.items():
            if name not in aliases and dependencies & aliases:
                aliases.add(name)
                changed = True
    return aliases


def manual_deserialize_impls(
    tokens: list[Any], target_aliases: set[str]
) -> list[tuple[int, str, str]]:
    """Find Deserialize implementations whose self type is a finalized C1 type."""
    trait_aliases = GENERATOR.LEGACY_SCANNER.imported_trait_aliases(
        tokens, {"Deserialize"}
    )
    implementations: list[tuple[int, str, str]] = []
    for opening, token in enumerate(tokens):
        if token.value != "impl":
            continue
        index = opening + 1
        if index < len(tokens) and tokens[index].value == "<":
            generic_end = GENERATOR.balanced_token_end(tokens, index, "<", ">")
            if generic_end is None:
                continue
            index = generic_end + 1
        header_end = index
        while header_end < len(tokens) and tokens[header_end].value not in {"{", ";"}:
            header_end += 1
        for_indexes = [
            position
            for position in range(index, header_end)
            if tokens[position].value == "for"
        ]
        if not for_indexes:
            continue
        for_index = for_indexes[-1]

        trait_tokens = list(tokens[index:for_index])
        if any(item.value == "!" for item in trait_tokens):
            continue
        trait_generic = next(
            (
                position
                for position, item in enumerate(trait_tokens)
                if item.value == "<"
            ),
            len(trait_tokens),
        )
        trait_names = [
            GENERATOR.canonical_identifier(item.value)
            for item in trait_tokens[:trait_generic]
            if re.fullmatch(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", item.value)
        ]
        if not trait_names or trait_names[-1] not in trait_aliases:
            continue

        self_end = for_index + 1
        while self_end < header_end and tokens[self_end].value != "where":
            self_end += 1
        self_tokens = GENERATOR.LEGACY_SCANNER.strip_outer_parentheses(
            tokens[for_index + 1 : self_end]
        )
        self_generic = next(
            (
                position
                for position, item in enumerate(self_tokens)
                if item.value == "<"
            ),
            len(self_tokens),
        )
        self_names = [
            GENERATOR.canonical_identifier(item.value)
            for item in self_tokens[:self_generic]
            if re.fullmatch(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", item.value)
        ]
        if not self_names or self_names[-1] not in target_aliases:
            continue
        implementations.append(
            (
                token.line,
                self_names[-1],
                " ".join(item.value for item in tokens[opening:header_end]),
            )
        )
    return implementations


def transitive_semantic_forbidden_names(
    corpus: list[tuple[Path, str, str, list[Any]]],
) -> set[str]:
    declarations: dict[str, set[str]] = {}
    for _path, _source, _searchable, tokens in corpus:
        local = declared_type_dependencies(tokens)
        for name, dependencies in local.items():
            declarations.setdefault(name, set()).update(dependencies)
        for binding in GENERATOR.LEGACY_SCANNER.use_bindings(tokens):
            if (
                binding.path
                and not binding.glob
                and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", binding.local)
                and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", binding.path[-1])
            ):
                declarations.setdefault(binding.local, set()).add(binding.path[-1])

    forbidden = set(SEMANTIC_FORBIDDEN_IDENTIFIERS)
    changed = True
    while changed:
        changed = False
        for name, dependencies in declarations.items():
            if name not in forbidden and dependencies & forbidden:
                forbidden.add(name)
                changed = True
    return forbidden


def semantic_serde_attribute(source: str, type_name: str) -> str | None:
    declaration = re.search(rf"\bpub\s+(?:struct|enum)\s+{type_name}\b", source)
    if declaration is None:
        return None
    attribute_start = source.rfind(
        '#[cfg_attr(feature = "serde", derive(', 0, declaration.start()
    )
    if attribute_start < 0:
        return ""
    attribute_end = source.find(")]", attribute_start, declaration.start())
    if attribute_end < 0:
        return ""
    return source[attribute_start : attribute_end + 2]


def semantic_builder_segment(
    searchable: str, start_marker: str, end_marker: str
) -> str | None:
    start = searchable.find(start_marker)
    if start < 0:
        return None
    end = searchable.find(end_marker, start + len(start_marker))
    return searchable[start:] if end < 0 else searchable[start:end]


def skip_outer_attributes(source: str, offset: int) -> int:
    while True:
        whitespace = re.match(r"\s*", source[offset:])
        assert whitespace is not None
        offset += whitespace.end()
        if offset >= len(source) or source[offset] != "#":
            return offset
        attribute = offset + 1
        while attribute < len(source) and source[attribute].isspace():
            attribute += 1
        if attribute < len(source) and source[attribute] == "!":
            attribute += 1
            while attribute < len(source) and source[attribute].isspace():
                attribute += 1
        if attribute >= len(source) or source[attribute] != "[":
            return offset
        depth = 1
        attribute += 1
        while attribute < len(source) and depth:
            if source[attribute] == "[":
                depth += 1
            elif source[attribute] == "]":
                depth -= 1
            attribute += 1
        if depth:
            return offset
        offset = attribute


def top_level_character_positions(source: str, character: str) -> list[int]:
    depths = {"(": 0, "[": 0, "{": 0}
    closers = {")": "(", "]": "[", "}": "{"}
    positions: list[int] = []
    for offset, current in enumerate(source):
        if current in depths:
            depths[current] += 1
        elif current in closers:
            opening = closers[current]
            if depths[opening]:
                depths[opening] -= 1
        elif current == character and not any(depths.values()):
            positions.append(offset)
    return positions


def strip_outer_pattern_parentheses(pattern: str) -> str:
    result = pattern.strip()
    while result.startswith("("):
        depth = 0
        closing = None
        for offset, character in enumerate(result):
            if character == "(":
                depth += 1
            elif character == ")":
                depth -= 1
                if depth == 0:
                    closing = offset
                    break
        if closing != len(result) - 1:
            break
        result = result[1:-1].strip()
    return result


def pattern_has_top_level_guard(pattern: str) -> bool:
    depths = {"(": 0, "[": 0, "{": 0}
    closers = {")": "(", "]": "[", "}": "{"}
    offset = 0
    while offset < len(pattern):
        character = pattern[offset]
        if character in depths:
            depths[character] += 1
            offset += 1
            continue
        if character in closers:
            opening = closers[character]
            if depths[opening]:
                depths[opening] -= 1
            offset += 1
            continue
        if not any(depths.values()) and (character.isalpha() or character == "_"):
            end = offset + 1
            while end < len(pattern) and (
                pattern[end].isalnum() or pattern[end] == "_"
            ):
                end += 1
            if pattern[offset:end] == "if" and pattern[max(0, offset - 2) : offset] != "r#":
                return True
            offset = end
            continue
        offset += 1
    return False


def irrefutable_pattern(pattern: str) -> bool:
    candidate = strip_outer_pattern_parentheses(pattern)
    separators = top_level_character_positions(candidate, "|")
    alternatives = [
        candidate[start:end].strip()
        for start, end in zip(
            [0, *(position + 1 for position in separators)],
            [*separators, len(candidate)],
        )
        if candidate[start:end].strip()
    ]
    if separators:
        return any(irrefutable_pattern(alternative) for alternative in alternatives)

    bindings = top_level_character_positions(candidate, "@")
    if bindings:
        binding = bindings[0]
        left = re.sub(r"^(?:(?:ref|mut)\s+)+", "", candidate[:binding].strip())
        return bool(
            re.fullmatch(r"(?:r#)?[a-z_][A-Za-z0-9_]*", left)
            and irrefutable_pattern(candidate[binding + 1 :])
        )

    candidate = re.sub(r"^(?:(?:ref|mut)\s+)+", "", candidate).strip()
    return bool(
        candidate == "_"
        or re.fullmatch(r"(?:r#)?[a-z_][A-Za-z0-9_]*", candidate)
    )


def match_arm_pattern(source: str, offset: int) -> tuple[str, int] | None:
    start = skip_outer_attributes(source, offset)
    depths = {"(": 0, "[": 0, "{": 0}
    closers = {")": "(", "]": "[", "}": "{"}
    cursor = start
    while cursor < len(source):
        character = source[cursor]
        if character in depths:
            depths[character] += 1
        elif character in closers:
            opening = closers[character]
            if depths[opening]:
                depths[opening] -= 1
            elif character == "}":
                return None
        elif not any(depths.values()):
            if character == "=" and cursor + 1 < len(source) and source[cursor + 1] == ">":
                return source[start:cursor].strip(), start
            if character in {",", ";"}:
                return None
        cursor += 1
    return None


def legacy_adapter_catch_all(source: str) -> int | None:
    for boundary in re.finditer(r"^|[,{}]", source, re.MULTILINE):
        arm = match_arm_pattern(source, boundary.end())
        if arm is None:
            continue
        pattern, offset = arm
        if not pattern_has_top_level_guard(pattern) and irrefutable_pattern(pattern):
            return offset
    return None


def future_boundary_failures(root: Path) -> list[Failure]:
    root = root.resolve()
    failures: list[Failure] = []
    schema_dependency = re.compile(r"\b(?:mech_runtime|mech_engine|crate::runtime|crate::engine)\b")
    corpus = analyzed_production_corpus(root)
    semantic_forbidden_names = transitive_semantic_forbidden_names(corpus)
    finalized_type_aliases = finalized_semantic_type_aliases(corpus)
    for path, source, searchable, tokens in corpus:
        resolved = path.resolve()
        relative = path.relative_to(root).as_posix()
        for line, type_name, implementation in manual_deserialize_impls(
            tokens, finalized_type_aliases
        ):
            failures.append(
                failure(
                    "C1-FINALIZED-SERDE-BOUNDARY",
                    type_name,
                    relative,
                    "no Deserialize implementation; construction requires the validated API",
                    implementation,
                    "src/core/tests/semantic_serde_contract.rs",
                    line,
                )
            )
        for type_name in FINALIZED_SEMANTIC_SERDE_TYPES.get(relative, set()):
            attribute = semantic_serde_attribute(source, type_name)
            if attribute is not None and attribute != '#[cfg_attr(feature = "serde", derive(Serialize))]':
                failures.append(
                    failure(
                        "C1-FINALIZED-SERDE-BOUNDARY",
                        type_name,
                        relative,
                        "Serialize only; construction requires the validated API",
                        attribute or "missing serde derive",
                        "src/core/tests/semantic_serde_contract.rs",
                    )
                )
        for type_name in OPEN_SEMANTIC_SERDE_TYPES.get(relative, set()):
            attribute = semantic_serde_attribute(source, type_name)
            if attribute is not None and attribute != '#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]':
                failures.append(
                    failure(
                        "C1-OPEN-SERDE-BOUNDARY",
                        type_name,
                        relative,
                        "Serialize + Deserialize on open semantic syntax",
                        attribute or "missing serde derive",
                        "src/core/tests/semantic_serde_contract.rs",
                    )
                )
        for type_name in NON_SERDE_SEMANTIC_TYPES.get(relative, set()):
            attribute = semantic_serde_attribute(source, type_name)
            if attribute:
                failures.append(
                    failure(
                        "C1-EPHEMERAL-HANDLE-SERDE-BOUNDARY",
                        type_name,
                        relative,
                        "no standalone Serde construction for builder-local handles",
                        attribute,
                        "src/core/tests/schema_contract.rs",
                    )
                )
        builder_spec = {
            "src/core/src/dimension.rs": (
                "pub fn declare(",
                "pub fn declarations(",
                "Result<DimensionParameterId, SemanticModelError>",
            ),
            "src/core/src/schema/table.rs": (
                "pub fn insert(",
                "pub fn finish(",
                "Result<SchemaHandle, SemanticModelError>",
            ),
        }.get(relative)
        if builder_spec is not None:
            start_marker, end_marker, result_type = builder_spec
            segment = semantic_builder_segment(searchable, start_marker, end_marker)
            if segment is not None:
                panic_api = re.search(r"\b(?:assert|panic)\s*!|\.\s*expect\s*\(", segment)
                compact_segment = "".join(segment.split())
                compact_result = "".join(result_type.split())
                if panic_api is not None or compact_result not in compact_segment:
                    actual = (
                        panic_api.group(0)
                        if panic_api is not None
                        else "missing structured Result signature"
                    )
                    failures.append(
                        failure(
                            "C1-SEMANTIC-BUILDER-ERROR",
                            start_marker,
                            relative,
                            f"{result_type} without assert!/panic!/expect",
                            actual,
                            "src/core/tests/dimension_contract.rs",
                        )
                    )
        c1_semantic_module = any(
            module_file_or_descendant(root, resolved, name)
            for name in (
                "semantic_identity",
                "nominal",
                "dimension",
                "kind_expr",
                "kind_scheme",
                "schema",
            )
        )
        if c1_semantic_module:
            direct_forbidden = next(
                (
                    token
                    for token in tokens
                    if GENERATOR.canonical_identifier(token.value)
                    in SEMANTIC_FORBIDDEN_IDENTIFIERS
                ),
                None,
            )
            declared_dependencies = set().union(
                *declared_type_dependencies(tokens).values()
            )
            imported_dependencies = {
                binding.local
                for binding in GENERATOR.LEGACY_SCANNER.use_bindings(tokens)
                if binding.path
                and not binding.glob
                and binding.path[-1] in semantic_forbidden_names
            }
            transitive_forbidden = next(
                iter(
                    sorted(
                        (declared_dependencies | imported_dependencies)
                        & (semantic_forbidden_names - SEMANTIC_FORBIDDEN_IDENTIFIERS)
                    )
                ),
                None,
            )
            physical = re.search(
                r"\bbuffer\s+strategy\b|\bstride\b|\bcapacity\b", searchable
            )
            if direct_forbidden is not None or transitive_forbidden is not None or physical is not None:
                symbol = (
                    direct_forbidden.value
                    if direct_forbidden is not None
                    else transitive_forbidden
                    if transitive_forbidden is not None
                    else physical.group(0)
                )
                transitive_token = (
                    next(
                        (
                            token
                            for token in tokens
                            if GENERATOR.canonical_identifier(token.value)
                            == transitive_forbidden
                        ),
                        None,
                    )
                    if transitive_forbidden is not None
                    else None
                )
                failures.append(
                    failure(
                        "C1-SEMANTIC-BOUNDARY",
                        symbol,
                        relative,
                        "semantic model independent of mutable values, resident state, and physical layout",
                        "forbidden production dependency",
                        "src/core/src/legacy_adapter/",
                        direct_forbidden.line
                        if direct_forbidden is not None
                        else transitive_token.line
                        if transitive_token is not None
                        else source.count("\n", 0, physical.start()) + 1,
                        direct_forbidden.column
                        if direct_forbidden is not None
                        else transitive_token.column
                        if transitive_token is not None
                        else None,
                    )
                )
        if module_file_or_descendant(root, resolved, "legacy_adapter"):
            catch_all = legacy_adapter_catch_all(searchable)
            if catch_all is not None:
                failures.append(
                    failure(
                        "C1-LEGACY-ADAPTER-EXHAUSTIVE",
                        "catch-all match arm",
                        relative,
                        "explicit outcome for every current Kind and ValueKind variant",
                        "binding or wildcard fallback",
                        "src/core/src/legacy_adapter/",
                        source.count("\n", 0, catch_all) + 1,
                    )
                )
        if module_file_or_descendant(root, resolved, "snapshot"):
            resolver = GENERATOR.LEGACY_SCANNER.TransparentTypeResolver(
                tokens, relative=relative
            )
            forbidden = next(
                (
                    token
                    for token in tokens
                    if GENERATOR.canonical_identifier(token.value)
                    == "ReactiveCellId"
                    or resolver.resolve([token]) == "Ref"
                ),
                None,
            )
            if forbidden is not None:
                failures.append(
                    failure(
                        "C0-SNAPSHOT-LEGACY-IMPORT",
                        GENERATOR.canonical_identifier(forbidden.value),
                        relative,
                        "snapshot module independent of legacy identity/storage",
                        "forbidden legacy symbol",
                        "src/core/src/legacy_adapter/",
                        forbidden.line,
                        forbidden.column,
                    )
                )
        if module_file_or_descendant(root, resolved, "schema"):
            match = schema_dependency.search(searchable)
            if match is not None:
                failures.append(
                    failure(
                        "C0-SCHEMA-DEPENDENCY",
                        match.group(0),
                        relative,
                        "schema module independent of runtime and engine crates",
                        "forbidden dependency",
                        "src/core/src/schema/",
                        source.count("\n", 0, match.start()) + 1,
                    )
                )
        mentions_legacy = re.search(r"\bLegacyValue\b", searchable) is not None
        mentions_snapshot = re.search(r"\bsnapshot\s*::\s*Value\b", searchable) is not None
        if (
            mentions_legacy
            and mentions_snapshot
            and not module_file_or_descendant(root, resolved, "legacy_adapter")
        ):
            failures.append(
                failure(
                    "C0-ADAPTER-COEXISTENCE",
                    "LegacyValue+snapshot::Value",
                    relative,
                    "both representations mentioned only by legacy_adapter",
                    "non-adapter module mentions both",
                    "src/core/src/legacy_adapter/",
                )
            )
        conversion = blanket_conversion(tokens)
        if conversion is not None:
            line, declaration = conversion
            failures.append(
                failure(
                    "C0-BLANKET-CONVERSION",
                    "legacy/snapshot conversion",
                    relative,
                    "fallible context-aware adapter function",
                    declaration,
                    "src/core/src/legacy_adapter/",
                    line,
                )
            )
        if (
            relative.startswith("src/engine/")
            and ("artifact" in path.parts or path.stem == "artifact")
            and mentions_legacy
        ):
            failures.append(
                failure(
                    "C0-ENGINE-LEGACY-ARTIFACT",
                    "LegacyValue",
                    relative,
                    "new engine artifact API accepts immutable snapshot values",
                    "LegacyValue dependency",
                    "tests/architecture/value-system/migration.json",
                )
            )
    canonical_test = root / "src/core/tests/canonical_schema_vectors.rs"
    if canonical_test.is_file():
        canonical_source = canonical_test.read_text(encoding="utf-8")
        for marker in (".finalize()", ".instantiate_shape("):
            if marker not in canonical_source:
                failures.append(
                    failure(
                        "C1-FINALIZED-CONSTRUCTION",
                        marker,
                        canonical_test.relative_to(root).as_posix(),
                        "canonical conformance constructs finalized values through validation APIs",
                        "validation marker absent",
                        "src/core/tests/semantic_serde_contract.rs",
                    )
                )
    validation_routes = {
        "src/core/src/kind_expr.rs": (
            "validate_kind_structure(kind)?;",
            "KindNameCategory::RecordField",
            "KindNameCategory::TableColumn",
        ),
        "src/core/src/kind_scheme.rs": ("validate_kind_structure(kind)?;",),
        "src/core/src/legacy_adapter/kind.rs": (
            "validate_legacy_kind_resolution(&kind, &dimension_parameters)?;",
            "canonicalize_dimension_environment(declarations, &all_declarations)?;",
            "normalize_dimension(dimension, declarations.len())",
        ),
        "src/core/src/schema/shape.rs": (".checked_mul(extent)",),
    }
    for relative, markers in validation_routes.items():
        path = root / relative
        if not path.is_file():
            continue
        source = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in source:
                failures.append(
                    failure(
                        "C1-VALIDATED-CONSTRUCTION-ROUTE",
                        marker,
                        relative,
                        "C1 semantic construction retains every validated entry route",
                        "validation marker absent",
                        "src/core/tests/semantic_serde_contract.rs",
                    )
                )
    return failures


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


GATE_B_SELF_PROTECTED_PATHS = {
    "scripts/check-value-system-contract.py",
    "scripts/generate-value-system-inventory.py",
    "scripts/value_system_legacy_scanner_v1.py",
    "scripts/tests/canonical_encoding_v1_reference.py",
    "tests/architecture/value-system/canonical-encoding-v1-schema.json",
    "tests/architecture/value-system/canonical-encoding-v1-vectors-schema.json",
    "tests/architecture/value-system/canonical-encoding-v1-vectors.json",
    "tests/architecture/value-system/canonical-encoding-v1.json",
    "tests/architecture/value-system/gate-b-regression.json",
    "tests/architecture/value-system/legacy-growth-baseline-schema.json",
    "tests/architecture/value-system/legacy-growth-baseline.json",
}
C0_INITIAL_SEMANTIC_CORE_BLOBS = {
    "src/core/src/value.rs": (
        "2903ed7345809a5711cbdd04316b5834f026e9bf",
        "2a6abce928d43072c3bbc19c5b20b4242bdef363",
    ),
}
REQUIRED_GATE_B_PROTECTED_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain",
    "rust-toolchain.toml",
    ".cargo/config",
    ".cargo/config.toml",
    "src/core/Cargo.toml",
    "src/engine/Cargo.toml",
    "src/runtime/Cargo.toml",
    "src/core/build.rs",
    "src/engine/build.rs",
    "src/runtime/build.rs",
    "src/core/src/lib.rs",
    "src/engine/src/lib.rs",
    "src/runtime/src/lib.rs",
    "src/core/src/resident_execution.rs",
    "src/runtime/src/resident_gate_b.rs",
    "src/runtime/src/turn_record.rs",
    "src/runtime/benches/resident_ekf.rs",
    "src/engine/tests/resident_ekf_contract.rs",
    "src/runtime/tests/resident_gate_b_contract.rs",
    "scripts/run-gate-b-benchmarks.py",
    "scripts/check-gate-b-contract.py",
    "scripts/generate-gate-b-ekf-trace.py",
    "scripts/generate-value-system-inventory.py",
    "scripts/value_system_legacy_scanner_v1.py",
    "scripts/tests/canonical_encoding_v1_reference.py",
    "benchmarks/runtime/gate-b/result-schema.json",
    "benchmarks/runtime/gate-b/ekf-v1.json",
    "benchmarks/runtime/gate-b/b0-controls.json",
    "benchmarks/runtime/gate-b/ekf-input-v1.bin",
    "benchmarks/runtime/gate-b/ekf-input-v1.sha256",
    "benchmarks/runtime/gate-b/numpy/ekf_v1.py",
    "benchmarks/runtime/gate-b/README.md",
    "tests/architecture/value-system/canonical-encoding-v1-schema.json",
    "tests/architecture/value-system/canonical-encoding-v1-vectors-schema.json",
    "tests/architecture/value-system/canonical-encoding-v1-vectors.json",
    "tests/architecture/value-system/canonical-encoding-v1.json",
    "tests/architecture/value-system/legacy-growth-baseline-schema.json",
    "tests/architecture/value-system/legacy-growth-baseline.json",
}
REQUIRED_GATE_B_PROTECTED_PREFIXES = {
    ".cargo/",
    "src/core/src/",
    "src/engine/src/resident/",
    "src/runtime/src/ledger/",
    "src/runtime/benches/support/gate_b/",
}


def gate_b_affected_path(contract: dict[str, Any], path: str) -> bool:
    protected = contract["protected_paths"]
    return path in GATE_B_SELF_PROTECTED_PATHS or path in set(protected["exact"]) or any(
        path.startswith(prefix) for prefix in protected["prefixes"]
    )


def gate_b_evidence_pointer_only_change(
    root: Path, report_commit: str, path: str
) -> bool:
    if path != "tests/architecture/value-system/gate-b-regression.json":
        return False
    documents: list[dict[str, Any]] = []
    for revision in (report_commit, "HEAD"):
        process = subprocess.run(
            ["git", "show", f"{revision}:{path}"],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if process.returncode != 0:
            return False
        try:
            document = json.loads(process.stdout)
        except json.JSONDecodeError:
            return False
        for field in ("evidence_commit", "evidence_sha256"):
            document.pop(field, None)
        documents.append(document)
    return documents[0] == documents[1]


def gate_b_initial_support_introduction(
    root: Path, report_commit: str, path: str
) -> bool:
    """Allow C0 to introduce its checker/config once, but freeze later edits."""
    if path not in GATE_B_SELF_PROTECTED_PATHS:
        return False
    existed = subprocess.run(
        ["git", "cat-file", "-e", f"{report_commit}:{path}"],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if existed.returncode == 0:
        return False
    additions = subprocess.run(
        ["git", "log", "--reverse", "--diff-filter=A", "--format=%H", "--", path],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    commits = additions.stdout.splitlines() if additions.returncode == 0 else []
    if not commits:
        return False
    unchanged = subprocess.run(
        ["git", "diff", "--quiet", f"{commits[0]}..HEAD", "--", path],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return unchanged.returncode == 0


def gate_b_initial_semantic_core_qualification(
    root: Path, report_commit: str, path: str
) -> bool:
    """Grandfather only C0's exact qualification-only ValueKind rewrite."""
    expected = C0_INITIAL_SEMANTIC_CORE_BLOBS.get(path)
    if expected is None:
        return False
    actual: list[str] = []
    for revision in (report_commit, "HEAD"):
        process = subprocess.run(
            ["git", "rev-parse", f"{revision}:{path}"],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if process.returncode != 0:
            return False
        actual.append(process.stdout.strip())
    return tuple(actual) == expected


def gate_b_failures(
    root: Path,
    contract: dict[str, Any],
    contract_path: Path,
    *,
    enforce_freshness: bool,
) -> list[Failure]:
    failures: list[Failure] = []
    protected = contract["protected_paths"]
    exact = set(protected["exact"])
    prefixes = set(protected["prefixes"])
    for subject, expected, actual in (
        ("required exact paths", REQUIRED_GATE_B_PROTECTED_EXACT, exact),
        ("required prefixes", REQUIRED_GATE_B_PROTECTED_PREFIXES, prefixes),
    ):
        missing = sorted(expected - actual)
        if missing:
            failures.append(
                failure(
                    "C0-GATE-B-PROTECTION-DRIFT",
                    subject,
                    str(contract_path),
                    repr(sorted(expected)),
                    f"missing {missing!r}",
                    f"{contract_path}:protected_paths",
                )
            )
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
    required_lanes = set(contract["validation_policy"]["required_rerun_lanes"])
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
    if enforce_freshness:
        report_commit = report.get("git_commit")
        ancestor = subprocess.run(
            ["git", "merge-base", "--is-ancestor", str(report_commit), "HEAD"],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if ancestor.returncode != 0:
            failures.append(
                failure(
                    "C0-GATE-B-EVIDENCE-STALE",
                    "evidence source commit",
                    ".git",
                    "Gate B report commit is an ancestor of HEAD",
                    ancestor.stderr.strip() or f"git exit {ancestor.returncode}",
                    str(contract_path),
                )
            )
            return failures
        process = subprocess.run(
            [
                "git",
                "diff",
                "--no-renames",
                "--name-only",
                f"{report_commit}..HEAD",
                "--",
            ],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if process.returncode != 0:
            failures.append(
                failure(
                    "C0-GATE-B-EVIDENCE-STALE",
                    "evidence source commit",
                    ".git",
                    "diffable Gate B evidence commit",
                    process.stderr.strip() or f"git exit {process.returncode}",
                    str(contract_path),
                )
            )
        else:
            affected = sorted(
                path
                for path in process.stdout.splitlines()
                if gate_b_affected_path(contract, path)
                and not gate_b_initial_support_introduction(
                    root, str(report_commit), path
                )
                and not gate_b_evidence_pointer_only_change(
                    root, str(report_commit), path
                )
                and not gate_b_initial_semantic_core_qualification(
                    root, str(report_commit), path
                )
            )
            if affected:
                failures.append(
                    failure(
                        "C0-GATE-B-EVIDENCE-STALE",
                        "semantic-core-or-resident-hot-path",
                        affected[0],
                        "fresh controlled Gate B evidence containing every affected change",
                        repr(affected),
                        "benchmarks/runtime/gate-b/b2-resident-turn.json",
                    )
                )
    return failures


def reference_failures(
    root: Path,
    inventory: dict[str, Any],
    migration: dict[str, Any],
    baseline: dict[str, Any],
    gate_b: dict[str, Any],
    verify_git: bool,
) -> list[Failure]:
    references = {
        "current-inventory.json": inventory.get("reference_commit"),
        "migration.json": migration.get("reference_commit"),
        "legacy-growth-baseline.json": baseline.get("reference_commit"),
        "gate-b-regression.json": gate_b.get("reference_commit"),
    }
    failures: list[Failure] = []
    if len(set(references.values())) != 1:
        failures.append(
            failure(
                "C0-REFERENCE-COMMIT",
                "reference_commit",
                "tests/architecture/value-system",
                "one exact reviewed B2 commit across all manifests",
                repr(references),
                "tests/architecture/value-system/*.json",
            )
        )
        return failures
    reference = next(iter(references.values()))
    if verify_git:
        process = subprocess.run(
            ["git", "merge-base", "--is-ancestor", str(reference), "HEAD"],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if process.returncode != 0:
            failures.append(
                failure(
                    "C0-REFERENCE-COMMIT",
                    str(reference),
                    ".git",
                    "existing reviewed B2 ancestor of HEAD",
                    process.stderr.strip() or f"git exit {process.returncode}",
                    "tests/architecture/value-system/*.json:reference_commit",
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


def audit(
    root: Path = ROOT,
    inventory_path: Path = DEFAULT_INVENTORY,
    migration_path: Path = DEFAULT_MIGRATION,
    gate_b_path: Path = DEFAULT_GATE_B,
    inventory_schema_path: Path = DEFAULT_INVENTORY_SCHEMA,
    migration_schema_path: Path = DEFAULT_MIGRATION_SCHEMA,
    *,
    baseline_path: Path = DEFAULT_LEGACY_BASELINE,
    baseline_schema_path: Path = DEFAULT_LEGACY_BASELINE_SCHEMA,
    canonical_path: Path = DEFAULT_CANONICAL_ENCODING,
    canonical_schema_path: Path = DEFAULT_CANONICAL_ENCODING_SCHEMA,
    canonical_vectors_path: Path = DEFAULT_CANONICAL_VECTORS,
    canonical_vectors_schema_path: Path = DEFAULT_CANONICAL_VECTORS_SCHEMA,
    frozen_targets_path: Path = DEFAULT_FROZEN_TARGETS,
    frozen_targets_schema_path: Path = DEFAULT_FROZEN_TARGETS_SCHEMA,
    verify_reference: bool = True,
    check_gate_a: bool = True,
    baseline_inventory: dict[str, Any] | None = None,
) -> list[Failure]:
    root = root.resolve()
    inventory = load_json(inventory_path)
    migration = load_json(migration_path)
    gate_b = load_json(gate_b_path)
    inventory_schema = load_json(inventory_schema_path)
    migration_schema = load_json(migration_schema_path)
    baseline = baseline_inventory or load_json(baseline_path)
    baseline_schema = load_json(baseline_schema_path)
    canonical = load_json(canonical_path)
    canonical_schema = load_json(canonical_schema_path)
    canonical_vectors = load_json(canonical_vectors_path)
    canonical_vectors_schema = load_json(canonical_vectors_schema_path)
    frozen_targets = load_json(frozen_targets_path)
    frozen_targets_schema = load_json(frozen_targets_schema_path)
    failures: list[Failure] = []
    for contract_id, payload, schema, path in (
        ("C0-INVENTORY-SCHEMA", inventory, inventory_schema, inventory_path),
        ("C0-MIGRATION-SCHEMA", migration, migration_schema, migration_path),
        ("C0-LEGACY-BASELINE-SCHEMA", baseline, baseline_schema, baseline_path),
        ("C0-CANONICAL-ENCODING-SCHEMA", canonical, canonical_schema, canonical_path),
        (
            "C0-CANONICAL-ENCODING-VECTORS-SCHEMA",
            canonical_vectors,
            canonical_vectors_schema,
            canonical_vectors_path,
        ),
        (
            "C0-FROZEN-TARGET-SCHEMA",
            frozen_targets,
            frozen_targets_schema,
            frozen_targets_path,
        ),
    ):
        for message in schema_errors(payload, schema):
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
    scanner_failures = immutable_baseline_failures(
        root, baseline, baseline_path, verify_git=False
    )
    if scanner_failures:
        return sorted_failures(scanner_failures)
    try:
        live = GENERATOR.generate(root, inventory["reference_commit"])
    except GENERATOR.CargoMetadataError as error:
        return [
            failure(
                "C0-CARGO-METADATA",
                "authoritative workspace source discovery",
                str(root),
                "successful cargo metadata with at least one workspace package and Rust target",
                str(error),
                f"{inventory_path}:workspace_packages",
            )
        ]
    except GENERATOR.AuxiliaryFixtureError as error:
        return [
            failure(
                "C0-AUXILIARY-FIXTURE",
                "trybuild fixture discovery",
                str(root),
                "literal fixture paths confined to their package",
                str(error),
                f"{inventory_path}:auxiliary_rust_fixtures",
            )
        ]
    except ValueError as error:
        return [
            failure(
                "C0-KIND-SCHEME-SEPARATION",
                "generated type-contract source shape",
                str(root),
                "reviewed declaration forms, field types, and separated semantic/runtime layers",
                str(error),
                f"{inventory_path}:type_contract_sources",
            )
        ]
    failures.extend(
        reference_failures(
            root, inventory, migration, baseline, gate_b, verify_reference
        )
    )
    if baseline_inventory is None:
        failures.extend(
            immutable_baseline_failures(
                root, baseline, baseline_path, verify_git=verify_reference
            )
        )
    failures.extend(coverage_failures(live, migration, migration_path))
    failures.extend(family_contract_failures(live, migration, migration_path))
    failures.extend(target_applicability_failures(migration, migration_path))
    failures.extend(occurrence_classification_failures(live, migration, migration_path))
    failures.extend(frozen_semantics_failures(migration, migration_path))
    failures.extend(matrix_value_classification_failures(migration, migration_path))
    failures.extend(
        frozen_target_failures(
            migration, frozen_targets, migration_path, frozen_targets_path
        )
    )
    failures.extend(
        frozen_occurrence_target_failures(
            migration, frozen_targets, migration_path, frozen_targets_path
        )
    )
    failures.extend(type_contract_source_failures(root, live, inventory_path))
    failures.extend(auxiliary_fixture_failures(inventory, live, inventory_path))
    failures.extend(workspace_source_coverage_failures(inventory, live, inventory_path))
    failures.extend(source_disposition_failures(inventory, inventory_path))
    failures.extend(qualification_failures(root, live))
    failures.extend(high_risk_failures(baseline, live, baseline_path))
    failures.extend(legacy_alias_baseline_failures(baseline, live, baseline_path))
    failures.extend(compatibility_alias_failures(baseline, live, baseline_path))
    failures.extend(raw_approved_alias_failures(live))
    if inventory_path.read_text(encoding="utf-8") != GENERATOR.render(live):
        failures.append(
            failure(
                "C0-INVENTORY-DRIFT",
                "generated inventory",
                str(inventory_path),
                "byte-equivalent live inventory",
                "committed inventory differs from live generation",
                str(inventory_path),
            )
        )
    if check_gate_a:
        failures.extend(gate_a_failures(root, inventory))
    failures.extend(future_boundary_failures(root))
    failures.extend(canonical_encoding_failures(canonical, canonical_path))
    failures.extend(
        canonical_vector_failures(
            canonical_vectors, canonical_vectors_path, canonical
        )
    )
    failures.extend(
        gate_b_failures(
            root,
            gate_b,
            gate_b_path,
            enforce_freshness=verify_reference,
        )
    )
    return sorted_failures(failures)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--migration", type=Path, default=DEFAULT_MIGRATION)
    parser.add_argument("--gate-b", type=Path, default=DEFAULT_GATE_B)
    parser.add_argument("--inventory-schema", type=Path, default=DEFAULT_INVENTORY_SCHEMA)
    parser.add_argument("--migration-schema", type=Path, default=DEFAULT_MIGRATION_SCHEMA)
    parser.add_argument("--legacy-baseline", type=Path, default=DEFAULT_LEGACY_BASELINE)
    parser.add_argument("--legacy-baseline-schema", type=Path, default=DEFAULT_LEGACY_BASELINE_SCHEMA)
    parser.add_argument("--canonical-encoding", type=Path, default=DEFAULT_CANONICAL_ENCODING)
    parser.add_argument("--canonical-encoding-schema", type=Path, default=DEFAULT_CANONICAL_ENCODING_SCHEMA)
    parser.add_argument("--canonical-vectors", type=Path, default=DEFAULT_CANONICAL_VECTORS)
    parser.add_argument("--canonical-vectors-schema", type=Path, default=DEFAULT_CANONICAL_VECTORS_SCHEMA)
    parser.add_argument("--frozen-targets", type=Path, default=DEFAULT_FROZEN_TARGETS)
    parser.add_argument("--frozen-targets-schema", type=Path, default=DEFAULT_FROZEN_TARGETS_SCHEMA)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        failures = audit(
            args.root,
            args.inventory,
            args.migration,
            args.gate_b,
            args.inventory_schema,
            args.migration_schema,
            baseline_path=args.legacy_baseline,
            baseline_schema_path=args.legacy_baseline_schema,
            canonical_path=args.canonical_encoding,
            canonical_schema_path=args.canonical_encoding_schema,
            canonical_vectors_path=args.canonical_vectors,
            canonical_vectors_schema_path=args.canonical_vectors_schema,
            frozen_targets_path=args.frozen_targets,
            frozen_targets_schema_path=args.frozen_targets_schema,
        )
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"value-system contract checker failed internally: {error}", file=sys.stderr)
        return 2
    if failures:
        print("value-system contract failed:", file=sys.stderr)
        for item in failures:
            print(f"  {item.render()}", file=sys.stderr)
        return 1
    print("value-system contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
