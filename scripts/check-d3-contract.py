#!/usr/bin/env python3
"""Enforce the D3 resident external-turn architecture contract."""

from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
GENERATOR_SPEC = importlib.util.spec_from_file_location(
    "generate_d3_contract", ROOT / "scripts/generate-d3-contract.py"
)
GENERATOR = importlib.util.module_from_spec(GENERATOR_SPEC)
assert GENERATOR_SPEC.loader is not None
GENERATOR_SPEC.loader.exec_module(GENERATOR)


def projection_errors(root: Path) -> list[str]:
    errors = []
    expected = GENERATOR.generated_files()
    for canonical_path, content in expected.items():
        path = root / canonical_path.relative_to(ROOT)
        if not path.exists() or path.read_bytes() != content:
            errors.append(f"stale D3 projection {path.relative_to(root)}")
    return errors


def source_errors(root: Path) -> list[str]:
    errors = []
    artifact = (root / "src/engine/src/artifact/model.rs").read_text()
    bytecode = (root / "src/engine/src/artifact/bytecode.rs").read_text()
    resident = "\n".join(
        path.read_text() for path in (root / "src/engine/src/resident").rglob("*.rs")
    )
    coordinator = (root / "src/runtime/src/resident_external/coordinator.rs").read_text()
    provider = (root / "src/runtime/src/resident_external/provider.rs").read_text()
    execution = (root / "src/engine/src/resident/general/execution.rs").read_text()
    resident_plan = (root / "src/engine/src/resident/general/mod.rs").read_text()
    runtime_manifest = (root / "src/runtime/Cargo.toml").read_text()
    core_bytecode = (root / "src/core/src/program/bytecode/header.rs").read_text()
    test_provider = (root / "src/runtime/src/resident_external/test_provider.rs").read_text()
    gate_d_runner = (root / "scripts/run-gate-d-benchmarks.py").read_text()
    effect_journal = (root / "src/runtime/src/effect_journal.rs").read_text()
    gate_d_test = (root / "src/runtime/tests/resident_external_gate_d3.rs").read_text()
    resident_external_tests = (
        root / "src/runtime/src/resident_external/tests.rs"
    ).read_text()
    runtime_lib = (root / "src/runtime/src/lib.rs").read_text()
    ci = (root / ".github/workflows/ci.yml").read_text()
    rust_sources = {
        path.relative_to(root).as_posix(): path.read_text()
        for path in (root / "src").rglob("*.rs")
    }
    authority_impl_paths = sorted(
        path
        for path, source in rust_sources.items()
        if "unsafe impl ResidentExternalPublicationAuthority" in source
    )
    publication_call_paths = sorted(
        path
        for path, source in rust_sources.items()
        if ".publish_external(" in source
        and path != "src/engine/src/resident/general/execution.rs"
    )

    required = {
        "artifact requirement authority": "requirements: super::ApplicationRequirementTable" in artifact,
        "node requirement identity": "pub requirement: Option<ApplicationRequirementId>" in artifact,
        "embedded bytecode requirement graph": "struct WireGraph" in bytecode,
        "provider contract resolver": "ResidentExternalContractResolver" in provider,
        "preallocated effect intents": "effect_intents: Vec::with_capacity(" in resident_plan,
        "preallocated effect payloads": all(
            token in resident
            for token in (
                "effect_payloads: TypedResidentArena",
                "capture_effect_payload(",
                "captured_payload: ResidentRegion",
            )
        ),
        "canonical captured values": "pub value: Value" in (root / "src/runtime/src/resident_external/input_facts.rs").read_text(),
        "resident external feature": re.search(r"^resident-external\s*=", runtime_manifest, re.M) is not None,
        "release publication": "Ordering::Release" in execution,
        "deterministic D3 providers": all(
            name in test_provider
            for name in ("D3InputProvider", "D3SceneProvider", "D3TransactionalProvider")
        ),
        "controlled D3 Gate D phase": "D3-resident-external" in gate_d_runner,
        "authoritative D3 Gate D invocation": all(
            token in ci
            for token in (
                "--report benchmarks/runtime/gate-d/d3-resident-external.json",
                "--expected-phase D3-resident-external",
            )
        ),
        "offline replay authority": "pub fn new_replay(" in coordinator,
        "recorded replay decisions": all(
            token in coordinator
            for token in (
                "record: &ResidentTurnRecord",
                "match record.header.status",
                "TurnRecordStatus::Rejected",
            )
        ),
        "fail-closed external publication": all(
            token in resident
            for token in (
                "StructuralOnly",
                "activate_external",
                "pub unsafe trait ResidentExternalPublicationAuthority",
                "pub fn publish_external<",
                ".plan\n            .has_external_steps()",
            )
        )
        and "Authorized" not in resident
        and "into_coordinator_parts" not in resident
        and "struct RuntimeResidentPublicationAuthority" in coordinator
        and "unsafe impl ResidentExternalPublicationAuthority for RuntimeResidentPublicationAuthority"
        in coordinator
        and "pub struct RuntimeResidentPublicationAuthority" not in coordinator
        and '#![deny(unsafe_code)]' in runtime_lib
        and coordinator.count("#[allow(unsafe_code)]") == 1
        and coordinator.count("unsafe impl ResidentExternalPublicationAuthority") == 1
        and authority_impl_paths == ["src/runtime/src/resident_external/coordinator.rs"]
        and publication_call_paths == ["src/runtime/src/resident_external/coordinator.rs"]
        and "prepared_turn\n            .publish_external(&self.publication_authority)"
        in coordinator,
        "external publication API regression": all(
            (root / path).exists()
            for path in (
                "src/runtime/tests/resident_external_sealed_api.rs",
                "src/runtime/tests/ui/resident_external_sealed/external_activation_decomposition.rs",
                "src/runtime/tests/ui/resident_external_sealed/external_publication_authority_safe_impl.rs",
            )
        ),
        "turn-coupled input capture": "pub fn capture_input_batch(" not in coordinator,
        "retained input release": "pub fn release_next_input_batch(" in coordinator,
        "retained receipt release": "pub fn release_next_receipt(" in coordinator,
        "measured outbox append probe": all(
            token in coordinator
            for token in ("outbox_batch_append_count", "self.outbox.append(prepared)")
        )
        and '"ordinary_outbox_appends": result.outbox_batch_appends' in gate_d_test,
        "sparse exact effect ordinals": "id.sequence < self.next_sequence" in effect_journal,
        "indeterminate compensation is not retried": all(
            token in effect_journal
            for token in (
                "CompensationIndeterminate",
                "entry.state = RuntimeEffectState::CompensationIndeterminate",
            )
        ),
        "preparation failure is not a delivery attempt": all(
            token in coordinator
            for token in (
                "RetainedDeliveryFailure::Preparation",
                "RetainedDeliveryFailure::Delivery",
            )
        ),
        "controlled post-candidate rejection": all(
            token in gate_d_test
            for token in (
                "D3SceneProvider::with_preparation_failures",
                '"post_candidate_rejections"',
                '"rejected_receipt_appends"',
                '"rejected_outbox_batch_appends"',
                '"rejected_provider_preparation_attempts"',
                '"rejected_delivery_count"',
            )
        )
        and all(
            token in gate_d_runner
            for token in (
                'structural["post_candidate_rejections"] == 1',
                'structural["rejected_receipt_appends"] == 1',
                'structural["rejected_outbox_batch_appends"] == 0',
                'structural["rejected_provider_preparation_attempts"] == 1',
                'structural["rejected_delivery_count"] == 0',
            )
        ),
        "volatile retry retention": all(
            token in resident_external_tests
            for token in (
                "volatile_retryable_effects_survive_failed_delivery",
                "volatile_commit_indeterminate_retains_undelivered_ordinary_effects",
            )
        )
        and "discard_volatile_outbox" not in coordinator,
        "idempotent retry admission": "requires_provider_idempotency(&effect.interaction)"
        in (root / "src/runtime/src/resident_external/provider.rs").read_text(),
        "instance-scoped effect identity": all(
            token in (root / "src/runtime/src/resident_external/outbox_delivery.rs").read_text()
            for token in ("instance.index()) << 96", "instance.generation()) << 64")
        ),
        "independent effect identity evidence": all(
            token in gate_d_test
            for token in ("receipt.effect_ids_hash", "receipt.idempotency_keys_hash")
        )
        and "effect_id_hash = effect_batch_hash.clone()" not in gate_d_test,
        "rejected turn evidence": "RejectedTurnEvidence" in coordinator
        and "input_batch_hash: evidence.input_batch_hash" in coordinator
        and "effect_batch_hash: evidence.effect_batch_hash" in coordinator,
        "executable ordinary delivery policies": all(
            token in resident_external_tests
            for token in (
                "ordinary_delivery_policies_have_executable_failure_lifecycles",
                "AfterCommitAtMostOnce",
                "AfterCommitAtLeastOnce",
                "idempotent_delivery_failure_retains_identity_and_key_for_retry",
            )
        ),
    }
    for label, present in required.items():
        if not present:
            errors.append(f"D3 source contract is missing {label}")
    for token in ("LegacyValue", "ValRef"):
        if re.search(rf"\b{token}\b", resident):
            errors.append(f"engine resident path contains forbidden {token}")
    for token in ("RuntimeExecutionTransaction", "commit_runtime", "MechStore"):
        if re.search(rf"\b{token}\b", coordinator):
            errors.append(f"resident coordinator contains forbidden {token}")
    if coordinator.count(".publish_external(&self.publication_authority)") != 1:
        errors.append("resident coordinator must publish each accepted candidate exactly once")
    reservation = coordinator.find("let input_permit = self.input_ledger.reserve")
    turn_allocation = coordinator.find("let turn = self.allocate_turn()?")
    if reservation < 0 or turn_allocation < reservation:
        errors.append("resident turn identity must be allocated after capacity preflight")
    if "BYTECODE_VERSION: u16 = 1" not in core_bytecode:
        errors.append("D3 changed bytecode away from v1")
    return errors


def topology_errors(root: Path) -> list[str]:
    if subprocess.run(
        ["git", "merge-base", "--is-ancestor", GENERATOR.D2_HEAD, "HEAD"], cwd=root
    ).returncode:
        return [f"D3 must descend from exact D2 head {GENERATOR.D2_HEAD}"]
    expected = [
        "feat(core,engine): make external requirements artifact authority [D3A]",
        "feat(engine): stage captured facts and resident effect intents [D3B]",
        "feat(runtime): coordinate resident external turns [D3C]",
    ]
    log = subprocess.run(
        ["git", "log", "--format=%s", "--reverse", f"{GENERATOR.D2_HEAD}..HEAD"],
        cwd=root,
        text=True,
        capture_output=True,
    )
    subjects = log.stdout.splitlines()
    if subjects != expected:
        return [f"D3 implementation stack must be {expected}; found {subjects}"]
    return []


def run(root: Path = ROOT, *, implementation_head: bool = False) -> list[str]:
    errors = projection_errors(root)
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
            print(f"D3 contract failure: {error}", file=sys.stderr)
        return 1
    print("D3 resident external-turn contract: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
