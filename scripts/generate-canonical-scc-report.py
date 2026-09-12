#!/usr/bin/env python3
"""Generate canonical SCC and recursive-core dependency reports."""

from __future__ import annotations

import argparse
import csv
import io
from collections import deque
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Optional

EXPECTED_RULES = 539
PHASE_ROOT = "expression"

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]

PRODUCTIONS = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/productions.tsv"
)

PORTS = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/ports.tsv"
)

SCC_OUTPUT = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/unported-sccs.tsv"
)

PHASE_OUTPUT = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/phase-2i-recursive-core.tsv"
)

DEPENDENCIES = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/canonical-dependencies.tsv"
)

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

DEPENDENCY_COLUMNS = [
    "grammar-name",
    "direct-children",
    "direct-parents",
]

SCC_COLUMNS = [
    "component-id",
    "component-size",
    "recursive",
    "members",
    "outgoing-unported-components",
    "outgoing-ported-rules",
]

PHASE_COLUMNS = [
    "grammar-name",
    "family",
    "component-id",
    "component-size",
    "recursive-component",
    "same-component-children",
    "closure-children",
    "ported-external-children",
]

IMPLEMENTED_STATUSES = {"syntax-ported", "parity-verified"}
ANCHOR_RULES = {
    "expression",
    "formula",
    "factor",
    "literal",
    "kind-annotation",
    "kind",
    "kind-scalar",
    "var",
    "subscript",
    "slice",
    "structure",
    "matrix",
    "map",
    "set",
    "tuple",
    "function-call",
    "pattern",
    "comprehension-qualifier",
    "variable-define",
    "fsm-pipe",
}
FORBIDDEN_FAMILIES = {"mechdown", "mika", "repl", "activation", "parser"}


@dataclass(frozen=True)
class Analysis:
    productions: dict[str, dict[str, str]]
    ports: dict[str, dict[str, str]]
    graph: dict[str, tuple[str, ...]]
    components: tuple[tuple[str, ...], ...]
    component_ids: dict[tuple[str, ...], str]
    component_by_rule: dict[str, tuple[str, ...]]
    unported_components: tuple[tuple[str, ...], ...]
    recursive: dict[tuple[str, ...], bool]
    closure_components: frozenset[tuple[str, ...]]
    closure_rules: frozenset[str]
    ported_external_rules: frozenset[str]


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


def parse_name_list(
    value: str,
    owner: str,
    field: str,
    separator: str = ",",
) -> tuple[str, ...]:
    value = value.strip()
    if value in {"", "none"}:
        return ()
    names = tuple(part.strip() for part in value.split(separator))
    if any(not name for name in names):
        raise SystemExit(f"{owner}: {field} contains an empty list element")
    if len(names) != len(set(names)):
        raise SystemExit(f"{owner}: {field} contains duplicate names")
    return names


