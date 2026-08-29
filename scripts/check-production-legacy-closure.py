#!/usr/bin/env python3
"""Reject legacy value machinery outside the explicit compatibility boundary."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE_ROOTS = ("src", "machines", "hosts")
IGNORED_PARTS = {"target", "tests", "benches", "examples", "fixtures"}
ALLOWED_FILES = {
    Path("src/core/src/value.rs"),
    Path("src/core/src/kind.rs"),
}
ALLOWED_PREFIX = Path("src/core/src/legacy_adapter")

PROHIBITED = (
    ("LegacyValue", re.compile(r"\bLegacyValue\b")),
    ("ValueKind", re.compile(r"\bValueKind\b")),
    ("MutableReference", re.compile(r"\bMutableReference\b")),
    ("snapshot_from_legacy", re.compile(r"\bsnapshot_from_legacy\b")),
    ("try_deep_snapshot", re.compile(r"\btry_deep_snapshot\b")),
    ("legacy materialization", re.compile(r"\bmaterialize[A-Za-z0-9_]*legacy\b")),
    ("canonical-to-legacy conversion", re.compile(r"\blegacy_value_from_canonical\b")),
    ("legacy function invocation", re.compile(r"\bfunction_invocation_from_legacy\b")),
    ("legacy source invocation", re.compile(r"\bspecialization_invocation_from_legacy\b")),
    ("FunctionArgs construction", re.compile(r"\bFunctionArgs\s*::")),
)

LEGACY_AGGREGATE_PORT_BACKINGS = (
    (
        "legacy aggregate function-port backing",
        re.compile(
            r"impl(?:\s*<[^{};]*>)?\s+function_port_backing::Sealed\s+for\s+"
            r"(?:crate::)?(?:Mech(?:Atom|Enum|Record|Map|Set|Table|Tuple)|Matrix\s*<)"
        ),
    ),
    (
        "legacy aggregate state-port backing",
        re.compile(
            r"impl(?:\s*<[^{};]*>)?\s+function_state_sealed::PortSealed\s+for\s+"
            r"(?:crate::)?(?:Mech(?:Atom|Enum|Record|Map|Set|Table|Tuple)|Matrix\s*<)"
        ),
    ),
)

CFG_TEST_ITEM = re.compile(
    r"#\s*\[\s*cfg\s*\([^\]]*\btest\b[^\]]*\)\s*\]",
    re.MULTILINE,
)


def matching_brace(source: str, opening: int) -> int | None:
    depth = 0
    offset = opening
    state = "code"
    raw_hashes = 0
    while offset < len(source):
        if state == "line":
            if source[offset] == "\n":
                state = "code"
            offset += 1
            continue
        if state == "block":
            if source.startswith("*/", offset):
                state = "code"
                offset += 2
            else:
                offset += 1
            continue
        if state == "string":
            if source[offset] == "\\":
                offset += 2
            elif source[offset] == '"':
                state = "code"
                offset += 1
            else:
                offset += 1
            continue
        if state == "raw":
            closing = '"' + ("#" * raw_hashes)
            if source.startswith(closing, offset):
                state = "code"
                offset += len(closing)
            else:
                offset += 1
            continue
        if source.startswith("//", offset):
            state = "line"
            offset += 2
        elif source.startswith("/*", offset):
            state = "block"
            offset += 2
        elif source[offset] == '"':
            state = "string"
            offset += 1
        elif source[offset] == "r":
            raw = re.match(r'r(#+)?"', source[offset:])
            if raw:
                raw_hashes = len(raw.group(1) or "")
                state = "raw"
                offset += len(raw.group(0))
            else:
                offset += 1
        elif source[offset] == "{":
            depth += 1
            offset += 1
        elif source[offset] == "}":
            depth -= 1
            offset += 1
            if depth == 0:
                return offset
        else:
            offset += 1
    return None


def mask_test_modules(source: str) -> str:
    masked = list(source)
    offset = 0
    while match := CFG_TEST_ITEM.search(source, offset):
        opening = source.find("{", match.end())
        semicolon = source.find(";", match.end())
        if semicolon >= 0 and (opening < 0 or semicolon < opening):
            end = semicolon + 1
        elif opening >= 0:
            end = matching_brace(source, opening)
            if end is None:
                offset = match.end()
                continue
        else:
            break
        for index in range(match.start(), end):
            if masked[index] != "\n":
                masked[index] = " "
        offset = end
    return "".join(masked)


def mask_comments_and_literals(source: str) -> str:
    masked = list(source)
    offset = 0
    while offset < len(source):
        if source.startswith("//", offset):
            end = source.find("\n", offset + 2)
            end = len(source) if end < 0 else end
        elif source.startswith("/*", offset):
            end = source.find("*/", offset + 2)
            end = len(source) if end < 0 else end + 2
        elif source[offset] == '"':
            end = offset + 1
            while end < len(source):
                if source[end] == "\\":
                    end += 2
                elif source[end] == '"':
                    end += 1
                    break
                else:
                    end += 1
        elif source[offset] == "r":
            raw = re.match(r'r(#+)?"', source[offset:])
            if raw is None:
                offset += 1
                continue
            closing = '"' + (raw.group(1) or "")
            end = source.find(closing, offset + len(raw.group(0)))
            end = len(source) if end < 0 else end + len(closing)
        else:
            offset += 1
            continue
        for index in range(offset, end):
            if masked[index] != "\n":
                masked[index] = " "
        offset = end
    return "".join(masked)


def ignored(relative: Path) -> bool:
    return (
        relative in ALLOWED_FILES
        or relative.is_relative_to(ALLOWED_PREFIX)
        or any(part in IGNORED_PARTS or part.endswith("_tests") for part in relative.parts)
        or relative.name == "tests.rs"
        or relative.name.endswith("_tests.rs")
        or relative.name == "port_tests.rs"
    )


def production_files() -> list[Path]:
    result: list[Path] = []
    for root in SOURCE_ROOTS:
        base = ROOT / root
        if not base.exists():
            continue
        for path in base.rglob("*.rs"):
            relative = path.relative_to(ROOT)
            if not ignored(relative):
                result.append(path)
    return sorted(result)


def audit() -> list[str]:
    failures: list[str] = []
    for path in production_files():
        source = path.read_text(encoding="utf-8")
        searchable = mask_comments_and_literals(mask_test_modules(source))
        relative = path.relative_to(ROOT)
        for label, pattern in PROHIBITED + LEGACY_AGGREGATE_PORT_BACKINGS:
            for match in pattern.finditer(searchable):
                line = source.count("\n", 0, match.start()) + 1
                text = source.splitlines()[line - 1].strip()
                failures.append(f"{relative}:{line}: {label}: {text}")
    return failures


def main() -> int:
    failures = audit()
    if failures:
        print("production legacy closure check failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("production legacy closure check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
