#!/usr/bin/env python3
"""Generate the deterministic Gate C0 inventory from Rust source."""

from __future__ import annotations

import argparse
import ast
import glob
import io
import json
import re
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
if str(SCRIPT_DIRECTORY) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIRECTORY))
import value_system_legacy_scanner_v2 as LEGACY_SCANNER

Token = LEGACY_SCANNER.Token
mask_non_code = LEGACY_SCANNER.mask_non_code
rust_tokens = LEGACY_SCANNER.rust_tokens
canonical_identifier = LEGACY_SCANNER.canonical_identifier
balanced_token_end = LEGACY_SCANNER.balanced_token_end


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPOSITORY_ROOT / "tests/architecture/value-system/current-inventory.json"
)
DEFAULT_LEGACY_BASELINE_OUTPUT = (
    REPOSITORY_ROOT / "tests/architecture/value-system/legacy-growth-baseline.json"
)
REFERENCE_COMMIT = "d5e41f6fd43c9d21c5858d80dab50e7ce64e9a10"
LEGACY_SCANNER_CONTRACT = {
    "version": "c2-legacy-growth-v2",
    "module": "scripts/value_system_legacy_scanner_v2.py",
    "identifier_comparison": "canonical-identifier-v1",
    "ref_alias_resolution": "transitive-generic-identity-wrapper-v1",
    "baseline_policy": "checked-in-archived-source-oracle-may-only-shrink-v1",
    "implementation_sha256": "78529f2ffce2e3c3fc0d3ffabd55c8df1846ace2edd11500c095e82c8a12eed3",
}
AUDITED_ENUMS = ("LegacyValue", "ValueKind", "Kind")
EXCLUDED_SOURCE_DIRECTORIES = {
    ".git",
    "target",
    "node_modules",
    ".venv",
    "vendor",
    "third_party",
}
SEMANTIC_KIND_PATHS = {
    ("mech_core", "kind", "Kind"),
    ("mech_core", "Kind"),
    ("crate", "kind", "Kind"),
    ("super", "kind", "Kind"),
}
SYNTAX_KIND_PATHS = {
    ("mech_core", "nodes", "Kind"),
    ("crate", "nodes", "Kind"),
    ("nodes", "Kind"),
}
SEMANTIC_VALUE_PATHS = {
    ("Value",),
    ("LegacyValue",),
    ("crate", "Value"),
    ("crate", "LegacyValue"),
    ("crate", "value", "Value"),
    ("crate", "legacy_value", "LegacyValue"),
    ("mech_core", "Value"),
    ("mech_core", "LegacyValue"),
    ("mech_core", "value", "Value"),
    ("mech_core", "legacy_value", "LegacyValue"),
    ("super", "Value"),
    ("super", "LegacyValue"),
}
SEMANTIC_VALUE_KIND_PATHS = {
    ("ValueKind",),
    ("crate", "ValueKind"),
    ("crate", "value", "ValueKind"),
    ("mech_core", "ValueKind"),
    ("mech_core", "value", "ValueKind"),
    ("super", "ValueKind"),
}
CFG_TOKEN = re.compile(
    r'\s*(?:(?P<identifier>[A-Za-z_][A-Za-z0-9_]*)|'
    r'(?P<string>"(?:\\.|[^"\\])*")|(?P<punctuation>[(),=]))'
)
@dataclass(frozen=True)
class UseBinding:
    path: tuple[str, ...]
    local: str
    token: Token
    aliased: bool
    raw: bool
    glob: bool
    grouped: bool


class AuxiliaryFixtureError(ValueError):
    """A literal auxiliary fixture declaration is invalid or unsafe."""


class TypeContractError(ValueError):
    """A reviewed semantic/runtime type-contract source changed shape."""


class CargoMetadataError(ValueError):
    """Cargo metadata could not prove complete workspace source coverage."""


def matching_delimiter(masked: str, opening: int) -> int:
    pairs = {"(": ")", "[": "]", "{": "}"}
    opener = masked[opening]
    if opener not in pairs:
        raise ValueError(f"unsupported delimiter at offset {opening}")
    closing = pairs[opener]
    depth = 1
    for offset in range(opening + 1, len(masked)):
        if masked[offset] == opener:
            depth += 1
        elif masked[offset] == closing:
            depth -= 1
            if depth == 0:
                return offset
    raise ValueError(f"unterminated {opener!r} at offset {opening}")


def normalize_rust(fragment: str) -> str:
    text = re.sub(r"\s+", " ", fragment.strip())
    text = re.sub(r"\s*([<>{}(),\[\]])\s*", r"\1", text)
    text = re.sub(r",([)\]}])", r"\1", text)
    text = re.sub(r"\s*=\s*", " = ", text)
    return text


def cfg_tokens(expression: str) -> list[tuple[str, str]] | None:
    tokens: list[tuple[str, str]] = []
    offset = 0
    while offset < len(expression):
        if not expression[offset:].strip():
            break
        match = CFG_TOKEN.match(expression, offset)
        if match is None or match.lastgroup is None:
            return None
        tokens.append((match.lastgroup, match.group(match.lastgroup)))
        offset = match.end()
    return tokens


class CfgExpressionParser:
    """Evaluate possible cfg values with `test` fixed to false."""

    UNKNOWN = frozenset({False, True})

    def __init__(self, tokens: list[tuple[str, str]]) -> None:
        self.tokens = tokens
        self.offset = 0

    def accept(self, punctuation: str) -> bool:
        if self.offset >= len(self.tokens):
            return False
        kind, value = self.tokens[self.offset]
        if kind != "punctuation" or value != punctuation:
            return False
        self.offset += 1
        return True

    def parse(self) -> frozenset[bool]:
        possible = self.parse_predicate()
        if self.offset != len(self.tokens):
            raise ValueError("trailing cfg tokens")
        return possible

    def parse_predicate(self) -> frozenset[bool]:
        if self.offset >= len(self.tokens):
            raise ValueError("missing cfg predicate")
        kind, name = self.tokens[self.offset]
        if kind != "identifier":
            raise ValueError("cfg predicate must start with an identifier")
        self.offset += 1
        if self.accept("("):
            arguments: list[frozenset[bool]] = []
            if not self.accept(")"):
                while True:
                    arguments.append(self.parse_predicate())
                    if self.accept(")"):
                        break
                    if not self.accept(","):
                        raise ValueError("cfg arguments must be comma-separated")
                    if self.accept(")"):
                        break
            if name == "not" and len(arguments) == 1:
                return frozenset(not value for value in arguments[0])
            if name in {"all", "any"}:
                possible = {name == "all"}
                for argument in arguments:
                    possible = {
                        (left and right) if name == "all" else (left or right)
                        for left in possible
                        for right in argument
                    }
                return frozenset(possible)
            return self.UNKNOWN
        if self.accept("="):
            if self.offset >= len(self.tokens):
                raise ValueError("missing cfg value")
            kind, _value = self.tokens[self.offset]
            if kind not in {"identifier", "string"}:
                raise ValueError("invalid cfg value")
            self.offset += 1
            return self.UNKNOWN
        if name == "test":
            return frozenset({False})
        return self.UNKNOWN


def cfg_requires_test(expression: str) -> bool:
    tokens = cfg_tokens(expression)
    if tokens is None:
        return False
    try:
        return True not in CfgExpressionParser(tokens).parse()
    except ValueError:
        return False


def cfg_expressions(fragment: str) -> list[str]:
    masked = mask_non_code(fragment)
    expressions: list[str] = []
    start = 0
    pattern = re.compile(r"#\s*\[\s*cfg\s*\(")
    while True:
        match = pattern.search(masked, start)
        if match is None:
            break
        opening = masked.find("(", match.start(), match.end())
        closing = matching_delimiter(masked, opening)
        expressions.append(normalize_rust(fragment[opening + 1 : closing]))
        start = closing + 1
    return expressions


def cfg_item_end(source: str, start: int) -> int | None:
    masked = mask_non_code(source)
    offset = start
    while offset < len(masked):
        if masked.startswith("#[", offset):
            closing = matching_delimiter(masked, offset + 1)
            offset = closing + 1
            while offset < len(masked) and masked[offset].isspace():
                offset += 1
            continue
        break
    stack: list[str] = []
    pairs = {"(": ")", "[": "]"}
    for index in range(offset, len(masked)):
        character = masked[index]
        if character in pairs:
            stack.append(pairs[character])
        elif stack and character == stack[-1]:
            stack.pop()
        elif not stack and character == ";":
            return index + 1
        elif not stack and character == "{":
            return matching_delimiter(masked, index) + 1
    return None


