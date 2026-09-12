#!/usr/bin/env python3
"""Generate the canonical S7 document grammar from the specification."""

from __future__ import annotations

import argparse
import csv
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPECIFICATION = ROOT / "docs/design/specification.mec"
PORTS = ROOT / "docs/design/grammar-audit/ports.tsv"
DEPENDENCIES = ROOT / "docs/design/grammar-audit/canonical-dependencies.tsv"
OUTPUT = (
    ROOT
    / "src/syntax/src/document/parser/canonical/document_grammar.rs"
)
AST_OUTPUT = ROOT / "src/syntax/src/document/ast/document_core.rs"
CERTIFICATION_OUTPUT = (
    ROOT / "docs/design/grammar-audit/s7-document-certification.tsv"
)
DISPOSITIONS_OUTPUT = ROOT / "docs/design/grammar-audit/s7-dispositions.tsv"
EXPECTED_DOCUMENT_RULES = 112


def rust_constant(name: str) -> str:
    return name.replace("-", "_").upper()


def rust_kind(name: str) -> str:
    return "".join(part.capitalize() for part in name.split("-"))


def document_reachability() -> set[str]:
    with DEPENDENCIES.open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    children = {
        row["grammar-name"]: (
            []
            if row["direct-children"] == "none"
            else row["direct-children"].split("|")
        )
        for row in rows
    }
    reachable: set[str] = set()
    pending = ["parse"]
    while pending:
        name = pending.pop()
        if name in reachable:
            continue
        reachable.add(name)
        pending.extend(children.get(name, []))
    return reachable


def s7_candidates(rows: list[dict[str, str]]) -> set[str]:
    return {
        row["grammar-name"]
        for row in rows
        if row["phase"] == "S7" or row["syntax-status"] == "unported"
    }


def document_rules() -> list[str]:
    with PORTS.open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    reachable = document_reachability()
    candidates = s7_candidates(rows)
    names = [
        row["grammar-name"]
        for row in rows
        if row["grammar-name"] in candidates & reachable
    ]
    activated = {
        row["grammar-name"] for row in rows if row["phase"] == "S7"
    }
    if activated != set(names):
        raise SystemExit(
            "S7 activation must equal the candidate closure reachable from parse"
        )
    return names


def grammar_definitions() -> dict[str, str]:
    source = SPECIFICATION.read_text(encoding="utf-8")
    definitions: dict[str, str] = {}
    for match in re.finditer(r"(?m)^([a-z][a-z0-9-]*)\s*:=", source):
        cursor = match.end()
        quoted = False
        escaped = False
        while cursor < len(source):
            character = source[cursor]
            if quoted:
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    quoted = False
            elif character == '"':
                quoted = True
            elif character == ";":
                definitions[match.group(1)] = source[match.end() : cursor]
                break
            cursor += 1
    return definitions


@dataclass(frozen=True)
class Token:
    kind: str
    value: str


def tokenize(source: str) -> list[Token]:
    tokens: list[Token] = []
    cursor = 0
    punctuation = set("|,()[]*+?>¬")
    while cursor < len(source):
        character = source[cursor]
        if character.isspace():
            cursor += 1
            continue
        if character == '"':
            start = cursor
            cursor += 1
            escaped = False
            value: list[str] = []
            while cursor < len(source):
                character = source[cursor]
                cursor += 1
                if escaped:
                    value.append(character)
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    break
                else:
                    value.append(character)
            else:
                raise ValueError(f"unterminated literal at {source[start:]!r}")
            tokens.append(Token("literal", "".join(value)))
            continue
        if character == "?":
            descriptive = re.match(r"\?([a-z][a-z0-9-]*)\?", source[cursor:])
            if descriptive:
                tokens.append(Token("descriptive", descriptive.group(1)))
                cursor += len(descriptive.group(0))
                continue
        if character in punctuation:
            tokens.append(Token(character, character))
            cursor += 1
            continue
        identifier = re.match(r"[a-z][a-z0-9-]*", source[cursor:])
        if identifier:
            value = identifier.group(0)
            tokens.append(Token("rule", value))
            cursor += len(value)
            continue
        raise ValueError(f"unexpected grammar source {source[cursor:cursor + 32]!r}")
    return tokens


