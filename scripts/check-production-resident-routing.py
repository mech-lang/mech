#!/usr/bin/env python3
"""Keep shipping Mech products on the resident-only execution boundary."""

from __future__ import annotations

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parent.parent
PRODUCT_ROOTS = (
    Path("src/cli"),
    Path("src/build/src"),
    Path("src/wasm/src"),
    Path("hosts/browser/src"),
    Path("hosts/terminal/src"),
)
PROHIBITED_ROUTE_REFERENCES = (
    "RuntimeExecutionMode",
    "ResidentRoutingPolicy",
    "ProgramRoutingConfig",
    "program_routing",
    "RuntimeProgramRoute::Legacy",
    "legacy_turns",
    "legacyTurns",
    "load_production_",
    "rootInterpreterId",
    "output_value_for_interpreter",
    "symbol_name_for_interpreter_output",
    "symbol_values_for_interpreter",
)
OLD_EXECUTOR_CALL = re.compile(
    r"\.(?:run_string(?:_with_context)?|run_source(?:_with_context)?|"
    r"run_tree(?:_with_context)?|run_bytecode(?:_with_services|_program(?:_with_services)?)?|"
    r"install_bytecode_with_context|"
    r"evaluate_bytecode_once_with_context|resolve_and_run_root_module(?:_with_context|_report)?)\s*\("
)
TEST_MODULE = re.compile(
    r"#\[cfg\([^\]]*\btest\b[^\]]*\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+[A-Za-z0-9_]+\s*\{",
    re.MULTILINE,
)


def rust_without_test_modules(source: str) -> str:
    """Mask cfg(test) modules while retaining line numbers for diagnostics."""
    chars = list(source)
    search_from = 0
    while match := TEST_MODULE.search(source, search_from):
        brace = source.find("{", match.start(), match.end())
        depth = 0
        end = brace
        in_string = False
        escaped = False
        for end in range(brace, len(source)):
            char = source[end]
            if in_string:
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    in_string = False
                continue
            if char == '"':
                in_string = True
            elif char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    end += 1
                    break
        for index in range(match.start(), end):
            if chars[index] != "\n":
                chars[index] = " "
        search_from = end
    return "".join(chars)


def product_sources() -> list[tuple[Path, str]]:
    sources: list[tuple[Path, str]] = []
    for relative_root in PRODUCT_ROOTS:
        for path in sorted((ROOT / relative_root).rglob("*.rs")):
            relative = path.relative_to(ROOT)
            if "tests" in relative.parts:
                continue
            sources.append((relative, rust_without_test_modules(path.read_text(encoding="utf-8"))))
    return sources


