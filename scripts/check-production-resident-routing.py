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
)
PROHIBITED_ROUTE_REFERENCES = (
    "ResidentRoutingPolicy::PreferResident",
    "ResidentRoutingPolicy::LegacyOnly",
    "RuntimeProgramRoute::Legacy",
)
OLD_EXECUTOR_CALL = re.compile(
    r"\.(?:run_string(?:_with_context)?|run_source(?:_with_context)?|"
    r"run_tree(?:_with_context)?|install_bytecode_with_context|"
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
                # The temporary developer facade is explicit authority, not a
                # direct old-executor call. Multi-line method chains need a
                # small look-behind window.
                context = "\n".join(lines[max(0, index - 4) : index + 1])
                if ".legacy_interpreter()" in context:
                    continue
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
    contracts = (
        ("Cargo.toml", "distribution-standard", False),
        ("Cargo.toml", "distribution-full", True),
        ("src/runtime/Cargo.toml", "source_default", False),
        ("src/runtime/Cargo.toml", "full_compiler", True),
        ("src/wasm/Cargo.toml", "browser_project", False),
        ("src/wasm/Cargo.toml", "full", True),
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
        "src/cli/commands/run.rs": "load_production_source_program",
        "src/build/src/project/render.rs": "load_production_bytecode_program",
        "src/wasm/src/project.rs": "load_production_root_program",
        "hosts/browser/src/config.rs": "validate_production_program_routing",
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
    if evaluate_boundary.search(wasm_source) is None:
        failures.append(
            "src/wasm/src/project.rs: WasmDocument.evaluate must exist only in legacy-interpreter builds"
        )
    if "pub(super) developer_runtime: MechRuntime" not in wasm_source or not re.search(
        r"self\s*\.developer_runtime\s*\.legacy_interpreter\(\)\s*\.run_string\(",
        wasm_source,
    ):
        failures.append(
            "src/wasm/src/project.rs: developer evaluation must use a separate interpreter runtime"
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
    required_surface_contracts = {
        "src/cli/commands/run.rs": (
            "production inputs cannot be combined with a REPL",
            "live_drain_limit(max_live_turns, completed_live_turns)",
        ),
        "src/cli/commands/build.rs": (
            "Exactly one resident source root or one .mecb bytecode file",
        ),
        "src/build/src/project/render.rs": (
            '\\"legacy_turns\\":{}',
            "limit.saturating_sub(completed_live_turns)",
        ),
        "src/build/src/lib.rs": (
            "validate_production_native_runtime_config(config)",
            "NativeActorBootstrapUnsupported",
        ),
        "scripts/smoke-formatted-document-browser.sh": (
            'submit(":whos answer")',
        ),
        "docs/getting-started/repl.mec": (
            "Production targets cannot be combined with `--repl`",
        ),
        "docs/reference/commands/test.mec": (
            "available only in the full",
            "--features distribution-full",
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
