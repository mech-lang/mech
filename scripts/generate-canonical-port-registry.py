#!/usr/bin/env python3
"""Generate the package-local canonical syntax port registry."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

EXPECTED_RULES = 539
REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
PORTS = REPOSITORY_ROOT / "docs/design/grammar-audit/ports.tsv"
OUTPUT = (
    REPOSITORY_ROOT
    / "src/syntax/src/document/parser/canonical_ports.rs"
)

EXPECTED_COLUMNS = [
    "grammar-name",
    "family",
    "syntax-status",
    "lowering-status",
    "node-policy",
    "phase",
    "notes",
]
SYNTAX_STATUSES = {
    "unported": "Unported",
    "syntax-ported": "SyntaxPorted",
    "parity-verified": "ParityVerified",
}
LOWERING_STATUSES = {
    "not-applicable": "NotApplicable",
    "pending": "Pending",
    "parity-verified": "ParityVerified",
}
FAMILIES = {
    "activation": "Activation",
    "base": "Base",
    "expressions": "Expressions",
    "functions": "Functions",
    "grammar": "Grammar",
    "imports": "Imports",
    "literals": "Literals",
    "mechdown": "Mechdown",
    "mika": "Mika",
    "parser": "Parser",
    "patterns": "Patterns",
    "repl": "Repl",
    "state_machines": "StateMachines",
    "statements": "Statements",
    "structures": "Structures",
}
PHASES = {
    "": "None",
    "2A": "Some(PortPhase::Phase2A)",
    "2B": "Some(PortPhase::Phase2B)",
    "2C": "Some(PortPhase::Phase2C)",
    "2D": "Some(PortPhase::Phase2D)",
    "2E": "Some(PortPhase::Phase2E)",
}


def rust_constant(name: str) -> str:
    return name.replace("-", "_").upper()


def rust_string(value: str) -> str:
    return (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )


def node_policy(value: str) -> str:
    if value == "undecided":
        return "NodePolicy::Undecided"
    if value == "token":
        return "NodePolicy::Token"
    if value == "transparent":
        return "NodePolicy::Transparent"
    if value.startswith("node:"):
        kind = value.removeprefix("node:")
        if not kind:
            raise SystemExit("node policy is missing its SyntaxKind")
        return f"NodePolicy::Node(SyntaxKind::{kind})"
    if value.startswith("root:"):
        kind = value.removeprefix("root:")
        if not kind:
            raise SystemExit("root policy is missing its SyntaxKind")
        return f"NodePolicy::Root(SyntaxKind::{kind})"
    raise SystemExit(f"unknown node policy: {value}")


def port_rows() -> list[dict[str, str]]:
    with PORTS.open(newline="", encoding="utf-8") as source:
        reader = csv.DictReader(source, delimiter="\t")
        if reader.fieldnames != EXPECTED_COLUMNS:
            raise SystemExit(
                f"expected ports.tsv columns {EXPECTED_COLUMNS}, "
                f"found {reader.fieldnames}"
            )
        rows = list(reader)
    if len(rows) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} port rows, found {len(rows)}"
        )
    names = [row["grammar-name"] for row in rows]
    if names != sorted(names):
        raise SystemExit("ports.tsv must be ordered by grammar-name")
    if len(names) != len(set(names)):
        raise SystemExit("ports.tsv contains duplicate grammar-name entries")
    for row in rows:
        name = row["grammar-name"]
        if row["family"] not in FAMILIES:
            raise SystemExit(f"{name}: unknown family {row['family']}")
        if row["syntax-status"] not in SYNTAX_STATUSES:
            raise SystemExit(
                f"{name}: unknown syntax status {row['syntax-status']}"
            )
        if row["lowering-status"] not in LOWERING_STATUSES:
            raise SystemExit(
                f"{name}: unknown lowering status "
                f"{row['lowering-status']}"
            )
        node_policy(row["node-policy"])
        if row["phase"] not in PHASES:
            raise SystemExit(f"{name}: unknown phase {row['phase']}")
    return rows


def render() -> str:
    rows = port_rows()
    lines = [
        "// Generated from docs/design/grammar-audit/ports.tsv.",
        "// Do not edit by hand.",
        "",
        "use crate::document::{RuleId, SyntaxKind};",
        "",
        "use super::canonical_rules::rules;",
        "",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub enum SyntaxPortStatus {",
        "  Unported,",
        "  SyntaxPorted,",
        "  ParityVerified,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub enum LoweringPortStatus {",
        "  NotApplicable,",
        "  Pending,",
        "  ParityVerified,",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub enum NodePolicy {",
        "  Undecided,",
        "  Token,",
        "  Transparent,",
        "  Node(SyntaxKind),",
        "  Root(SyntaxKind),",
        "}",
        "",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub enum RuleFamily {",
    ]
    lines.extend(f"  {family}," for family in FAMILIES.values())
    lines.extend(
        [
            "}",
            "",
            "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
            "pub enum PortPhase {",
            "  Phase2A,",
            "  Phase2B,",
            "  Phase2C,",
            "  Phase2D,",
            "  Phase2E,",
            "}",
            "",
            "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
            "pub struct RulePort {",
            "  pub name: &'static str,",
            "  pub rule: RuleId,",
            "  pub family: RuleFamily,",
            "  pub syntax: SyntaxPortStatus,",
            "  pub lowering: LoweringPortStatus,",
            "  pub node_policy: NodePolicy,",
            "  pub phase: Option<PortPhase>,",
            "  pub notes: &'static str,",
            "}",
            "",
            f"pub const CANONICAL_PORT_COUNT: usize = {EXPECTED_RULES};",
            "",
            "pub static CANONICAL_PORTS: &[RulePort] = &[",
        ]
    )
    for row in rows:
        lines.extend(
            [
                "  RulePort {",
                f'    name: "{rust_string(row["grammar-name"])}",',
                f"    rule: rules::{rust_constant(row['grammar-name'])},",
                f"    family: RuleFamily::{FAMILIES[row['family']]},",
                "    syntax: SyntaxPortStatus::"
                f"{SYNTAX_STATUSES[row['syntax-status']]},",
                "    lowering: LoweringPortStatus::"
                f"{LOWERING_STATUSES[row['lowering-status']]},",
                f"    node_policy: {node_policy(row['node-policy'])},",
                f"    phase: {PHASES[row['phase']]},",
                f'    notes: "{rust_string(row["notes"])}",',
                "  },",
            ]
        )
    lines.extend(["];", ""])
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
                "canonical port registry is stale; run "
                "python3 scripts/generate-canonical-port-registry.py"
            )
        return
    OUTPUT.write_text(generated, encoding="utf-8")


if __name__ == "__main__":
    main()