Expression = tuple


class GrammarParser:
    def __init__(self, tokens: list[Token]) -> None:
        self.tokens = tokens
        self.cursor = 0

    def parse(self) -> Expression:
        expression = self.choice()
        if self.cursor != len(self.tokens):
            raise ValueError(f"unconsumed tokens: {self.tokens[self.cursor:]}")
        return expression

    def peek(self, kind: str) -> bool:
        return self.cursor < len(self.tokens) and self.tokens[self.cursor].kind == kind

    def take(self, kind: str) -> Token:
        if not self.peek(kind):
            found = self.tokens[self.cursor] if self.cursor < len(self.tokens) else None
            raise ValueError(f"expected {kind}, found {found}")
        token = self.tokens[self.cursor]
        self.cursor += 1
        return token

    def choice(self) -> Expression:
        items = [self.sequence()]
        while self.peek("|"):
            self.take("|")
            items.append(self.sequence())
        return items[0] if len(items) == 1 else ("choice", tuple(items))

    def sequence(self) -> Expression:
        items = [self.prefix()]
        while self.peek(","):
            self.take(",")
            items.append(self.prefix())
        return items[0] if len(items) == 1 else ("sequence", tuple(items))

    def prefix(self) -> Expression:
        if any(self.peek(kind) for kind in ("?", "*", "+", ">", "¬")):
            operator = self.tokens[self.cursor].kind
            self.cursor += 1
            names = {
                "?": "optional",
                "*": "zero_or_more",
                "+": "one_or_more",
                ">": "peek",
                "¬": "not",
            }
            return (names[operator], self.prefix())
        return self.atom()

    def atom(self) -> Expression:
        if self.peek("("):
            self.take("(")
            expression = self.choice()
            self.take(")")
            return expression
        if self.peek("["):
            self.take("[")
            separator = self.prefix()
            self.take(",")
            item = self.prefix()
            self.take("]")
            return ("separated", separator, item)
        if self.peek("rule"):
            return ("rule", self.take("rule").value)
        if self.peek("literal"):
            return ("literal", self.take("literal").value)
        if self.peek("descriptive"):
            description = self.take("descriptive").value
            if description != "empty-input":
                raise ValueError(f"unsupported descriptive terminal {description}")
            return ("empty",)
        found = self.tokens[self.cursor] if self.cursor < len(self.tokens) else None
        raise ValueError(f"expected grammar atom, found {found}")


def choice_count(expression: Expression) -> int:
    kind = expression[0]
    if kind == "choice":
        return 1 + sum(choice_count(item) for item in expression[1])
    if kind == "sequence":
        return sum(choice_count(item) for item in expression[1])
    if kind == "separated":
        return choice_count(expression[1]) + choice_count(expression[2])
    if kind in {"optional", "zero_or_more", "one_or_more", "peek", "not"}:
        return choice_count(expression[1])
    return 0


def mika_expression_triples() -> tuple[tuple[str, str, str], ...]:
    source = SPECIFICATION.read_text(encoding="utf-8")
    paragraph = next(
        block
        for block in source.split("\n\n")
        if "At the baseline the accepted" in block and "triples are" in block
    )
    values = [
        value
        for value in re.findall(r"`([^`]+)`", paragraph)
        if value != "mika-expression-inner"
    ]
    if len(values) != 25:
        raise SystemExit(
            f"expected 25 registered Mika expression triples, found {len(values)}"
        )
    noses = re.findall(r'"([^"]+)"', grammar_definitions()["mika-nose"])
    triples = []
    for value in values:
        matches = [nose for nose in noses if nose in value]
        if len(matches) != 1:
            raise SystemExit(f"registered Mika expression has no unique nose: {value!r}")
        nose = matches[0]
        left, right = value.split(nose, 1)
        if not left or not right:
            raise SystemExit(f"registered Mika expression has an empty eye: {value!r}")
        triples.append((left, nose, right))
    return tuple(triples)


