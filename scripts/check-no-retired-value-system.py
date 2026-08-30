#!/usr/bin/env python3
"""Fail if the retired universal value model reappears in executable Rust."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SURFACE_PATH = (
    ROOT / "tests/architecture/value-system/retired-public-surface-v1.json"
)
SKIPPED_PARTS = {".git", "target", "node_modules"}


def retired_surface(root: Path) -> dict:
    path = root / SURFACE_PATH.relative_to(ROOT)
    return json.loads(path.read_text(encoding="utf-8"))


def rust_sources(root: Path):
    for path in root.rglob("*.rs"):
        if not SKIPPED_PARTS.isdisjoint(path.parts):
            continue
        yield path


INCLUDE = re.compile(r"\binclude\s*!\s*\(")
STATIC_INCLUDE_ARGUMENT = re.compile(
    r'\s*"(?P<path>[A-Za-z0-9_./-]+)"\s*\)',
)
CHAR_LITERAL = re.compile(
    r"(?:b)?'(?:\\(?:[nrt0\\'\"]|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]{1,6}\})|[^\\'\r\n])'"
)


def mask_non_code(source: str) -> str:
    """Mask Rust comments and literals while retaining line offsets."""
    masked = list(source)

    def blank(start: int, end: int) -> None:
        for index in range(start, end):
            if masked[index] != "\n":
                masked[index] = " "

    index = 0
    while index < len(source):
        if source.startswith("//", index):
            end = source.find("\n", index + 2)
            end = len(source) if end < 0 else end
            blank(index, end)
            index = end
            continue

        if source.startswith("/*", index):
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
            blank(index, end)
            index = end
            continue

        character = CHAR_LITERAL.match(source, index)
        if character is not None:
            blank(index, character.end())
            index = character.end()
            continue

        raw = re.match(r"(?:br|rb|r)(?P<hashes>#{0,255})\"", source[index:])
        if raw is not None:
            terminator = '"' + raw.group("hashes")
            content = index + raw.end()
            close = source.find(terminator, content)
            end = len(source) if close < 0 else close + len(terminator)
            blank(index, end)
            index = end
            continue

        prefix = 2 if source.startswith(('b"', 'c"'), index) else 1
        if source[index] == '"' or prefix == 2:
            end = index + prefix
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
            blank(index, end)
            index = end
            continue

        index += 1

    return "".join(masked)


def normalize_raw_identifiers(source: str) -> str:
    """Normalize Rust's ``r#name`` spelling without changing byte offsets."""
    return re.sub(r"\br#(?=[A-Za-z_][A-Za-z0-9_]*)", "  ", source)


def executable_rust_sources(root: Path) -> tuple[list[Path], list[str]]:
    """Return the closed source set, following literal ``include!`` targets."""
    root = root.resolve()
    pending = list(rust_sources(root))
    found: list[Path] = []
    seen: set[Path] = set()
    failures: list[str] = []
    while pending:
        path = pending.pop()
        resolved = path.resolve()
        if resolved in seen:
            continue
        seen.add(resolved)
        found.append(path)
        source = path.read_text(encoding="utf-8")
        masked = mask_non_code(source)
        relative = path.relative_to(root).as_posix()
        for include in INCLUDE.finditer(masked):
            argument = STATIC_INCLUDE_ARGUMENT.match(source, include.end())
            line = source.count("\n", 0, include.start()) + 1
            if argument is None:
                failures.append(
                    f"{relative}:{line}: executable include! target is not a static path"
                )
                continue
            included = (path.parent / argument.group("path")).resolve()
            try:
                included.relative_to(root)
            except ValueError:
                failures.append(
                    f"{relative}:{line}: executable include! target escapes repository: "
                    f"{argument.group('path')}"
                )
                continue
            if not included.is_file():
                failures.append(
                    f"{relative}:{line}: executable include! target is missing: "
                    f"{argument.group('path')}"
                )
                continue
            pending.append(included)
    return found, failures


def failures(root: Path) -> list[str]:
    root = root.resolve()
    failures: list[str] = []
    surface = retired_surface(root)
    for relative in surface["forbidden_paths"]:
        if (root / relative).exists():
            failures.append(f"retired path still exists: {relative}")

    symbols = re.compile(
        r"\b(?:" + "|".join(map(re.escape, surface["retired_symbols"])) + r")\b"
    )
    conversions = re.compile(
        r"\b(?:"
        + "|".join(map(re.escape, surface["retired_conversions"]))
        + r")\b"
    )
    retired_module = re.compile(
        r"\b(?:pub\s*(?:\([^)]*\))?\s+)?mod\s+(?:r#)?(?:"
        + "|".join(map(re.escape, surface["forbidden_modules"]))
        + r")\s*(?:;|\{)"
    )
    declarations = [
        (
            re.compile(entry["pattern"]),
            entry["label"],
            set(entry.get("allowed_paths", [])),
        )
        for entry in surface["retired_declarations"]
    ]
    for entry in surface["retained_declarations"]:
        relative = entry["path"]
        path = root / relative
        if not path.is_file():
            failures.append(
                f"retained canonical declaration is missing: {entry['symbol']} ({relative})"
            )
            continue
        source = normalize_raw_identifiers(
            mask_non_code(path.read_text(encoding="utf-8"))
        )
        if re.search(entry["pattern"], source) is None:
            failures.append(
                f"retained canonical declaration is missing: {entry['symbol']} ({relative})"
            )
    sources, include_failures = executable_rust_sources(root)
    failures.extend(include_failures)
    for path in sources:
        source = normalize_raw_identifiers(
            mask_non_code(path.read_text(encoding="utf-8"))
        )
        relative = path.relative_to(root).as_posix()
        for pattern, label in (
            (symbols, "retired symbol"),
            (conversions, "retired conversion"),
            (retired_module, "retired module declaration"),
        ):
            for match in pattern.finditer(source):
                line = source.count("\n", 0, match.start()) + 1
                failures.append(f"{relative}:{line}: {label}: {match.group(0)}")
        for pattern, label, allowed_paths in declarations:
            if relative in allowed_paths:
                continue
            for match in pattern.finditer(source):
                line = source.count("\n", 0, match.start()) + 1
                failures.append(f"{relative}:{line}: {label}: {match.group(0)}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    found = failures(args.root.resolve())
    if not found:
        print("retired value-system absence contract passed")
        return 0
    print("retired value-system absence contract failed:", file=sys.stderr)
    for item in found:
        print(f"  {item}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
