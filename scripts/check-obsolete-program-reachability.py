#!/usr/bin/env python3
"""Reject obsolete mech-program package reachability while preserving ABI data."""

from __future__ import annotations

import argparse
import re
from dataclasses import dataclass
from pathlib import Path


ALLOWED_DOMAIN_OWNER = Path("src/engine/src/artifact/encoding.rs")
ALLOWED_DOMAIN_LITERAL = 'hash.update(b"mech-program-v1\\0");'
OBSOLETE_PACKAGE = "mech-" + "program"
OBSOLETE_CRATE = "mech_" + "program"
CRATE_REACHABILITY = re.compile(
    rf"(?:\buse\s+{OBSOLETE_CRATE}\b|\bextern\s+crate\s+{OBSOLETE_CRATE}\b|"
    rf"\b{OBSOLETE_CRATE}::)"
)


@dataclass(frozen=True)
class Finding:
    path: Path
    line: int
    text: str

    def render(self) -> str:
        return f"{self.path}:{self.line}:{self.text.strip()}"


def rust_and_manifest_sources(root: Path):
    for pattern in ("Cargo.toml", "*.rs"):
        for path in root.rglob(pattern):
            relative = path.relative_to(root)
            if relative.parts and relative.parts[0] == "target":
                continue
            if relative.parts[:2] == ("tests", "architecture"):
                continue
            yield relative, path


def is_allowed_compatibility_data(relative: Path, line: str) -> bool:
    return relative == ALLOWED_DOMAIN_OWNER and line.strip() == ALLOWED_DOMAIN_LITERAL


def is_css_class_literal(line: str) -> bool:
    """Keep HTML/CSS class contracts distinct from obsolete crate reachability."""
    return (
        OBSOLETE_PACKAGE in line
        and (f".{OBSOLETE_PACKAGE}" in line or 'class=\\"' in line)
    )


def findings(root: Path) -> list[Finding]:
    result = []
    for relative, path in rust_and_manifest_sources(root):
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8").splitlines(), start=1
        ):
            if is_allowed_compatibility_data(relative, line):
                continue
            if OBSOLETE_PACKAGE in line and not is_css_class_literal(line):
                result.append(Finding(relative, line_number, line))
                continue
            if CRATE_REACHABILITY.search(line):
                result.append(Finding(relative, line_number, line))
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    rejected = findings(args.root.resolve())
    if rejected:
        for item in rejected:
            print(item.render())
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
