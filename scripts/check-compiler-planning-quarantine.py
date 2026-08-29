#!/usr/bin/env python3
"""Keep compiler-planning machinery private and absent from shipping execution."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ENGINE_LIB = Path("src/engine/src/lib.rs")
PROGRAM_MOD = Path("src/engine/src/program/mod.rs")
PLANNING_MODULE = Path("src/engine/src/program/compiler_planning.rs")
REMOVED_INSTANCE = Path("src/engine/src/program/instance.rs")

GLOBAL_REMOVED = (
    "MechProgram",
    "MechProgramConfig",
    "MechProgramEnvironment",
    "ProgramSolveOutcome",
    "run_profiled_string",
)
SHIPPING_ROOTS = (
    Path("src/runtime/src/runtime/program"),
    Path("src/engine/src/resident"),
    Path("src/cli"),
    Path("src/build/src"),
    Path("src/wasm/src"),
    Path("hosts"),
)
SHIPPING_EXECUTOR_PATTERNS = {
    "Interpreter": re.compile(r"\bInterpreter(?:Ref)?\b"),
    "MechProgram": re.compile(r"\bMechProgram\b"),
    "run_bytecode": re.compile(r"\brun_bytecode(?:_with_services|_program(?:_with_services)?)?\s*\("),
    "run_string": re.compile(r"\brun_string(?:_with_services)?\s*\("),
    "run_source": re.compile(r"\brun_source(?:_with_services)?\s*\("),
    "legacy_interpreter": re.compile(r"\blegacy_interpreter\s*\("),
}
TEST_MODULE = re.compile(
    r"#\[cfg\([^\]]*\btest\b[^\]]*\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+[A-Za-z0-9_]+\s*\{",
    re.MULTILINE,
)

# These are exact provider/value conversion adapters already governed by the
# value-system boundary. The quarantine checker deliberately grants no parent
# directory or filename-pattern exception.
APPROVED_LEGACY_VALUE_ADAPTERS = {
    Path("src/runtime/src/runtime/program/compiler.rs"),
    # The resident compatibility adapter is compiled only by runtime tests.
    Path("src/runtime/src/runtime/program/external/value_adapter_tests.rs"),
    Path("src/runtime/src/runtime/program/value.rs"),
    Path("hosts/browser/src/config.rs"),
    Path("hosts/browser/src/provider.rs"),
    Path("hosts/console/src/provider.rs"),
    Path("hosts/gpu/src/compute_provider.rs"),
    Path("hosts/robot-arm/src/provider.rs"),
    Path("hosts/scene/src/provider.rs"),
    Path("hosts/scene/src/schema.rs"),
    Path("hosts/terminal/src/provider.rs"),
    Path("hosts/time/src/lib.rs"),
    Path("hosts/time/src/provider.rs"),
    Path("hosts/timer/src/provider.rs"),
}


def rust_sources(root: Path) -> list[Path]:
    paths: set[Path] = set()
    for relative in (Path("src"), Path("machines"), Path("hosts"), Path("tests")):
        directory = root / relative
        if directory.exists():
            paths.update(path.relative_to(root) for path in directory.rglob("*.rs"))
    return sorted(paths)


def line_number(source: str, offset: int) -> int:
    return source.count("\n", 0, offset) + 1


def rust_without_test_modules(source: str) -> str:
    chars = list(source)
    search_from = 0
    while match := TEST_MODULE.search(source, search_from):
        opening = source.find("{", match.start(), match.end())
        depth = 0
        in_string = False
        escaped = False
        end = opening
        for end in range(opening, len(source)):
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


def check_module_boundary(root: Path) -> list[str]:
    failures: list[str] = []
    engine_lib = (root / ENGINE_LIB).read_text(encoding="utf-8")
    if re.search(r"(?m)^\s*pub\s+mod\s+interpreter\s*;", engine_lib):
        failures.append(f"{ENGINE_LIB}: interpreter module is public")
    if not re.search(
        r'#\[cfg\(feature = "semantic-compiler"\)\]\s*mod\s+interpreter\s*;',
        engine_lib,
    ):
        failures.append(f"{ENGINE_LIB}: interpreter is not semantic-compiler-only")
    if not re.search(
        r'#\[cfg\(feature = "semantic-compiler"\)\]\s*pub\(crate\)\s+use\s+interpreter::',
        engine_lib,
    ):
        failures.append(f"{ENGINE_LIB}: interpreter symbols are not crate-private")
    planning = root / PLANNING_MODULE
    if not planning.exists():
        failures.append(f"{PLANNING_MODULE}: compiler-planning module is missing")
    program_mod = (root / PROGRAM_MOD).read_text(encoding="utf-8")
    if not re.search(
        r'#\[cfg\(feature = "semantic-compiler"\)\]\s*mod\s+compiler_planning\s*;',
        program_mod,
    ):
        failures.append(f"{PROGRAM_MOD}: compiler_planning is not semantic-compiler-only")
    if (root / REMOVED_INSTANCE).exists():
        failures.append(f"{REMOVED_INSTANCE}: obsolete mutable program instance remains")
    interpreter_source = (root / "src/engine/src/interpreter/mod.rs").read_text(
        encoding="utf-8"
    )
    if re.search(r"(?m)^pub\s+type\s+InterpreterRef\b", interpreter_source):
        failures.append("src/engine/src/interpreter/mod.rs: InterpreterRef remains public")
    return failures


def check_removed_surface(root: Path) -> list[str]:
    failures: list[str] = []
    for relative in rust_sources(root):
        source = (root / relative).read_text(encoding="utf-8")
        searchable = source
        if relative == Path("src/engine/src/artifact/encoding.rs"):
            searchable = searchable.replace('b"mech-program-v1\\0"', "")
        for token in GLOBAL_REMOVED:
            for match in re.finditer(rf"\b{re.escape(token)}\b", searchable):
                failures.append(
                    f"{relative}:{line_number(searchable, match.start())}: removed {token} surface"
                )
        for match in re.finditer(r'"Cycle "\s*"Time:"|"Cycle Time:"', searchable):
            failures.append(
                f"{relative}:{line_number(searchable, match.start())}: removed profiling output"
            )
    return failures


def shipping_sources(root: Path) -> list[Path]:
    paths: set[Path] = set()
    for relative_root in SHIPPING_ROOTS:
        directory = root / relative_root
        if not directory.exists():
            continue
        paths.update(
            path.relative_to(root)
            for path in directory.rglob("*.rs")
            if "tests" not in path.relative_to(root).parts
            and "query_tests" not in path.relative_to(root).parts
            and path.name not in {"test_provider.rs", "tests.rs"}
        )
    return sorted(paths)


def check_shipping_reachability(root: Path) -> list[str]:
    failures: list[str] = []
    for relative in shipping_sources(root):
        source = rust_without_test_modules((root / relative).read_text(encoding="utf-8"))
        for name, pattern in SHIPPING_EXECUTOR_PATTERNS.items():
            for match in pattern.finditer(source):
                failures.append(
                    f"{relative}:{line_number(source, match.start())}: shipping {name} reachability"
                )
        legacy_restricted = (
            relative.is_relative_to(Path("src/runtime/src/runtime/program"))
            or relative.is_relative_to(Path("src/engine/src/resident"))
            or relative.is_relative_to(Path("hosts"))
        )
        if (
            legacy_restricted
            and "LegacyValue" in source
            and relative not in APPROVED_LEGACY_VALUE_ADAPTERS
        ):
            failures.append(f"{relative}: LegacyValue is outside an exact approved adapter")
    return failures


def run(root: Path = ROOT) -> list[str]:
    return (
        check_module_boundary(root)
        + check_removed_surface(root)
        + check_shipping_reachability(root)
    )


def main() -> int:
    failures = run()
    if failures:
        print("Compiler-planning quarantine failed:", file=sys.stderr)
        print(*failures, sep="\n", file=sys.stderr)
        return 1
    print("Compiler-planning quarantine passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
