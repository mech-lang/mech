#!/usr/bin/env python3
"""Verify frozen coverage accounting and evidence consistency, not readiness."""
import collections
import csv
import hashlib
import json
import pathlib
import re

D = pathlib.Path(__file__).resolve().parent
R = D.parents[3]

def tsv(path):
    with path.open() as f:
        return list(csv.DictReader(f, delimiter='\t'))

def exact(rows, key, expected):
    actual = [row[key] for row in rows]
    assert len(actual) == len(set(actual)), f'duplicate {key}'
    assert set(actual) == set(expected), (key, set(actual) ^ set(expected))

gaps = tsv(D / 'gaps.tsv')
exact(gaps, 'gap-id', [f'G{i:02}' for i in range(2, 27)])
allowed_gaps = {row['gap-id'] for row in gaps}
fixture_path = R / 'tests/fixtures/s8-replacement-audit/semantic-cases.json'
cases = json.loads(fixture_path.read_text())
sem = tsv(D / 'semantic-obligations.tsv')
exact(sem, 'case-id', [case['id'] for case in cases])
observations = json.loads((D / 'observations.json').read_text())
obs = [r['value'] for r in observations if r['record'] == 'AUDIT']
exact(obs, 'id', [case['id'] for case in cases])
by_id = {r['id']: r for r in obs}
by_case = {r['id']: r for r in cases}
for row in sem:
    observed = by_id[row['case-id']]
    case = by_case[row['case-id']]
    assert row['observed-stage'] == observed['stage']
    assert row['expected-stage'] == case.get('expected_rejection_stage', 'pass')
    expected_comparison = 'pass' if row['observed-stage'] == row['expected-stage'] and (case.get('expected_rejection_detail') is None or observed['detail'] == case['expected_rejection_detail']) else 'fail'
    assert row['contract-comparison'] == observed['contract_status'] == expected_comparison
    if expected_comparison != 'pass' and row['disposition'] != 'invalid-original-positive':
        assert row['gap-id'] in allowed_gaps, row
    if row['gap-id']:
        assert row['gap-id'] in allowed_gaps
    if row['oracle'] == 'source-bytecode-equivalence-only':
        assert row['open-obligation'] == 'O03'
    if row['oracle'] == 'invalid-original-positive':
        assert not row['gap-id'], 'invalid positive cannot prove a missing implementation'
    assert f"MECH_AUDIT_CASE={row['case-id']} " in row['witness']
    if case.get('milestone_capability_group'):
        assert row['gap-id'] and row['milestone-positive-witness']
        assert case.get('scope_exclusion_accepted') is False
metadata = json.loads((D / 'observation-metadata.json').read_text())
assert metadata['fixture-sha256'] == hashlib.sha256(fixture_path.read_bytes()).hexdigest()
assert metadata['harness-sha256'] == hashlib.sha256((R / 'src/runtime/tests/s8_replacement_gap_audit.rs').read_bytes()).hexdigest()
assert metadata['records'] == {'AUDIT': len(cases), 'AUDIT_ROUTE': 18, 'AUDIT_GRAPH': 1,
                               'AUDIT_BROWSER': 1, 'AUDIT_CATALOG': 120, 'AUDIT_VISIBILITY': 12}
assert metadata['semantic-stages'] == dict(collections.Counter(r['stage'] for r in obs))
assert metadata['semantic-contract-comparisons'] == dict(collections.Counter(r['contract_status'] for r in obs))
for gap in gaps:
    for case in filter(None, gap['semantic-witnesses'].split(';')):
        assert any(s['case-id'] == case and s['gap-id'] == gap['gap-id'] for s in sem)
    assert gap['semantic-witnesses'] or gap['additional-executable-witnesses'], gap

consumers = tsv(D / 'consumer-contracts.tsv')
frozen_consumers = tsv(D.parent / 'source-parser-consumers.tsv')
exact(consumers, 'consumer-id', [x['consumer-id'] for x in frozen_consumers])
assert len(consumers) == 27
consumer_cells = tsv(D / 'consumer-acceptance-cells.tsv')
exact(consumer_cells, 'cell-id', [x['cell-id'] for x in consumer_cells])
assert len(consumer_cells) == 54
for row in consumers:
    linked = [c for c in consumer_cells if c['consumer-id'] == row['consumer-id']]
    assert len(linked) == 2
    assert set(row['acceptance-cells'].split(';')) == {c['cell-id'] for c in linked}

rules = tsv(D / 'rule-crosswalk.tsv')
assert len(rules) == 211
for inventory, source in [('phase-2i', 'phase-2i-certification.tsv'), ('s7', 's7-dispositions.tsv')]:
    exact([r for r in rules if r['inventory'] == inventory], 'rule', [x['grammar-name'] for x in tsv(D.parent / source)])
methods = re.findall(r'^    pub fn (\w+)', (R / 'src/runtime/src/runtime/program/compiler.rs').read_text(), re.M)
exact(tsv(D / 'compiler-methods.tsv'), 'method', methods)
assert len(methods) == 36
frontend = re.findall(r'^    pub fn (\w+)', (R / 'src/engine/src/source_semantics/frontend.rs').read_text(), re.M)
exact(tsv(D / 'frontend-apis.tsv'), 'method', frontend)
assert len(frontend) == 24
catalog = [r['value']['name'] for r in observations if r['record'] == 'AUDIT_CATALOG']
exact(tsv(D / 'catalog-overloads.tsv'), 'export', catalog)
assert len(catalog) == 120
signatures = tsv(D / 'catalog-signatures.tsv')
candidate_cells = tsv(D / 'catalog-acceptance-cells.tsv')
keys = lambda rows: {(r['export'], r['overload-id']) for r in rows}
assert len(signatures) == len(candidate_cells) == len(keys(signatures)) == len(keys(candidate_cells)) == 480
assert keys(signatures) == keys(candidate_cells)
assert {r['export'] for r in signatures} == set(catalog)
assert len({r['acceptance-group'] for r in candidate_cells}) == 34
for row in candidate_cells:
    assert all(row[k] for k in ['candidate-layout', 'kind-domain', 'configured-target-domain', 'source-exposure', 'source-recipe-json', 'boundary-cells', 'reference-oracle', 'implementation-authority'])
    assert isinstance(json.loads(row['source-recipe-json']), str)
for name, key, count in [('scalar-types.tsv', 'kind', 17), ('schema-families.tsv', 'schema', 20)]:
    rows = tsv(D / name)
    assert len(rows) == len({r[key] for r in rows}) == count
paths = tsv(D / 'patch-ownership.tsv')
assert len(paths) == len({r['path'] for r in paths}) == 91
print(f'Accounting verified: {len(sem)} source observations, {len(gaps)} groups, 54 consumer cells, 36+3 compiler entrances, 24 frontend/program methods, 211 rule rows, 480 catalog candidates/34 families, 17 scalar kinds, 20 schema families and91 patch paths. This is not an implementation seal.')
