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


def braced_body(source: str, marker: str) -> str:
    """Return one Rust item body so checks are tied to a specific boundary."""
    start = source.find(marker)
    if start < 0:
        return ""
    opening = source.find("{", start + len(marker))
    if opening < 0:
        return ""
    depth = 0
    for index in range(opening, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[opening + 1 : index]
    return ""


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
        "\n@clock := timer://clock/tick{:read(tick)}\n"
        "@scene := scene://orbit/frame{:write(points)}\n\n"
        "  days-per-year := 365.24",
        1,
    ).replace("  Δt := 0.01", "  Δt := @clock/tick * 0.0 + 0.01", 1)
    expected = expected.replace(
        "\n  positions := x",
        "\n  display-position := x[:,[1,2]]\n"
        "  display-distance² := stats/sum/column(display-position ^ 2)\n"
        "  display-scale := (display-distance² + 0.000000000000000000000000000001) ^ -0.25 * 44.0\n"
        "  screen-x := 300.0 + display-position[:,1] * display-scale\n"
        "  screen-y := 300.0 - display-position[:,2] * display-scale\n"
        "  screen-points := [screen-x screen-y]\n\n"
        "  @scene/points <- screen-points\n"
        "  positions := x",
        1,
    )
    actual = (root / "examples/resident-n-body/n-body.mec").read_text()
    return [] if actual == expected else ["n-body source is not the exact allowed D2 transformation"]


