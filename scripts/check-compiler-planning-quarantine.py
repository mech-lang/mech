#!/usr/bin/env python3
"""Keep compiler-planning machinery private and absent from shipping execution."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ENGINE_LIB = Path("src/engine/src/lib.rs")
CORE_LIB = Path("src/core/src/lib.rs")
PROGRAM_MOD = Path("src/engine/src/program/mod.rs")
PLANNING_MODULE = Path("src/engine/src/program/compiler_planning.rs")
REMOVED_INSTANCE = Path("src/engine/src/program/instance.rs")
REMOVED_WORKSPACE_PATHS = (
    Path("src/engine/src/interpreter"),
    Path("src/engine/src/expressions"),
    Path("src/engine/src/literals.rs"),
    Path("src/engine/src/structures.rs"),
    Path("src/core/src/nodes.rs"),
)

GLOBAL_REMOVED = (
    "MechProgram",
    "MechProgramConfig",
    "MechProgramEnvironment",
    "ProgramSolveOutcome",
    "run_profiled_string",
    "Interpreter",
    "InterpreterRef",
    "CompilerPlanningProgram",
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
RETIRED_NAMESPACES = {
    "mech_engine": {"interpreter", "expressions", "literals", "structures"},
    "mech_core": {"nodes"},
}
QUALIFIED_NAMESPACE = re.compile(r"\b(?P<owner>mech_engine|mech_core)\s*::\s*")
RUST_IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
PRECEDING_PATH_MEMBER = re.compile(r"(?P<member>[A-Za-z_][A-Za-z0-9_]*)\s*::\s*$")
USE_GROUP_BRACE = re.compile(r"[{}]")
RAW_LITERAL = re.compile(r'(?:br|rb|cr|r)(?P<hashes>#{0,255})"')
CHAR_LITERAL = re.compile(
    r"(?:b)?'(?:\\(?:[nrt0\\'\"]|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]{1,6}\})|[^\\'\r\n])'"
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


def rust_code(source: str) -> str:
    """Mask comments/literals, as in the other architecture gates, retaining offsets."""
    masked = list(source)

    def blank(start: int, end: int) -> None:
        for index in range(start, end):
            if masked[index] not in "\r\n":
                masked[index] = " "

    index = 0
    while index < len(source):
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = len(source) if end < 0 else end
        elif source.startswith("/*", index):
            depth = 1
            end = index + 2
            while end < len(source) and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
        elif character := CHAR_LITERAL.match(source, index):
            end = character.end()
        elif raw := RAW_LITERAL.match(source, index):
            terminator = '"' + raw.group("hashes")
            close = source.find(terminator, raw.end())
            end = len(source) if close < 0 else close + len(terminator)
        elif source[index] == '"' or source.startswith(('b"', 'c"'), index):
            end = index + (1 if source[index] == '"' else 2)
            escaped = False
            while end < len(source):
                character = source[end]
                end += 1
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    break
        else:
            index += 1
            continue
        blank(index, end)
        index = end
    # A raw identifier is the same namespace, not an exemption from retirement.
    return re.sub(r"\br#(?=[A-Za-z_][A-Za-z0-9_]*)", "  ", "".join(masked))


def retired_root_members(source: str, start: int, owner: str) -> list[tuple[int, str]]:
    """Inspect only root members of a qualified path or grouped Rust use tree."""
    while start < len(source) and source[start].isspace():
        start += 1
    identifier = RUST_IDENTIFIER.match(source, start)
    if identifier is not None:
        module = identifier.group()
        if module in RETIRED_NAMESPACES[owner]:
            return [(start, module)]
        if module == "self":
            separator = re.match(r"\s*::\s*", source[identifier.end():])
            if separator is not None:
                return retired_root_members(source, identifier.end() + separator.end(), owner)
        return []
    if start >= len(source) or source[start] != "{":
        return []
    members: list[tuple[int, str]] = []
    depth = 1
    member_start = start + 1
    for delimiter in re.finditer(r"[{},]", source[start + 1:]):
        offset = start + 1 + delimiter.start()
        if delimiter.group() == "{":
            depth += 1
        elif delimiter.group() == "}":
            depth -= 1
            if depth == 0:
                members.extend(retired_root_members(source, member_start, owner))
                break
        elif depth == 1:
            members.extend(retired_root_members(source, member_start, owner))
            member_start = offset + 1
    return members


def qualified_use_groups(source: str) -> list[tuple[int, int]]:
    """Find named use-tree groups, whose children are not external crate roots."""
    groups: list[tuple[int, int]] = []
    for use in re.finditer(r"\buse\b[^;]*;", source):
        openings: list[tuple[int, bool]] = []
        for brace in USE_GROUP_BRACE.finditer(source, use.start(), use.end()):
            if brace.group() == "{":
                named = PRECEDING_PATH_MEMBER.search(source, use.start(), brace.start())
                openings.append((brace.start(), named is not None))
            elif openings:
                opening, named = openings.pop()
                if named:
                    groups.append((opening, brace.start()))
    return groups


def check_retired_namespaces(root: Path) -> list[str]:
    failures: list[str] = []
    for relative in rust_sources(root):
        source = rust_code((root / relative).read_text(encoding="utf-8"))
        local_groups = qualified_use_groups(source)
        found: set[tuple[int, str]] = set()
        for qualified in QUALIFIED_NAMESPACE.finditer(source):
            preceding = PRECEDING_PATH_MEMBER.search(source, 0, qualified.start())
            # `use ::mech_engine` starts an absolute path, whereas
            # `project::mech_engine` names an unrelated local module.
            if preceding is not None and preceding.group("member") not in {
                "use", "as", "return", "break", "yield",
            }:
                continue
            if any(opening < qualified.start() < closing for opening, closing in local_groups):
                continue
            owner = qualified.group("owner")
            for offset, module in retired_root_members(source, qualified.end(), owner):
                found.add((offset, f"{owner}::{module}"))
        for offset, namespace in sorted(found):
            failures.append(
                f"{relative}:{line_number(source, offset)}: retired {namespace} namespace"
            )
    return failures


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
    for module in ("interpreter", "expressions", "literals", "structures"):
        if re.search(rf"\bmod\s+{module}\s*;|\b{module}::", engine_lib):
            failures.append(f"{ENGINE_LIB}: retired {module} module remains reachable")
    core_lib = root / CORE_LIB
    if core_lib.exists():
        core_source = core_lib.read_text(encoding="utf-8")
        if re.search(r"\bmod\s+nodes\s*;|\bnodes::", core_source):
            failures.append(f"{CORE_LIB}: retired source-tree module/export remains reachable")
        if re.search(r"\b(?:struct|impl)\s+IndexedString\b", core_source):
            failures.append(f"{CORE_LIB}: retired IndexedString AST formatting helper remains")
    for relative in REMOVED_WORKSPACE_PATHS:
        path = root / relative
        if path.is_file() or (path.is_dir() and any(child.is_file() for child in path.rglob("*"))):
            failures.append(f"{relative}: retired AST workspace remains")
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
    return failures


def check_removed_surface(root: Path) -> list[str]:
    failures: list[str] = []
    for relative in rust_sources(root):
        source = (root / relative).read_text(encoding="utf-8")
        searchable = source
        # SourceScope::Interpreter is the maintained source namespace identity,
        # not the removed engine executor. Preserve this exact transport label
        # without admitting a declaration or import of the old executor type.
        searchable = re.sub(
            r"\bSourceScope\s*::\s*Interpreter\b",
            lambda match: re.sub(r"[^\r\n]", " ", match.group()),
            searchable,
        )
        if relative == Path("src/runtime/src/resolver/index.rs"):
            searchable = re.sub(
                r"\bInterpreter\s*\(\s*SourceInterpreterId\s*\)",
                lambda match: re.sub(r"[^\r\n]", " ", match.group()),
                searchable,
            )
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
        + check_retired_namespaces(root)
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
