#!/usr/bin/env python3
"""Enforce the D2 general pure numeric resident architecture contract."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROJECTION_DIR = ROOT / "tests/architecture/resident-activation"
FILES = {
    "profile": "d2-profile-v1.json",
    "artifact": "d2-nbody-artifact-v1.json",
    "layout": "d2-nbody-layout-v1.json",
    "execution": "d2-nbody-execution-v1.json",
    "ekf": "d2-ekf-regression-v1.json",
    "reconfiguration": "d2-reconfiguration-v1.json",
}
D1_FINAL = "7ff20887ea2d267b790917608c4bc8826b031762"
IMPLEMENTATION_SUBJECTS = [
    "feat(engine,machines): close the ordinary n-body ProgramArtifact [D2A]",
    "refactor(engine,core): generalize resident activation and storage [D2B]",
    "feat(runtime): execute general numeric artifacts and n-body residently [D2C]",
]
FROZEN_NBODY_TRAJECTORIES = {
    "aarch64-macos": "c6b22824484158404a84bdd19de823d605aa31b5f35622b89af2fc61591268ac",
    "x86_64-linux": "b4d33b7c35c30f890d22e8a7074e415cc54681c1789fac49a80c581204fe86db",
    "x86_64-macos": "5aa064d6b4fcd14952d9391b21d8e4862e754c29180fb2768e29164baef1a9f2",
}


def load(root: Path) -> tuple[dict[str, dict], list[str]]:
    projections: dict[str, dict] = {}
    errors: list[str] = []
    for name, filename in FILES.items():
        path = root / "tests/architecture/resident-activation" / filename
        try:
            projections[name] = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as error:
            errors.append(f"unable to read {filename}: {error}")
    return projections, errors


def projection_errors(projections: dict[str, dict]) -> list[str]:
    errors: list[str] = []
    if len(projections) != len(FILES):
        return errors
    profile = projections["profile"]
    artifact = projections["artifact"]
    layout = projections["layout"]
    execution = projections["execution"]
    ekf = projections["ekf"]
    reconfiguration = projections["reconfiguration"]

    expected = {
        ("profile", "bytecode_format"): "v1",
        ("profile", "integrity_default"): "Checked",
        ("profile", "integrity_modes"): ["Checked", "Unchecked"],
        (
            "profile",
            "unchecked_integrity",
        ): "explicit opt-in; constraint-violating candidates may publish",
        ("profile", "legacy_opaque"): 0,
        ("profile", "normal_runtime_routing_changed"): False,
        ("artifact", "legacy_opaque_contracts"): 0,
        ("artifact", "unclassified_nodes"): 0,
        ("artifact", "source_bytecode_revision_equal"): True,
        ("artifact", "stable_topological_execution"): True,
        ("artifact", "state_slots"): 2,
        ("layout", "candidate_bytes"): 480,
        ("layout", "candidate_seed_bytes"): 480,
        ("layout", "candidate_materialized_bytes"): 480,
        ("layout", "dual_state_payload_bytes"): 960,
        ("layout", "fixed_width_node_mask"): False,
        ("layout", "publication_store_count"): 1,
        ("execution", "dirty_propagation"): True,
        ("execution", "legacy_trajectory_equal"): True,
        ("execution", "raw_rust_trajectory_equal"): True,
        ("execution", "source_bytecode_trajectory_equal"): True,
        ("execution", "steady_state_allocations"): 0,
        ("execution", "turns"): 4096,
        ("ekf", "production_dirty_propagation"): True,
        ("ekf", "source_bytecode_trajectory_equal"): True,
        ("ekf", "steady_state_allocations"): 0,
        ("ekf", "candidate_bytes"): 96,
        ("ekf", "candidate_seed_bytes"): 0,
        ("reconfiguration", "activation_fact_identity_participates"): True,
        ("reconfiguration", "compatible_state_requires_explicit_mapping"): True,
        ("reconfiguration", "failed_migration_is_atomic"): True,
        ("reconfiguration", "swap_insert_delete_regressions"): True,
    }
    for (projection, field), value in expected.items():
        if projections[projection].get(field) != value:
            errors.append(f"D2 {projection} projection has wrong {field}")
    if execution.get("trajectory_sha256_by_platform") != FROZEN_NBODY_TRAJECTORIES:
        errors.append("D2 signed n-body trajectory hash changed")
    if abs(float(execution.get("energy_drift", 1.0))) > 1.0e-3:
        errors.append("D2 signed n-body energy drift exceeds the frozen bound")
    if ekf.get("generic_trajectory_sha256") != ekf.get("d1_frozen_trajectory_sha256"):
        errors.append("D2 generic EKF trajectory differs from frozen D1")
    if profile.get("dimension_lifetimes") != ["CompileTime", "Activation"]:
        errors.append("D2 admitted dimension lifetime profile changed")
    if profile.get("forbidden_dimension_lifetimes") != ["Turn"]:
        errors.append("D2 must reject Turn dimensions")
    return errors


def source_errors(root: Path) -> list[str]:
    errors: list[str] = []
    general = (root / "src/engine/src/resident/general/mod.rs").read_text()
    execution = (root / "src/engine/src/resident/general/execution.rs").read_text()
    resident = general + execution
    for token in ("LegacyValue", "ValRef", "eager_turn"):
        if re.search(rf"\b{re.escape(token)}\b", resident):
            errors.append(f"D2 generic resident path contains forbidden {token}")
    for marker in (
        "ResidentActivationOptions::default()",
        "ResidentIntegrityMode::Checked",
        "ResidentIntegrityMode::Unchecked",
        "activate_with_options",
        "stable_topological_order",
        "same_turn_downstream_masks: Box<[Box<[u64]>]>",
        "TurnDimension",
        "DimensionLifetime::Turn",
        "OutputConstruction::ReadModifyWrite",
        "Ordering::Release",
        "Ordering::Acquire",
    ):
        if marker not in resident:
            errors.append(f"D2 generic resident path is missing {marker}")
    if re.search(r"\bunsafe\s*\{", resident):
        errors.append("D2 generic resident path contains an unsafe execution shortcut")
    if re.search(r"(?:dirty|executed|downstream|root|mandatory)[A-Za-z0-9_ ]*mask\s*:\s*(?:Box<\[)?u32", resident, re.I):
        errors.append("D2 generic resident path retains a fixed-width u32 node mask")
    numeric = (root / "src/engine/src/resident/numeric.rs").read_text()
    if "contract.outputs.len() != 1" not in numeric:
        errors.append("D2 resident binders do not enforce one output")
    if "Build node reached the resident turn graph" not in (
        root / "tests/fixtures/d2-contract-generator/src/main.rs"
    ).read_text():
        errors.append("D2 fixture does not enforce Build-only activation")
    engine_manifest = (root / "src/engine/Cargo.toml").read_text()
    if re.search(r"^resident-ekf-artifact\s*=", engine_manifest, re.M):
        errors.append("D2 retained the obsolete resident-ekf-artifact feature alias")
    benchmark = (root / "src/runtime/benches/resident_ekf.rs").read_text()
    for lane in (
        "mech-resident-artifact-kernel-source-unchecked",
        "mech-resident-artifact-kernel-bytecode-unchecked",
    ):
        if lane not in benchmark:
            errors.append(f"D2 benchmark is missing explicit {lane} lane")
    regression = (root / "src/engine/tests/resident_ekf_program_execution.rs").read_text()
    if "unchecked_integrity_is_explicit_and_omits_constraint_only_nodes" not in regression:
        errors.append("D2 is missing the checked/unchecked integrity regression")
    return errors


def topology_errors(root: Path) -> list[str]:
    errors: list[str] = []
    ancestor = subprocess.run(
        ["git", "merge-base", "--is-ancestor", D1_FINAL, "HEAD"], cwd=root
    )
    if ancestor.returncode != 0:
        return [f"D2 must descend from exact D1 head {D1_FINAL}"]
    log = subprocess.run(
        ["git", "log", "--format=%s", "--reverse", f"{D1_FINAL}..HEAD"],
        cwd=root,
        text=True,
        capture_output=True,
    )
    subjects = log.stdout.splitlines() if log.returncode == 0 else []
    if subjects != IMPLEMENTATION_SUBJECTS:
        errors.append(f"D2A-D2C commit stack must be {IMPLEMENTATION_SUBJECTS}; found {subjects}")
    return errors


def run(root: Path = ROOT, *, implementation_head: bool = False) -> list[str]:
    projections, errors = load(root)
    errors.extend(projection_errors(projections))
    errors.extend(source_errors(root))
    if implementation_head:
        errors.extend(topology_errors(root))
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--implementation-head", action="store_true")
    args = parser.parse_args()
    errors = run(implementation_head=args.implementation_head)
    if errors:
        for error in errors:
            print(f"D2 contract failure: {error}", file=sys.stderr)
        return 1
    print("D2 general pure numeric resident contract: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
