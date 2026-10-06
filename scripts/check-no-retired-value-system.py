#!/usr/bin/env python3
"""Fail if the retired universal value model reappears in executable Rust."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rust_source import rust_code


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


def normalize_raw_identifiers(source: str) -> str:
    """Normalize Rust's ``r#name`` spelling without changing source offsets."""
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
        masked = rust_code(source)
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
            rust_code(path.read_text(encoding="utf-8"))
        )
        if re.search(entry["pattern"], source) is None:
            failures.append(
                f"retained canonical declaration is missing: {entry['symbol']} ({relative})"
            )
    sources, include_failures = executable_rust_sources(root)
    failures.extend(include_failures)
    for path in sources:
        source = normalize_raw_identifiers(
            rust_code(path.read_text(encoding="utf-8"))
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