def production_source(source: str) -> str:
    """Mask inline/direct Rust items that require cfg(test)."""
    masked = mask_non_code(source)
    spans: list[tuple[int, int]] = []
    start = 0
    pattern = re.compile(r"#\s*\[\s*cfg\s*\(")
    while True:
        match = pattern.search(masked, start)
        if match is None:
            break
        opening = masked.find("(", match.start(), match.end())
        closing = matching_delimiter(masked, opening)
        expression = normalize_rust(source[opening + 1 : closing])
        if cfg_requires_test(expression):
            end = cfg_item_end(source, closing + 1)
            if end is not None:
                spans.append((match.start(), end))
                start = end
                continue
        start = closing + 1
    result = list(source)
    for span_start, span_end in spans:
        for offset in range(span_start, span_end):
            if result[offset] != "\n":
                result[offset] = " "
    return "".join(result)


def source_path_is_included(path: Path, directory: Path) -> bool:
    parts = path.relative_to(directory).parts
    return not any(part in EXCLUDED_SOURCE_DIRECTORIES for part in parts)


def rust_files_under(directory: Path) -> list[Path]:
    directory = directory.resolve()
    return sorted(
        (
            path.resolve()
            for path in directory.rglob("*.rs")
            if path.is_file() and source_path_is_included(path, directory)
        ),
        key=lambda path: path.relative_to(directory).as_posix(),
    )


def cfg_production_viable(expressions: Sequence[str]) -> bool:
    """Whether all cfg attributes can be true with cfg(test) fixed false."""
    for expression in expressions:
        tokens = cfg_tokens(expression)
        if tokens is None:
            continue
        try:
            if True not in CfgExpressionParser(tokens).parse():
                return False
        except ValueError:
            continue
    return True


def external_modules(
    path: Path, source: str, *, crate_root: bool = False
) -> list[tuple[Path, bool]]:
    masked = mask_non_code(source)
    declaration = re.compile(
        r"(?P<attrs>(?:\s*#\s*\[[^\]]*\])*)\s*"
        r"(?:pub(?:\([^)]*\))?\s+)?mod\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;"
    )
    modules: list[tuple[Path, bool]] = []
    for match in declaration.finditer(masked):
        attrs = source[match.start("attrs") : match.end("attrs")]
        name = match.group("name")
        expressions = cfg_expressions(attrs)
        production_viable = cfg_production_viable(expressions)
        path_match = re.search(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]', attrs)
        if path_match is not None:
            candidate = path.parent / path_match.group(1)
        else:
            base = (
                path.parent
                if crate_root or path.name in {"lib.rs", "main.rs", "mod.rs"}
                else path.parent / path.stem
            )
            direct = base / f"{name}.rs"
            nested = base / name / "mod.rs"
            candidate = direct if direct.exists() else nested
        if candidate.exists():
            modules.append((candidate.resolve(), production_viable))
    return modules