def load_inputs() -> tuple[
    dict[str, dict[str, str]],
    dict[str, dict[str, str]],
    dict[str, tuple[str, ...]],
]:
    production_rows = read_tsv(PRODUCTIONS, PRODUCTION_COLUMNS)
    canonical_rows = [
        row
        for row in production_rows
        if row["spec-location"].startswith(
            "docs/design/specification.mec::"
        )
    ]
    if len(canonical_rows) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} canonical production rows, "
            f"found {len(canonical_rows)}"
        )

    production_names = [row["grammar-name"] for row in canonical_rows]
    production_ids = [row["id"] for row in canonical_rows]
    if len(set(production_names)) != EXPECTED_RULES:
        raise SystemExit("canonical productions contain duplicate grammar names")
    if len(set(production_ids)) != EXPECTED_RULES:
        raise SystemExit("canonical productions contain duplicate production IDs")
    productions = {row["grammar-name"]: row for row in canonical_rows}

    port_rows = read_tsv(PORTS, PORT_COLUMNS)
    if len(port_rows) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} port rows, found {len(port_rows)}"
        )
    port_names = [row["grammar-name"] for row in port_rows]
    if len(set(port_names)) != EXPECTED_RULES:
        raise SystemExit("ports.tsv contains duplicate grammar-name entries")
    ports = {row["grammar-name"]: row for row in port_rows}
    if set(productions) != set(ports):
        missing_ports = sorted(set(productions) - set(ports))
        unknown_ports = sorted(set(ports) - set(productions))
        raise SystemExit(
            "canonical production and port names differ: "
            f"missing ports={missing_ports}, unknown ports={unknown_ports}"
        )

    for name, row in ports.items():
        status = row["syntax-status"]
        if status == "unported":
            if row["phase"]:
                raise SystemExit(f"{name}: unported rule has phase {row['phase']}")
            if row["node-policy"] != "undecided":
                raise SystemExit(
                    f"{name}: unported rule has node policy {row['node-policy']}"
                )
            if row["lowering-status"] != "pending":
                raise SystemExit(
                    f"{name}: unported rule has lowering status "
                    f"{row['lowering-status']}"
                )
        elif status in IMPLEMENTED_STATUSES:
            if not row["phase"]:
                raise SystemExit(f"{name}: syntax-implemented rule has no phase")
        else:
            raise SystemExit(f"{name}: unknown syntax status {status}")

    dependency_rows = read_tsv(DEPENDENCIES, DEPENDENCY_COLUMNS)
    if len(dependency_rows) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} canonical dependency rows, "
            f"found {len(dependency_rows)}"
        )
    dependency_names = [row["grammar-name"] for row in dependency_rows]
    if dependency_names != sorted(dependency_names):
        raise SystemExit("canonical-dependencies.tsv must be ordered by grammar-name")
    if len(set(dependency_names)) != EXPECTED_RULES:
        raise SystemExit("canonical-dependencies.tsv contains duplicate grammar names")
    if set(dependency_names) != set(productions):
        raise SystemExit("canonical dependency and production name sets differ")

    graph: dict[str, tuple[str, ...]] = {}
    reported_parents: dict[str, tuple[str, ...]] = {}
    canonical_names = set(productions)
    for row in dependency_rows:
        name = row["grammar-name"]
        children = parse_name_list(
            row["direct-children"], name, "direct-children", "|"
        )
        parents = parse_name_list(
            row["direct-parents"], name, "direct-parents", "|"
        )
        unknown = sorted((set(children) | set(parents)) - canonical_names)
        if unknown:
            raise SystemExit(
                f"{name}: unknown canonical dependencies: {', '.join(unknown)}"
            )
        graph[name] = tuple(sorted(children))
        reported_parents[name] = tuple(sorted(parents))

    reversed_graph = {name: set() for name in graph}
    for parent, children in graph.items():
        for child in children:
            reversed_graph[child].add(parent)
    for name in sorted(graph):
        if set(reported_parents[name]) != reversed_graph[name]:
            raise SystemExit(f"{name}: direct-parents does not reverse direct-children")
    return productions, ports, graph


def strongly_connected_components(
    graph: dict[str, tuple[str, ...]],
) -> tuple[tuple[str, ...], ...]:
    index = 0
    indices: dict[str, int] = {}
    lowlinks: dict[str, int] = {}
    stack: list[str] = []
    on_stack: set[str] = set()
    components: list[tuple[str, ...]] = []

    def visit(node: str) -> None:
        nonlocal index
        indices[node] = index
        lowlinks[node] = index
        index += 1
        stack.append(node)
        on_stack.add(node)

        for child in graph[node]:
            if child not in indices:
                visit(child)
                lowlinks[node] = min(lowlinks[node], lowlinks[child])
            elif child in on_stack:
                lowlinks[node] = min(lowlinks[node], indices[child])

        if lowlinks[node] != indices[node]:
            return
        members: list[str] = []
        while True:
            member = stack.pop()
            on_stack.remove(member)
            members.append(member)
            if member == node:
                break
        components.append(tuple(sorted(members)))

    for node in sorted(graph):
        if node not in indices:
            visit(node)

    return tuple(sorted(components, key=lambda members: (-len(members), members)))