def rust_expression(
    expression: Expression,
    known_rules: set[str],
    best_choice: bool = False,
) -> str:
    kind = expression[0]
    if kind == "rule":
        if expression[1] not in known_rules:
            return f"GrammarExpression::Builtin({json.dumps(expression[1])})"
        return f"GrammarExpression::Rule(rules::{rust_constant(expression[1])})"
    if kind == "literal":
        return f"GrammarExpression::Literal({json.dumps(expression[1], ensure_ascii=False)})"
    if kind == "empty":
        return "GrammarExpression::Empty"
    if kind == "mika_expression_triples":
        triples = ", ".join(
            "(" + ", ".join(json.dumps(part, ensure_ascii=False) for part in triple) + ")"
            for triple in expression[1]
        )
        return f"GrammarExpression::MikaExpressionTriples(&[{triples}])"
    if kind in {"sequence", "choice"}:
        if kind == "sequence":
            variant = "Sequence"
        elif best_choice:
            variant = "BestChoice"
        else:
            variant = "Choice"
        items = ", ".join(
            rust_expression(item, known_rules, best_choice)
            for item in expression[1]
        )
        return f"GrammarExpression::{variant}(&[{items}])"
    if kind == "separated":
        return (
            "GrammarExpression::Separated { separator: &"
            + rust_expression(expression[1], known_rules, best_choice)
            + ", item: &"
            + rust_expression(expression[2], known_rules, best_choice)
            + " }"
        )
    variants = {
        "optional": "Optional",
        "zero_or_more": "ZeroOrMore",
        "one_or_more": "OneOrMore",
        "peek": "Peek",
        "not": "Not",
    }
    return (
        f"GrammarExpression::{variants[kind]}"
        f"(&{rust_expression(expression[1], known_rules, best_choice)})"
    )


TRANSPARENT = {"parse-mech", "program"}
BEST_CHOICE_RULES = {"statement", "mech-code-alt", "section-element"}
ROOT_KINDS = {"parse": "Document"}
EXISTING_KINDS = {
    "body": "Body",
    "paragraph": "Paragraph",
    "paragraph-element": "ParagraphElement",
    "section": "Section",
    "section-element": "SectionElement",
    "subtitle": "Subtitle",
    "title": "Title",
    "ul-subtitle": "UlSubtitle",
}

SAMPLE_OVERRIDES = {
    "context-send": "x<u8><-1 + 2",
    "fsm-arm": "42 |42 ->+> x",
    "fsm-guard": "|42 ->+> x",
    "fsm-implementation": "#machine(x) -> :start\n:start |42 ->+> x\n.",
    "fsm-state-definition-variables": "(x<u8>)",
    "fsm-transition": "42 ->{}",
    "function-define-statements": "=out<u8>:=value:=1.",
    "inline-mech-code": "{{1}}",
    "mika-expression-inner": "¬◯¬",
    "op-assign": "x += 1 + 2",
}


def load_rules() -> tuple[list[str], dict[str, Expression]]:
    definitions = grammar_definitions()
    names = document_rules()
    missing = [name for name in names if name not in definitions]
    if missing:
        raise SystemExit(f"missing canonical definitions: {', '.join(missing)}")

    parsed = {
        name: GrammarParser(tokenize(definitions[name])).parse() for name in names
    }
    parsed["mika-expression-inner"] = (
        "mika_expression_triples",
        mika_expression_triples(),
    )
    return names, parsed


