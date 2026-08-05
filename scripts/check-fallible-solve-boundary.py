#!/usr/bin/env python3
"""Reject infallible or error-discarding production function execution."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCAN_ROOTS = ("src", "machines", "hosts", "tests")
REMOVED_SOLVE = (
    ("removed MechFunctionImpl::solve declaration", re.compile(r"\bfn\s+solve\s*\(\s*&self\s*\)")),
    ("call to removed MechFunctionImpl::solve", re.compile(r"\.\s*solve\s*\(\s*\)")),
)
PRODUCTION_ONLY = (
    (
        "discarded solve_result error",
        re.compile(r"\blet\s+_\s*=\s*[^;]{0,512}?\bsolve_result\s*\(", re.DOTALL),
    ),
    (
        "panic conversion around solve_result",
        re.compile(
            r"\bsolve_result\s*\([^;]{0,256}?\)\s*\.\s*(?:unwrap|expect)\s*\(",
            re.DOTALL,
        ),
    ),
)
TEST_PATH_PARTS = {"tests", "benches"}


def mask_comments_and_literals(source: str) -> str:
    """Replace comments and literals with spaces while preserving newlines."""
    result = list(source)
    index = 0
    length = len(source)
    while index < length:
        if source.startswith("//", index):
            end = source.find("\n", index)
            end = length if end < 0 else end
            for position in range(index, end):
                result[position] = " "
            index = end
        elif source.startswith("/*", index):
            depth = 1
            end = index + 2
            while end < length and depth:
                if source.startswith("/*", end):
                    depth += 1
                    end += 2
                elif source.startswith("*/", end):
                    depth -= 1
                    end += 2
                else:
                    end += 1
            for position in range(index, end):
                if result[position] != "\n":
                    result[position] = " "
            index = end
        elif source[index] in {'"', "'"}:
            quote = source[index]
            end = index + 1
            while end < length:
                if source[end] == "\\":
                    end += 2
                    continue
                if source[end] == quote:
                    end += 1
                    break
                end += 1
            for position in range(index, min(end, length)):
                if result[position] != "\n":
                    result[position] = " "
            index = end
        else:
            index += 1
    return "".join(result)


def mask_cfg_test_modules(source: str) -> str:
    """Mask inline cfg(test) modules so test assertions may unwrap errors."""
    masked = mask_comments_and_literals(source)
    result = list(source)
    module = re.compile(
        r"#\s*\[\s*cfg\s*\([^\]]*\btest\b[^\]]*\)\s*\]\s*mod\s+\w+\s*\{",
        re.DOTALL,
    )
    for match in module.finditer(masked):
        open_brace = masked.find("{", match.start(), match.end())
        depth = 0
        end = open_brace
        while end < len(masked):
            if masked[end] == "{":
                depth += 1
            elif masked[end] == "}":
                depth -= 1
                if depth == 0:
                    end += 1
                    break
            end += 1
        for position in range(match.start(), end):
            if result[position] != "\n":
                result[position] = " "
    return "".join(result)


def is_test_path(path: Path) -> bool:
    relative = path.relative_to(ROOT)
    return bool(TEST_PATH_PARTS.intersection(relative.parts)) or path.name == "tests.rs"


def record_matches(
    violations: list[tuple[Path, int, str]],
    path: Path,
    source: str,
    checks: tuple[tuple[str, re.Pattern[str]], ...],
) -> None:
    for description, pattern in checks:
        for match in pattern.finditer(source):
            line_number = source.count("\n", 0, match.start()) + 1
            violations.append((path.relative_to(ROOT), line_number, description))


def main() -> int:
    violations: list[tuple[Path, int, str]] = []
    try:
        for relative_root in SCAN_ROOTS:
            scan_root = ROOT / relative_root
            if not scan_root.is_dir():
                raise RuntimeError(f"required scan root is missing: {relative_root}")
            for path in sorted(scan_root.rglob("*.rs")):
                source = mask_comments_and_literals(path.read_text(encoding="utf-8"))
                record_matches(violations, path, source, REMOVED_SOLVE)
                if not is_test_path(path):
                    production = mask_cfg_test_modules(path.read_text(encoding="utf-8"))
                    production = mask_comments_and_literals(production)
                    record_matches(violations, path, production, PRODUCTION_ONLY)
    except Exception as error:
        print(f"fallible solve boundary audit failed internally: {error}", file=sys.stderr)
        return 2

    for path, line_number, description in violations:
        print(f"{path}:{line_number}: {description}", file=sys.stderr)
    if violations:
        return 1

    print("fallible solve boundary audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
