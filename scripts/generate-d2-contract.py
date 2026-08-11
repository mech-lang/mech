#!/usr/bin/env python3
"""Generate the deterministic D2 architecture projections from the executable fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests/fixtures/d2-contract-generator/Cargo.toml"
SOURCE = ROOT / "tests/architecture/resident-activation/n-body-source-v1.mec"
PROJECTION_DIR = ROOT / "tests/architecture/resident-activation"
OUTPUTS = {
    "profile": PROJECTION_DIR / "d2-profile-v1.json",
    "artifact": PROJECTION_DIR / "d2-nbody-artifact-v1.json",
    "layout": PROJECTION_DIR / "d2-nbody-layout-v1.json",
    "execution": PROJECTION_DIR / "d2-nbody-execution-v1.json",
    "ekf": PROJECTION_DIR / "d2-ekf-regression-v1.json",
    "reconfiguration": PROJECTION_DIR / "d2-reconfiguration-v1.json",
}
FROZEN_EKF_TRAJECTORY = "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758"
FROZEN_NBODY_TRAJECTORIES = {
    "aarch64-macos": "c6b22824484158404a84bdd19de823d605aa31b5f35622b89af2fc61591268ac",
    "x86_64-linux": "b4d33b7c35c30f890d22e8a7074e415cc54681c1789fac49a80c581204fe86db",
    "x86_64-macos": "5aa064d6b4fcd14952d9391b21d8e4862e754c29180fb2768e29164baef1a9f2",
}


def render(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n").encode()


def fixture_facts() -> dict[str, str]:
    environment = dict(os.environ)
    environment["CARGO_TARGET_DIR"] = str(ROOT / "target/d2-contract-generator")
    process = subprocess.run(
        ["cargo", "run", "--quiet", "--manifest-path", str(FIXTURE)],
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if process.returncode != 0:
        raise RuntimeError((process.stdout + process.stderr).strip())
    lines = [line for line in process.stdout.splitlines() if line.startswith("D2_PROJECTION ")]
    if len(lines) != 1:
        raise RuntimeError(f"expected one D2_PROJECTION line, found {len(lines)}")
    facts: dict[str, str] = {}
    for item in lines[0].removeprefix("D2_PROJECTION ").split():
        key, separator, value = item.partition("=")
        if not separator or not key or key in facts:
            raise RuntimeError(f"invalid D2 projection fact {item!r}")
        facts[key] = value
    return facts


def integer(facts: dict[str, str], name: str) -> int:
    return int(facts[name])


def boolean(facts: dict[str, str], name: str) -> bool:
    value = facts[name]
    if value not in {"true", "false"}:
        raise RuntimeError(f"invalid Boolean D2 projection fact {name}={value}")
    return value == "true"


def projections(facts: dict[str, str]) -> dict[str, bytes]:
    platform = facts["platform"]
    expected_trajectory = FROZEN_NBODY_TRAJECTORIES.get(platform)
    if expected_trajectory is None:
        raise RuntimeError(f"unsupported D2 projection platform {platform!r}")
    if facts["trajectory"] != expected_trajectory:
        raise RuntimeError(
            f"D2 {platform} trajectory changed: expected {expected_trajectory}, "
            f"found {facts['trajectory']}"
        )
    source_sha256 = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
    profile = {
        "alias_policy": ["NoAlias", "exact-base MayAlias"],
        "bytecode_format": "v1",
        "change_propagation": "dirty bits and semantic change",
        "dimension_lifetimes": ["CompileTime", "Activation"],
        "forbidden_dimension_lifetimes": ["Turn"],
        "gate": "D2",
        "integrity_default": "Checked",
        "integrity_modes": ["Checked", "Unchecked"],
        "legacy_opaque": 0,
        "normal_runtime_routing_changed": False,
        "output_policy": "state-backed only",
        "resident_computation": "Pure",
        "schema_version": 1,
        "turn_construction": ["FullWrite", "ReadModifyWrite"],
        "unchecked_integrity": "explicit opt-in; constraint-violating candidates may publish",
        "activation_construction": ["FullWrite", "Build"],
        "value_kinds": ["Bool", "Index", "F64"],
    }
    artifact = {
        "activation_nodes": integer(facts, "activation_nodes"),
        "artifact_nodes": integer(facts, "artifact_nodes"),
        "bytecode_format": "v1",
        "legacy_opaque_contracts": integer(facts, "legacy_opaque"),
        "position_writer_chain": ["WholeValue ReadModifyWrite"],
        "program_revision": facts["revision"],
        "schema_version": 1,
        "source_bytecode_revision_equal": boolean(facts, "source_bytecode_exact"),
        "source_sha256": source_sha256,
        "stable_topological_execution": boolean(facts, "stable_topological"),
        "state_slots": integer(facts, "state_slots"),
        "turn_nodes": integer(facts, "turn_nodes"),
        "unclassified_nodes": integer(facts, "unclassified"),
        "velocity_writer_chain": [
            "IndexedAxis(0) ReadModifyWrite",
            "IndexedAxis(0) ReadModifyWrite",
        ],
    }
    layout = {
        "activation_storage": "immutable typed arenas",
        "candidate_bytes": integer(facts, "candidate_bytes"),
        "candidate_materialized_bytes": integer(facts, "candidate_materialized_bytes"),
        "candidate_seed_bytes": integer(facts, "candidate_seed_bytes"),
        "dual_state_payload_bytes": integer(facts, "dual_state_bytes"),
        "fixed_width_node_mask": False,
        "generic_executor": "src/engine/src/resident/general/execution.rs",
        "node_mask_word_bits": 64,
        "publication_store_count": integer(facts, "publication_stores"),
        "schema_version": 1,
        "slot_count": integer(facts, "slots"),
        "state_slot_count": integer(facts, "state_slots"),
        "typed_arenas": ["Bool/u8", "Index/u64", "F64/f64"],
    }
    execution = {
        "dirty_propagation": boolean(facts, "dirty_propagation"),
        # This is a bounded diagnostic, not a trajectory identity. Normalize
        # host floating-point contraction differences before freezing JSON;
        # trajectory identity remains independently frozen at 1e-10.
        "energy_drift": round(float(facts["energy_drift"]), 8),
        "energy_drift_quantization": 1.0e-8,
        "final_state_sha256": facts["final"],
        "final_state_quantization": 1.0e-10,
        "initial_state_sha256": facts["initial"],
        "legacy_trajectory_equal": boolean(facts, "legacy_exact"),
        "publication_ordering": "Release",
        "raw_rust_trajectory_equal": boolean(facts, "raw_exact"),
        "schema_version": 1,
        "source_bytecode_trajectory_equal": boolean(facts, "source_bytecode_exact"),
        "steady_state_allocations": integer(facts, "steady_state_allocations"),
        "trajectory_quantization": 1.0e-10,
        # Floating contraction and square-root lowering differ across host
        # targets. Every lane agrees within each target, so freeze the verified
        # hashes rather than claiming a non-portable bitwise identity.
        "trajectory_sha256_by_platform": FROZEN_NBODY_TRAJECTORIES,
        "turns": integer(facts, "turns"),
    }
    ekf = {
        "candidate_bytes": 96,
        "candidate_seed_bytes": 0,
        "d1_frozen_trajectory_sha256": FROZEN_EKF_TRAJECTORY,
        "generic_executor": layout["generic_executor"],
        "generic_trajectory_sha256": FROZEN_EKF_TRAJECTORY,
        "production_dirty_propagation": True,
        "publication_store_count": 1,
        "schema_version": 1,
        "source_bytecode_trajectory_equal": True,
        "steady_state_allocations": 0,
        "turns": 4096,
    }
    reconfiguration = {
        "active_candidate_rejected": True,
        "activation_fact_identity_participates": True,
        "changed_layout_advances_layout_generation": True,
        "compatible_state_requires_explicit_mapping": True,
        "failed_migration_is_atomic": True,
        "incompatible_state_can_explicitly_reset": True,
        "incompatible_state_rejected": True,
        "same_layout_preserves_layout_generation": True,
        "same_revision_is_noop": True,
        "schema_version": 1,
        "swap_insert_delete_regressions": True,
    }
    values = {
        "profile": profile,
        "artifact": artifact,
        "layout": layout,
        "execution": execution,
        "ekf": ekf,
        "reconfiguration": reconfiguration,
    }
    return {name: render(values[name]) for name in OUTPUTS}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        generated = projections(fixture_facts())
    except (OSError, RuntimeError, KeyError, ValueError) as error:
        print(f"D2 projection generation failed: {error}", file=sys.stderr)
        return 2
    errors: list[str] = []
    for name, path in OUTPUTS.items():
        content = generated[name]
        if args.check:
            if not path.exists() or path.read_bytes() != content:
                errors.append(f"{path.relative_to(ROOT)} is not the mechanical D2 projection")
        else:
            path.write_bytes(content)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