def render_grammar(names: list[str], parsed: dict[str, Expression]) -> str:
    known_rules = set(grammar_definitions())
    lines = [
        "// Generated from docs/design/specification.mec and grammar-audit/ports.tsv.",
        "// Do not edit by hand.",
        "",
        "use crate::document::{RuleId, SyntaxKind};",
        "",
        "use super::super::rule::rules;",
        "",
        "#[derive(Clone, Copy, Debug)]",
        "pub(crate) enum GrammarExpression {",
        "    Rule(RuleId),",
        "    Builtin(&'static str),",
        "    Literal(&'static str),",
        "    MikaExpressionTriples(&'static [(&'static str, &'static str, &'static str)]),",
        "    Empty,",
        "    Sequence(&'static [GrammarExpression]),",
        "    Choice(&'static [GrammarExpression]),",
        "    BestChoice(&'static [GrammarExpression]),",
        "    Optional(&'static GrammarExpression),",
        "    ZeroOrMore(&'static GrammarExpression),",
        "    OneOrMore(&'static GrammarExpression),",
        "    Peek(&'static GrammarExpression),",
        "    Not(&'static GrammarExpression),",
        "    Separated {",
        "        separator: &'static GrammarExpression,",
        "        item: &'static GrammarExpression,",
        "    },",
        "}",
        "",
        "pub(crate) struct DocumentRule {",
        "    pub(crate) rule: RuleId,",
        "    pub(crate) expression: GrammarExpression,",
        "    pub(crate) kind: Option<SyntaxKind>,",
        "    pub(crate) root: bool,",
        "}",
        "",
        f"pub(crate) const DOCUMENT_RULE_COUNT: usize = {len(names)};",
        "",
        "pub(crate) static DOCUMENT_RULES: &[DocumentRule] = &[",
    ]
    for name in names:
        if name in BEST_CHOICE_RULES and choice_count(parsed[name]) != 1:
            raise ValueError(
                f"{name} must contain exactly one alt_best choice"
            )
        if name in TRANSPARENT:
            kind = "None"
        else:
            syntax_kind = ROOT_KINDS.get(name, EXISTING_KINDS.get(name, rust_kind(name)))
            kind = f"Some(SyntaxKind::{syntax_kind})"
        lines.extend(
            [
                "    DocumentRule {",
                f"        rule: rules::{rust_constant(name)},",
                f"        expression: {rust_expression(parsed[name], known_rules, name in BEST_CHOICE_RULES)},",
                f"        kind: {kind},",
                f"        root: {'true' if name in ROOT_KINDS else 'false'},",
                "    },",
            ]
        )
    lines.extend(["];"])
    return "\n".join(lines) + "\n"


def node_kind(name: str) -> str | None:
    if name in TRANSPARENT:
        return None
    return ROOT_KINDS.get(name, EXISTING_KINDS.get(name, rust_kind(name)))


def render_ast(names: list[str]) -> str:
    typed = [
        (name, node_kind(name))
        for name in names
        if name not in {"parse", "section", "paragraph"}
        and node_kind(name) is not None
    ]
    lines = [
        "// Generated from the canonical S7 document rule closure.",
        "// Do not edit by hand.",
        "",
        "use crate::document::{AstNode, SyntaxKind, SyntaxNode};",
        "",
        "macro_rules! document_ast_node {",
        "    ($name:ident, $kind:ident) => {",
        "        #[derive(Clone, Debug)]",
        "        pub struct $name(pub(crate) SyntaxNode);",
        "",
        "        impl AstNode for $name {",
        "            fn can_cast(kind: SyntaxKind) -> bool {",
        "                kind == SyntaxKind::$kind",
        "            }",
        "",
        "            fn cast(syntax: SyntaxNode) -> Option<Self> {",
        "                Self::can_cast(syntax.kind()).then_some(Self(syntax))",
        "            }",
        "",
        "            fn syntax(&self) -> &SyntaxNode {",
        "                &self.0",
        "            }",
        "        }",
        "    };",
        "}",
        "",
    ]
    for name, kind in typed:
        lines.append(f"document_ast_node!({rust_kind(name)}Syntax, {kind});")
    lines.extend(
        [
            "",
            "#[derive(Clone, Debug)]",
            "pub enum CanonicalDocumentNode {",
        ]
    )
    for name, _ in typed:
        rust = rust_kind(name)
        lines.append(f"    {rust}({rust}Syntax),")
    lines.extend(["}", "", "impl CanonicalDocumentNode {", "    pub fn cast(node: SyntaxNode) -> Option<Self> {", "        match node.kind() {"])
    for name, kind in typed:
        rust = rust_kind(name)
        lines.append(
            f"            SyntaxKind::{kind} => {rust}Syntax::cast(node).map(Self::{rust}),"
        )
    lines.extend(
        [
            "            _ => None,",
            "        }",
            "    }",
            "",
            "    pub fn syntax(&self) -> &SyntaxNode {",
            "        match self {",
        ]
    )
    for name, _ in typed:
        rust = rust_kind(name)
        lines.append(f"            Self::{rust}(node) => node.syntax(),")
    lines.extend(["        }", "    }", "}"])
    return "\n".join(lines) + "\n"


