#!/usr/bin/env python3
"""Reject public or unchecked runtime-factory escape hatches."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCAN_ROOTS = ("src", "machines", "hosts", "tests/fixtures")
FORBIDDEN = (
    ("Value::as_unchecked", re.compile(r"Value::as_unchecked")),
    (
        "unchecked value extraction",
        re.compile(r"\.as_unchecked(?:\s*::\s*<[^>]+>)?\s*\("),
    ),
    (
        "public runtime factory field",
        re.compile(r"pub\s+factory\s*:\s*RuntimeFunctionFactory"),
    ),
    (
        "public raw runtime factory lookup",
        re.compile(r"pub\s+fn\s+runtime_factory\s*\("),
    ),
    (
        "public raw RuntimeFunctionFactory",
        re.compile(r"pub(?:\([^)]*\))?\s+(?:type\s+)?RuntimeFunctionFactory\b"),
    ),
    (
        "forbidden runtime contract escape hatch",
        re.compile(r"RuntimeFunctionContract::(?:unchecked|unknown|infer_from_name|best_effort)\b", re.IGNORECASE),
    ),
)
WHOLE_FILE_FORBIDDEN = (
    (
        "placeholder runtime shape validator",
        re.compile(
            r"RuntimeFunctionContract::custom\s*\(.{0,512}?\|\s*_\s*\|\s*Ok\s*\(\s*\(\s*\)\s*\)",
            re.DOTALL,
        ),
    ),
    (
        "raw factory argument to catalog insertion",
        re.compile(
            r"fn\s+insert_runtime_factory(?:_with_linkage)?\b"
            r"\s*(?:<[^>{}]*>)?\s*"
            r"\([^)]*(?:RuntimeFunctionFactory|\bfactory\s*:)[^)]*\)",
            re.DOTALL,
        ),
    ),
)


def native_declaration_blocks(text: str) -> list[tuple[int, str]]:
    marker = "declare_native_runtime_factory!"
    blocks = []
    search_from = 0
    while True:
        start = text.find(marker, search_from)
        if start < 0:
            return blocks
        opening = text.find("{", start + len(marker))
        if opening < 0:
            raise RuntimeError("unterminated native runtime factory declaration")
        depth = 0
        for index in range(opening, len(text)):
            if text[index] == "{":
                depth += 1
            elif text[index] == "}":
                depth -= 1
                if depth == 0:
                    blocks.append((start, text[start : index + 1]))
                    search_from = index + 1
                    break
        else:
            raise RuntimeError("unterminated native runtime factory declaration")


def main() -> int:
    violations: list[tuple[Path, int, str]] = []
    try:
        for relative_root in SCAN_ROOTS:
            scan_root = ROOT / relative_root
            if not scan_root.is_dir():
                raise RuntimeError(f"required scan root is missing: {relative_root}")
            for path in sorted(scan_root.rglob("*.rs")):
                text = path.read_text(encoding="utf-8")
                for line_number, line in enumerate(text.splitlines(), start=1):
                    for description, pattern in FORBIDDEN:
                        if pattern.search(line):
                            violations.append(
                                (path.relative_to(ROOT), line_number, description)
                            )
                for description, pattern in WHOLE_FILE_FORBIDDEN:
                    for match in pattern.finditer(text):
                        line_number = text.count("\n", 0, match.start()) + 1
                        violations.append(
                            (path.relative_to(ROOT), line_number, description)
                        )
                for start, block in native_declaration_blocks(text):
                    line_number = text.count("\n", 0, start) + 1
                    fields = set(
                        re.findall(
                            r"(?:^|,)\s*([A-Za-z_][A-Za-z0-9_]*)\s*:",
                            block,
                            re.MULTILINE,
                        )
                    )
                    for forbidden in ("factory", "cargo_features"):
                        if forbidden in fields:
                            violations.append(
                                (
                                    path.relative_to(ROOT),
                                    line_number,
                                    f"native declaration field `{forbidden}:`",
                                )
                            )
                    for required in ("factory_type", "extra_cargo_features"):
                        if required not in fields:
                            violations.append(
                                (
                                    path.relative_to(ROOT),
                                    line_number,
                                    f"native declaration missing `{required}:`",
                                )
                            )
    except Exception as error:
        print(f"runtime factory safety audit failed internally: {error}", file=sys.stderr)
        return 2

    for path, line_number, description in violations:
        print(f"{path}:{line_number}: {description}", file=sys.stderr)
    if violations:
        return 1

    print("runtime factory safety audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
