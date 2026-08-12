#!/usr/bin/env python3
"""Generate the deterministic D3 resident-external architecture projections."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "tests/architecture/resident-external"
EFFECT_SOURCE = DIRECTORY / "d3-effect-source-v1.mec"
TRANSACTIONAL_SOURCE = DIRECTORY / "d3-transactional-source-v1.mec"
D2_HEAD = "96fd051608f9d9df9eb4e9b345af7c23279c6c67"
OUTPUTS = {
    name: DIRECTORY / f"d3-{name}-v1.json"
    for name in (
        "profile",
        "requirements",
        "effect-artifact",
        "transactional-artifact",
        "replay",
        "failure-matrix",
    )
}
SCHEMAS = {
    name: DIRECTORY / f"d3-{name}-v1-schema.json" for name in OUTPUTS
}


def render(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def values() -> dict[str, object]:
    profile = {
        "bytecode_format": "v1",
        "d2_semantic_head": D2_HEAD,
        "durability": ["Volatile", "Retained"],
        "feature": "resident-external",
        "forbidden_delivery": ["ProviderDefined", "Stream", "Future"],
        "gate": "D3",
        "host_function_requirements": "rejected",
        "normal_product_routing_changed": False,
        "resident_candidate_provider_calls": 0,
        "schema_version": 1,
        "supported_values": ["Bool", "Index", "F64", "dense matrix"],
    }
    requirements = {
        "artifact_authority": "ApplicationRequirementTable",
        "effect_program_revision": "44fb824d14fdd39eb9f484309920fcbc36befab657800390ac5c42547382cead",
        "effect_source_sha256": sha256(EFFECT_SOURCE),
        "hidden_operation_name_is_authority": False,
        "node_requirement_field": "Option<ApplicationRequirementId>",
        "requirement_count_per_fixture": 2,
        "schema_version": 1,
        "source_bytecode_equal": True,
        "transactional_program_revision": "9f2cf91a5016a418c827c45e7aa3c98d4ece8b7d0e7ccae3c5f26aa1362fd5be",
        "transactional_source_sha256": sha256(TRANSACTIONAL_SOURCE),
    }
    effect_artifact = {
        "effect": {
            "base_uri": "gate-d3://scene/output",
            "context_name": "output",
            "delivery": "Snapshot",
            "idempotency": "Required",
            "intent": "Send",
            "operation": "write",
            "path": "frame",
            "protocol": "AfterCommit/IdempotentRetry",
            "semantic_outputs": 0,
        },
        "legacy_opaque": 0,
        "observation": {
            "base_uri": "gate-d3://input/value",
            "context_name": "value",
            "delivery": "Live",
            "intent": "Read",
            "operation": "read",
            "path": "sample",
            "replay": "CaptureAsInputFact",
        },
        "program_revision": requirements["effect_program_revision"],
        "schema_version": 1,
        "unclassified_nodes": 0,
    }
    transactional_artifact = {
        "effect": {
            "base_uri": "gate-d3://transactional/state",
            "context_name": "state",
            "delivery": "Snapshot",
            "intent": "Send",
            "operation": "write",
            "path": "value",
            "protocol": "PrepareCommitCompensate",
            "semantic_outputs": 0,
        },
        "legacy_opaque": 0,
        "observation_requirement": effect_artifact["observation"],
        "program_revision": requirements["transactional_program_revision"],
        "schema_version": 1,
        "unclassified_nodes": 0,
    }
    replay = {
        "effect_batch_hash_exact": True,
        "effect_ids_exact": True,
        "idempotency_keys_exact": True,
        "provider_prepare_calls": 0,
        "provider_reads": 0,
        "schema_version": 1,
        "state_hash_exact": True,
        "uses_canonical_captured_values": True,
    }
    cases = [
        "unauthorized requirement",
        "provider missing",
        "provider contract mismatch",
        "input-ledger reservation failure",
        "provider observation failure",
        "input schema mismatch",
        "candidate execution failure",
        "integrity rejection",
        "effect payload materialization failure",
        "provider effect preparation failure",
        "wrong PreparedRuntimeEffect protocol",
        "accepted receipt preparation failure",
        "outbox preparation failure",
        "transactional prepare failure",
        "reverse transactional abort",
        "compensatable apply failure",
        "reverse compensation",
        "cleanup failure and poison",
        "successful publication",
        "transactional commit failure after publication",
        "after-commit delivery failure",
        "AtMostOnce terminal failure",
        "AtLeastOnce retained failure",
        "IdempotentRetry same-key retry",
        "replay with zero provider reads",
        "source/bytecode equivalence",
    ]
    failure_matrix = {
        "accepted_publication_stores": 1,
        "cases": [{"name": case, "required": True} for case in cases],
        "prepublication_deliveries": 0,
        "rejected_publication_stores": 0,
        "schema_version": 1,
    }
    return {
        "profile": profile,
        "requirements": requirements,
        "effect-artifact": effect_artifact,
        "transactional-artifact": transactional_artifact,
        "replay": replay,
        "failure-matrix": failure_matrix,
    }


def schema(name: str, value: object) -> object:
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": f"https://mech-lang.org/contracts/d3-{name}-v1-schema.json",
        "const": value,
        "title": f"Mech D3 {name} contract v1",
    }


def generated_files() -> dict[Path, bytes]:
    projections = values()
    generated = {OUTPUTS[name]: render(value) for name, value in projections.items()}
    generated.update(
        {SCHEMAS[name]: render(schema(name, projections[name])) for name in projections}
    )
    return generated


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    failures = []
    for path, content in generated_files().items():
        if args.check:
            if not path.exists() or path.read_bytes() != content:
                failures.append(f"{path.relative_to(ROOT)} is not the mechanical D3 projection")
        else:
            path.write_bytes(content)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
