#!/usr/bin/env python3
"""Generate the package-local Phase 0 canonical RuleId registry."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

EXPECTED_RULES = 540
REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
INVENTORY = REPOSITORY_ROOT / "docs/design/grammar-audit/productions.tsv"
OUTPUT = (
    REPOSITORY_ROOT
    / "src/syntax/src/document/parser/canonical_rules.rs"
)


def stable_hash(name: str) -> int:
    value = 0x811C9DC5
    for byte in name.encode("utf-8"):
        value ^= byte
        value = (value * 0x01000193) & 0xFFFFFFFF
    return value


def canonical_rules() -> list[tuple[str, int]]:
    with INVENTORY.open(newline="", encoding="utf-8") as source:
        rows = csv.DictReader(source, delimiter="\t")
        rules = {
            row["grammar-name"]
            for row in rows
            if row["spec-location"].startswith(
                "docs/design/specification.mec::"
            )
        }
    if len(rules) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} canonical rules, found {len(rules)}"
        )
    ordered = [(name, stable_hash(name)) for name in sorted(rules)]
    hashes: dict[int, str] = {}
    for name, rule_id in ordered:
        previous = hashes.setdefault(rule_id, name)
        if previous != name:
            raise SystemExit(
                f"RuleId collision between {previous} and {name}: "
                f"{rule_id:08x}"
            )
    return ordered


def rust_constant(name: str) -> str:
    return name.replace("-", "_").upper()


def render() -> str:
    rules = canonical_rules()
    constants = [rust_constant(name) for name, _ in rules]
    if len(constants) != len(set(constants)):
        raise SystemExit("canonical rule names collide as Rust constants")
    lines = [
        "// Generated from docs/design/grammar-audit/productions.tsv.",
        "// Do not edit by hand.",
        "",
        "use crate::document::RuleId;",
        "",
        f"pub const CANONICAL_RULE_COUNT: usize = {EXPECTED_RULES};",
        "",
        "pub static CANONICAL_RULES: &[(&str, RuleId)] = &[",
    ]
    lines.extend(
        f'  ("{name}", RuleId(0x{rule_id:08x})),'
        for name, rule_id in rules
    )
    lines.extend(
        [
            "];",
            "",
            "pub mod rules {",
            "  use crate::document::RuleId;",
            "",
        ]
    )
    lines.extend(
        f"  pub const {rust_constant(name)}: RuleId = RuleId(0x{rule_id:08x});"
        for name, rule_id in rules
    )
    lines.extend(["}", ""])
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when the checked-in registry differs",
    )
    args = parser.parse_args()
    generated = render()
    if args.check:
        existing = OUTPUT.read_text(encoding="utf-8")
        if existing != generated:
            raise SystemExit(
                "canonical rule registry is stale; run "
                "python3 scripts/generate-canonical-rule-registry.py"
            )
        return
    OUTPUT.write_text(generated, encoding="utf-8")


if __name__ == "__main__":
    main()
