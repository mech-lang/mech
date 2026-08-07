#!/usr/bin/env python3
"""Reject unapproved growth of legacy value/turn execution dependencies."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = (
    REPOSITORY_ROOT / "tests/architecture/value-execution/legacy-boundary.json"
)
SOURCE_ROOTS = ("src", "machines", "hosts")
IGNORED_COMPONENTS = {".git", "target", "tests", "benches", "examples", "fixtures"}
CFG_ATTRIBUTE = re.compile(
    r"#\s*\[\s*cfg\s*\((?P<expression>[^\]]*?)\)\s*\]",
    re.MULTILINE,
)
FOLLOWING_ATTRIBUTE = re.compile(r"\s*#\s*\[[^\]]*\]", re.MULTILINE)
CFG_TOKEN = re.compile(
    r'\s*(?:(?P<identifier>[A-Za-z_][A-Za-z0-9_]*)|'
    r'(?P<string>"(?:\\.|[^"\\])*")|(?P<punctuation>[(),=]))'
)


@dataclass(frozen=True, order=True)
class Occurrence:
    path: str
    offset: int
    line: int
    text: str


def ignored(path: Path) -> bool:
    return any(
        part in IGNORED_COMPONENTS
        or part.endswith("_tests")
        or part == "tests.rs"
        or part.endswith("_tests.rs")
        for part in path.parts
    )


def production_files(root: Path) -> Iterable[Path]:
    files: list[Path] = []
    for source_root in SOURCE_ROOTS:
        base = root / source_root
        if not base.exists():
            continue
        files.extend(
            path
            for path in base.rglob("*.rs")
            if not ignored(path.relative_to(root))
        )
    return sorted(files, key=lambda path: path.relative_to(root).as_posix())


def matching_brace_end(source: str, opening: int) -> int | None:
    """Find a Rust block end while ignoring braces in comments and strings."""
    depth = 0
    offset = opening
    while offset < len(source):
        if source.startswith("//", offset):
            newline = source.find("\n", offset + 2)
            offset = len(source) if newline < 0 else newline + 1
            continue
        if source.startswith("/*", offset):
            comment_depth = 1
            offset += 2
            while offset < len(source) and comment_depth:
                if source.startswith("/*", offset):
                    comment_depth += 1
                    offset += 2
                elif source.startswith("*/", offset):
                    comment_depth -= 1
                    offset += 2
                else:
                    offset += 1
            continue
        if source[offset] == '"':
            offset += 1
            while offset < len(source):
                if source[offset] == "\\":
                    offset += 2
                elif source[offset] == '"':
                    offset += 1
                    break
                else:
                    offset += 1
            continue
        if source[offset] in {"r", "b"}:
            raw_start = offset + (1 if source[offset] == "r" else 2)
            if source[offset] == "b" and not source.startswith("br", offset):
                raw_start = -1
            if raw_start >= 0:
                quote = raw_start
                while quote < len(source) and source[quote] == "#":
                    quote += 1
                if quote < len(source) and source[quote] == '"':
                    hashes = source[raw_start:quote]
                    closing = source.find('"' + hashes, quote + 1)
                    offset = len(source) if closing < 0 else closing + 1 + len(hashes)
                    continue
        if source[offset] == "'":
            closing = offset + 2
            if offset + 1 < len(source) and source[offset + 1] == "\\":
                closing = offset + 3
            if closing < len(source) and source[closing] == "'":
                offset = closing + 1
                continue
        if source[offset] == "{":
            depth += 1
        elif source[offset] == "}":
            depth -= 1
            if depth == 0:
                return offset + 1
        offset += 1
    return None


def cfg_tokens(expression: str) -> list[tuple[str, str]] | None:
    tokens: list[tuple[str, str]] = []
    offset = 0
    while offset < len(expression):
        if not expression[offset:].strip():
            break
        match = CFG_TOKEN.match(expression, offset)
        if match is None:
            return None
        if match.lastgroup is None:
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
            raise ValueError("trailing cfg expression tokens")
        return possible

    def parse_predicate(self) -> frozenset[bool]:
        if self.offset >= len(self.tokens):
            raise ValueError("missing cfg predicate")
        kind, name = self.tokens[self.offset]
        if kind != "identifier":
            raise ValueError("cfg predicate must begin with an identifier")
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
                    if name == "all":
                        possible = {
                            left and right for left in possible for right in argument
                        }
                    else:
                        possible = {
                            left or right for left in possible for right in argument
                        }
                return frozenset(possible)
            return self.UNKNOWN

        if self.accept("="):
            if self.offset >= len(self.tokens):
                raise ValueError("missing cfg predicate value")
            value_kind, _value = self.tokens[self.offset]
            if value_kind not in {"identifier", "string"}:
                raise ValueError("invalid cfg predicate value")
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
        possible_without_test = CfgExpressionParser(tokens).parse()
    except ValueError:
        return False
    return True not in possible_without_test


def cfg_item_end(source: str, start: int) -> int | None:
    """Find the end of the Rust item immediately following a cfg attribute."""
    offset = start
    while True:
        attribute = FOLLOWING_ATTRIBUTE.match(source, offset)
        if attribute is None:
            break
        offset = attribute.end()

    parentheses = 0
    brackets = 0
    while offset < len(source):
        if source.startswith("//", offset):
            newline = source.find("\n", offset + 2)
            offset = len(source) if newline < 0 else newline + 1
            continue
        if source.startswith("/*", offset):
            comment_depth = 1
            offset += 2
            while offset < len(source) and comment_depth:
                if source.startswith("/*", offset):
                    comment_depth += 1
                    offset += 2
                elif source.startswith("*/", offset):
                    comment_depth -= 1
                    offset += 2
                else:
                    offset += 1
            continue
        if source[offset] == '"':
            offset += 1
            while offset < len(source):
                if source[offset] == "\\":
                    offset += 2
                elif source[offset] == '"':
                    offset += 1
                    break
                else:
                    offset += 1
            continue
        character = source[offset]
        if character == "(":
            parentheses += 1
        elif character == ")":
            parentheses = max(0, parentheses - 1)
        elif character == "[":
            brackets += 1
        elif character == "]":
            brackets = max(0, brackets - 1)
        elif parentheses == 0 and brackets == 0:
            if character == ";":
                return offset + 1
            if character == "{":
                return matching_brace_end(source, offset)
        offset += 1
    return None


def production_source(source: str) -> str:
    """Mask Rust items whose cfg cannot be true without `test`."""
    spans: list[tuple[int, int]] = []
    start = 0
    while True:
        match = CFG_ATTRIBUTE.search(source, start)
        if match is None:
            break
        if not cfg_requires_test(match.group("expression")):
            start = match.end()
            continue
        end = cfg_item_end(source, match.end())
        if end is None:
            start = match.end()
            continue
        spans.append((match.start(), end))
        start = end
    if not spans:
        return source
    masked = list(source)
    for span_start, span_end in spans:
        for offset in range(span_start, span_end):
            if masked[offset] != "\n":
                masked[offset] = " "
    return "".join(masked)


def occurrences(root: Path, pattern: str) -> list[Occurrence]:
    found: list[Occurrence] = []
    for path in production_files(root):
        source = path.read_text(encoding="utf-8")
        searchable = production_source(source)
        start = 0
        while True:
            offset = searchable.find(pattern, start)
            if offset < 0:
                break
            line = source.count("\n", 0, offset) + 1
            line_start = source.rfind("\n", 0, offset) + 1
            line_end = source.find("\n", offset)
            if line_end < 0:
                line_end = len(source)
            found.append(
                Occurrence(
                    path.relative_to(root).as_posix(),
                    offset,
                    line,
                    source[line_start:line_end].strip(),
                )
            )
            start = offset + len(pattern)
    return found


def brace_span(source: str, anchor: int) -> tuple[int, int] | None:
    opening = source.find("{", anchor)
    if opening < 0:
        return None
    semicolon = source.find(";", anchor, opening)
    if semicolon >= 0:
        return None
    depth = 0
    for offset in range(opening, len(source)):
        character = source[offset]
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
            if depth == 0:
                return (anchor, offset + 1)
    return None


def allowed_spans(root: Path, approval: dict[str, Any]) -> list[tuple[int, int]]:
    path = root / approval["path"]
    if not path.is_file():
        return []
    source = path.read_text(encoding="utf-8")
    scope = approval["scope_contains"]
    if scope == "<file>":
        return [(0, len(source))]
    spans: list[tuple[int, int]] = []
    start = 0
    while True:
        anchor = source.find(scope, start)
        if anchor < 0:
            break
        span = brace_span(source, anchor)
        if span is not None:
            spans.append(span)
        start = anchor + len(scope)
    return spans


def validate_manifest(payload: dict[str, Any]) -> None:
    if payload.get("schema_version") != 1:
        raise ValueError("legacy boundary manifest schema_version must be 1")
    identifiers: set[str] = set()
    for boundary in payload.get("boundaries", []):
        identifier = boundary.get("id")
        if not identifier or identifier in identifiers:
            raise ValueError(f"missing or duplicate boundary id: {identifier!r}")
        identifiers.add(identifier)
        if not boundary.get("pattern") or not boundary.get("description"):
            raise ValueError(f"{identifier}: pattern and description are required")
        if "allowed" not in boundary or not isinstance(boundary["allowed"], list):
            raise ValueError(f"{identifier}: allowed must be a list (empty means zero use)")
        for approval in boundary["allowed"]:
            required = {"path", "scope_contains", "max_occurrences"}
            if not required.issubset(approval):
                raise ValueError(f"{identifier}: incomplete approval {approval!r}")
            if not isinstance(approval["max_occurrences"], int) or approval["max_occurrences"] < 0:
                raise ValueError(f"{identifier}: max_occurrences must be non-negative")


def audit(root: Path, manifest: Path) -> list[str]:
    payload = json.loads(manifest.read_text(encoding="utf-8"))
    validate_manifest(payload)
    failures: list[str] = []
    for boundary in payload["boundaries"]:
        identifier = boundary["id"]
        actual = occurrences(root, boundary["pattern"])
        approved_occurrences: set[Occurrence] = set()
        for approval in boundary["allowed"]:
            spans = allowed_spans(root, approval)
            scoped = [
                occurrence
                for occurrence in actual
                if occurrence.path == approval["path"]
                and any(start <= occurrence.offset < end for start, end in spans)
            ]
            approved_occurrences.update(scoped)
            if len(scoped) > approval["max_occurrences"]:
                failures.append(
                    f"{identifier}: {approval['path']} scope "
                    f"{approval['scope_contains']!r} has {len(scoped)} occurrences; "
                    f"maximum is {approval['max_occurrences']}"
                )
        for occurrence in actual:
            if occurrence not in approved_occurrences:
                failures.append(
                    f"{identifier}: unapproved occurrence at "
                    f"{occurrence.path}:{occurrence.line}: {occurrence.text}"
                )
    return sorted(failures)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=REPOSITORY_ROOT)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    manifest = args.manifest.resolve()
    try:
        failures = audit(root, manifest)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"value execution boundary audit failed internally: {error}", file=sys.stderr)
        return 2
    if failures:
        print("value execution boundary audit failed:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1
    print("value execution boundary audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
