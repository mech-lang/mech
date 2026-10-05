#!/usr/bin/env python3
"""Generate the package-local canonical syntax port registry."""

from __future__ import annotations

import argparse
import csv
import subprocess
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
    "semantic-status",
    "grammar-scope",
    "node-policy",
    "component",
    "notes",
]
SYNTAX_STATUSES = {
    "unported": "Unported",
    "certified": "Certified",
}
SEMANTIC_STATUSES = {
    "pending": "Pending",
    "syntax-only": "SyntaxOnly",
    "certified": "Certified",
}
GRAMMAR_SCOPES = {
    "outside-document": "OutsideDocument",
    "executable-core": "ExecutableCore",
    "supporting": "Supporting",
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
COMPONENTS = {
    "": "None",
    "lexical": "Some(GrammarComponent::Lexical)",
    "document-markup": "Some(GrammarComponent::DocumentMarkup)",
    "literal-path-kind": "Some(GrammarComponent::LiteralPathKind)",
    "expression": "Some(GrammarComponent::Expression)",
    "module-import": "Some(GrammarComponent::ModuleImport)",
    "declaration": "Some(GrammarComponent::Declaration)",
    "primitive": "Some(GrammarComponent::Primitive)",
    "structure": "Some(GrammarComponent::Structure)",
    "executable": "Some(GrammarComponent::Executable)",
    "document": "Some(GrammarComponent::Document)",
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
        if row["semantic-status"] not in SEMANTIC_STATUSES:
            raise SystemExit(
                f"{name}: unknown semantic status "
                f"{row['semantic-status']}"
            )
        if row["grammar-scope"] not in GRAMMAR_SCOPES:
            raise SystemExit(
                f"{name}: unknown grammar scope "
                f"{row['grammar-scope']}"
            )
        if (
            row["syntax-status"] == "unported"
            and row["grammar-scope"] != "outside-document"
        ):
            raise SystemExit(f"{name}: unported rule is inside Document scope")
        if (
            row["syntax-status"] == "certified"
            and row["grammar-scope"] == "outside-document"
        ):
            raise SystemExit(f"{name}: certified rule is outside Document scope")
        if row["grammar-scope"] == "executable-core" and row["component"] != "executable":
            raise SystemExit(f"{name}: only executable grammar belongs to the executable core")
        node_policy(row["node-policy"])
        if row["component"] not in COMPONENTS:
            raise SystemExit(f"{name}: unknown component {row['component']}")
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
        "/// Rule-level syntax evidence; this metadata does not select a parser root.",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub enum SyntaxPortStatus {",
        "  Unported,",
        "  Certified,",
        "}",
        "",
        "/// Rule-level source-semantic disposition evidence.",
        "///",
        "/// `Certified` records source-semantic evidence. Runtime execution support is",
        "/// established by engine and runtime behavior tests.",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub enum SemanticPortStatus {",
        "  Pending,",
        "  SyntaxOnly,",
        "  Certified,",
        "}",
        "",
        "/// Grammar dependency grouping; independent of execution support.",
        "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
        "pub enum GrammarScope {",
        "  OutsideDocument,",
        "  ExecutableCore,",
        "  Supporting,",
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
            "pub enum GrammarComponent {",
            "  Lexical,",
            "  DocumentMarkup,",
            "  LiteralPathKind,",
            "  Expression,",
            "  ModuleImport,",
            "  Declaration,",
            "  Primitive,",
            "  Structure,",
            "  Executable,",
            "  Document,",
            "}",
            "",
            "/// Generated audit metadata; parser and runtime dispatch do not read it.",
            "#[derive(Clone, Copy, Debug, Eq, PartialEq)]",
            "pub struct RulePort {",
            "  pub name: &'static str,",
            "  pub rule: RuleId,",
            "  pub family: RuleFamily,",
            "  pub syntax: SyntaxPortStatus,",
            "  pub semantic: SemanticPortStatus,",
            "  pub scope: GrammarScope,",
            "  pub node_policy: NodePolicy,",
            "  pub component: Option<GrammarComponent>,",
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
                "    semantic: SemanticPortStatus::"
                f"{SEMANTIC_STATUSES[row['semantic-status']]},",
                "    scope: GrammarScope::"
                f"{GRAMMAR_SCOPES[row['grammar-scope']]},",
                f"    node_policy: {node_policy(row['node-policy'])},",
                f"    component: {COMPONENTS[row['component']]},",
                f'    notes: "{rust_string(row["notes"])}",',
                "  },",
            ]
        )
    lines.extend(["];", ""])
    # Keep generated Rust identical to the workspace's formatting contract.
    return subprocess.run(
        ["rustfmt", "+nightly-2026-03-03", "--edition", "2024"],
        input="\n".join(lines), text=True, capture_output=True, check=True,
    ).stdout


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