def shortest_dependency_path(
    graph: dict[str, tuple[str, ...]],
    start: str,
    target: str,
) -> tuple[str, ...]:
    pending = deque([start])
    previous: dict[str, Optional[str]] = {start: None}
    while pending:
        node = pending.popleft()
        if node == target:
            path: list[str] = []
            cursor: Optional[str] = node
            while cursor is not None:
                path.append(cursor)
                cursor = previous[cursor]
            return tuple(reversed(path))
        for child in graph[node]:
            if child not in previous:
                previous[child] = node
                pending.append(child)
    return ()


def analyze() -> Analysis:
    productions, ports, graph = load_inputs()
    components = strongly_connected_components(graph)
    component_ids = {
        component: f"SCC-{index:04d}"
        for index, component in enumerate(components, start=1)
    }
    component_by_rule = {
        member: component
        for component in components
        for member in component
    }
    recursive = {
        component: len(component) > 1 or component[0] in graph[component[0]]
        for component in components
    }

    unported_components: list[tuple[str, ...]] = []
    for component in components:
        unported = tuple(
            member
            for member in component
            if ports[member]["syntax-status"] == "unported"
        )
        implemented = tuple(
            member
            for member in component
            if ports[member]["syntax-status"] in IMPLEMENTED_STATUSES
        )
        if unported and implemented:
            raise SystemExit(
                "mixed port-status SCC:\n"
                f"  members: {', '.join(component)}\n"
                f"  unported: {', '.join(unported)}\n"
                f"  syntax-implemented: {', '.join(implemented)}"
            )
        if unported:
            unported_components.append(component)

    root_component = component_by_rule.get(PHASE_ROOT)
    if root_component is None:
        raise SystemExit(f"Phase 2I root is not canonical: {PHASE_ROOT}")
    if ports[PHASE_ROOT]["syntax-status"] != "unported":
        raise SystemExit(f"Phase 2I root is already syntax-implemented: {PHASE_ROOT}")

    closure_components: set[tuple[str, ...]] = set()
    pending = [root_component]
    while pending:
        component = pending.pop()
        if component in closure_components:
            continue
        if any(
            ports[member]["syntax-status"] != "unported"
            for member in component
        ):
            continue
        closure_components.add(component)
        targets = {
            component_by_rule[child]
            for member in component
            for child in graph[member]
            if component_by_rule[child] != component
        }
        for target in sorted(targets, key=lambda item: component_ids[item], reverse=True):
            if ports[target[0]]["syntax-status"] == "unported":
                pending.append(target)

    closure_rules = frozenset(
        member
        for component in closure_components
        for member in component
    )
    if PHASE_ROOT not in closure_rules:
        raise SystemExit(f"Phase 2I closure does not contain {PHASE_ROOT}")
    for name in sorted(closure_rules):
        row = ports[name]
        if row["syntax-status"] != "unported":
            raise SystemExit(f"{name}: Phase 2I member is already syntax-implemented")
        if row["phase"]:
            raise SystemExit(f"{name}: Phase 2I member already has phase {row['phase']}")
        forbidden = row["family"]
        if forbidden in FORBIDDEN_FAMILIES:
            raise SystemExit(
                f"{name}: forbidden Phase 2I family {forbidden}; "
                "inspect canonical grammar metadata"
            )

    missing_anchors = sorted(ANCHOR_RULES - closure_rules)
    if missing_anchors:
        details = []
        for anchor in missing_anchors:
            path = shortest_dependency_path(graph, PHASE_ROOT, anchor)
            rendered = " -> ".join(path) if path else "none"
            details.append(f"  {anchor}: shortest dependency path: {rendered}")
        raise SystemExit(
            "Phase 2I closure is missing anchor rules:\n" + "\n".join(details)
        )

    ported_external_rules: set[str] = set()
    for name in sorted(closure_rules):
        for child in graph[name]:
            if child in closure_rules:
                continue
            status = ports[child]["syntax-status"]
            if status == "unported":
                raise SystemExit(
                    f"{name}: unported child outside Phase 2I closure: {child}"
                )
            if status not in IMPLEMENTED_STATUSES:
                raise SystemExit(f"{name}: invalid external child status for {child}")
            ported_external_rules.add(child)

    return Analysis(
        productions=productions,
        ports=ports,
        graph=graph,
        components=components,
        component_ids=component_ids,
        component_by_rule=component_by_rule,
        unported_components=tuple(unported_components),
        recursive=recursive,
        closure_components=frozenset(closure_components),
        closure_rules=closure_rules,
        ported_external_rules=frozenset(ported_external_rules),
    )


