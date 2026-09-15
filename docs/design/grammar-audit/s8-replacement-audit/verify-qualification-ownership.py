#!/usr/bin/env python3
"""Check complete qualification ownership and O03 links, not behavioral success."""

import collections
import csv
import json
import pathlib
import re


D = pathlib.Path(__file__).resolve().parent


def rows(name):
    with (D / name).open() as source:
        return list(csv.DictReader(source, delimiter="\t"))


def split(value):
    return [item for item in value.split(";") if item]


def unique(values, description):
    assert len(values) == len(set(values)), ("duplicate", description)
    return set(values)


ownership = rows("qualification-ownership.tsv")
owned = {row["acceptance-cell"]: row for row in ownership}
unique([row["acceptance-cell"] for row in ownership], "qualification cells")
boundaries = set(re.findall(r"^\| ((?:E|R)\d+) ", (D / "PR-STACK.md").read_text(), re.M))
assert boundaries == {f"E{i}" for i in range(1, 12)} | {f"R{i:02}" for i in range(1, 24)}

expected = {}
ledgers = [
    ("compiler", "compiler-acceptance-cells.tsv", "cell-id"),
    ("schema", "schema-acceptance-cells.tsv", "cell-id"),
    ("rule", "rule-acceptance-cells.tsv", "cell-id"),
    ("control", "control-acceptance-cells.tsv", "cell_id"),
    ("consumer", "consumer-acceptance-cells.tsv", "cell-id"),
    ("target", "target-rejection-cells.tsv", "cell-id"),
    ("gap", "gaps.tsv", "gap-id"),
]
for namespace, name, key in ledgers:
    source = rows(name)
    unique([row[key] for row in source], name)
    for row in source:
        expected[f"{namespace}:{row[key]}"] = (name, {row[key]})

catalog = rows("catalog-acceptance-cells.tsv")
unique([f'{row["export"]}#{row["overload-id"]}' for row in catalog], "catalog candidates")
groups = collections.defaultdict(set)
export_groups = collections.defaultdict(set)
for row in catalog:
    groups[row["acceptance-group"]].add(f'{row["export"]}#{row["overload-id"]}')
    export_groups[row["export"]].add(row["acceptance-group"])
for group, candidates in groups.items():
    expected[f"catalog:{group}"] = ("catalog-acceptance-cells.tsv", candidates)
assert len(catalog) == 480 and len(export_groups) == 120 and len(groups) == 34
assert set(owned) == set(expected), ("missing or extra ownership", set(expected) - set(owned), set(owned) - set(expected))

gap_ids = {row["gap-id"] for row in rows("gaps.tsv")}
for cell, row in owned.items():
    ledger, source_ids = expected[cell]
    assert row["source-ledger"] == ledger, cell
    assert unique(split(row["source-ids"]), cell) == source_ids, cell
    primary = row["primary-review-boundary"]
    supporting = unique(split(row["supporting-review-boundaries"]), cell)
    assert primary in boundaries and supporting <= boundaries and primary not in supporting, cell
    assert row["owning-responsibility"].strip(), cell
    assert set(split(row["related-gaps"])) <= gap_ids, cell

observations = rows("semantic-obligations.tsv")
o03 = [row for row in observations if row["open-obligation"] == "O03"]
assert len(o03) == 115
catalog_links = {}
for row in rows("catalog-overloads.tsv"):
    assert set(split(row["acceptance-groups"])) == export_groups[row["export"]], row["export"]
    catalog_links[row["witness"]] = {f"catalog:{group}" for group in export_groups[row["export"]]}

noncatalog_links = {
    "option-update": {"schema:Q08", "schema:QG-Option"},
    "dynamic-payload": {"schema:Q02", "schema:QG-Dynamic"},
    "nominal-atom": {"schema:Q21", "schema:QG-Atom"},
    "reified-kind": {"schema:Q28", "schema:Q33"},
    "reified-inferred-matrix": {"schema:Q28", "schema:Q33"},
    "kind-alias": {"rule:R12"},
    "enum-declaration": {"schema:Q27", "schema:Q32"},
    "comprehension-i32": {"rule:R08", "schema:QT-i32"},
    "comprehension-string": {"rule:R08", "schema:QT-string"},
    "comprehension-composite": {"rule:R08", "schema:QG-Tuple"},
    "comprehension-nested": {"rule:R09"},
    "comprehension-match": {"rule:R09", "schema:Q26"},
    "comprehension-computed-pattern": {"rule:R10"},
    "match-dynamic-result": {"schema:Q02", "schema:Q26"},
}
catalog_count = 0
for row in o03:
    links = unique(split(row["acceptance-cells"]), row["case-id"])
    expected_links = catalog_links.get(row["case-id"], noncatalog_links.get(row["case-id"]))
    assert expected_links and links == expected_links, row["case-id"]
    assert row["oracle"] == "source-bytecode-equivalence-only", row["case-id"]
    assert json.loads(row["expected-json"]) is None, ("O03 requires an independent result oracle", row["case-id"])
    catalog_count += row["case-id"] in catalog_links
assert catalog_count == 101
assert {row["case-id"] for row in o03 if row["case-id"] not in catalog_links} == set(noncatalog_links)

# Earlier control links use unqualified ACT/FSM/FUN IDs. Accept that existing
# convention while requiring all new namespaced links to resolve exactly.
for row in observations:
    for link in split(row["acceptance-cells"]):
        normalized = link if ":" in link else f"control:{link}"
        assert normalized in owned, (row["case-id"], link)

untested = {}
for namespace, ledger in [("compiler", "compiler-acceptance-cells.tsv"), ("schema", "schema-acceptance-cells.tsv")]:
    selected = [row for row in rows(ledger) if row["evidence-status"].startswith("untested")]
    untested[namespace] = len(selected)
    for row in selected:
        assert owned[f'{namespace}:{row["cell-id"]}']["primary-review-boundary"] != "R23", row["cell-id"]
assert untested == {"compiler": 14, "schema": 4}

counts = collections.Counter(cell.split(":")[0] for cell in owned)
print(f"Qualification ownership verified: {len(owned)} relations: {dict(sorted(counts.items()))}.")
print("All 480 catalog candidates map to 34 owned groups; 115 O03 observations link to exact acceptance groups (101 catalog, 14 noncatalog).")
print("All 14 untested compiler and four untested schema constructions have responsibility-specific owners. No behavioral tests were executed; O03 oracles remain open.")