def certified_samples() -> dict[str, str]:
    samples = {
        "abstract-sigil": "%%",
        "alpha": "a",
        "alpha-token": "a",
        "alphanumeric": "a",
        "any": "a",
        "assign-operator": "=",
        "async-transition-operator": "~>",
        "atom": ":state",
        "bar": "|",
        "blank-line": "\n",
        "box-bl": "└",
        "box-t-left": "├",
        "codeblock-sigil": "```",
        "colon": ":",
        "comma": ",",
        "comment": "--",
        "context-declaration": "@x := x://x",
        "dash": "-",
        "define-operator": ":=",
        "digit": "1",
        "digit-token": "1",
        "emoji": "😀",
        "emphasis-sigil": "*",
        "enum-separator": "|",
        "equal": "=",
        "equation": "$$x",
        "error-alt-sigil": "(✗)>",
        "error-sigil": "(x)>",
        "exclamation": "!",
        "export-declaration": "<+ x",
        "float-left": "<<:",
        "float-right": ":>>",
        "footnote-prefix": "[^",
        "footnote-reference": "[^x]",
        "grave": "`",
        "guard-operator": "|",
        "hashtag": "#",
        "highlight-sigil": "!!",
        "idea-sigil": "(*)>",
        "identifier": "x",
        "img-prefix": "![",
        "import-declaration": "+> x",
        "info-sigil": "(i)>",
        "inline-code": "`x`",
        "inline-equation": "$$x$$",
        "left-angle": "<",
        "left-brace": "{",
        "left-bracket": "[",
        "left-parenthesis": "(",
        "list-separator": ",",
        "mika-section-close": "⸥",
        "mika-section-open": "⸢",
        "module-import": "+>x",
        "new-line": "\n",
        "number": "1",
        "op-assign-operator": "+=",
        "output-operator": "=>",
        "paragraph-text": "x",
        "period": ".",
        "prefixed-context-path": "@x/y",
        "prompt-sigil": ">:",
        "question": "?",
        "question-sigil": "(?)>",
        "quote-sigil": ">",
        "raw-hyperlink": "httpx",
        "reference": "[x]",
        "right-angle": ">",
        "right-brace": "}",
        "right-bracket": "]",
        "right-parenthesis": ")",
        "section-reference": "§x",
        "semicolon": ";",
        "send-operator": "<-",
        "space": " ",
        "space-tab0": "",
        "space-tab1": " ",
        "statement-separator": ";",
        "string": '"x"',
        "strong-sigil": "**",
        "subscript": "[1]",
        "success-check-sigil": "(✓)>",
        "success-sigil": "(+)>",
        "table-separator": "|",
        "text": "x",
        "thematic-break": "*\n",
        "tilde": "~",
        "transition-operator": "->",
        "underscore": "_",
        "warning-sigil": "(!)>",
        "whitespace0": "",
        "whitespace1": " ",
        "matching-codeblock-sigil": "```",
        "eof": "",
    }
    certification = (
        ROOT / "docs/design/grammar-audit/phase-2i-certification.tsv"
    ).read_text(encoding="utf-8")
    for line in certification.splitlines()[1:]:
        fields = line.split("\t")
        samples[fields[0]] = json.loads(fields[1])
    return samples


