#!/usr/bin/env python3
"""Generate the deterministic D4 production resident-routing projections."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "tests/architecture/production-resident"
D3_HEAD = "8e0e7dfe48fc1c2b9a69606ce77b2b83ed48f89a"
N_BODY_SOURCE = ROOT / "examples/resident-n-body/n-body.mec"
N_BODY_CONFIG = ROOT / "examples/resident-n-body/mech.mcfg"
PUBLIC_N_BODY_SOURCE = ROOT / "examples/n-body/n-body.mec"
PUBLIC_N_BODY_CONFIG = ROOT / "examples/n-body/mech.mcfg"
OUTPUTS = {
    name: DIRECTORY / f"d4-{name}-v1.json"
    for name in (
        "profile",
        "routing",
        "hosts",
        "native",
        "browser",
        "nbody",
        "failure-matrix",
    )
}
SCHEMAS = {
    name: DIRECTORY / f"d4-{name}-v1-schema.json" for name in OUTPUTS
}


def render(value: object) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def values() -> dict[str, object]:
    profile = {
        "browser_project_feature": "resident-routing-source",
        "bytecode_format": "v1",
        "d3_semantic_head": D3_HEAD,
        "default_policy": "PreferResident",
        "gate": "D4",
        "implementation_commits_before_evidence": 3,
        "normal_product_routing_changed": True,
        "resident_production_implies_compiler": False,
        "resident_production_source_implies_compiler": True,
        "schema_version": 1,
        "standard_runtime_feature": "resident-routing",
    }
    routing = {
        "activation_order": [
            "provider-contract-presence",
            "semantic-preflight",
            "capability-authority",
            "provider-binding",
            "instance-id-allocation",
            "activation",
        ],
        "fallback_eligible": ["SemanticUnsupported"],
        "fallback_forbidden": [
            "AuthorizationDenied",
            "ProviderUnavailable",
            "ProviderContractMismatch",
            "InvalidArtifact",
            "InvalidBytecode",
            "AlreadyActive",
        ],
        "policies": ["PreferResident", "RequireResident", "LegacyOnly"],
        "provider_registry_frozen_while_active": True,
        "capability_grants_revalidated_each_turn": True,
        "finite_use_capabilities_rejected": True,
        "exact_input_driver_ownership": True,
        "legacy_owner_state_cleared_on_unload": True,
        "retained_admission_before_packet_release": True,
        "schema_version": 1,
        "successful_activation_can_fallback": False,
        "wildcard_fallback": False,
    }
    hosts = {
        "coalescing": "latest value from each matched packet source becomes one turn",
        "packet_values_authoritative": True,
        "duplicate_observations_share_source_value": True,
        "absent_observations_capture_bound_provider_snapshot": True,
        "admission_failure_preserves_ordered_packets": True,
        "driver_private_backlog_cleared_on_stop": True,
        "effect_only_activation_turns": 1,
        "resident_failures_surface_to_public_drain": True,
        "turn_duration_checked_before_publication": True,
        "publication_before_scene_delivery": True,
        "rejected_turn_scene_deliveries": 0,
        "scene": {
            "delivery": "AtMostOnce",
            "idempotency": "NotRequired",
            "legacy_path": "replace",
            "points_path": "points",
            "points_shape": "N×2 f64",
        },
        "schema_version": 1,
        "timer": {
            "frequency_hz": 60,
            "observation": "tick",
            "queue_policy": "latest",
            "replay": "CaptureAsInputFact",
        },
    }
    native = {
        "generated_loader": "MechRuntime::load_bytecode_program",
        "generated_runtime_compiler": False,
        "legacy_turns": 0,
        "normal_bytecode_loader": "MechRuntime::load_bytecode_program",
        "normal_source_loader": "MechRuntime::load_root_program",
        "required_release_turns": 120,
        "route": "resident-external",
        "schema_version": 1,
    }
    browser = {
        "accepted_turns_minimum": 60,
        "body_count": 10,
        "diagnostics": ["__MECH_RUNTIME_INFO__", "__MECH_LAST_FRAME__"],
        "legacy_turns": 0,
        "loader": "MechRuntime::load_root_program",
        "page_errors": 0,
        "orbit_radius_stable": True,
        "routing_policy_in_signed_payload": True,
        "public_config_sha256": sha256(PUBLIC_N_BODY_CONFIG),
        "public_source_sha256": sha256(PUBLIC_N_BODY_SOURCE),
        "render_after_input_drain": True,
        "rendered_updates_minimum": 60,
        "route": "resident-external",
        "schema_version": 1,
        "sun_fixed_at": [300.0, 300.0],
    }
    nbody = {
        "accepted_turns": 4096,
        "config_sha256": sha256(N_BODY_CONFIG),
        "durability": "Volatile",
        "final_state_sha256": "8f25d0b2dbdebb62e1ea1667e72a37eabbaf8a254f680935bb77275e1a9e640b",
        "legacy_turns": 0,
        "policy": "RequireResident",
        "requirements": 2,
        "scene_payload_source_bytecode_exact": True,
        "schema_version": 1,
        "source_bytecode_exact": True,
        "source_sha256": sha256(N_BODY_SOURCE),
        "trajectory_sha256_by_platform": {
            "aarch64-macos": "c6b22824484158404a84bdd19de823d605aa31b5f35622b89af2fc61591268ac",
            "x86_64-linux": "b4d33b7c35c30f890d22e8a7074e415cc54681c1789fac49a80c581204fe86db",
            "x86_64-macos": "5aa064d6b4fcd14952d9391b21d8e4862e754c29180fb2768e29164baef1a9f2",
        },
    }
    cases = [
        ("semantic unsupported under PreferResident", "legacy"),
        ("semantic unsupported under RequireResident", "fail-closed"),
        ("authorization denied", "fail-closed"),
        ("provider unavailable", "fail-closed"),
        ("provider contract mismatch", "fail-closed"),
        ("malformed bytecode v1", "fail-closed"),
        ("successful resident activation", "resident"),
        ("rejected resident turn", "resident-state-unchanged"),
        ("retained admission capacity exhausted", "packet-preserved-for-retry"),
        ("resident turn duration exceeded", "rejected-before-publication"),
        ("effect-only resident program", "execute-on-activation"),
    ]
    failure_matrix = {
        "cases": [
            {"case": case, "required_result": result} for case, result in cases
        ],
        "schema_version": 1,
        "wildcard_fallback": False,
    }
    return {
        "profile": profile,
        "routing": routing,
        "hosts": hosts,
        "native": native,
        "browser": browser,
        "nbody": nbody,
        "failure-matrix": failure_matrix,
    }


def schema(name: str, value: object) -> object:
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": f"https://mech-lang.org/contracts/d4-{name}-v1-schema.json",
        "const": value,
        "title": f"Mech D4 {name} contract v1",
    }


def generated_files() -> dict[Path, bytes]:
    projections = values()
    files = {OUTPUTS[name]: render(value) for name, value in projections.items()}
    files.update(
        {SCHEMAS[name]: render(schema(name, projections[name])) for name in projections}
    )
    return files


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    failures = []
    DIRECTORY.mkdir(parents=True, exist_ok=True)
    for path, content in generated_files().items():
        if args.check:
            if not path.exists() or path.read_bytes() != content:
                failures.append(f"{path.relative_to(ROOT)} is not the mechanical D4 projection")
        else:
            path.write_bytes(content)
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