def joined(values: Iterable[str]) -> str:
    ordered = sorted(values)
    return "|".join(ordered) if ordered else "none"


def render_tsv(columns: list[str], rows: list[list[str]]) -> str:
    output = io.StringIO(newline="")
    writer = csv.writer(output, delimiter="\t", lineterminator="\n")
    writer.writerow(columns)
    for row in rows:
        if any("\n" in cell or "\r" in cell or "\t" in cell for cell in row):
            raise SystemExit("generated report contains a multiline or tabbed cell")
        writer.writerow(row)
    return output.getvalue()


def render_scc_report(analysis: Analysis) -> str:
    rows: list[list[str]] = []
    for component in analysis.unported_components:
        outgoing_components: set[str] = set()
        outgoing_ported_rules: set[str] = set()
        for member in component:
            for child in analysis.graph[member]:
                target = analysis.component_by_rule[child]
                if target == component:
                    continue
                if analysis.ports[child]["syntax-status"] == "unported":
                    outgoing_components.add(analysis.component_ids[target])
                else:
                    outgoing_ported_rules.add(child)
        rows.append(
            [
                analysis.component_ids[component],
                str(len(component)),
                str(analysis.recursive[component]).lower(),
                joined(component),
                joined(outgoing_components),
                joined(outgoing_ported_rules),
            ]
        )
    return render_tsv(SCC_COLUMNS, rows)


def render_phase_report(analysis: Analysis) -> str:
    rows: list[list[str]] = []
    for name in sorted(analysis.closure_rules):
        component = analysis.component_by_rule[name]
        same_component: set[str] = set()
        closure_children: set[str] = set()
        ported_external: set[str] = set()
        for child in analysis.graph[name]:
            target = analysis.component_by_rule[child]
            if target == component:
                same_component.add(child)
            elif child in analysis.closure_rules:
                closure_children.add(child)
            elif analysis.ports[child]["syntax-status"] in IMPLEMENTED_STATUSES:
                ported_external.add(child)
            else:
                raise SystemExit(
                    f"{name}: unported child outside Phase 2I closure: {child}"
                )
        rows.append(
            [
                name,
                analysis.ports[name]["family"],
                analysis.component_ids[component],
                str(len(component)),
                str(analysis.recursive[component]).lower(),
                joined(same_component),
                joined(closure_children),
                joined(ported_external),
            ]
        )
    return render_tsv(PHASE_COLUMNS, rows)


def summary(analysis: Analysis) -> str:
    unported_rules = sum(
        1
        for row in analysis.ports.values()
        if row["syntax-status"] == "unported"
    )
    recursive_unported = sum(
        analysis.recursive[component]
        for component in analysis.unported_components
    )
    return "\n".join(
        [
            f"canonical rules: {len(analysis.productions)}",
            f"unported rules: {unported_rules}",
            f"unported SCCs: {len(analysis.unported_components)}",
            f"recursive unported SCCs: {recursive_unported}",
            f"Phase 2I root: {PHASE_ROOT}",
            f"Phase 2I SCCs: {len(analysis.closure_components)}",
            f"Phase 2I rules: {len(analysis.closure_rules)}",
            "ported external dependencies: "
            f"{len(analysis.ported_external_rules)}",
            "unported outgoing dependencies: 0",
        ]
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when checked-in SCC reports differ",
    )
    args = parser.parse_args()

    analysis = analyze()
    generated = {
        SCC_OUTPUT: render_scc_report(analysis).encode("utf-8"),
        PHASE_OUTPUT: render_phase_report(analysis).encode("utf-8"),
    }
    if args.check:
        stale = [
            path
            for path, contents in generated.items()
            if not path.exists() or path.read_bytes() != contents
        ]
        if stale:
            raise SystemExit(
                "recursive-core SCC reports are stale; run:\n"
                "  python3 scripts/generate-canonical-scc-report.py"
            )
        return

    for path, contents in generated.items():
        path.write_bytes(contents)
    print(summary(analysis))


if __name__ == "__main__":
    main()
