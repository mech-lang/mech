#!/usr/bin/env python3
"""Enforce the D4 production resident-routing architecture contract."""

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
GENERATOR_SPEC = importlib.util.spec_from_file_location(
    "generate_d4_contract", ROOT / "scripts/generate-d4-contract.py"
)
GENERATOR = importlib.util.module_from_spec(GENERATOR_SPEC)
assert GENERATOR_SPEC.loader is not None
GENERATOR_SPEC.loader.exec_module(GENERATOR)

IMPLEMENTATION_SUBJECTS = [
    "feat(runtime): select and own production resident programs [D4A]",
    "feat(hosts): bind resident timer and scene production providers [D4B]",
    "feat(build,wasm,browser): run resident n-body end to end [D4C]",
]
EVIDENCE_SUBJECT = "test(architecture): record D4 production resident-routing evidence [D4D]"


def projection_errors(root: Path) -> list[str]:
    errors = []
    for canonical, content in GENERATOR.generated_files().items():
        path = root / canonical.relative_to(ROOT)
        if not path.exists() or path.read_bytes() != content:
            errors.append(f"stale D4 projection {path.relative_to(root)}")
    return errors


def exact_nbody_errors(root: Path) -> list[str]:
    d2 = (root / "tests/architecture/resident-activation/n-body-source-v1.mec").read_text()
    expected = d2.replace(
        "\n  days-per-year := 365.24",
        "\n@clock := timer://clock/tick{:read(delta-seconds)}\n"
        "@scene := scene://orbit/frame\n\n"
        "  days-per-year := 365.24",
        1,
    ).replace("  Δt := 0.01", "  Δt := @clock/delta-seconds * 0.6", 1)
    expected = expected.replace("\n  positions := x", "\n  @scene/points <- x\n  positions := x", 1)
    actual = (root / "examples/resident-n-body/n-body.mec").read_text()
    return [] if actual == expected else ["n-body source is not the exact allowed D2 transformation"]


