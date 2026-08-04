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
)


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
