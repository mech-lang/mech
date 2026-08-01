#!/usr/bin/env python3
"""Generate direct canonical grammar dependencies from specification EBNF."""

from __future__ import annotations

import argparse
import csv
import io
import re
from pathlib import Path

EXPECTED_RULES = 539
FENCE_OPEN = "```ebnf:canonical"
FENCE_CLOSE = "```"

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
SPECIFICATION = REPOSITORY_ROOT / "docs/design/specification.mec"
PRODUCTIONS = REPOSITORY_ROOT / "docs/design/grammar-audit/productions.tsv"
PORTS = REPOSITORY_ROOT / "docs/design/grammar-audit/ports.tsv"
OUTPUT = REPOSITORY_ROOT / "docs/design/grammar-audit/canonical-dependencies.tsv"

PRODUCTION_COLUMNS = [
    "id",
    "grammar-name",
    "module",
    "rust-function",
    "classification",
    "feature-gate",
    "entry-point",
    "output-type",
    "parent-rules",
    "child-rules",
    "selection-behavior",
    "termination",
    "whitespace",
    "spec-location",
    "conformance-cases",
    "implementation-path",
    "notes",
]

PORT_COLUMNS = [
    "grammar-name",
    "family",
    "syntax-status",
    "lowering-status",
    "node-policy",
    "phase",
    "notes",
]

OUTPUT_COLUMNS = ["grammar-name", "direct-children", "direct-parents"]
IDENTIFIER = re.compile(r"[a-z][a-z0-9-]*")

REQUIRED_EDGES = {
    "literal": {
        "number",
        "string",
        "atom",
        "boolean",
        "empty",
        "kind-annotation",
    },
    "kind": {
        "kind-any",
        "kind-atom",
        "kind-empty",
        "kind-map",
        "kind-matrix",
        "kind-record",
        "kind-scalar",
        "kind-set",
        "kind-table",
        "kind-tuple",
        "kind-kind",
    },
    "kind-annotation": {"left-angle", "kind-with-option", "right-angle"},
    "kind-with-option": {"kind", "question"},
    "kind-scalar": {"identifier", "range-expression"},
    "var": {"prefixed-context-path", "identifier", "kind-annotation"},
    "matrix-comprehension": {"expression", "comprehension-qualifier"},
    "set-comprehension": {"expression", "comprehension-qualifier"},
    "comprehension-qualifier": {"generator", "variable-define", "expression"},
    "generator": {"pattern", "generator-arrow", "generator-arrow-u", "expression"},
    "variable-define": {"tilde", "var", "define-operator", "expression"},
    "subscript": {
        "swizzle-subscript",
        "dot-subscript",
        "dot-subscript-int",
        "bracket-subscript",
        "brace-subscript",
    },
}


def read_tsv(path: Path, expected_columns: list[str]) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as source:
        reader = csv.DictReader(source, delimiter="\t")
        if reader.fieldnames != expected_columns:
            raise SystemExit(
                f"expected {path.name} columns {expected_columns}, "
                f"found {reader.fieldnames}"
            )
        rows = list(reader)
    for line_number, row in enumerate(rows, start=2):
        if None in row or any(value is None for value in row.values()):
            raise SystemExit(f"{path.name}:{line_number}: invalid field count")
    return rows


def canonical_inventory_names() -> set[str]:
    rows = read_tsv(PRODUCTIONS, PRODUCTION_COLUMNS)
    canonical = [
        row
        for row in rows
        if row["spec-location"].startswith("docs/design/specification.mec::")
    ]
    if len(canonical) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} canonical production rows, "
            f"found {len(canonical)}"
        )
    names = [row["grammar-name"] for row in canonical]
    if len(set(names)) != EXPECTED_RULES:
        raise SystemExit("canonical productions contain duplicate grammar names")
    return set(names)


def port_names() -> set[str]:
    rows = read_tsv(PORTS, PORT_COLUMNS)
    if len(rows) != EXPECTED_RULES:
        raise SystemExit(f"expected {EXPECTED_RULES} port rows, found {len(rows)}")
    names = [row["grammar-name"] for row in rows]
    if len(set(names)) != EXPECTED_RULES:
        raise SystemExit("ports.tsv contains duplicate grammar-name entries")
    return set(names)


def canonical_fence() -> str:
    lines = SPECIFICATION.read_text(encoding="utf-8").splitlines()
    openings = [index for index, line in enumerate(lines) if line == FENCE_OPEN]
    if len(openings) != 1:
        raise SystemExit(
            f"expected exactly one {FENCE_OPEN} fence, found {len(openings)}"
        )
    start = openings[0]
    closings = [
        index
        for index in range(start + 1, len(lines))
        if lines[index] == FENCE_CLOSE
    ]
    if not closings:
        raise SystemExit("canonical EBNF fence is missing its closing marker")
    end = closings[0]
    if any(line == FENCE_OPEN for line in lines[end + 1 :]):
        raise SystemExit("canonical EBNF fence marker is not unique")
    return "\n".join(lines[start + 1 : end]) + "\n"