def source_errors(root: Path) -> list[str]:
    errors = []
    runtime_manifest = (root / "src/runtime/Cargo.toml").read_text()
    wasm_manifest = (root / "src/wasm/Cargo.toml").read_text()
    root_manifest = (root / "Cargo.toml").read_text()
    config = (root / "examples/resident-n-body/mech.mcfg").read_text()
    loader = (root / "src/runtime/src/runtime/resident_program/load.rs").read_text()
    admission = (root / "src/runtime/src/runtime/resident_program/admission.rs").read_text()
    authority = (root / "src/runtime/src/runtime/resident_program/authority.rs").read_text()
    routing = (root / "src/runtime/src/runtime/resident_program/route.rs").read_text()
    runtime_state = (root / "src/runtime/src/runtime/state.rs").read_text()
    resident_mod = (root / "src/runtime/src/runtime/resident_program/mod.rs").read_text()
    resident_input = (root / "src/runtime/src/runtime/resident_program/input.rs").read_text()
    resident_tests = (root / "src/runtime/src/runtime/resident_program/tests.rs").read_text()
    coordinator = (root / "src/runtime/src/resident_external/coordinator.rs").read_text()
    timer = (root / "hosts/timer/src/provider.rs").read_text()
    scene = (root / "hosts/scene/src/provider.rs").read_text()
    wasm = (root / "src/wasm/src/project.rs").read_text()
    generated = (root / "src/build/src/project/render.rs").read_text()
    javascript = (root / "include/project.js").read_text()
    runtime_config = (root / "src/runtime/src/config/mod.rs").read_text()
    engine_public = (root / "src/engine/src/lib.rs").read_text()
    engine_resident = (root / "src/engine/src/resident/mod.rs").read_text()
    production_resident_files = [
        path
        for path in (root / "src/runtime/src/runtime/resident_program").glob("*.rs")
        if path.name != "tests.rs"
    ]
    production_resident = "\n".join(path.read_text() for path in production_resident_files)

    required = {
        "public resident engine module": '#[cfg(feature = "resident-artifact")]\npub mod resident;' in engine_public,
        "production resident engine imports": "resident::ReactiveInstance" in resident_mod
        and "__resident" not in production_resident,
        "normal source ProgramArtifact loader": all(
            token in loader
            for token in ("pub fn load_source_program", "pub fn load_root_program", "plan_source_product")
        ),
        "normal bytecode selector": "pub fn load_bytecode_program" in loader
        and loader.count("install_resident_artifact") >= 3,
        "resident-production excludes compiler": re.search(
            r'^resident-production\s*=\s*\["resident-external"\]$', runtime_manifest, re.M
        ) is not None,
        "resident-production-source includes compiler": re.search(
            r'^resident-production-source\s*=\s*\["resident-production",\s*"compiler"\]$',
            runtime_manifest,
            re.M,
        ) is not None,
        "standard runtime resident production": '"mech-runtime?/resident-production"' in root_manifest,
        "browser project resident source": "resident-production-source" in wasm_manifest
        and "browser_project =" in wasm_manifest,
        "default PreferResident policy": "pub enum ResidentRoutingPolicy" in runtime_config
        and "#[default]\n    PreferResident" in runtime_config,
        "n-body required volatile route": 'resident-routing: "require-resident"' in config
        and 'resident-durability: "volatile"' in config,
        "explicit semantic fallback": "fallback_eligible" in admission
        and "ResidentRouteFailureClass::SemanticUnsupported" in admission,
        "no wildcard fallback": "_ => true" not in admission,
        "fail-closed fallback classes": all(
            name in routing
            for name in (
                "AuthorizationDenied",
                "ProviderUnavailable",
                "ProviderContractMismatch",
                "InvalidArtifact",
                "InvalidBytecode",
            )
        ),
        "provider registry frozen": "ensure_resident_environment_mutable" in routing,
        "capability-backed authority": "authorize_resource_with_context" in authority,
        "timer observation contract": "resource_observation_contract" in timer
        and "semantic_read_contract" in timer,
        "scene AtMostOnce contract": "EffectDeliveryPolicy::AtMostOnce" in scene
        and "IdempotencyRequirement::NotRequired" in scene,
        "scene points N by 3": "scene_snapshot_from_points" in scene
        and "columns != 3" in scene,
        "legacy replace path": '"replace"' in scene,
        "host input coalescing": all(
            token in resident_input
            for token in ("coalesced_packets", "matched_packets", "coordinator.execute_turn")
        ),
        "WASM normal loader": "runtime.load_root_program(" in wasm,
        "generated normal loader": "runtime.load_bytecode_program(" in generated,
        "WASM diagnostics": all(
            token in javascript for token in ("__MECH_RUNTIME_INFO__", "__MECH_LAST_FRAME__")
        ),
        "bounded volatile production history": all(
            token in resident_tests
            for token in (
                "input_facts().count(), 0",
                "receipts().count(), 0",
                "pending_outbox_count(), 0",
                "max_retained_values, 30",
            )
        ),
        "4096 product trajectory": "product_nbody_source_and_bytecode_match_d2_for_4096_accepted_turns" in resident_tests
        and "4_096" in resident_tests,
        "scene after publication tests": "provider_prepare_failure_and_wrong_protocol_reject_before_publication" in (
            root / "src/runtime/src/resident_external/tests.rs"
        ).read_text(),
        "bytecode v1": "BYTECODE_VERSION: u16 = 1" in (
            root / "src/core/src/program/bytecode/header.rs"
        ).read_text(),
        "production resident surface": "ResidentActivationOptions" in engine_resident,
    }
    for label, present in required.items():
        if not present:
            errors.append(f"D4 source contract is missing {label}")

    for token in ("efficacy", "RuntimeExecutionTransaction", "commit_runtime"):
        if re.search(rf"\b{re.escape(token)}\b", production_resident):
            errors.append(f"production resident path contains forbidden {token}")
    if re.search(r"legacy.*journal|journal.*legacy", production_resident, re.I):
        errors.append("production resident path contains legacy journal capture")
    if "resolve_and_run_root_module" in generated:
        # Planning templates may still compile legacy products, but the hosted
        # generated executable itself must use the normal bytecode load API.
        hosted_start = generated.find("fn runtime_info_json")
        hosted_end = generated.find("pub fn render_runtime_source")
        if hosted_start >= 0 and "resolve_and_run_root_module" in generated[hosted_start:hosted_end]:
            errors.append("generated hosted executable bypasses normal runtime loading")
    errors.extend(exact_nbody_errors(root))
    return errors


def topology_errors(root: Path, *, implementation_head: bool) -> list[str]:
    if subprocess.run(
        ["git", "merge-base", "--is-ancestor", GENERATOR.D3_HEAD, "HEAD"], cwd=root
    ).returncode:
        return [f"D4 must descend from exact D3 semantic head {GENERATOR.D3_HEAD}"]
    log = subprocess.run(
        ["git", "log", "--format=%s", "--reverse", f"{GENERATOR.D3_HEAD}..HEAD"],
        cwd=root,
        text=True,
        capture_output=True,
        check=True,
    ).stdout.splitlines()
    expected = IMPLEMENTATION_SUBJECTS if implementation_head else IMPLEMENTATION_SUBJECTS + [EVIDENCE_SUBJECT]
    return [] if log == expected else [f"D4 canonical stack must be {expected}; found {log}"]


def run(
    root: Path = ROOT,
    *,
    topology: bool = False,
    implementation_head: bool = False,
) -> list[str]:
    errors = projection_errors(root)
    errors.extend(source_errors(root))
    if topology:
        errors.extend(topology_errors(root, implementation_head=implementation_head))
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--implementation-head", action="store_true")
    parser.add_argument("--skip-topology", action="store_true")
    args = parser.parse_args()
    errors = run(
        topology=not args.skip_topology,
        implementation_head=args.implementation_head,
    )
    if errors:
        for error in errors:
            print(f"D4 contract failure: {error}", file=sys.stderr)
        return 1
    print("D4 production resident-routing contract: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