def cargo_metadata(
    root: Path, manifest_path: Path | None = None
) -> dict[str, object]:
    command = ["cargo", "metadata", "--no-deps", "--format-version", "1"]
    if manifest_path is not None:
        command.extend(["--manifest-path", str(manifest_path.resolve())])
    process = subprocess.run(
        command,
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if process.returncode != 0:
        raise CargoMetadataError(
            "cargo metadata failed: " + process.stderr.strip()
        )
    try:
        return json.loads(process.stdout)
    except json.JSONDecodeError as error:
        raise CargoMetadataError("cargo metadata returned invalid JSON") from error


RUST_STRING_LITERAL = re.compile(
    r'(?P<raw>r(?P<hashes>#{0,255})"(?P<raw_body>.*?)"(?P=hashes))'
    r'|(?P<ordinary>"(?:\\.|[^"\\])*")',
    re.DOTALL,
)
TRYBUILD_VARIABLE = re.compile(
    r"\blet\s+(?:mut\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*"
    r"(?:(?:trybuild\s*::\s*)?TestCases)\s*::\s*new\s*\(\s*\)\s*;"
)
TRYBUILD_CALL = re.compile(
    r"(?P<receiver>[A-Za-z_][A-Za-z0-9_]*|"
    r"(?:(?:trybuild\s*::\s*)?TestCases)\s*::\s*new\s*\(\s*\))"
    r"\s*\.\s*(?P<method>compile_fail|pass)\s*\(\s*"
    + r"(?:"
    + RUST_STRING_LITERAL.pattern
    + r")"
    + r"\s*\)",
    re.DOTALL,
)


def rust_string_value(match: re.Match[str]) -> str:
    if match.group("raw") is not None:
        return match.group("raw_body")
    value = ast.literal_eval(match.group("ordinary"))
    if not isinstance(value, str):
        raise AuxiliaryFixtureError("trybuild pattern is not a Rust string")
    return value


def is_code_match(searchable: str, match: re.Match[str]) -> bool:
    return any(
        not character.isspace()
        for character in searchable[match.start() : match.start("method") + 1]
    )


def trybuild_calls(source: str) -> list[tuple[str, str]]:
    searchable = mask_non_code(source)
    variables = {
        match.group("name")
        for match in TRYBUILD_VARIABLE.finditer(source)
        if any(
            not character.isspace()
            for character in searchable[match.start() : match.start("name") + 1]
        )
    }
    calls: list[tuple[str, str]] = []
    for match in TRYBUILD_CALL.finditer(source):
        if not is_code_match(searchable, match):
            continue
        receiver = re.sub(r"\s+", "", match.group("receiver"))
        if receiver not in variables and "TestCases::new()" not in receiver:
            continue
        calls.append((match.group("method"), rust_string_value(match)))
    return calls


def trybuild_fixture_records(
    root: Path,
    test_targets: Iterable[tuple[Path, Path]],
    files: Sequence[Path],
) -> tuple[list[dict[str, object]], set[Path]]:
    root = root.resolve()
    targets = sorted(
        ((driver.resolve(), manifest_dir.resolve()) for driver, manifest_dir in test_targets),
        key=lambda item: item[0].relative_to(root).as_posix(),
    )
    file_set = {path.resolve() for path in files}
    crate_roots = {driver for driver, _manifest_dir in targets}
    graph = {
        path.resolve(): [
            (child, viable)
            for child, viable in external_modules(
                path,
                path.read_text(encoding="utf-8"),
                crate_root=path.resolve() in crate_roots,
            )
            if child in file_set
        ]
        for path in files
    }
    records: list[dict[str, object]] = []
    fixture_paths: set[Path] = set()
    seen_calls: set[tuple[Path, Path, str, str]] = set()
    for driver, manifest_dir in targets:
        callsites = reachable_files([driver], graph, production_only=False)
        for callsite in sorted(
            callsites, key=lambda path: path.relative_to(root).as_posix()
        ):
            source = callsite.read_text(encoding="utf-8")
            for method, pattern in trybuild_calls(source):
                key = (callsite, manifest_dir, method, pattern)
                if key in seen_calls:
                    continue
                seen_calls.add(key)
                pattern_path = Path(pattern)
                if pattern_path.is_absolute() or ".." in pattern_path.parts:
                    raise AuxiliaryFixtureError(
                        f"{callsite.relative_to(root).as_posix()}: {method} pattern "
                        f"escapes package root: {pattern!r}"
                    )
                matches: list[Path] = []
                for candidate_name in sorted(glob.glob(str(manifest_dir / pattern))):
                    candidate = Path(candidate_name).resolve()
                    try:
                        candidate.relative_to(manifest_dir)
                    except ValueError as error:
                        raise AuxiliaryFixtureError(
                            f"{callsite.relative_to(root).as_posix()}: {method} pattern "
                            f"escapes package root: {pattern!r}"
                        ) from error
                    if candidate.is_file() and candidate.suffix == ".rs":
                        matches.append(candidate)
                        fixture_paths.add(candidate)
                records.append(
                    {
                        "driver": callsite.relative_to(root).as_posix(),
                        "method": method,
                        "pattern": pattern,
                        "paths": [path.relative_to(root).as_posix() for path in matches],
                    }
                )
    return records, fixture_paths


def workspace_source_inventory(
    root: Path, metadata: dict[str, object]
) -> tuple[list[dict[str, object]], list[Path]]:
    root = root.resolve()
    members = set(metadata.get("workspace_members", []))
    packages: list[dict[str, object]] = []
    all_files: set[Path] = set()
    for package in metadata.get("packages", []):
        if members and package.get("id") not in members:
            continue
        manifest = Path(str(package["manifest_path"])).resolve()
        directory = manifest.parent
        try:
            relative_manifest = manifest.relative_to(root).as_posix()
            relative_directory = directory.relative_to(root).as_posix() or "."
        except ValueError as error:
            raise CargoMetadataError(
                f"workspace package escapes repository: {manifest}"
            ) from error
        files = rust_files_under(directory)
        safe_files: list[Path] = []
        for path in files:
            try:
                path.relative_to(root)
            except ValueError as error:
                raise CargoMetadataError(
                    f"workspace Rust source escapes repository: {path}"
                ) from error
            safe_files.append(path)
        all_files.update(safe_files)
        packages.append(
            {
                "name": str(package["name"]),
                "manifest": relative_manifest,
                "directory": relative_directory,
                "rust_file_count": len(safe_files),
            }
        )
    if not packages:
        raise CargoMetadataError("cargo metadata returned no workspace packages")
    return (
        sorted(packages, key=lambda row: (str(row["manifest"]), str(row["name"]))),
        sorted(all_files, key=lambda path: path.relative_to(root).as_posix()),
    )


def cargo_target_inventory(
    root: Path,
    metadata: dict[str, object],
) -> tuple[set[Path], list[dict[str, object]], list[tuple[Path, Path]]]:
    root = root.resolve()
    production: set[Path] = set()
    auxiliary_targets: list[dict[str, object]] = []
    test_targets: list[tuple[Path, Path]] = []
    excluded_kinds = {"test", "bench", "example"}
    for package in metadata.get("packages", []):
        manifest = Path(str(package["manifest_path"])).resolve()
        manifest_dir = manifest.parent
        try:
            relative_manifest = manifest.relative_to(root).as_posix()
            package_relative = manifest_dir.relative_to(root).parts
        except ValueError:
            continue
        fixture_package = package_relative[:2] == ("tests", "fixtures")
        for target in package.get("targets", []):
            source = Path(str(target["src_path"])).resolve()
            try:
                relative_source = source.relative_to(root).as_posix()
            except ValueError as error:
                raise CargoMetadataError(
                    f"Cargo target escapes repository: {source}"
                ) from error
            kinds = sorted(str(kind) for kind in target.get("kind", []))
            if set(kinds) & excluded_kinds or fixture_package:
                auxiliary_targets.append(
                    {
                        "manifest": relative_manifest,
                        "package": str(package["name"]),
                        "target_name": str(target["name"]),
                        "kinds": kinds,
                        "root": relative_source,
                        "_root_path": source,
                        "_cfg_test_projection": False,
                    }
                )
            else:
                production.add(source)
                auxiliary_targets.append(
                    {
                        "manifest": relative_manifest,
                        "package": str(package["name"]),
                        "target_name": f"{target['name']}-cfg-test",
                        "kinds": [*kinds, "cfg-test"],
                        "root": relative_source,
                        "_root_path": source,
                        "_cfg_test_projection": True,
                    }
                )
            if "test" in kinds:
                test_targets.append((source, manifest_dir))
    return production, auxiliary_targets, test_targets


def nested_cargo_fixture_targets(
    root: Path, workspace_metadata: dict[str, object]
) -> list[dict[str, object]]:
    root = root.resolve()
    workspace_manifests = {
        Path(str(package["manifest_path"])).resolve()
        for package in workspace_metadata.get("packages", [])
    }
    records: list[dict[str, object]] = []
    fixture_root = root / "tests/fixtures"
    if not fixture_root.is_dir():
        return records
    for manifest in sorted(fixture_root.rglob("Cargo.toml")):
        manifest = manifest.resolve()
        if manifest in workspace_manifests:
            continue
        metadata = cargo_metadata(root, manifest)
        matched = False
        for package in metadata.get("packages", []):
            package_manifest = Path(str(package["manifest_path"])).resolve()
            if package_manifest != manifest:
                continue
            matched = True
            for target in package.get("targets", []):
                source = Path(str(target["src_path"])).resolve()
                try:
                    relative_source = source.relative_to(root).as_posix()
                except ValueError as error:
                    raise CargoMetadataError(
                        f"auxiliary Cargo fixture target escapes repository: {source}"
                    ) from error
                records.append(
                    {
                        "manifest": manifest.relative_to(root).as_posix(),
                        "package": str(package["name"]),
                        "target_name": str(target["name"]),
                        "kinds": sorted(str(kind) for kind in target.get("kind", [])),
                        "root": relative_source,
                        "_root_path": source,
                        "_cfg_test_projection": False,
                    }
                )
        if not matched:
            raise CargoMetadataError(
                f"cargo metadata did not return exact fixture manifest {manifest}"
            )
    return records


def build_module_graph(
    root: Path, files: Iterable[Path], crate_roots: set[Path]
) -> tuple[dict[Path, list[tuple[Path, bool]]], list[Path]]:
    root = root.resolve()
    known = {path.resolve() for path in files}
    pending = list(known)
    graph: dict[Path, list[tuple[Path, bool]]] = {}
    while pending:
        path = pending.pop()
        children: list[tuple[Path, bool]] = []
        for child, viable in external_modules(
            path,
            path.read_text(encoding="utf-8"),
            crate_root=path in crate_roots,
        ):
            child = child.resolve()
            try:
                relative = child.relative_to(root)
            except ValueError as error:
                raise AuxiliaryFixtureError(
                    f"{path.relative_to(root).as_posix()}: module path escapes repository: {child}"
                ) from error
            if any(part in EXCLUDED_SOURCE_DIRECTORIES for part in relative.parts):
                raise AuxiliaryFixtureError(
                    f"{path.relative_to(root).as_posix()}: module path enters excluded build output: {relative.as_posix()}"
                )
            if child.suffix != ".rs" or not child.is_file():
                continue
            children.append((child, viable))
            if child not in known:
                known.add(child)
                pending.append(child)
        graph[path] = children
    return graph, sorted(known, key=lambda path: path.relative_to(root).as_posix())


def cargo_target_roots(root: Path) -> tuple[set[Path], set[Path]]:
    metadata = cargo_metadata(root)
    production, auxiliary_targets, _test_targets = cargo_target_inventory(
        root, metadata
    )
    return production, {
        Path(str(target["_root_path"])).resolve() for target in auxiliary_targets
    }


def reachable_files(
    roots: Iterable[Path],
    graph: dict[Path, list[tuple[Path, bool]]],
    *,
    production_only: bool,
) -> set[Path]:
    reached: set[Path] = set()
    pending = [path.resolve() for path in roots]
    while pending:
        path = pending.pop()
        if path in reached:
            continue
        reached.add(path)
        for child, production_viable in graph.get(path, []):
            if not production_only or production_viable:
                pending.append(child)
    return reached


def auxiliary_cargo_records(
    root: Path,
    targets: Sequence[dict[str, object]],
    graph: dict[Path, list[tuple[Path, bool]]],
    production_reachable: set[Path],
) -> tuple[list[dict[str, object]], set[Path]]:
    grouped: dict[tuple[str, str], list[dict[str, object]]] = {}
    effective: set[Path] = set()
    for target in targets:
        target_root = Path(str(target["_root_path"])).resolve()
        reachable = reachable_files([target_root], graph, production_only=False)
        if bool(target.get("_cfg_test_projection")):
            reachable -= reachable_files(
                [target_root], graph, production_only=True
            )
        effective.update(reachable - production_reachable)
        if not reachable:
            continue
        row = {
            "name": target["target_name"],
            "kinds": target["kinds"],
            "root": target["root"],
            "reachable_rust_files": sorted(
                path.relative_to(root).as_posix() for path in reachable
            ),
        }
        grouped.setdefault(
            (str(target["manifest"]), str(target["package"])), []
        ).append(row)
    records = [
        {
            "manifest": manifest,
            "package": package,
            "targets": sorted(rows, key=lambda row: (str(row["name"]), str(row["root"]))),
        }
        for (manifest, package), rows in sorted(grouped.items())
    ]
    return records, effective


def production_inventory_details(
    root: Path, target_roots: Iterable[Path] | None = None
) -> tuple[
    list[Path],
    list[dict[str, object]],
    list[dict[str, object]],
    list[dict[str, object]],
    list[Path],
]:
    """Give every repository Rust file an audited or proven-auxiliary disposition."""
    requested_root = root
    root = root.resolve()
    files = rust_files_under(root)
    if target_roots is None:
        metadata = cargo_metadata(root)
        workspace_packages, _workspace_files = workspace_source_inventory(root, metadata)
        production_roots, workspace_auxiliary, test_targets = cargo_target_inventory(
            root, metadata
        )
        nested_auxiliary = nested_cargo_fixture_targets(root, metadata)
        auxiliary_targets = [*workspace_auxiliary, *nested_auxiliary]
        if not production_roots and not auxiliary_targets:
            raise CargoMetadataError("cargo metadata returned no Rust target roots")
    else:
        workspace_packages = []
        production_roots = {path.resolve() for path in target_roots}
        auxiliary_targets = []
        test_targets = []
    auxiliary_roots = {
        Path(str(target["_root_path"])).resolve() for target in auxiliary_targets
    }
    crate_roots = production_roots | auxiliary_roots | {
        driver.resolve() for driver, _manifest_dir in test_targets
    }
    graph, files = build_module_graph(root, files, crate_roots)
    fixture_records, fixture_roots = trybuild_fixture_records(
        root, test_targets, files
    )
    graph, files = build_module_graph(
        root, files, crate_roots | {path.resolve() for path in fixture_roots}
    )
    for record in fixture_records:
        roots = [root / str(path) for path in record["paths"]]
        record["reachable_rust_files"] = sorted(
            path.relative_to(root).as_posix()
            for path in reachable_files(roots, graph, production_only=False)
        )
    production_reachable = reachable_files(
        production_roots, graph, production_only=True
    )
    cfg_test_auxiliary = reachable_files(
        production_roots, graph, production_only=False
    ) - production_reachable
    cargo_records, cargo_auxiliary = auxiliary_cargo_records(
        root, auxiliary_targets, graph, production_reachable
    )
    trybuild_reachable = reachable_files(
        fixture_roots, graph, production_only=False
    ) - production_reachable
    auxiliary = cargo_auxiliary | trybuild_reachable | cfg_test_auxiliary
    audited = [path for path in files if path not in auxiliary]
    return (
        [requested_root / path.relative_to(root) for path in audited],
        fixture_records,
        workspace_packages,
        cargo_records,
        [requested_root / path.relative_to(root) for path in files],
    )


def production_inventory(
    root: Path, target_roots: Iterable[Path] | None = None
) -> tuple[list[Path], list[dict[str, object]]]:
    production, fixtures, _packages, _cargo, _files = production_inventory_details(
        root, target_roots
    )
    return production, fixtures


def production_files(
    root: Path, target_roots: Iterable[Path] | None = None
) -> list[Path]:
    return production_inventory(root, target_roots)[0]


def enum_body(source: str, name: str) -> tuple[int, int]:
    masked = mask_non_code(source)
    match = re.search(rf"\bpub\s+enum\s+{re.escape(name)}\b", masked)
    if match is None:
        raise ValueError(f"pub enum {name} was not found")
    opening = masked.find("{", match.end())
    if opening < 0:
        raise ValueError(f"pub enum {name} has no body")
    return opening + 1, matching_delimiter(masked, opening)


def split_top_level_items(source: str) -> list[str]:
    masked = mask_non_code(source)
    stack: list[str] = []
    pairs = {"(": ")", "[": "]", "{": "}", "<": ">"}
    items: list[str] = []
    start = 0
    for offset, character in enumerate(masked):
        if character in pairs:
            stack.append(pairs[character])
        elif stack and character == stack[-1]:
            stack.pop()
        elif character == "," and not stack:
            if source[start:offset].strip():
                items.append(source[start:offset])
            start = offset + 1
    if source[start:].strip():
        items.append(source[start:])
    return items


def strip_attributes(item: str) -> str:
    masked = mask_non_code(item)
    offset = 0
    while True:
        while offset < len(masked) and masked[offset].isspace():
            offset += 1
        if not masked.startswith("#[", offset):
            return item[offset:].strip()
        offset = matching_delimiter(masked, offset + 1) + 1


def variant_payload(item: str) -> tuple[str, str | None]:
    declaration = strip_attributes(item)
    match = re.match(r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)", declaration)
    if match is None:
        raise ValueError(f"cannot parse enum variant: {declaration!r}")
    name = match.group("name")
    rest = declaration[match.end() :].strip()
    if not rest:
        return name, None
    if rest.startswith("(") and rest.endswith(")"):
        inner = rest[1:-1]
        fields = split_top_level_items(inner)
        if len(fields) == 1:
            return name, normalize_rust(fields[0])
        return name, "(" + ", ".join(normalize_rust(field) for field in fields) + ")"
    return name, normalize_rust(rest)


def storage_ownership(name: str, payload: str | None) -> str:
    if payload is None:
        return "inline-control-token"
    if payload.startswith("Ref<") or payload == "MutableReference":
        return "shared-mutable-reference"
    if payload.startswith("Matrix<"):
        return "owned-mutable-matrix-handle"
    if payload.startswith("Box<") or payload.startswith("(Box<"):
        return "owned-wrapper"
    if payload == "ValueKind":
        return "inline-type-metadata"
    if name == "Id" or payload in {"u64", "usize"}:
        return "inline-scalar"
    return "owned-inline-payload"


def current_roles(name: str) -> list[str]:
    special = {
        "MutableReference": ["mutable-storage", "reactive-identity"],
        "Typed": ["semantic-payload", "type-wrapper"],
        "Kind": ["compiler-type-data", "machine-argument"],
        "IndexAll": ["selection-ir"],
        "EmptyKind": ["semantic-payload", "type-wrapper"],
        "Empty": ["semantic-payload", "mutable-storage", "machine-output"],
    }
    return special.get(
        name, ["semantic-payload", "machine-argument", "machine-output"]
    )


def parse_enum(source: str, name: str, *, value: bool) -> list[dict[str, object]]:
    start, end = enum_body(source, name)
    variants: list[dict[str, object]] = []
    for item in split_top_level_items(source[start:end]):
        variant, payload = variant_payload(item)
        expressions = cfg_expressions(item)
        cfg = None
        if len(expressions) == 1:
            cfg = expressions[0]
        elif expressions:
            cfg = "all(" + ", ".join(expressions) + ")"
        record: dict[str, object] = {
            "name": variant,
            "cfg": cfg,
            "payload_type": payload,
        }
        if value:
            record["storage_ownership"] = storage_ownership(variant, payload)
            record["current_roles"] = current_roles(variant)
        variants.append(record)
    return variants


def use_statement_indexes(tokens: Sequence[Token]) -> set[int]:
    indexes: set[int] = set()
    offset = 0
    while offset < len(tokens):
        if tokens[offset].value != "use":
            offset += 1
            continue
        end = offset
        while end < len(tokens) and tokens[end].value != ";":
            indexes.add(end)
            end += 1
        if end < len(tokens):
            indexes.add(end)
        offset = end + 1
    return indexes


def split_top_level_tokens(tokens: Sequence[Token]) -> list[list[Token]]:
    parts: list[list[Token]] = []
    start = 0
    brace_depth = 0
    angle_depth = 0
    for index, token in enumerate(tokens):
        if token.value == "{":
            brace_depth += 1
        elif token.value == "}" and brace_depth:
            brace_depth -= 1
        elif token.value == "<":
            angle_depth += 1
        elif token.value == ">" and angle_depth:
            angle_depth -= 1
        elif token.value == "," and brace_depth == 0 and angle_depth == 0:
            if start < index:
                parts.append(list(tokens[start:index]))
            start = index + 1
    if start < len(tokens):
        parts.append(list(tokens[start:]))
    return parts


def use_bindings(tokens: Sequence[Token]) -> list[UseBinding]:
    def identifiers(items: Sequence[Token]) -> tuple[str, ...]:
        return tuple(
            canonical_identifier(token.value)
            for token in items
            if re.fullmatch(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*|\*", token.value)
        )

    def expand(
        items: Sequence[Token], prefix: tuple[str, ...], grouped: bool = False
    ) -> list[UseBinding]:
        records: list[UseBinding] = []
        for part in split_top_level_tokens(items):
            opening = next(
                (index for index, token in enumerate(part) if token.value == "{"),
                None,
            )
            if opening is not None:
                closing = balanced_token_end(part, opening, "{", "}")
                if closing is None:
                    continue
                head = list(part[:opening])
                while head and head[-1].value == "::":
                    head.pop()
                records.extend(
                    expand(
                        part[opening + 1 : closing],
                        prefix + identifiers(head),
                        True,
                    )
                )
                continue
            alias_index = next(
                (index for index, token in enumerate(part) if token.value == "as"),
                None,
            )
            body = part[:alias_index] if alias_index is not None else part
            path_suffix = identifiers(body)
            path = prefix if path_suffix == ("self",) else prefix + path_suffix
            if not path:
                continue
            alias_token = (
                part[alias_index + 1]
                if alias_index is not None and alias_index + 1 < len(part)
                else None
            )
            local = canonical_identifier(
                alias_token.value if alias_token is not None else path[-1]
            )
            records.append(
                UseBinding(
                    path=path,
                    local=local,
                    token=alias_token or body[-1],
                    aliased=alias_token is not None,
                    raw=any(token.value.startswith("r#") for token in body),
                    glob=path[-1] == "*",
                    grouped=grouped,
                )
            )
        return records

    records: list[UseBinding] = []
    offset = 0
    while offset < len(tokens):
        if tokens[offset].value != "use":
            offset += 1
            continue
        end = offset + 1
        while end < len(tokens) and tokens[end].value != ";":
            end += 1
        records.extend(expand(tokens[offset + 1 : end], ()))
        offset = end + 1
    return records


def crate_root_bindings(
    root: Path, path: Path, tokens: Sequence[Token]
) -> list[UseBinding]:
    """Resolve names imported by an explicit ``use crate::*`` from lib.rs."""
    if not any(binding.path == ("crate", "*") for binding in use_bindings(tokens)):
        return []
    root = root.resolve()
    path = path.resolve()
    directory = path.parent
    while directory == root or root in directory.parents:
        if (directory / "Cargo.toml").is_file():
            crate_root = directory / "src/lib.rs"
            if not crate_root.is_file() or crate_root.resolve() == path:
                return []
            source = crate_root.read_text(encoding="utf-8")
            searchable = mask_non_code(production_source(source))
            return use_bindings(rust_tokens(source, searchable))
        if directory == root:
            break
        directory = directory.parent
    return []


def identifier_path(tokens: Sequence[Token], index: int) -> tuple[str, ...]:
    start = index
    while (
        start >= 2
        and tokens[start - 1].value == "::"
        and re.fullmatch(
            r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", tokens[start - 2].value
        )
    ):
        start -= 2
    return tuple(
        canonical_identifier(tokens[position].value)
        for position in range(start, index + 1, 2)
    )


def kind_path_classification(path: tuple[str, ...]) -> str | None:
    if path in SEMANTIC_KIND_PATHS or (
        path
        and path[-1] == "Kind"
        and len(path) >= 2
        and path[-2] == "kind"
    ):
        return "semantic"
    if path in SYNTAX_KIND_PATHS or (
        path
        and path[-1] == "Kind"
        and len(path) >= 2
        and path[-2] == "nodes"
    ):
        return "syntax"
    return None


def audited_enum_path(path: tuple[str, ...]) -> str | None:
    if path in SEMANTIC_VALUE_PATHS:
        return "LegacyValue"
    if path in SEMANTIC_VALUE_KIND_PATHS:
        return "ValueKind"
    if kind_path_classification(path) == "semantic":
        return "Kind"
    return None


def lexical_scope_tree(
    tokens: Sequence[Token], relative: str = ""
) -> list[dict[str, object]]:
    scopes: list[dict[str, object]] = [
        {"parent": None, "start": 0, "end": len(tokens), "bindings": [], "globs": []}
    ]
    stack = [0]
    use_indexes = use_statement_indexes(tokens)
    for index, token in enumerate(tokens):
        if index in use_indexes:
            continue
        if token.value == "{":
            scopes.append(
                {
                    "parent": stack[-1],
                    "start": index + 1,
                    "end": len(tokens),
                    "bindings": [],
                    "globs": [],
                }
            )
            stack.append(len(scopes) - 1)
        elif token.value == "}" and len(stack) > 1:
            scopes[stack[-1]]["end"] = index
            stack.pop()

    def containing(index: int) -> int:
        candidates = [
            scope_index
            for scope_index, scope in enumerate(scopes)
            if int(scope["start"]) <= index < int(scope["end"])
        ]
        return max(candidates, key=lambda item: int(scopes[item]["start"]))

    token_indexes = {id(token): index for index, token in enumerate(tokens)}
    for binding in use_bindings(tokens):
        index = token_indexes.get(id(binding.token))
        if index is None:
            continue
        scope = scopes[containing(index)]
        if binding.glob:
            scope["globs"].append(binding)
        elif binding.local == "Kind":
            classification = kind_path_classification(binding.path)
            scope["bindings"].append(classification or "ambiguous")
    for index in range(len(tokens) - 1):
        declaration = canonical_identifier(tokens[index].value)
        name = canonical_identifier(tokens[index + 1].value)
        if declaration in {"enum", "struct", "type", "trait", "union"} and name == "Kind":
            declaration_scope = containing(index)
            classification = "semantic" if (
                relative == "src/core/src/kind.rs" and declaration_scope == 0
            ) else "syntax"
            scopes[declaration_scope]["bindings"].append(classification)
    return scopes


def innermost_scope(scopes: Sequence[dict[str, object]], index: int) -> int:
    candidates = [
        scope_index
        for scope_index, scope in enumerate(scopes)
        if int(scope["start"]) <= index < int(scope["end"])
    ]
    return max(candidates, key=lambda item: int(scopes[item]["start"]))


def kind_qualifier_classification(
    relative: str,
    tokens: Sequence[Token],
    index: int,
    bindings: Sequence[UseBinding],
) -> str:
    path = identifier_path(tokens, index)
    if len(path) > 1:
        return kind_path_classification(path) or "ambiguous"
    local = path[0]
    inherited = {
        classification
        for binding in bindings
        if binding.local == local
        for classification in [kind_path_classification(binding.path)]
        if classification is not None
    }
    scopes = lexical_scope_tree(tokens, relative)
    scope_index: int | None = innermost_scope(scopes, index)
    while scope_index is not None:
        exact = set(scopes[scope_index]["bindings"])
        if scope_index == 0:
            exact.update(inherited)
        if len(exact) == 1:
            return next(iter(exact))
        if exact:
            return "ambiguous"
        parent = scopes[scope_index]["parent"]
        scope_index = int(parent) if parent is not None else None
    if inherited:
        return "ambiguous"
    scope_index = innermost_scope(scopes, index)
    while True:
        if scopes[scope_index]["globs"]:
            return "ambiguous"
        parent = scopes[scope_index]["parent"]
        if parent is None:
            break
        scope_index = int(parent)
    return "ambiguous"


def variant_uses(
    relative: str,
    tokens: Sequence[Token],
    variants_by_enum: dict[str, set[str]],
    inherited_bindings: Sequence[UseBinding] = (),
) -> list[dict[str, object]]:
    imports = use_statement_indexes(tokens)
    records: list[dict[str, object]] = []
    for index in range(len(tokens) - 2):
        if index in imports or tokens[index + 1].value != "::":
            continue
        enum_name = canonical_identifier(tokens[index].value)
        if enum_name == "Value" and "LegacyValue" in variants_by_enum:
            enum_name = "LegacyValue"
        variant = canonical_identifier(tokens[index + 2].value)
        if enum_name not in variants_by_enum or variant not in variants_by_enum[enum_name]:
            continue
        if enum_name == "Kind":
            classification = kind_qualifier_classification(
                relative, tokens, index, inherited_bindings
            )
            if classification != "semantic":
                continue
        records.append(
            {
                "enum": enum_name,
                "variant": variant,
                "path": relative,
                "line": tokens[index].line,
                "column": tokens[index].column,
            }
        )
    return records


def audited_path_name(
    tokens: Sequence[Token], variants_by_enum: dict[str, set[str]]
) -> str | None:
    if not tokens:
        return None
    path = list(tokens)
    if "as" in [token.value for token in path]:
        path = path[: next(index for index, token in enumerate(path) if token.value == "as")]
    if not path:
        return None
    terminal = canonical_identifier(path[-1].value)
    if terminal == "Value" and "LegacyValue" in variants_by_enum:
        terminal = "LegacyValue"
    if terminal not in variants_by_enum:
        return None
    for index, token in enumerate(path):
        if index % 2 == 0:
            if not re.fullmatch(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*", token.value):
                return None
        elif token.value != "::":
            return None
    return terminal


def impl_ranges(
    tokens: Sequence[Token], variants_by_enum: dict[str, set[str]]
) -> list[tuple[str, int, int]]:
    ranges: list[tuple[str, int, int]] = []
    offset = 0
    while offset < len(tokens):
        if tokens[offset].value != "impl":
            offset += 1
            continue
        angle_depth = 0
        paren_depth = 0
        opening = None
        for index in range(offset + 1, len(tokens)):
            value = tokens[index].value
            if value == "<":
                angle_depth += 1
            elif value == ">" and angle_depth:
                angle_depth -= 1
            elif value == "(":
                paren_depth += 1
            elif value == ")" and paren_depth:
                paren_depth -= 1
            elif value == "{" and angle_depth == 0 and paren_depth == 0:
                opening = index
                break
            elif value == ";" and angle_depth == 0 and paren_depth == 0:
                break
        if opening is None:
            offset += 1
            continue
        closing = balanced_token_end(tokens, opening, "{", "}")
        if closing is None:
            break
        header = list(tokens[offset + 1 : opening])
        if header and header[0].value == "<":
            generic_end = balanced_token_end(header, 0, "<", ">")
            header = header[generic_end + 1 :] if generic_end is not None else []
        angle_depth = 0
        for_index = None
        where_index = None
        for index, token in enumerate(header):
            if token.value == "<":
                angle_depth += 1
            elif token.value == ">" and angle_depth:
                angle_depth -= 1
            elif angle_depth == 0 and token.value == "for":
                for_index = index
            elif angle_depth == 0 and token.value == "where":
                where_index = index
                break
        self_type = header[(for_index + 1) if for_index is not None else 0 : where_index]
        receiver = audited_path_name(self_type, variants_by_enum)
        if receiver is not None:
            ranges.append((receiver, opening, closing))
        offset = closing + 1
    return ranges


def audited_type_alias_violations(
    relative: str,
    tokens: Sequence[Token],
    inherited_bindings: Sequence[UseBinding] = (),
) -> list[dict[str, object]]:
    return [
        violation
        for violation in LEGACY_SCANNER.audited_type_alias_violations(
            relative, tokens, inherited_bindings
        )
        if violation["enum"] != "Ref"
    ]


def resolved_ref_type_aliases(tokens: Sequence[Token]) -> dict[str, Token]:
    return LEGACY_SCANNER.resolved_ref_type_aliases(tokens)


def ref_type_alias_violations(
    relative: str,
    tokens: Sequence[Token],
    inherited_bindings: Sequence[UseBinding] = (),
) -> list[dict[str, object]]:
    return [
        violation
        for violation in LEGACY_SCANNER.audited_type_alias_violations(
            relative, tokens, inherited_bindings
        )
        if violation["enum"] == "Ref"
    ]


def qualification_violations(
    relative: str,
    tokens: Sequence[Token],
    variants_by_enum: dict[str, set[str]] | None = None,
    inherited_bindings: Sequence[UseBinding] = (),
) -> list[dict[str, object]]:
    violations = audited_type_alias_violations(
        relative, tokens, inherited_bindings
    )
    violations.extend(
        ref_type_alias_violations(relative, tokens, inherited_bindings)
    )
    bindings = use_bindings(tokens)
    for binding in bindings:
        audited = audited_enum_path(binding.path)
        if audited is not None and binding.raw:
            violations.append(
                {
                    "enum": audited,
                    "kind": "raw-audited-alias",
                    "path": relative,
                    "line": binding.token.line,
                    "column": binding.token.column,
                }
            )
        if (
            kind_path_classification(binding.path) == "semantic"
            and binding.local != "Kind"
            and not binding.raw
        ):
            violations.append(
                {
                    "enum": "Kind",
                    "kind": "semantic-kind-alias",
                    "path": relative,
                    "line": binding.token.line,
                    "column": binding.token.column,
                }
            )
        if binding.path and binding.path[-1] == "Ref" and binding.local != "Ref":
            violations.append(
                {
                    "enum": "Ref",
                    "kind": "ref-alias",
                    "path": relative,
                    "line": binding.token.line,
                    "column": binding.token.column,
                }
            )
    for binding in bindings:
        if binding.raw:
            continue
        enum_index = next(
            (
                index
                for index in range(len(binding.path))
                if audited_enum_path(binding.path[: index + 1]) is not None
            ),
            None,
        )
        if enum_index is None:
            continue
        declared_enum_name = binding.path[enum_index]
        enum_name = audited_enum_path(binding.path[: enum_index + 1])
        assert enum_name is not None
        suffix = binding.path[enum_index + 1 :]
        kind = None
        variant = None
        if not suffix and binding.local != declared_enum_name:
            kind = "enum-alias"
        elif suffix and suffix[0] == "*":
            kind = "glob-import"
        elif suffix:
            variant = suffix[0]
            kind = (
                "variant-alias"
                if binding.aliased
                else "grouped-variant-import"
                if binding.grouped
                else "single-variant-import"
            )
        if kind is not None:
            violations.append(
                {
                    "enum": enum_name,
                    "variant": variant,
                    "kind": kind,
                    "path": relative,
                    "line": binding.token.line,
                    "column": binding.token.column,
                }
            )
    if variants_by_enum is None:
        return violations

    imports = [*bindings, *inherited_bindings]
    for index in range(len(tokens) - 2):
        if tokens[index + 1].value != "::":
            continue
        variant = canonical_identifier(tokens[index + 2].value)
        if variant not in variants_by_enum["Kind"]:
            continue
        if canonical_identifier(tokens[index].value) == "Kind" or any(
            binding.local == canonical_identifier(tokens[index].value)
            and kind_path_classification(binding.path) is not None
            for binding in imports
        ):
            classification = kind_qualifier_classification(
                relative, tokens, index, inherited_bindings
            )
            if classification == "ambiguous":
                violations.append(
                    {
                        "enum": "Kind",
                        "variant": variant,
                        "kind": "kind-qualifier-ambiguous",
                        "path": relative,
                        "line": tokens[index].line,
                        "column": tokens[index].column,
                    }
                )

    def add_alternate(enum_name: str, variant: str, token: Token, kind: str) -> None:
        violations.append(
            {
                "enum": enum_name,
                "variant": variant,
                "kind": kind,
                "path": relative,
                "line": token.line,
                "column": token.column,
            }
        )

    # Raw identifiers are valid Rust but not the one canonical spelling audited
    # by variant_uses(). Reject them instead of silently losing the occurrence.
    for index in range(len(tokens) - 2):
        enum_spelling = tokens[index].value
        variant_spelling = tokens[index + 2].value
        enum_name = canonical_identifier(enum_spelling)
        if enum_name == "Value" and "LegacyValue" in variants_by_enum:
            enum_name = "LegacyValue"
        variant = canonical_identifier(variant_spelling)
        if (
            enum_name in variants_by_enum
            and tokens[index + 1].value == "::"
            and variant in variants_by_enum[enum_name]
            and (enum_spelling.startswith("r#") or variant_spelling.startswith("r#"))
        ):
            add_alternate(
                enum_name,
                variant,
                tokens[index],
                "raw-identifier-qualified-variant",
            )

    # Reject type-qualified spellings such as <Value>::Empty,
    # <crate::Value>::Empty, and <Value as Trait>::Empty.
    for opening in range(len(tokens)):
        if tokens[opening].value != "<":
            continue
        closing = balanced_token_end(tokens, opening, "<", ">")
        if closing is None or closing + 2 >= len(tokens):
            continue
        if tokens[closing + 1].value != "::":
            continue
        receiver = audited_path_name(tokens[opening + 1 : closing], variants_by_enum)
        if receiver is None:
            continue
        spelling = tokens[closing + 2].value
        variant = canonical_identifier(spelling)
        if variant in variants_by_enum[receiver]:
            add_alternate(
                receiver,
                variant,
                tokens[opening],
                "type-qualified-variant",
            )

    # Reject generic/turbofish qualifiers between an audited enum and variant.
    for index, token in enumerate(tokens):
        receiver = canonical_identifier(token.value)
        if receiver == "Value" and "LegacyValue" in variants_by_enum:
            receiver = "LegacyValue"
        if (
            receiver not in variants_by_enum
            or index + 2 >= len(tokens)
            or tokens[index + 1].value != "::"
            or tokens[index + 2].value != "<"
        ):
            continue
        closing = balanced_token_end(tokens, index + 2, "<", ">")
        if (
            closing is not None
            and closing + 2 < len(tokens)
            and tokens[closing + 1].value == "::"
        ):
            variant = canonical_identifier(tokens[closing + 2].value)
            if variant in variants_by_enum[receiver]:
                add_alternate(
                    receiver,
                    variant,
                    token,
                    "generic-qualified-variant",
                )

    # Self is resolvable only inside the exact audited enum's impl range.
    for receiver, opening, closing in impl_ranges(tokens, variants_by_enum):
        for index in range(opening + 1, closing - 1):
            variant = canonical_identifier(tokens[index + 2].value)
            if (
                tokens[index].value == "Self"
                and tokens[index + 1].value == "::"
                and variant in variants_by_enum[receiver]
            ):
                add_alternate(
                    receiver,
                    variant,
                    tokens[index],
                    "self-qualified-variant",
                )
            if tokens[index].value == "<" and tokens[index + 1].value == "Self":
                angle_end = balanced_token_end(tokens, index, "<", ">")
                if (
                    angle_end is not None
                    and angle_end + 2 < closing
                    and tokens[angle_end + 1].value == "::"
                ):
                    variant = canonical_identifier(tokens[angle_end + 2].value)
                    if variant in variants_by_enum[receiver]:
                        add_alternate(
                            receiver,
                            variant,
                            tokens[index],
                            "self-type-qualified-variant",
                        )
    return violations


HIGH_RISK_PATTERNS = LEGACY_SCANNER.HIGH_RISK_PATTERNS


def aliases(
    corpus: list[tuple[str, str, str, list[Token]]]
) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    return LEGACY_SCANNER.alias_records(corpus)


def high_risk_uses(
    corpus: list[tuple[str, str, str, list[Token]]], identifier: str
) -> list[dict[str, object]]:
    return LEGACY_SCANNER.high_risk_uses(corpus, identifier)


def legacy_scanner_implementation_sha256() -> str:
    return LEGACY_SCANNER.scanner_module_sha256()


def mechanism_records(
    corpus: list[tuple[str, str, str, list[Token]]], identifiers: Iterable[str]
) -> list[dict[str, object]]:
    return LEGACY_SCANNER.mechanism_records(corpus, identifiers)


TYPE_CONTRACT_SOURCE_SPECS = {
    "kind_expression_sources": (
        ("src/core/src/kind.rs", "Kind"),
        ("src/core/src/nodes.rs", "Kind"),
        ("src/core/src/nodes.rs", "KindAnnotation"),
        ("src/core/src/nodes.rs", "Var.kind"),
        ("src/core/src/nodes.rs", "Binding.kind"),
        ("src/core/src/nodes.rs", "Field.kind"),
        ("src/core/src/nodes.rs", "KindDefine.kind"),
        ("src/core/src/nodes.rs", "EnumVariant.value"),
        ("src/core/src/nodes.rs", "Fsm.kind"),
    ),
    "kind_scheme_sources": (
        ("src/core/src/nodes.rs", "FunctionArgument.kind"),
        ("src/core/src/nodes.rs", "FunctionDefine.input"),
        ("src/core/src/nodes.rs", "FunctionDefine.output"),
        ("src/core/src/nodes.rs", "FsmSpecification.input"),
        ("src/core/src/nodes.rs", "FsmSpecification.output"),
    ),
    "runtime_representation_sources": (
        ("src/core/src/function/argument.rs", "FunctionArgumentRole"),
        ("src/core/src/function/argument.rs", "FunctionMatrixRepresentation"),
        ("src/core/src/function/argument.rs", "FunctionMatrixDescriptor"),
        ("src/core/src/function/signature.rs", "FunctionMatrixElement"),
        ("src/core/src/function/signature.rs", "FunctionMatrixStoragePattern"),
        ("src/core/src/function/signature.rs", "FunctionValueRepresentation"),
        ("src/core/src/function/signature.rs", "RuntimeFunctionInputs"),
        ("src/core/src/function/signature.rs", "RuntimeFunctionSignature"),
        ("src/core/src/function/signature.rs", "FunctionRuntimeType"),
        ("src/core/src/function/signature.rs", "NativeValueFeature"),
    ),
}
TYPE_CONTRACT_TARGETS = {
    "kind_expression_sources": ("KindExpr", "C1"),
    "kind_scheme_sources": ("KindScheme", "C1"),
    "runtime_representation_sources": (
        "RuntimeRepresentationSignature and native-lowering metadata",
        "existing-runtime-contract",
    ),
}
TYPE_CONTRACT_DECLARATION_FORMS = {
    ("src/core/src/kind.rs", "Kind"): "enum",
    ("src/core/src/nodes.rs", "Kind"): "enum",
    ("src/core/src/nodes.rs", "KindAnnotation"): "struct",
    ("src/core/src/function/argument.rs", "FunctionArgumentRole"): "enum",
    ("src/core/src/function/argument.rs", "FunctionMatrixRepresentation"): "enum",
    ("src/core/src/function/argument.rs", "FunctionMatrixDescriptor"): "struct",
    ("src/core/src/function/signature.rs", "FunctionMatrixElement"): "enum",
    ("src/core/src/function/signature.rs", "FunctionMatrixStoragePattern"): "enum",
    ("src/core/src/function/signature.rs", "FunctionValueRepresentation"): "enum",
    ("src/core/src/function/signature.rs", "RuntimeFunctionInputs"): "enum",
    ("src/core/src/function/signature.rs", "RuntimeFunctionSignature"): "struct",
    ("src/core/src/function/signature.rs", "FunctionRuntimeType"): "trait",
    ("src/core/src/function/signature.rs", "NativeValueFeature"): "enum",
}
TYPE_CONTRACT_FIELD_FORMS = {
    ("src/core/src/nodes.rs", "Var.kind"): "Option<KindAnnotation>",
    ("src/core/src/nodes.rs", "Binding.kind"): "Option<KindAnnotation>",
    ("src/core/src/nodes.rs", "Field.kind"): "Option<KindAnnotation>",
    ("src/core/src/nodes.rs", "KindDefine.kind"): "KindAnnotation",
    ("src/core/src/nodes.rs", "EnumVariant.value"): "Option<KindAnnotation>",
    ("src/core/src/nodes.rs", "Fsm.kind"): "Option<KindAnnotation>",
    ("src/core/src/nodes.rs", "FunctionArgument.kind"): "KindAnnotation",
    ("src/core/src/nodes.rs", "FunctionDefine.input"): "Vec<FunctionArgument>",
    ("src/core/src/nodes.rs", "FunctionDefine.output"): "Vec<FunctionArgument>",
    ("src/core/src/nodes.rs", "FsmSpecification.input"): "Vec<Var>",
    ("src/core/src/nodes.rs", "FsmSpecification.output"): "Option<KindAnnotation>",
}
RUNTIME_REPRESENTATION_FORBIDDEN_TYPES = {
    "KindScheme",
    "InputKindScheme",
    "KindParameter",
    "KindConstraint",
    "KindExpr",
    "DimensionParameter",
    "DimensionExpr",
}


def declaration_form(source: str, name: str) -> str | None:
    match = re.search(
        rf"\b(?:pub\s+)?(?P<form>struct|enum|trait|type)\s+{re.escape(name)}\b",
        mask_non_code(source),
    )
    return match.group("form") if match is not None else None


def declaration_source(source: str, name: str) -> str:
    searchable = mask_non_code(source)
    match = re.search(
        rf"\b(?:pub\s+)?(?:struct|enum|trait|type)\s+{re.escape(name)}\b",
        searchable,
    )
    if match is None:
        return ""
    opening = searchable.find("{", match.end())
    semicolon = searchable.find(";", match.end())
    if opening >= 0 and (semicolon < 0 or opening < semicolon):
        try:
            return searchable[match.start() : matching_delimiter(searchable, opening) + 1]
        except ValueError:
            return searchable[match.start() :]
    return searchable[match.start() : semicolon + 1] if semicolon >= 0 else searchable[match.start() :]


def field_type(source: str, owner: str, field: str) -> str | None:
    searchable = mask_non_code(source)
    declaration = re.search(
        rf"\b(?:pub\s+)?struct\s+{re.escape(owner)}\b", searchable
    )
    if declaration is None:
        return None
    opening = searchable.find("{", declaration.end())
    if opening < 0:
        return None
    try:
        closing = matching_delimiter(searchable, opening)
    except ValueError:
        return None
    body = searchable[opening + 1 : closing]
    match = re.search(
        rf"\b(?:pub(?:\([^)]*\))?\s+)?{re.escape(field)}\s*:\s*",
        body,
    )
    if match is None:
        return None
    start = match.end()
    angle = bracket = paren = 0
    end = start
    while end < len(body):
        value = body[end]
        if value == "<":
            angle += 1
        elif value == ">" and angle:
            angle -= 1
        elif value == "[":
            bracket += 1
        elif value == "]" and bracket:
            bracket -= 1
        elif value == "(":
            paren += 1
        elif value == ")" and paren:
            paren -= 1
        elif value == "," and angle == 0 and bracket == 0 and paren == 0:
            break
        end += 1
    return re.sub(r"\s+", "", body[start:end])


def expected_type_contract_sources() -> dict[str, list[dict[str, str]]]:
    inventory: dict[str, list[dict[str, str]]] = {}
    for group, specs in TYPE_CONTRACT_SOURCE_SPECS.items():
        target, gate = TYPE_CONTRACT_TARGETS[group]
        records: list[dict[str, str]] = []
        for relative, symbol in specs:
            if "." in symbol:
                expected_form = TYPE_CONTRACT_FIELD_FORMS[(relative, symbol)]
                source_kind = "field"
                source_form = f"field:{expected_form}"
            else:
                expected_form = TYPE_CONTRACT_DECLARATION_FORMS[(relative, symbol)]
                source_kind = "declaration"
                source_form = expected_form
            records.append(
                {
                    "path": relative,
                    "symbol": symbol,
                    "source_kind": source_kind,
                    "source_form": source_form,
                    "target": target,
                    "implementation_gate": gate,
                }
            )
        inventory[group] = records
    return inventory


def type_contract_sources(root: Path) -> dict[str, list[dict[str, str]]]:
    expected = expected_type_contract_sources()
    for group, records in expected.items():
        for record in records:
            relative = record["path"]
            symbol = record["symbol"]
            path = root / relative
            if not path.is_file():
                raise TypeContractError(
                    f"type contract source is missing: {relative}::{symbol}"
                )
            source = path.read_text(encoding="utf-8")
            if record["source_kind"] == "field":
                owner, field = symbol.split(".", 1)
                actual_form = field_type(source, owner, field)
                expected_form = TYPE_CONTRACT_FIELD_FORMS[(relative, symbol)]
            else:
                actual_form = declaration_form(source, symbol)
                expected_form = TYPE_CONTRACT_DECLARATION_FORMS[(relative, symbol)]
            if actual_form != expected_form:
                raise TypeContractError(
                    f"type contract source shape changed: {relative}::{symbol}; "
                    f"expected {expected_form}, found {actual_form}"
                )
            if group == "runtime_representation_sources":
                declaration = declaration_source(source, symbol)
                forbidden = sorted(
                    name
                    for name in RUNTIME_REPRESENTATION_FORBIDDEN_TYPES
                    if re.search(rf"\b{re.escape(name)}\b", declaration)
                )
                if forbidden:
                    raise TypeContractError(
                        f"runtime representation source crosses into semantic typing: "
                        f"{relative}::{symbol} references {forbidden}"
                    )
    return expected


def generate(
    root: Path,
    reference_commit: str = REFERENCE_COMMIT,
    *,
    target_roots: Iterable[Path] | None = None,
    validate_type_contract_sources: bool = True,
) -> dict[str, object]:
    digest = legacy_scanner_implementation_sha256()
    if digest != LEGACY_SCANNER_CONTRACT["implementation_sha256"]:
        raise ValueError(
            "C0-LEGACY-SCANNER-DRIFT: complete scanner module digest "
            f"{digest} != {LEGACY_SCANNER_CONTRACT['implementation_sha256']}"
        )
    value_path = root / "src/core/src/value.rs"
    kind_path = root / "src/core/src/kind.rs"
    if not value_path.is_file() or not kind_path.is_file():
        raise ValueError("src/core/src/value.rs and src/core/src/kind.rs are required")
    value_source = value_path.read_text(encoding="utf-8")
    kind_source = kind_path.read_text(encoding="utf-8")
    legacy_enum_spelling = (
        "LegacyValue"
        if re.search(r"\benum\s+LegacyValue\b", mask_non_code(value_source))
        else "Value"
    )
    value_variants = parse_enum(value_source, legacy_enum_spelling, value=True)
    value_kind_variants = parse_enum(value_source, "ValueKind", value=False)
    kind_variants = parse_enum(kind_source, "Kind", value=False)
    enum_records = {
        "LegacyValue": {
            "source": "src/core/src/value.rs",
            "variants": value_variants,
        },
        "ValueKind": {
            "source": "src/core/src/value.rs",
            "variants": value_kind_variants,
        },
        "Kind": {
            "source": "src/core/src/kind.rs",
            "variants": kind_variants,
        },
    }
    variant_names = {
        enum_name: {str(variant["name"]) for variant in record["variants"]}
        for enum_name, record in enum_records.items()
    }

    (
        production,
        auxiliary_fixtures,
        workspace_packages,
        auxiliary_cargo_fixtures,
        enumerated_files,
    ) = production_inventory_details(root, target_roots)
    corpus: list[tuple[str, str, str, list[Token]]] = []
    for path in production:
        source = path.read_text(encoding="utf-8")
        searchable = mask_non_code(production_source(source))
        corpus.append(
            (
                path.relative_to(root).as_posix(),
                source,
                searchable,
                rust_tokens(source, searchable),
            )
        )

    uses = sorted(
        (
            record
            for relative, _source, _searchable, tokens in corpus
            for record in variant_uses(
                relative,
                tokens,
                variant_names,
                crate_root_bindings(root, root / relative, tokens),
            )
        ),
        key=lambda record: (
            str(record["enum"]),
            str(record["variant"]),
            str(record["path"]),
            int(record["line"]),
            int(record["column"]),
        ),
    )

    identity_ids = (
        "reactive-cell-id",
        "ref-as-ptr-definition",
        "ref-as-mut-ptr-definition",
        "ref-addr-definition",
        "ref-id-definition",
        "ref-as-ptr-ufcs",
        "ref-as-mut-ptr-ufcs",
        "ref-addr-ufcs",
        "ref-id-ufcs",
    )
    journal_ids = (
        "value-state-journal",
        "reactive-turn-journal",
        "transaction-state-values-api",
    )
    legacy_aliases, compatibility_aliases = aliases(corpus)
    compatibility_aliases = [
        record
        for record in compatibility_aliases
        if record.get("visibility") == "pub"
        and bool(record.get("public_reexport_route"))
    ]
    return {
        "schema_version": 6,
        "reference_commit": reference_commit,
        "workspace_packages": workspace_packages,
        "enumerated_rust_files": [
            path.relative_to(root).as_posix() for path in enumerated_files
        ],
        "audited_rust_files": [
            path.relative_to(root).as_posix() for path in production
        ],
        "enums": enum_records,
        "snapshot_types": {
            "Value": {
                "source": "src/core/src/snapshot/mod.rs",
                "category": "immutable-snapshot",
            }
        },
        "variant_uses": uses,
        "type_contract_sources": (
            type_contract_sources(root)
            if validate_type_contract_sources
            else expected_type_contract_sources()
        ),
        "auxiliary_rust_fixtures": auxiliary_fixtures,
        "auxiliary_cargo_fixtures": auxiliary_cargo_fixtures,
        "legacy_aliases": legacy_aliases,
        "required_compatibility_aliases": compatibility_aliases,
        "raw_approved_aliases": LEGACY_SCANNER.raw_approved_aliases(
            legacy_aliases, compatibility_aliases
        ),
        "identity_mechanisms": mechanism_records(corpus, identity_ids),
        "journal_mechanisms": mechanism_records(corpus, journal_ids),
        "high_risk_api_uses": {
            identifier: high_risk_uses(corpus, identifier)
            for identifier in sorted(HIGH_RISK_PATTERNS)
        },
    }


def render(payload: dict[str, object]) -> str:
    return json.dumps(payload, indent=2, sort_keys=True) + "\n"


def display_path(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return str(path)


def legacy_baseline(inventory: dict[str, object], reference: str) -> dict[str, object]:
    return {
        "schema_version": 4,
        "reference_commit": reference,
        "scanner_contract": LEGACY_SCANNER_CONTRACT,
        "legacy_aliases": inventory["legacy_aliases"],
        "required_compatibility_aliases": inventory[
            "required_compatibility_aliases"
        ],
        "high_risk_api_uses": inventory["high_risk_api_uses"],
    }


def archived_inventory(root: Path, reference: str) -> dict[str, object]:
    digest = legacy_scanner_implementation_sha256()
    if digest != LEGACY_SCANNER_CONTRACT["implementation_sha256"]:
        raise ValueError(
            "C0-LEGACY-SCANNER-DRIFT: complete scanner module digest "
            f"{digest} != {LEGACY_SCANNER_CONTRACT['implementation_sha256']}"
        )
    process = subprocess.run(
        ["git", "archive", "--format=tar", reference],
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if process.returncode != 0:
        raise ValueError(
            "cannot archive frozen reference: "
            + process.stderr.decode("utf-8", errors="replace").strip()
        )
    with tempfile.TemporaryDirectory() as temporary:
        destination = Path(temporary).resolve()
        with tarfile.open(fileobj=io.BytesIO(process.stdout), mode="r:") as archive:
            for member in archive.getmembers():
                candidate = (destination / member.name).resolve()
                if destination != candidate and destination not in candidate.parents:
                    raise ValueError("frozen reference archive contains an unsafe path")
            archive.extractall(destination)
        return generate(destination, reference)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=REPOSITORY_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--reference-commit", default=REFERENCE_COMMIT)
    parser.add_argument("--git-ref")
    parser.add_argument("--legacy-baseline-output", type=Path)
    parser.add_argument("--check-legacy-baseline", action="store_true")
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    output = args.output
    if not output.is_absolute():
        output = root / output
    try:
        if args.git_ref or args.legacy_baseline_output or args.check_legacy_baseline:
            reference = args.git_ref or REFERENCE_COMMIT
            baseline_output = args.legacy_baseline_output or DEFAULT_LEGACY_BASELINE_OUTPUT
            if not baseline_output.is_absolute():
                baseline_output = root / baseline_output
            rendered = render(
                legacy_baseline(archived_inventory(root, reference), reference)
            )
            if args.check_legacy_baseline:
                if baseline_output.read_text(encoding="utf-8") != rendered:
                    print(
                        "legacy-growth baseline is stale: regenerate "
                        f"{display_path(baseline_output, root)}",
                        file=sys.stderr,
                    )
                    return 1
                print("legacy-growth baseline is current")
                return 0
            baseline_output.parent.mkdir(parents=True, exist_ok=True)
            baseline_output.write_text(rendered, encoding="utf-8")
            print(f"wrote {display_path(baseline_output, root)}")
            return 0
        rendered = render(generate(root, args.reference_commit))
        if args.check:
            actual = output.read_text(encoding="utf-8")
            if actual != rendered:
                print(
                    "value-system inventory is stale: regenerate "
                    f"{display_path(output, root)}",
                    file=sys.stderr,
                )
                return 1
            print("value-system inventory is current")
            return 0
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
        print(f"wrote {display_path(output, root)}")
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"value-system inventory generation failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
