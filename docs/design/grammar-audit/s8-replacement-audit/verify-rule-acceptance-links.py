#!/usr/bin/env python3
"""Read-only finite rule responsibility accounting. Does not execute Rust tests."""
import csv
import json
import pathlib
import re

D = pathlib.Path(__file__).resolve().parent
ROOT = D.parents[3]

def rows(path):
    with path.open() as stream:
        return list(csv.DictReader(stream, delimiter='\t'))

def unique(values):
    assert len(values) == len(set(values)), 'duplicate identifier'
    return set(values)

def raw_rows(path):
    lines = path.read_text().splitlines()
    names = lines[0].split('\t')
    return [dict(zip(names, line.split('\t'))) for line in lines[1:]]

links = rows(D / 'rule-acceptance-links.tsv')
crosswalk = rows(D / 'rule-crosswalk.tsv')
key = lambda row: (row['inventory'], row['rule'])
assert len(links) == 211
assert unique([key(row) for row in links]) == unique([key(row) for row in crosswalk])
assert sum(row['inventory'] == 'phase-2i' for row in links) == 80
assert sum(row['inventory'] == 's7' for row in links) == 131
assert sum(bool(row['control-acceptance-cells']) for row in links) == 42
rule_cells = rows(D / 'rule-acceptance-cells.tsv')
assert unique([row['cell-id'] for row in rule_cells]) == {f'R{i:02}' for i in range(1, 33)}
allowed = {
    'rule': {row['cell-id'] for row in rule_cells},
    'compiler': {row['cell-id'] for row in rows(D / 'compiler-acceptance-cells.tsv')},
    'schema': {row['cell-id'] for row in rows(D / 'schema-acceptance-cells.tsv')},
    'catalog': {row['acceptance-group'] for row in rows(D / 'catalog-acceptance-cells.tsv')},
    'control': {row['cell_id'] for row in rows(D / 'control-acceptance-cells.tsv')},
}
phase = {row['grammar-name']: row for row in raw_rows(D.parent / 'phase-2i-certification.tsv')}
s7 = {row['grammar-name']: row for row in raw_rows(D.parent / 's7-document-certification.tsv')}
ports = (ROOT / 'src/syntax/src/document/parser/canonical_ports.rs').read_text()
used_rules = {'R01', 'R02'}  # syntax-evidence column, rather than value-owner column
for row in links:
    assert row['owning-layer'] and row['responsibility-class'] and row['coverage-boundary'], key(row)
    names = row['acceptance-cells'].split(';')
    assert names and len(names) == len(set(names)), key(row)
    for name in names:
        namespace, identifier = name.split(':', 1)
        assert identifier in allowed[namespace], (key(row), name)
        if namespace == 'rule':
            used_rules.add(identifier)
    controls = list(filter(None, row['control-acceptance-cells'].split(';')))
    assert all(f'control:{cell}' in names for cell in controls), key(row)
    original = phase.get(row['rule']) if row['inventory'] == 'phase-2i' else s7.get(row['rule'])
    if original:
        assert json.loads(row['accepted-source-json']) == json.loads(original['accepted-source-json']), key(row)
        if row['inventory'] == 'phase-2i':
            for field in ['rejected-source-json', 'recovery-source-json']:
                assert json.loads(row[field]) == json.loads(original[field]), (key(row), field)
    else:
        assert row['declared-disposition'] in {'historical-command', 'outside-document-closure'}, key(row)
        port = re.search(r'RulePort \{\s*name: "' + re.escape(row['rule']) + r'",(?P<body>.*?)\n    \},', ports, re.S)
        assert port and 'activation: RegistryActivationStatus::Inactive' in port.group('body'), key(row)
    if row['declared-disposition'] == 'historical-command':
        for field in ['command-positive-json', 'command-negative-json']:
            witness = json.loads(row[field])
            assert witness['source'] and witness['expected'], (key(row), field)
assert used_rules == allowed['rule'], ('unlinked rule cells', allowed['rule'] - used_rules)

symbols = set()
def check_test_objects(value):
    if isinstance(value, dict):
        if isinstance(value.get('path'), str) and isinstance(value.get('test'), str):
            path, name = value['path'], value['test']
            assert re.search(r'^\s*fn ' + re.escape(name) + r'\(', (ROOT / path).read_text(), re.M), (path, name)
            symbols.add((path, name))
        for child in value.values():
            check_test_objects(child)
    elif isinstance(value, list):
        for child in value:
            check_test_objects(child)
for row in rule_cells:
    assert row['owning-layer'] and row['responsibility'] and row['expected-result'] and row['evidence-status'], row['cell-id']
    for field in ['positive-witness-json', 'negative-or-boundary-witness-json']:
        witness = json.loads(row[field])
        assert witness, (row['cell-id'], field)
        check_test_objects(witness)
print(f'Rule accounting verified: 211 unique inventory links, 32 rule responsibilities, {len(symbols)} existing exact-test symbols, 19 inactive entries. No Rust tests executed; no runtime success claimed.')