def line_number(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def check_product_references() -> list[str]:
    failures: list[str] = []
    for path, source in product_sources():
        for token in PROHIBITED_ROUTE_REFERENCES:
            for match in re.finditer(re.escape(token), source):
                failures.append(f"{path}:{line_number(source, match.start())}: prohibited {token}")

        lines = source.splitlines()
        for index, line in enumerate(lines):
            for match in OLD_EXECUTOR_CALL.finditer(line):
                failures.append(
                    f"{path}:{index + 1}: direct old executor call {match.group(0).strip()}"
                )
    return failures


def manifest_features(relative: str) -> dict[str, list[str]]:
    manifest = (ROOT / relative).read_text(encoding="utf-8")
    match = re.search(r"(?ms)^\[features\]\s*$\n(.*?)(?=^\[[^\n]+\]\s*$|\Z)", manifest)
    if match is None:
        return {}
    features: dict[str, list[str]] = {}
    name: str | None = None
    value: list[str] = []
    for line in match.group(1).splitlines():
        if name is None:
            assignment = re.match(r"^([A-Za-z0-9_-]+)\s*=\s*(.*)$", line)
            if assignment is None:
                continue
            name = assignment.group(1)
            remainder = assignment.group(2)
        else:
            remainder = line
        value.extend(re.findall(r'"([^"]+)"', remainder))
        if "]" in remainder:
            features[name] = value
            name = None
            value = []
    return features


def feature_closure(features: dict[str, list[str]], root: str) -> set[str]:
    if root not in features:
        raise KeyError(root)
    closure: set[str] = set()
    pending = [root]
    while pending:
        feature = pending.pop()
        if feature in closure:
            continue
        closure.add(feature)
        for route in features.get(feature, []):
            if route in features:
                pending.append(route)
    return closure


def check_feature_boundaries() -> list[str]:
    failures: list[str] = []
    for manifest in sorted(ROOT.rglob("Cargo.toml")):
        source = manifest.read_text(encoding="utf-8")
        if re.search(r"(?m)^legacy-interpreter\s*=", source):
            failures.append(
                f"{manifest.relative_to(ROOT)}: obsolete legacy-interpreter feature remains"
            )
    contracts = (
        ("Cargo.toml", "distribution-standard", False),
        ("Cargo.toml", "distribution-full", False),
        ("src/runtime/Cargo.toml", "source_default", False),
        ("src/runtime/Cargo.toml", "full_compiler", False),
        ("src/wasm/Cargo.toml", "browser_project", False),
        ("src/wasm/Cargo.toml", "full", False),
    )
    for manifest, root, expected in contracts:
        features = manifest_features(manifest)
        try:
            closure = feature_closure(features, root)
        except KeyError:
            failures.append(f"{manifest}: missing feature {root}")
            continue
        actual = "legacy-interpreter" in closure
        if actual != expected:
            disposition = "include" if expected else "exclude"
            failures.append(f"{manifest}: feature {root} must {disposition} legacy-interpreter")
    return failures


def check_required_product_seams() -> list[str]:
    required = {
        "src/cli/commands/run.rs": "load_source_program",
        "src/build/src/project/render.rs": "load_bytecode_program",
        "src/wasm/src/project.rs": "load_root_program",
        "hosts/browser/src/config.rs": "resident_durability",
        "hosts/terminal/src/provider.rs": "CLI_OUTPUT_EFFECT_CONTRACT",
    }
    failures: list[str] = []
    for relative, needle in required.items():
        source = (ROOT / relative).read_text(encoding="utf-8")
        if needle not in source:
            failures.append(f"{relative}: missing resident production seam {needle}")
    wasm_source = (ROOT / "src/wasm/src/project.rs").read_text(encoding="utf-8")
    evaluate_boundary = re.compile(
        r'#\[cfg\(feature = "legacy-interpreter"\)\]\s*pub fn evaluate\s*\('
    )
    if evaluate_boundary.search(wasm_source) is not None:
        failures.append(
            "src/wasm/src/project.rs: WasmDocument.evaluate must not remain in the shipping wrapper"
        )
    if "pub(super) developer_runtime: MechRuntime" in wasm_source or re.search(
        r"self\s*\.developer_runtime\s*\.legacy_interpreter\(\)\s*\.run_string\(",
        wasm_source,
    ):
        failures.append(
            "src/wasm/src/project.rs: developer interpreter runtime must not remain in the shipping wrapper"
        )
    controller_source = (ROOT / "include/document.js").read_text(encoding="utf-8")
    if (
        "interpreterIdByName" in wasm_source
        or "interpreterIdByName" in controller_source
        or "resolveNamedInterpreter" in controller_source
    ):
        failures.append(
            "standard documents must not export, call, or carry named legacy document lookup"
        )
    for relative in (
        "docs/mechdown/template-placeholders.mec",
        "scripts/smoke-served-rich-document-browser.sh",
    ):
        source = (ROOT / relative).read_text(encoding="utf-8")
        if re.search(r"\{\{VAR:[^}\n]*@[^}\n]*\}\}", source):
            failures.append(
                f"{relative}: named legacy placeholders must not be advertised or exercised"
            )
    build_source = (ROOT / "src/build/src/lib.rs").read_text(encoding="utf-8")
    if 'runtime_features.insert("legacy-interpreter"' in build_source:
        failures.append(
            "src/build/src/lib.rs: production native plans must not select legacy-interpreter"
        )
    runtime_config = (ROOT / "src/runtime/src/config/mod.rs").read_text(encoding="utf-8")
    if "pub resident_durability: ResidentDurabilityPolicy" not in runtime_config:
        failures.append(
            "src/runtime/src/config/mod.rs: RuntimeConfig must directly own resident durability"
        )
    for token in ("ResidentRoutingPolicy", "ProgramRoutingConfig", "program_routing"):
        if token in runtime_config:
            failures.append(
                f"src/runtime/src/config/mod.rs: obsolete routing policy remains: {token}"
            )
    compiler_source = (ROOT / "src/runtime/src/runtime/program/compiler.rs").read_text(
        encoding="utf-8"
    )
    if "pub struct ProgramCompiler" not in compiler_source:
        failures.append(
            "src/runtime/src/runtime/program/compiler.rs: ProgramCompiler must own source compilation"
        )
    if re.search(r"\bValRef\b", compiler_source):
        failures.append(
            "src/runtime/src/runtime/program/compiler.rs: compiler modules must not share ValRef identity"
        )
    engine_lib = (ROOT / "src/engine/src/lib.rs").read_text(encoding="utf-8")
    if re.search(r"(?m)^\s*pub\s+mod\s+interpreter\s*;", engine_lib):
        failures.append("src/engine/src/lib.rs: interpreter module must remain private")
    if not re.search(
        r'#\[cfg\(feature = "semantic-compiler"\)\]\s*mod\s+interpreter\s*;',
        engine_lib,
    ):
        failures.append(
            "src/engine/src/lib.rs: private interpreter module must be semantic-compiler-only"
        )
    if (ROOT / "src/engine/src/program/instance.rs").exists():
        failures.append("src/engine/src/program/instance.rs: obsolete program instance remains")
    terminal_provider = (ROOT / "hosts/terminal/src/provider.rs").read_text(
        encoding="utf-8"
    )
    for terminal_contract in (
        "semantic_read_contract",
        "resource_observation_contract",
        "observation_requires_input_driver",
        "semantic_write_contract",
        "RuntimeResourceWriteIntent::Send",
        "EffectDeliveryPolicy::AtMostOnce",
        "IdempotencyRequirement::NotRequired",
        "PreparedRuntimeEffect::AfterCommit",
        "require a scalar string payload",
    ):
        if terminal_contract not in terminal_provider:
            failures.append(
                "hosts/terminal/src/provider.rs: missing retained resident terminal contract "
                f"{terminal_contract}"
            )
    build_compiler = (ROOT / "src/cli/module_execution.rs").read_text(encoding="utf-8")
    for build_contract in (
        'providers.insert("cli".to_string())',
        'name: "cli".to_string()',
        "cli_grants.to_run_resource_grants()",
    ):
        if build_contract not in build_compiler:
            failures.append(
                "src/cli/module_execution.rs: terminal source planning must retain "
                f"{build_contract}"
            )
    terminal_run_factory = (ROOT / "src/cli/host_factories.rs").read_text(
        encoding="utf-8"
    )
    if "CliHostFactory::new" not in terminal_run_factory:
        failures.append(
            "src/cli/host_factories.rs: normal mech run must install CliHostFactory"
        )
    root_manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    for terminal_feature in (
        'cli_host = ["mech-runtime", "dep:mech-terminal", "mech-terminal/provider"]',
        'mech-terminal = { version = "0.3.5", default-features = false, optional = true }',
    ):
        if terminal_feature not in root_manifest:
            failures.append(
                f"Cargo.toml: retained terminal product closure is missing {terminal_feature}"
            )
    retired_time_surfaces = {
        "src/cli/app/mod.rs": ('Arg::new("time")', 'get_flag("time")'),
        "src/cli/commands/run.rs": ('Arg::new("time")', 'get_flag("time")'),
        "src/cli/commands/build.rs": ('get_flag("time")',),
        "src/cli/run_options.rs": ("PreparedRunOptions.time",),
        "src/cli/runtime_plan.rs": ("root.time",),
        "docs/getting-started/build-and-run.mec": ("--time",),
        "docs/reference/commands/run.mec": ("--time",),
    }
    for relative, obsolete_tokens in retired_time_surfaces.items():
        source = (ROOT / relative).read_text(encoding="utf-8")
        for obsolete_token in obsolete_tokens:
            if obsolete_token in source:
                failures.append(
                    f"{relative}: retired interpreter profiling surface remains: "
                    f"{obsolete_token}"
                )
    artifact_model = (ROOT / "src/engine/src/artifact/model.rs").read_text(
        encoding="utf-8"
    )
    if "Output" not in artifact_model or "pub enum SlotRole" not in artifact_model:
        failures.append(
            "src/engine/src/artifact/model.rs: SlotRole::Output must own resident publication storage"
        )
    resident_authority_path = Path(
        "src/runtime/src/runtime/program/external/admission.rs"
    )
    resident_authority = (ROOT / resident_authority_path).read_text(encoding="utf-8")
    if "ResidentAdmissionProof" not in resident_authority:
        failures.append(f"{resident_authority_path}: missing ResidentAdmissionProof")
    for obsolete_owner in (
        "src/interpreter",
        "src/bin/interpreter2.rs",
        "src/runtime/src/runtime/resident_program",
        "src/runtime/src/resident_external",
        "src/runtime/src/runtime/execution/query.rs",
    ):
        if (ROOT / obsolete_owner).exists():
            failures.append(f"{obsolete_owner}: obsolete competing program owner remains")
    runtime_sources = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted((ROOT / "src/runtime/src").rglob("*.rs"))
    )
    for obsolete_resource_surface in (
        "RuntimeResourceBinding",
        "resource_binding(",
        "read_bound_resource(",
        "write_bound_resource(",
        "context_export_binding(",
    ):
        if obsolete_resource_surface in runtime_sources:
            failures.append(
                "src/runtime/src: obsolete direct resource-binding surface remains: "
                f"{obsolete_resource_surface}"
            )
    coordinator_path = Path(
        "src/runtime/src/runtime/program/external/coordinator.rs"
    )
    coordinator = (ROOT / coordinator_path).read_text(encoding="utf-8")
    authority_impls = coordinator.count(
        "impl ResidentExternalPublicationAuthority for RuntimeResidentPublicationAuthority"
    )
    publication_callers = len(re.findall(r"\.publish_external\s*\(", coordinator))
    if authority_impls != 1:
        failures.append(
            f"{coordinator_path}: expected one publication authority impl, found {authority_impls}"
        )
    if publication_callers != 1:
        failures.append(
            f"{coordinator_path}: expected one publication authority caller, found {publication_callers}"
        )
    required_surface_contracts = {
        "src/cli/commands/run.rs": (
            "live_drain_limit(max_live_turns, completed_live_turns)",
        ),
        "src/cli/commands/build.rs": (
            "Exactly one resident source root or one .mecb bytecode file",
        ),
        "src/build/src/project/render.rs": (
            '\\"resident_accepted_turns\\":{}',
            "limit.saturating_sub(completed_live_turns)",
        ),
        "src/build/src/lib.rs": (
            "validate_production_native_runtime_config(config)",
            "NativeActorBootstrapUnsupported",
        ),
        "scripts/smoke-formatted-document-browser.sh": (
            'submit(":whos answer")',
        ),
        "docs/guides/native-applications.mec": (
            "accepts exactly one",
        ),
    }
    for relative, needles in required_surface_contracts.items():
        source = (ROOT / relative).read_text(encoding="utf-8")
        for needle in needles:
            if needle not in source:
                failures.append(f"{relative}: missing production surface contract {needle}")
    for relative in (
        "docs/getting-started/repl.mec",
        "docs/reference/commands/test.mec",
    ):
        if (ROOT / relative).exists():
            failures.append(f"{relative}: retired interpreter command documentation remains")
    for relative in (
        "scripts/check-d3-contract.py",
        "scripts/generate-d3-contract.py",
        "scripts/check-d4-contract.py",
        "scripts/generate-d4-contract.py",
        "scripts/tests/test_check_d3_contract.py",
        "scripts/tests/test_generate_d3_contract.py",
        "scripts/tests/test_check_d4_contract.py",
        "scripts/tests/test_generate_d4_contract.py",
        "tests/architecture/resident-external",
        "tests/architecture/production-resident",
    ):
        if (ROOT / relative).exists():
            failures.append(f"{relative}: retired D3/D4 migration scaffold remains")
    return failures


def main() -> int:
    failures = (
        check_product_references()
        + check_feature_boundaries()
        + check_required_product_seams()
    )
    if failures:
        print("Production resident-routing contract failed:", file=sys.stderr)
        print(*failures, sep="\n", file=sys.stderr)
        return 1
    print("Production resident-routing contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
