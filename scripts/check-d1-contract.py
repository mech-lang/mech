#!/usr/bin/env python3
"""Enforce the complete D1 public-ProgramArtifact resident EKF contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
D0_FINAL = "a9422eff9908e967e0537f7ec1fa56e7bd05eb8d"
D1_FINAL = "7ff20887ea2d267b790917608c4bc8826b031762"
BRANCH = "feat/resident-ekf-program-artifact-path"
COMMIT_SUBJECTS = [
    "feat(engine): close the frozen EKF ProgramArtifact [D1A]",
    "feat(engine): activate the frozen EKF ProgramArtifact [D1B]",
    "feat(runtime): execute the activated EKF resident plan [D1C]",
    "bench(architecture): record D1 Gate B evidence [D1D]",
]
EVIDENCE = "benchmarks/runtime/gate-b/b2-resident-turn.json"
GATE_B_REGRESSION = "tests/architecture/value-system/gate-b-regression.json"
PROJECTION_DIR = Path("tests/architecture/resident-activation")
PROJECTION_SHA256 = {
    "d1-artifact-v1.json": "8bba3ee55ecc7a5324853b8181a544bd3a37543383d975702d41fb0c284b9e24",
    "d1-activation-v1.json": "8e8f4c9260ae4b7e8fbfc5bde0998def7daf49fc7ff85da1810dc834cec64bda",
    "d1-execution-v1.json": "9f2dede0a0893af64b90e08432beb7bde9944f48338a51d9ecaba0f98ec13d2a",
}
D1_ALLOWED_CHANGES = (
    ".github/workflows/ci.yml",
    EVIDENCE,
    "benchmarks/runtime/gate-b/result-schema.json",
    "docs/design/program-artifact-resident-activation.md",
    "docs/spec/bytecode-v1.md",
    "scripts/check-d1-contract.py",
    "scripts/check-bytecode-v1-format.py",
    "scripts/check-gate-b-contract.py",
    "scripts/check-operation-contract.py",
    "scripts/check-program-artifact-contract.py",
    "scripts/check-resident-activation-contract.py",
    "scripts/check-value-system-contract.py",
    "scripts/generate-d1-contract.py",
    "scripts/run-gate-b-benchmarks.py",
    "scripts/tests/test_check_gate_b_contract.py",
    "scripts/tests/test_check_d1_contract.py",
    "scripts/tests/test_check_resident_activation_contract.py",
    "scripts/tests/test_check_value_system_contract.py",
    "scripts/tests/test_generate_d1_contract.py",
    "scripts/tests/test_generate_resident_activation_contract.py",
    "scripts/tests/test_run_gate_b_benchmarks.py",
    "src/build/src/analysis/requirements/external.rs",
    "src/build/src/analysis/requirements/tests.rs",
    "src/bytecode/src/context.rs",
    "src/core/src/execution.rs",
    "src/core/src/program/bytecode/reader.rs",
    "src/core/src/program/bytecode/runtime_contracts.rs",
    "src/core/src/program/bytecode/tests.rs",
    "src/core/src/program/compiler/api.rs",
    "src/engine/Cargo.toml",
    "src/engine/src/artifact/compiler.rs",
    "src/engine/src/artifact/model.rs",
    "src/engine/src/efficacy/",
    "src/engine/src/expressions/tests/variables.rs",
    "src/engine/src/expressions/variables.rs",
    "src/engine/src/function/external/mod.rs",
    "src/engine/src/function/external/resource_read.rs",
    "src/engine/src/interpreter/mod.rs",
    "src/engine/src/interpreter/tests/bytecode.rs",
    "src/engine/src/intrinsics/assign/mod.rs",
    "src/engine/src/intrinsics/define.rs",
    "src/engine/src/lib.rs",
    "src/engine/src/program/instance.rs",
    "src/engine/src/resident/",
    "src/engine/src/statements/context.rs",
    "src/engine/src/statements/errors.rs",
    "src/engine/src/statements/mod.rs",
    "src/engine/src/statements/tests/context.rs",
    "src/engine/src/statements/tests/mod.rs",
    "src/engine/tests/bytecode_plan_topology.rs",
    "src/engine/tests/resident_activation_contract.rs",
    "src/engine/tests/resident_ekf_artifact_closure.rs",
    "src/engine/tests/resident_ekf_program_activation.rs",
    "src/engine/tests/resident_ekf_program_execution.rs",
    "src/runtime/Cargo.toml",
    "src/runtime/benches/resident_ekf.rs",
    "src/runtime/benches/support/gate_b/legacy_atomic.rs",
    "src/runtime/benches/support/gate_b/mod.rs",
    "src/runtime/benches/support/gate_b/resident_artifact.rs",
    "src/runtime/src/resident_gate_b.rs",
    "src/runtime/src/runtime/execution/tests/source.rs",
    "src/runtime/src/runtime/execution_session.rs",
    "src/runtime/src/runtime/test_support/providers.rs",
    "src/runtime/src/runtime/transaction/reactive.rs",
    "src/runtime/tests/native_live_resource.rs",
    "src/runtime/tests/resident_gate_b_contract.rs",
    "src/runtime/tests/ui/sealed/reactive_participant_reuse_after_commit.stderr",
    "src/runtime/tests/ui/sealed/reactive_participant_reuse_after_rollback.stderr",
    "src/runtime/tests/ui/sealed/runtime_value_snapshot_raw_access.stderr",
    "tests/architecture/bytecode-v1/",
    "tests/architecture/resident-activation/",
    "tests/architecture/value-execution/legacy-boundary.json",
    "tests/architecture/value-system/current-inventory.json",
    "tests/architecture/value-system/frozen-semantic-targets-v1.json",
    GATE_B_REGRESSION,
    "tests/architecture/value-system/migration-schema.json",
    "tests/architecture/value-system/migration.json",
    "tests/fixtures/d1-contract-generator/",
)
RESIDENT_FORBIDDEN = (
    "LegacyValue",
    "ValRef",
    "MutableReference",
    "ReactiveCellId",
    "ValueStateJournal",
    "ReactiveTurnJournal",
    "RuntimeExecutionTransaction",
    "RuntimeTransaction",
    "transaction_state_values",
    "commit_runtime",
)


def command(args: list[str], root: Path = ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=root, text=True, capture_output=True)


def allowed_path(path: str) -> bool:
    return any(
        path == allowed or (allowed.endswith("/") and path.startswith(allowed))
        for allowed in D1_ALLOWED_CHANGES
    )


def changed_path_errors(paths: list[str]) -> list[str]:
    unexpected = sorted(path for path in paths if not allowed_path(path))
    return [] if not unexpected else ["D1 changed path is outside the exact allowlist: " + ", ".join(unexpected)]


def branch_name_errors(result: subprocess.CompletedProcess[str]) -> list[str]:
    if result.returncode != 0:
        return ["unable to determine the D1 branch name"]
    branch = result.stdout.strip()
    if branch and branch != BRANCH:
        return [f"D1 branch must remain {BRANCH}; found {branch}"]
    return []


def projection_errors(projections: dict[str, dict]) -> list[str]:
    errors: list[str] = []
    artifact = projections["artifact"]
    activation = projections["activation"]
    execution = projections["execution"]
    expected_artifact = {
        "artifact_nodes": 21,
        "resident_kernels": 15,
        "integrity_predicates": 3,
        "state_updates": 2,
        "integrity_constraints": 3,
        "observation_roots": 1,
        "change_detection": {
            "observation_always_changed": 1,
            "pure_kernel_kernel_reported": 17,
            "pure_predicate_exact_scalar": 3,
        },
        "legacy_opaque_contracts": 0,
        "unclassified_nodes": 0,
        "source_bytecode_revision_equal": True,
        "bytecode_format": "v1",
    }
    expected_activation = {
        "activated_node_count": 20,
        "resident_kernel_count": 15,
        "predicate_kernel_count": 3,
        "state_copy_count": 2,
        "constraint_count": 3,
        "physical_slot_count": 21,
        "persistent_candidate_bytes": 96,
        "activation_executes_turn": False,
        "deterministic_reactivation": True,
        "published_epoch": 0,
        "first_candidate_epoch": 1,
    }
    expected_execution = {
        "turns": 4096,
        "numeric_kernels_per_turn": 15,
        "predicate_kernels_per_turn": 3,
        "state_copies_per_turn": 2,
        "constraints_per_turn": 3,
        "candidate_seed_bytes": 0,
        "candidate_written_bytes": 96,
        "published_buffer_copy_bytes": 0,
        "publication_store_count": 1,
        "publication_ordering": "Release",
        "reader_ordering": "Acquire",
        "commit_runtime_calls": 0,
        "legacy_journal_captures": 0,
        "source_bytecode_trajectory_equal": True,
        "gate_b_control_trajectory_equal": True,
        "abort_preserves_published_epoch": True,
        "normal_runtime_routing_changed": False,
        "ordinary_ekf_vertical_slice": "complete",
        "admitted_artifacts": 1,
        "migrated_state_slots": 2,
        "global_d_targets_implemented": 0,
        "legacy_targets_removed": 0,
        "legacy_occurrences_migrated": 0,
    }
    for name, projection, expected in (
        ("artifact", artifact, expected_artifact),
        ("activation", activation, expected_activation),
        ("execution", execution, expected_execution),
    ):
        for field, value in expected.items():
            if projection.get(field) != value:
                errors.append(f"D1 {name} projection has wrong {field}")
    if artifact.get("program_revision") != activation.get("program_revision"):
        errors.append("D1 artifact and activation ProgramRevision differ")
    if execution.get("trajectory_sha256") != "ddca8ab17cb390839d4c77e7cecc5203122f249685f5a28c36fd342cf303a758":
        errors.append("D1 execution trajectory hash changed")
    if execution.get("trace_sha256") != "ab901e1d115aa92166dc2a6d45a28732e6a548363b829997aa410ae4c2d77c8b":
        errors.append("D1 execution trace hash changed")
    return errors


def source_contract_errors(root: Path) -> list[str]:
    errors: list[str] = []
    resident_dir = root / "src/engine/src/resident"
    resident_sources = {
        path.relative_to(root).as_posix(): path.read_text()
        for path in resident_dir.rglob("*.rs")
    }
    for path, source in resident_sources.items():
        for token in RESIDENT_FORBIDDEN:
            if re.search(rf"\b{re.escape(token)}\b", source):
                errors.append(f"D1 resident source {path} contains forbidden {token}")

    declarations = []
    declaration = re.compile(r"(?m)^\s*(?P<visibility>pub(?:\([^)]*\))?\s+)?struct\s+ProgramArtifact\s*\{")
    for path in (root / "src").glob("*/src/**/*.rs"):
        source = path.read_text()
        for match in declaration.finditer(source):
            declarations.append(
                (path.relative_to(root).as_posix(), (match.group("visibility") or "").strip())
            )
    if declarations != [("src/engine/src/artifact/model.rs", "pub")]:
        errors.append(f"D1 must retain one public ProgramArtifact authority; found {declarations}")

    execution_path = resident_dir / "program_execution.rs"
    activation_path = resident_dir / "program_activation.rs"
    if not execution_path.exists():
        execution_path = resident_dir / "general" / "execution.rs"
        activation_path = resident_dir / "general/mod.rs"
    execution = execution_path.read_text()
    for token in (
        "ProgramArtifact",
        "SchemaTable",
        "OperationContractTable",
        "canonical_bytes",
        "value_hash",
        "snapshot_from_legacy",
        "legacy_from_snapshot",
    ):
        if token in execution:
            errors.append(f"D1 hot artifact execution performs forbidden {token} lookup")
    for required in ("Ordering::Release", ".store(", "Ordering::Acquire", ".load("):
        if required not in execution and required not in activation_path.read_text():
            errors.append(f"D1 publication boundary is missing {required}")

    route_markers = re.compile(r"compile_frozen_ekf_source|__gate_d|resident-ekf-artifact")
    for package in ("engine", "runtime"):
        directory = root / f"src/{package}/src"
        for path in directory.rglob("*.rs"):
            relative = path.relative_to(root).as_posix()
            if relative.endswith("/tests.rs") or "/tests/" in relative:
                continue
            if relative == "src/engine/src/lib.rs" or relative.startswith(
                ("src/engine/src/efficacy/", "src/engine/src/resident/")
            ) or relative in {
                "src/engine/src/intrinsics/assign/mod.rs",
                "src/runtime/src/resident_gate_b.rs",
            }:
                continue
            if route_markers.search(path.read_text()):
                errors.append(f"D1 efficacy route escaped into normal production routing at {relative}")

    benchmark = (root / "src/runtime/benches/resident_ekf.rs").read_text()
    for lane in ("mech-resident-artifact-source", "mech-resident-artifact-bytecode"):
        if lane not in benchmark:
            errors.append(f"D1 Gate B benchmark lane {lane} is missing")
    return errors


def projection_files(root: Path) -> tuple[dict[str, dict], list[str]]:
    errors = []
    projections = {}
    for filename, expected_hash in PROJECTION_SHA256.items():
        path = root / PROJECTION_DIR / filename
        content = path.read_bytes()
        digest = hashlib.sha256(content).hexdigest()
        if digest != expected_hash:
            errors.append(f"D1 projection {filename} digest {digest} != pinned {expected_hash}")
        projections[filename.removeprefix("d1-").removesuffix("-v1.json")] = json.loads(content)
    return projections, errors


def topology_errors(root: Path, implementation_head: bool) -> list[str]:
    errors = []
    ancestor = command(["git", "merge-base", "--is-ancestor", D0_FINAL, "HEAD"], root)
    if ancestor.returncode != 0:
        return [f"D1 must descend from exact final D0 head {D0_FINAL}"]
    expected = COMMIT_SUBJECTS[:3] if implementation_head else COMMIT_SUBJECTS
    log = command(["git", "log", "--format=%s", "--reverse", f"{D0_FINAL}..HEAD"], root)
    subjects = log.stdout.splitlines() if log.returncode == 0 else []
    if subjects != expected:
        errors.append(f"D1 canonical commit stack must be {expected}; found {subjects}")
    errors.extend(branch_name_errors(command(["git", "branch", "--show-current"], root)))
    changed = command(["git", "diff", "--name-only", D0_FINAL, "HEAD"], root)
    if changed.returncode != 0:
        errors.append("unable to enumerate D1 changed paths")
    else:
        errors.extend(changed_path_errors(changed.stdout.splitlines()))
    if implementation_head:
        return errors
    evidence_diff = command(["git", "diff", "--name-only", "HEAD^", "HEAD"], root)
    expected_evidence_paths = sorted((EVIDENCE, GATE_B_REGRESSION))
    if sorted(evidence_diff.stdout.splitlines()) != expected_evidence_paths:
        errors.append("D1D must change only the Gate B evidence report and regression pointer")
    implementation = command(["git", "rev-parse", "HEAD^"], root).stdout.strip()
    try:
        report_bytes = (root / EVIDENCE).read_bytes()
        report = json.loads(report_bytes)
        if report.get("git_commit") != implementation:
            errors.append("D1D Gate B report must name the exact D1C implementation SHA")
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"unable to read D1D Gate B evidence: {error}")
        report_bytes = None
    try:
        pointer = json.loads((root / GATE_B_REGRESSION).read_text())
        if pointer.get("evidence_path") != EVIDENCE:
            errors.append("D1D Gate B regression pointer must name the exact evidence path")
        if pointer.get("evidence_commit") != implementation:
            errors.append("D1D Gate B regression pointer must name the exact D1C implementation SHA")
        if report_bytes is not None:
            report_digest = hashlib.sha256(report_bytes).hexdigest()
            if pointer.get("evidence_sha256") != report_digest:
                errors.append("D1D Gate B regression pointer must pin the exact evidence digest")
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"unable to read D1D Gate B regression pointer: {error}")
    return errors


def subprocess_error(label: str, process: subprocess.CompletedProcess[str]) -> list[str]:
    if process.returncode == 0:
        return []
    output = (process.stdout + process.stderr).strip()
    return [f"{label} failed: {output}"]


def run(root: Path = ROOT, *, contract_only: bool = False, implementation_head: bool = False) -> list[str]:
    projections, errors = projection_files(root)
    errors.extend(projection_errors(projections))
    errors.extend(source_contract_errors(root))
    generator = command([sys.executable, "scripts/generate-d1-contract.py", "--check"], root)
    errors.extend(subprocess_error("D1 mechanical projection", generator))
    if contract_only:
        return errors
    errors.extend(topology_errors(root, implementation_head))
    for label, args in (
        ("D0 inherited contract", [sys.executable, "scripts/check-resident-activation-contract.py"]),
        ("C3 inherited contract", [sys.executable, "scripts/check-program-artifact-contract.py"]),
        ("bytecode-v1 contract", [sys.executable, "scripts/check-bytecode-v1-format.py"]),
    ):
        errors.extend(subprocess_error(label, command(args, root)))
    if not implementation_head:
        implementation = command(["git", "rev-parse", "HEAD^"], root).stdout.strip()
        gate_b = command(
            [
                sys.executable,
                "scripts/check-gate-b-contract.py",
                "--report",
                EVIDENCE,
                "--expected-commit",
                implementation,
            ],
            root,
        )
        errors.extend(subprocess_error("D1 Gate B evidence", gate_b))
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract-only", action="store_true")
    parser.add_argument("--implementation-head", action="store_true")
    args = parser.parse_args()
    inherited_descendant = (
        not args.contract_only
        and not args.implementation_head
        and command(["git", "rev-parse", "HEAD"]).stdout.strip() != D1_FINAL
        and command(["git", "merge-base", "--is-ancestor", D1_FINAL, "HEAD"]).returncode == 0
    )
    errors = run(
        contract_only=args.contract_only or inherited_descendant,
        implementation_head=args.implementation_head,
    )
    if errors:
        for error in errors:
            print(f"D1 contract failure: {error}", file=sys.stderr)
        return 1
    mode = "inherited contract" if inherited_descendant else "contract"
    print(f"D1 public-ProgramArtifact resident EKF {mode}: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