def scan_productions(source: str) -> list[str]:
    productions: list[str] = []
    start = 0
    quoted = False
    escaped = False
    for index, character in enumerate(source):
        if quoted:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                quoted = False
            continue
        if character == '"':
            quoted = True
        elif character == ";":
            production = source[start:index].strip()
            if not production:
                raise SystemExit("canonical EBNF contains an empty production")
            productions.append(production)
            start = index + 1
    if quoted:
        raise SystemExit("canonical EBNF contains an unclosed quoted terminal")
    if source[start:].strip():
        raise SystemExit("canonical EBNF contains an unterminated production")
    return productions


def split_production(production: str) -> tuple[str, str]:
    markers: list[int] = []
    quoted = False
    escaped = False
    index = 0
    while index < len(production):
        character = production[index]
        if quoted:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                quoted = False
            index += 1
            continue
        if character == '"':
            quoted = True
            index += 1
            continue
        if production.startswith(":=", index):
            markers.append(index)
            index += 2
        else:
            index += 1
    if len(markers) != 1:
        raise SystemExit(f"invalid canonical production: {production[:80]!r}")
    marker = markers[0]
    left = production[:marker].strip()
    right = production[marker + 2 :].strip()
    if IDENTIFIER.fullmatch(left) is None:
        raise SystemExit(f"invalid canonical rule name: {left!r}")
    if not right:
        raise SystemExit(f"{left}: canonical production has an empty RHS")
    return left, right


def descriptive_primitive(rhs: str) -> bool:
    return len(rhs) >= 2 and rhs.startswith("?") and rhs.endswith("?")


def identifier_tokens(rhs: str) -> list[str]:
    tokens: list[str] = []
    index = 0
    quoted = False
    escaped = False
    while index < len(rhs):
        character = rhs[index]
        if quoted:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                quoted = False
            index += 1
            continue
        if character == '"':
            quoted = True
            index += 1
            continue
        match = IDENTIFIER.match(rhs, index)
        if match is not None:
            tokens.append(match.group(0))
            index = match.end()
        else:
            index += 1
    if quoted:
        raise SystemExit("canonical RHS contains an unclosed quoted terminal")
    return tokens


def dependency_graph() -> tuple[dict[str, set[str]], int]:
    inventory = canonical_inventory_names()
    ports = port_names()
    if inventory != ports:
        raise SystemExit("canonical production and port name sets differ")

    parsed = [split_production(item) for item in scan_productions(canonical_fence())]
    if len(parsed) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} canonical EBNF definitions, found {len(parsed)}"
        )
    names = [name for name, _ in parsed]
    if len(set(names)) != EXPECTED_RULES:
        raise SystemExit("canonical EBNF contains duplicate rule definitions")
    if set(names) != inventory:
        missing = sorted(inventory - set(names))
        unknown = sorted(set(names) - inventory)
        raise SystemExit(
            f"canonical EBNF and inventory names differ: missing={missing}, unknown={unknown}"
        )

    primitives = 0
    graph: dict[str, set[str]] = {}
    for name, rhs in parsed:
        if descriptive_primitive(rhs):
            graph[name] = set()
            primitives += 1
            continue
        graph[name] = {
            token
            for token in identifier_tokens(rhs)
            if token in inventory
        }

    for name, required in REQUIRED_EDGES.items():
        missing = sorted(required - graph[name])
        if missing:
            raise SystemExit(
                f"{name}: canonical dependency regressions missing "
                + ", ".join(missing)
            )
    return graph, primitives


def joined(values: set[str]) -> str:
    return "|".join(sorted(values)) if values else "none"


def render() -> tuple[str, int, int]:
    graph, primitives = dependency_graph()
    parents = {name: set() for name in graph}
    for parent, children in graph.items():
        for child in children:
            parents[child].add(parent)

    output = io.StringIO(newline="")
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow(OUTPUT_COLUMNS)
    for name in sorted(graph):
        writer.writerow([name, joined(graph[name]), joined(parents[name])])
    edge_count = sum(len(children) for children in graph.values())
    return output.getvalue(), edge_count, primitives


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when the dependency report differs",
    )
    args = parser.parse_args()

    generated, edge_count, primitives = render()
    contents = generated.encode("utf-8")
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != contents:
            raise SystemExit(
                "canonical dependency report is stale; run:\n"
                "  python3 scripts/generate-canonical-dependencies.py"
            )
        return
    OUTPUT.write_bytes(contents)
    print(f"canonical rules: {EXPECTED_RULES}")
    print(f"canonical dependency edges: {edge_count}")
    print(f"descriptive primitives: {primitives}")


if __name__ == "__main__":
    main()
