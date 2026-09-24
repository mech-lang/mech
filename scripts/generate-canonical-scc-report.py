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

PORTS = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/ports.tsv"
)

SCC_OUTPUT = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/inactive-sccs.tsv"
)

PHASE_OUTPUT = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/phase-2i-recursive-core.tsv"
)

DEPENDENCIES = (
    REPOSITORY_ROOT
    / "docs/design/grammar-audit/canonical-dependencies.tsv"
)

PORT_COLUMNS = [
    "grammar-name",
    "family",
    "syntax-status",
    "semantic-status",
    "activation-status",
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
    "outgoing-inactive-components",
    "outgoing-active-rules",
]

PHASE_COLUMNS = [
    "grammar-name",
    "family",
    "component-id",
    "component-size",
    "recursive-component",
    "same-component-children",
    "closure-children",
    "active-external-children",
]

CERTIFIED_STATUSES = {"certified"}
SEMANTIC_STATUSES = {"pending", "syntax-only", "certified"}
ACTIVATION_STATUSES = {"inactive", "candidate", "active"}
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
    ports: dict[str, dict[str, str]]
    graph: dict[str, tuple[str, ...]]
    components: tuple[tuple[str, ...], ...]
    component_ids: dict[tuple[str, ...], str]
    component_by_rule: dict[str, tuple[str, ...]]
    inactive_components: tuple[tuple[str, ...], ...]
    recursive: dict[tuple[str, ...], bool]
    closure_components: frozenset[tuple[str, ...]]
    closure_rules: frozenset[str]
    active_external_rules: frozenset[str]


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
    dict[str, tuple[str, ...]],
]:
    port_rows = read_tsv(PORTS, PORT_COLUMNS)
    if len(port_rows) != EXPECTED_RULES:
        raise SystemExit(
            f"expected {EXPECTED_RULES} port rows, found {len(port_rows)}"
        )
    port_names = [row["grammar-name"] for row in port_rows]
    if len(set(port_names)) != EXPECTED_RULES:
        raise SystemExit("ports.tsv contains duplicate grammar-name entries")
    ports = {row["grammar-name"]: row for row in port_rows}

    for name, row in ports.items():
        status = row["syntax-status"]
        if status == "unported":
            if row["phase"]:
                raise SystemExit(f"{name}: unported rule has phase {row['phase']}")
            if row["node-policy"] != "undecided":
                raise SystemExit(
                    f"{name}: unported rule has node policy {row['node-policy']}"
                )
            if row["semantic-status"] != "pending":
                raise SystemExit(
                    f"{name}: unported rule has semantic status "
                    f"{row['semantic-status']}"
                )
        elif status in CERTIFIED_STATUSES:
            if not row["phase"]:
                raise SystemExit(f"{name}: certified rule has no phase")
            if row["semantic-status"] == "pending":
                raise SystemExit(f"{name}: certified rule has pending semantics")
        else:
            raise SystemExit(f"{name}: unknown syntax status {status}")
        if row["semantic-status"] not in SEMANTIC_STATUSES:
            raise SystemExit(
                f"{name}: unknown semantic status {row['semantic-status']}"
            )
        activation = row["activation-status"]
        if activation not in ACTIVATION_STATUSES:
            raise SystemExit(f"{name}: unknown activation status {activation}")
        if status == "unported" and activation != "inactive":
            raise SystemExit(f"{name}: unported rule is not inactive")
        if status == "certified" and activation == "inactive":
            raise SystemExit(f"{name}: certified rule is inactive")
        if activation == "candidate" and row["phase"] != "2I":
            raise SystemExit(f"{name}: only Phase 2I may be an activation candidate")

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
    if set(dependency_names) != set(ports):
        raise SystemExit("canonical dependency and port name sets differ")

    graph: dict[str, tuple[str, ...]] = {}
    reported_parents: dict[str, tuple[str, ...]] = {}
    canonical_names = set(ports)
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
    return ports, graph


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
    ports, graph = load_inputs()
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

    inactive_components: list[tuple[str, ...]] = []
    for component in components:
        inactive = tuple(
            member
            for member in component
            if ports[member]["activation-status"] != "active"
        )
        active = tuple(
            member
            for member in component
            if ports[member]["activation-status"] == "active"
        )
        if inactive and active:
            raise SystemExit(
                "mixed activation-status SCC:\n"
                f"  members: {', '.join(component)}\n"
                f"  inactive: {', '.join(inactive)}\n"
                f"  active: {', '.join(active)}"
            )
        if inactive:
            inactive_components.append(component)

    closure_rules = frozenset(
        name for name, row in ports.items() if row["phase"] == "2I"
    )
    if PHASE_ROOT not in closure_rules:
        raise SystemExit(f"Phase 2I certification does not contain {PHASE_ROOT}")
    closure_components = {
        component_by_rule[name] for name in closure_rules
    }
    component_members = frozenset(
        member for component in closure_components for member in component
    )
    if component_members != closure_rules:
        raise SystemExit("Phase 2I certification splits a canonical SCC")
    for name in sorted(closure_rules):
        row = ports[name]
        if row["syntax-status"] != "certified":
            raise SystemExit(f"{name}: Phase 2I member is not certified")
        if row["semantic-status"] != "certified":
            raise SystemExit(f"{name}: Phase 2I semantics are not certified")
        if row["activation-status"] not in {"candidate", "active"}:
            raise SystemExit(f"{name}: invalid Phase 2I activation status")
        forbidden = row["family"]
        if forbidden in FORBIDDEN_FAMILIES:
            raise SystemExit(
                f"{name}: forbidden Phase 2I family {forbidden}; "
                "inspect canonical grammar metadata"
            )

    phase_activation = {
        ports[name]["activation-status"] for name in closure_rules
    }
    if len(phase_activation) != 1:
        raise SystemExit("Phase 2I activation must change atomically")

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

    active_external_rules: set[str] = set()
    for name in sorted(closure_rules):
        for child in graph[name]:
            if child in closure_rules:
                continue
            status = ports[child]["syntax-status"]
            if ports[child]["activation-status"] != "active":
                raise SystemExit(
                    f"{name}: inactive child outside Phase 2I closure: {child}"
                )
            if status not in CERTIFIED_STATUSES:
                raise SystemExit(f"{name}: invalid external child status for {child}")
            active_external_rules.add(child)

    return Analysis(
        ports=ports,
        graph=graph,
        components=components,
        component_ids=component_ids,
        component_by_rule=component_by_rule,
        inactive_components=tuple(inactive_components),
        recursive=recursive,
        closure_components=frozenset(closure_components),
        closure_rules=closure_rules,
        active_external_rules=frozenset(active_external_rules),
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
    for component in analysis.inactive_components:
        outgoing_components: set[str] = set()
        outgoing_active_rules: set[str] = set()
        for member in component:
            for child in analysis.graph[member]:
                target = analysis.component_by_rule[child]
                if target == component:
                    continue
                if analysis.ports[child]["activation-status"] != "active":
                    outgoing_components.add(analysis.component_ids[target])
                else:
                    outgoing_active_rules.add(child)
        rows.append(
            [
                analysis.component_ids[component],
                str(len(component)),
                str(analysis.recursive[component]).lower(),
                joined(component),
                joined(outgoing_components),
                joined(outgoing_active_rules),
            ]
        )
    return render_tsv(SCC_COLUMNS, rows)


def render_phase_report(analysis: Analysis) -> str:
    rows: list[list[str]] = []
    for name in sorted(analysis.closure_rules):
        component = analysis.component_by_rule[name]
        same_component: set[str] = set()
        closure_children: set[str] = set()
        active_external: set[str] = set()
        for child in analysis.graph[name]:
            target = analysis.component_by_rule[child]
            if target == component:
                same_component.add(child)
            elif child in analysis.closure_rules:
                closure_children.add(child)
            elif analysis.ports[child]["activation-status"] == "active":
                active_external.add(child)
            else:
                raise SystemExit(
                    f"{name}: inactive child outside Phase 2I closure: {child}"
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
                joined(active_external),
            ]
        )
    return render_tsv(PHASE_COLUMNS, rows)


def summary(analysis: Analysis) -> str:
    unported_rules = sum(
        1
        for row in analysis.ports.values()
        if row["syntax-status"] == "unported"
    )
    recursive_inactive = sum(
        analysis.recursive[component]
        for component in analysis.inactive_components
    )
    return "\n".join(
        [
            f"canonical rules: {len(analysis.ports)}",
            f"unported rules: {unported_rules}",
            f"inactive SCCs: {len(analysis.inactive_components)}",
            f"recursive inactive SCCs: {recursive_inactive}",
            f"Phase 2I root: {PHASE_ROOT}",
            f"Phase 2I SCCs: {len(analysis.closure_components)}",
            f"Phase 2I rules: {len(analysis.closure_rules)}",
            "active external dependencies: "
            f"{len(analysis.active_external_rules)}",
            "inactive outgoing dependencies: 0",
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