def synthesize(
    expression: Expression,
    parsed: dict[str, Expression],
    samples: dict[str, str],
    stack: frozenset[str],
) -> str | None:
    kind = expression[0]
    if kind == "rule":
        name = expression[1]
        if name in SAMPLE_OVERRIDES:
            return SAMPLE_OVERRIDES[name]
        if name in parsed and name not in stack:
            return synthesize(parsed[name], parsed, samples, stack | {name})
        return samples.get(name)
    if kind == "literal":
        return expression[1]
    if kind == "empty":
        return ""
    if kind == "sequence":
        items = [synthesize(item, parsed, samples, stack) for item in expression[1]]
        return None if any(item is None for item in items) else "".join(items)
    if kind == "choice":
        items = [
            item
            for item in (
                synthesize(candidate, parsed, samples, stack)
                for candidate in expression[1]
            )
            if item is not None
        ]
        return min(items, key=lambda item: (len(item.encode("utf-8")), item)) if items else None
    if kind in {"optional", "zero_or_more", "peek", "not"}:
        return ""
    if kind == "one_or_more":
        return synthesize(expression[1], parsed, samples, stack)
    if kind == "separated":
        return synthesize(expression[2], parsed, samples, stack)
    raise ValueError(f"unknown expression kind {kind}")


def render_certification(names: list[str], parsed: dict[str, Expression]) -> str:
    samples = certified_samples()
    with PORTS.open(newline="", encoding="utf-8") as source:
        ports = {
            row["grammar-name"]: row
            for row in csv.DictReader(source, delimiter="\t")
        }
    rows = [
        "grammar-name\taccepted-source-json\tnode-policy\tsemantic-status\tspec-location"
    ]
    for name in names:
        accepted = SAMPLE_OVERRIDES.get(name)
        if accepted is None:
            accepted = synthesize(parsed[name], parsed, samples, frozenset({name}))
        if accepted is None:
            raise SystemExit(f"unable to synthesize canonical source for {name}")
        rows.append(
            "\t".join(
                [
                    name,
                    json.dumps(accepted, ensure_ascii=False),
                    ports[name]["node-policy"],
                    ports[name]["semantic-status"],
                    f"docs/design/specification.mec::{name}",
                ]
            )
        )
    return "\n".join(rows) + "\n"


def render_dispositions(document_names: list[str]) -> str:
    with PORTS.open(newline="", encoding="utf-8") as source:
        ports = list(csv.DictReader(source, delimiter="\t"))
    rows = ["grammar-name\tdisposition\trationale"]
    candidates = s7_candidates(ports)
    document_names = set(document_names)
    for port in ports:
        name = port["grammar-name"]
        if name in document_names:
            if name in {"parse", "parse-mech"}:
                disposition = "maintained-root"
                rationale = "Public canonical Document root and its direct grammar alias."
            else:
                disposition = "document-dependency"
                rationale = "Generated member of the canonical Document closure."
        elif port["syntax-status"] == "unported" and port["family"] == "repl":
            disposition = "historical-command"
            rationale = (
                "Runtime REPL requests own this command; it is not document syntax."
            )
        elif name in candidates:
            disposition = "outside-document-closure"
            rationale = (
                "Not reachable from the canonical Document root and not activated."
            )
        else:
            continue
        rows.append("\t".join([name, disposition, rationale]))
    return "\n".join(rows) + "\n"


def format_rust(source: str) -> str:
    return subprocess.run(
        ["rustfmt", "+nightly-2026-03-03", "--edition", "2024"],
        input=source,
        text=True,
        capture_output=True,
        check=True,
    ).stdout


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    names, parsed = load_rules()
    if len(names) != EXPECTED_DOCUMENT_RULES:
        raise SystemExit(
            f"expected {EXPECTED_DOCUMENT_RULES} canonical Document rules, "
            f"found {len(names)}"
        )
    generated = {
        OUTPUT: format_rust(render_grammar(names, parsed)),
        AST_OUTPUT: format_rust(render_ast(names)),
        CERTIFICATION_OUTPUT: render_certification(names, parsed),
        DISPOSITIONS_OUTPUT: render_dispositions(names),
    }
    if args.check:
        stale = [
            path
            for path, contents in generated.items()
            if not path.exists() or path.read_text(encoding="utf-8") != contents
        ]
        if stale:
            raise SystemExit(
                "canonical document outputs are stale: "
                + ", ".join(str(path.relative_to(ROOT)) for path in stale)
            )
        return
    for path, contents in generated.items():
        path.write_text(contents, encoding="utf-8")


if __name__ == "__main__":
    main()