def source_errors(root: Path) -> list[str]:
    errors = []
    engine_manifest = (root / "src/engine/Cargo.toml").read_text()
    d1_generator_manifest = (
        root / "tests/fixtures/d1-contract-generator/Cargo.toml"
    ).read_text()
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
    provider_binding = (root / "src/runtime/src/resident_external/provider.rs").read_text()
    resources = (root / "src/runtime/src/resource.rs").read_text()
    ingress = (root / "src/runtime/src/input.rs").read_text()
    resident_authority = (root / "src/runtime/src/runtime/resident_program/authority.rs").read_text()
    timer = (root / "hosts/timer/src/provider.rs").read_text()
    timer_config = (root / "hosts/timer/src/config.rs").read_text()
    timer_delivery = (root / "hosts/timer/src/delivery.rs").read_text()
    scene = (root / "hosts/scene/src/provider.rs").read_text()
    wasm = (root / "src/wasm/src/project.rs").read_text()
    generated = (root / "src/build/src/project/render.rs").read_text()
    javascript = (root / "include/project.js").read_text()
    runtime_config = (root / "src/runtime/src/config/mod.rs").read_text()
    engine_public = (root / "src/engine/src/lib.rs").read_text()
    engine_resident = (root / "src/engine/src/resident/mod.rs").read_text()
    browser_smoke = (root / "scripts/smoke-served-resident-nbody-browser.sh").read_text()
    timer_native = (root / "hosts/timer/src/native.rs").read_text()
    timer_browser = (root / "hosts/timer/src/browser.rs").read_text()
    timer_manual = (root / "hosts/timer/src/manual.rs").read_text()
    browser_delegation = (root / "hosts/browser/src/delegation.rs").read_text()
    runtime_limits = (root / "src/runtime/src/runtime/limits.rs").read_text()
    production_resident_files = [
        path
        for path in (root / "src/runtime/src/runtime/resident_program").glob("*.rs")
        if path.name != "tests.rs"
    ]
    production_resident = "\n".join(path.read_text() for path in production_resident_files)
    load_selection = braced_body(loader, "fn load_with_selection")
    bytecode_loader = braced_body(loader, "pub fn load_bytecode_program")
    typed_fallback = braced_body(routing, "pub fn resident_fallback_eligible")
    host_drain = braced_body(resident_input, "pub(crate) fn drain_resident_host_inputs")
    grant_build = braced_body(resident_authority, "pub(crate) fn build_resident_authority")
    grant_revalidation = braced_body(resident_authority, "pub(crate) fn revalidate")
    outbox_retry = braced_body(routing, "pub fn retry_resident_outbox")
    driver_admission = braced_body(loader, "fn ensure_exact_resident_input_drivers")
    host_admission = braced_body(coordinator, "pub(crate) fn admit_host_turn")
    capture = braced_body(coordinator, "fn capture_with_providers")
    reserve_turn = braced_body(coordinator, "fn reserve_live_turn")
    publish_turn = braced_body(coordinator, "fn prepare_and_publish")
    unload = braced_body(routing, "pub fn unload_active_program")
    public_drain = braced_body(
        (root / "src/runtime/src/runtime/execution/input_drivers.rs").read_text(),
        "pub fn drain_host_inputs",
    )
    standard_runtime_feature = re.search(
        r"^standard_runtime\s*=\s*\[(.*?)^\s*\]$",
        root_manifest,
        re.M | re.S,
    )

    required = {
        "public resident engine module": '#[cfg(feature = "resident-artifact")]\npub mod resident;' in engine_public,
        "production resident engine imports": "resident::ReactiveInstance" in resident_mod
        and "__resident" not in production_resident,
        "normal source ProgramArtifact loader": all(
            token in loader
            for token in ("pub fn load_source_program", "pub fn load_root_program", "plan_source_product")
        ),
        "normal bytecode selector": "self.load_with_selection" in bytecode_loader
        and "runtime.decode_artifact(bytecode)" in bytecode_loader
        and "install_resident_artifact" in load_selection,
        "resident-routing excludes compiler": re.search(
            r'^resident-routing\s*=\s*\["resident-external"\]$', runtime_manifest, re.M
        ) is not None,
        "resident-routing-source includes compiler": re.search(
            r'^resident-routing-source\s*=\s*\["resident-routing",\s*"compiler"\]$',
            runtime_manifest,
            re.M,
        ) is not None,
        "resident artifact preserves product value closure": all(
            token in engine_manifest
            for token in (
                'resident-artifact = [',
                '"resident-ekf", "runtime", "artifact-codec",',
                '"f64", "matrixd", "row_vectord",',
            )
        )
        and "runtime_default" not in engine_manifest[
            engine_manifest.index("resident-artifact = [") : engine_manifest.index(
                "]", engine_manifest.index("resident-artifact = [")
            )
        ],
        "D1 compiler fixture requests compiler explicitly": 'features = ["resident-artifact", "compiler_default"]'
        in d1_generator_manifest,
        "standard runtime resident production": '"mech-runtime?/resident-routing"' in root_manifest,
        "standard resident profile excludes combinatorics": standard_runtime_feature is not None
        and "combinatorics_default" not in standard_runtime_feature.group(1),
        "browser project resident source": "resident-routing-source" in wasm_manifest
        and "browser_project =" in wasm_manifest,
        "default PreferResident policy": "pub enum ResidentRoutingPolicy" in runtime_config
        and "#[default]\n    PreferResident" in runtime_config,
        "n-body required volatile route": 'resident-routing: "require-resident"' in config
        and 'resident-durability: "volatile"' in config,
        "typed semantic fallback": "resident_fallback_eligible(&error)" in load_selection
        and "kind_as::<ResidentRouteFailure>()" in typed_fallback
        and "failure.class == ResidentRouteFailureClass::SemanticUnsupported" in typed_fallback
        and "kind_message" not in typed_fallback,
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
        "capability-backed authority": "preview_capability_for_execution" in grant_build
        and "max_uses.is_some()" in grant_build
        and "ResidentGrantSet" in grant_build
        and "preview_capability_for_execution" in grant_revalidation
        and "get_capability" in grant_revalidation,
        "per-turn and retry grant validation": "revalidate_active_resident_grants" in host_drain
        and "revalidate_active_resident_grants" in outbox_retry,
        "owned provider bindings": "Rc<dyn RuntimeResourceProvider>" in resources
        and "Rc::clone(&target.provider)" in resources
        and "provider_binding: Option<RuntimeResidentProviderBinding>" in provider_binding,
        "exact input driver ownership": "match owners" in driver_admission
        and "0 =>" in driver_admission
        and "1 =>" in driver_admission
        and "_ =>" in driver_admission,
        "timer observation contract": "resource_observation_contract" in timer
        and "semantic_read_contract" in timer,
        "scene AtMostOnce contract": "EffectDeliveryPolicy::AtMostOnce" in scene
        and "IdempotencyRequirement::NotRequired" in scene,
        "scene points N by 2": "scene_snapshot_from_points" in scene
        and "columns != 2" in scene
        and "values[rows + row]" in scene,
        "legacy replace path": '"replace"' in scene,
        "packet-authoritative host input": "latest_updates.insert(update.source.clone(), update.value.clone())" in host_drain
        and "execute_admitted_host_turn" in host_drain
        and "validate_host_updates(updates)" in host_admission
        and "if let Some(update) = packet_value" in capture,
        "duplicate observation fanout and provider-complete snapshots": "BTreeSet<_>" in coordinator
        and "for observation in matching" in coordinator
        and "provider_binding.read" in capture,
        "retained admission precedes irreversible packet consumption": "admit_host_turn" in host_drain
        and "restore_resident_host_packets(packets)" in host_drain
        and "input_ledger.reserve" in reserve_turn
        and "receipt_ledger.reserve" in reserve_turn,
        "driver backlog cleared at ownership release": all(
            "pending" in source and ".clear();" in braced_body(source, "fn stop")
            for source in (timer_native, timer_browser, timer_manual)
        ),
        "legacy executable cleared on unload": "self.program = replacement" in unload
        and "self.live_input_bindings.clear()" in unload
        and "self.live_context_template = None" in unload,
        "resident outcomes are public and failures surface": "resident_turn" in ingress
        and "resident_host_turn_error" in public_drain,
        "resident duration checked before publication": "prepublication()" in publish_turn
        and "enforce_turn_duration_limit" in runtime_limits
        and "prepared.abort()" in routing,
        "effect-only activation executes once": "if trigger_sources.is_empty()" in loader
        and "execute_admitted_turn" in loader,
        "signed browser routing policy": browser_delegation.count("encode_program_routing") >= 3
        and "modified_resident_routing_fails_signature_verification" in browser_delegation,
        "latest timer queue": "pub fn submit_latest" in ingress
        and "TimerQueuePolicy::Latest => ingress.submit_latest(packet)" in timer_delivery
        and '"latest" => TimerQueuePolicy::Latest' in timer_config,
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
                "max_retained_values, 20",
            )
        ),
        "root import closure regression": "resident_root_plans_the_resolved_source_import_closure_before_route_selection" in resident_tests,
        "public fixed orbit regression": "public_nbody_viewer_preserves_the_working_fixed_sun_orbits_residently" in resident_tests
        and "expected_radii" in resident_tests
        and "4_096" in resident_tests,
        "browser fixed orbit proof": all(
            token in browser_smoke
            for token in ("mechSunFixed", "mechOrbitStable", 'data-mech-legacy="0"')
        ),
        "4096 product trajectory": "product_nbody_source_and_bytecode_match_d2_for_4096_accepted_turns" in resident_tests
        and "4_096" in resident_tests,
        "review correction regressions": all(
            name in resident_tests
            for name in (
                "duplicate_observations_share_one_authoritative_host_update",
                "independent_observations_capture_absent_values_from_the_bound_provider",
                "retained_admission_failure_leaves_the_ordered_packet_available_for_retry",
                "invalidated_admitted_grant_blocks_next_drain_before_dequeue_or_publication",
                "invalidated_admitted_grant_blocks_outbox_retry_before_preparation_or_delivery",
                "unloading_a_legacy_owner_removes_its_program_and_live_state",
                "public_host_drain_exposes_the_clean_resident_turn",
                "resident_turn_duration_rejects_before_scene_publication_and_surfaces_publicly",
                "effect_only_resident_program_executes_once_during_activation",
            )
        ),
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
