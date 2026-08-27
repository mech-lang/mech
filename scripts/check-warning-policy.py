#!/usr/bin/env python3
"""Enforce warnings-as-errors and audit every intentional lint exception."""

import json
import os
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(
    os.environ.get("WARNING_POLICY_ROOT", Path(__file__).resolve().parents[1])
).resolve()
EXCEPTIONS = ROOT / "scripts/warning-exceptions.json"
LINT_BODY = re.compile(
    r'\s*(?P<lint>[A-Za-z_][A-Za-z0-9_:]*)\s*,\s*reason\s*=\s*"(?P<reason>[^"]+)"\s*'
)
UNSUPPORTED_DYLIB = re.compile(r'^\s*crate-type\s*=\s*\[[^\]]*"dylib"', re.MULTILINE)
RAW_LITERAL = re.compile(r'(?:br|r)(?P<hashes>#{0,255})"')
CHARACTER_LITERAL = re.compile(r"'(?:\\.|[^\\'])'")


def fail(message: str) -> None:
    print(f"warning policy failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def skip_non_code(source: str, offset: int) -> int | None:
    """Return the first byte after a Rust comment or literal at offset."""
    if source.startswith("//", offset):
        newline = source.find("\n", offset + 2)
        return len(source) if newline < 0 else newline + 1
    if source.startswith("/*", offset):
        depth = 1
        cursor = offset + 2
        while cursor < len(source) and depth:
            if source.startswith("/*", cursor):
                depth += 1
                cursor += 2
            elif source.startswith("*/", cursor):
                depth -= 1
                cursor += 2
            else:
                cursor += 1
        return cursor

    raw = RAW_LITERAL.match(source, offset)
    if raw is not None:
        terminator = '"' + raw.group("hashes")
        end = source.find(terminator, raw.end())
        return len(source) if end < 0 else end + len(terminator)

    quote_offset = offset + 1 if source.startswith('b"', offset) else offset
    if quote_offset < len(source) and source[quote_offset] == '"':
        cursor = quote_offset + 1
        while cursor < len(source):
            if source[cursor] == "\\":
                cursor += 2
            elif source[cursor] == '"':
                return cursor + 1
            else:
                cursor += 1
        return len(source)

    character = CHARACTER_LITERAL.match(source, offset)
    if character is not None:
        return character.end()
    return None


def rust_attributes(source: str) -> list[str]:
    """Extract real Rust attributes while ignoring comments and literals."""
    attributes = []
    cursor = 0
    while cursor < len(source):
        skipped = skip_non_code(source, cursor)
        if skipped is not None:
            cursor = skipped
            continue
        if source[cursor] != "#":
            cursor += 1
            continue

        start = cursor
        cursor += 1
        if cursor < len(source) and source[cursor] == "!":
            cursor += 1
        while cursor < len(source) and source[cursor].isspace():
            cursor += 1
        if cursor >= len(source) or source[cursor] != "[":
            continue

        depth = 1
        cursor += 1
        while cursor < len(source) and depth:
            skipped = skip_non_code(source, cursor)
            if skipped is not None:
                cursor = skipped
                continue
            if source[cursor] == "[":
                depth += 1
            elif source[cursor] == "]":
                depth -= 1
            cursor += 1
        attributes.append(source[start:cursor])
    return attributes


def mask_non_code(source: str) -> str:
    """Replace comments and literals with spaces while preserving token offsets."""
    masked = list(source)
    cursor = 0
    while cursor < len(source):
        skipped = skip_non_code(source, cursor)
        if skipped is None:
            cursor += 1
            continue
        masked[cursor:skipped] = " " * (skipped - cursor)
        cursor = skipped
    return "".join(masked)


def split_top_level(source: str) -> list[str]:
    """Split comma-separated Rust meta items without splitting nested delimiters."""
    items = []
    start = 0
    depths = {"(": 0, "[": 0, "{": 0}
    closing = {")": "(", "]": "[", "}": "{"}
    for offset, character in enumerate(source):
        if character in depths:
            depths[character] += 1
        elif character in closing:
            opener = closing[character]
            depths[opener] = max(0, depths[opener] - 1)
        elif character == "," and not any(depths.values()):
            items.append(source[start:offset])
            start = offset + 1
    items.append(source[start:])
    return items


def attribute_meta(attribute: str) -> str | None:
    opening = attribute.find("[")
    closing = attribute.rfind("]")
    if opening < 0 or closing <= opening:
        return None
    return attribute[opening + 1 : closing]


def meta_head(meta: str) -> tuple[str, int] | None:
    """Return a normalized Rust meta-item name and its end offset."""
    masked = mask_non_code(meta)
    head = re.match(
        r"\s*(?:r#)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        masked,
    )
    if head is None:
        return None
    return head.group("name"), head.end()


def meta_call_body(meta: str, head_end: int) -> str | None:
    """Return one structurally balanced meta-item call body."""
    masked = mask_non_code(meta)
    cursor = head_end
    while cursor < len(masked) and masked[cursor].isspace():
        cursor += 1
    if cursor >= len(masked) or masked[cursor] != "(":
        return None
    start = cursor + 1
    depth = 1
    cursor += 1
    while cursor < len(masked) and depth:
        if masked[cursor] == "(":
            depth += 1
        elif masked[cursor] == ")":
            depth -= 1
        cursor += 1
    if depth or masked[cursor:].strip():
        return None
    return meta[start : cursor - 1]


def conditional_policy_directive(attribute: str) -> str | None:
    """Find a warning-policy directive recursively nested inside cfg_attr."""
    outer = attribute_meta(attribute)
    if outer is None:
        return None

    def inspect(meta: str) -> str | None:
        head = meta_head(meta)
        if head is None:
            return "indirect"
        name, head_end = head
        if name in {"allow", "expect", "deprecated"}:
            return name
        if name != "cfg_attr":
            return None
        body = meta_call_body(meta, head_end)
        if body is None:
            return "indirect"
        arguments = split_top_level(mask_non_code(body))
        if len(arguments) < 2:
            return "indirect"
        for nested in arguments[1:]:
            directive = inspect(nested)
            if directive is not None:
                return directive
        return None

    root = meta_head(outer)
    if root is None or root[0] != "cfg_attr":
        return None
    return inspect(outer)


def repository_rust_sources() -> list[Path]:
    """List source candidates without confusing tracked `target` modules with builds."""
    tracked = subprocess.run(
        [
            "git",
            "-C",
            str(ROOT),
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if tracked.returncode == 0:
        return sorted(
            path
            for line in tracked.stdout.splitlines()
            if (path := ROOT / line).is_file()
        )
    return sorted(
        path
        for path in ROOT.rglob("*.rs")
        if not path.is_relative_to(ROOT / "target")
        and not path.is_relative_to(ROOT / ".git")
    )


contracts = json.loads(EXCEPTIONS.read_text(encoding="utf-8"))
if contracts.get("schema_version") != 1:
    fail("warning exception contract has an unsupported schema version")

expected_lints = {}
for exception in contracts.get("lint_exceptions", []):
    required = {
        "directive",
        "expiry_condition",
        "lint",
        "occurrences",
        "owner",
        "path",
        "reason",
    }
    if set(exception) != required:
        fail("lint exception fields do not match the reviewed schema")
    if not all(exception[field] for field in required - {"occurrences"}):
        fail("lint exception contains an empty reviewed field")
    if exception["directive"] not in {"allow", "expect"}:
        fail("lint exception has an invalid directive")
    if not isinstance(exception["occurrences"], int) or exception["occurrences"] < 1:
        fail("lint exception occurrences must be a positive integer")
    key = (
        exception["path"],
        exception["directive"],
        exception["lint"],
        exception["reason"],
    )
    if key in expected_lints:
        fail(f"duplicate lint exception for {exception['path']}")
    expected_lints[key] = exception["occurrences"]

expected_deprecations = {}
for exception in contracts.get("deprecated_apis", []):
    required = {"attribute", "expiry_condition", "occurrences", "owner", "path", "reason"}
    if set(exception) != required:
        fail("deprecation exception fields do not match the reviewed schema")
    if not all(exception[field] for field in required - {"occurrences"}):
        fail("deprecation exception contains an empty reviewed field")
    if not isinstance(exception["occurrences"], int) or exception["occurrences"] < 1:
        fail("deprecation exception occurrences must be a positive integer")
    key = (exception["path"], " ".join(exception["attribute"].split()))
    if key in expected_deprecations:
        fail(f"duplicate deprecation exception for {exception['path']}")
    expected_deprecations[key] = exception["occurrences"]

actual_lints = {}
actual_deprecations = {}
for path in repository_rust_sources():
    source = path.read_text(encoding="utf-8")
    relative = path.relative_to(ROOT).as_posix()
    for attribute in rust_attributes(source):
        conditional = conditional_policy_directive(attribute)
        if conditional is not None:
            if conditional == "indirect":
                fail(
                    f"indirect or unparseable attribute in cfg_attr is not permitted "
                    f"in {relative}"
                )
            fail(
                f"conditional {conditional} in cfg_attr is not permitted in {relative}; "
                "use an unconditional audited exception"
            )
        meta = attribute_meta(attribute)
        if meta is None:
            continue
        head = meta_head(meta)
        if head is None:
            if "$" in mask_non_code(meta):
                fail(f"indirect attribute is not permitted in {relative}")
            continue
        name, head_end = head
        if name in {"allow", "expect"}:
            call_body = meta_call_body(meta, head_end)
            body = LINT_BODY.fullmatch(call_body or "")
            if body is None:
                fail(
                    f"lint exception in {relative} must name one lint and a literal reason"
                )
            key = (relative, name, body.group("lint"), body.group("reason"))
            actual_lints[key] = actual_lints.get(key, 0) + 1
        if name == "deprecated":
            key = (relative, " ".join(attribute.split()))
            actual_deprecations[key] = actual_deprecations.get(key, 0) + 1

if actual_lints != expected_lints:
    fail(
        "reviewed lint exceptions differ from scripts/warning-exceptions.json: "
        f"expected={expected_lints!r} actual={actual_lints!r}"
    )
if actual_deprecations != expected_deprecations:
    fail(
        "reviewed deprecations differ from scripts/warning-exceptions.json: "
        f"expected={expected_deprecations!r} actual={actual_deprecations!r}"
    )

for path in sorted(ROOT.rglob("Cargo.toml")):
    if "target" in path.parts or ".git" in path.parts:
        continue
    manifest = path.read_text(encoding="utf-8")
    if UNSUPPORTED_DYLIB.search(manifest):
        fail(
            f"target-agnostic Rust dylib crate type remains in {path.relative_to(ROOT)}; "
            "it warns on WASM builds"
        )

config = (ROOT / ".cargo/config.toml").read_text(encoding="utf-8")
if 'rustflags = ["-D", "warnings"]' not in config:
    fail("Cargo no longer promotes compiler warnings to build errors")
if 'rustdocflags = ["-D", "warnings"]' not in config:
    fail("Cargo no longer promotes documentation warnings to build errors")

print("warning policy contract passed")
