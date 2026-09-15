#!/usr/bin/env python3
"""Check finite API/schema responsibility accounting, not behavioral success."""
import collections, csv, hashlib, json, pathlib, re, subprocess
D = pathlib.Path(__file__).resolve().parent
R = D.parents[3]
def rows(name):
    with (D / name).open() as f:
        return list(csv.DictReader(f, delimiter='\t'))
def unique(values):
    assert len(values) == len(set(values)), 'duplicate identifiers'
    return set(values)
def source_at(head, path):
    # Audit-only harness is not production source at the frozen implementation head.
    if path.endswith(('s8_replacement_gap_audit.rs', 's8_replacement_schema_audit.rs')):
        return (R / path).read_text()
    result = subprocess.run(['git', 'show', f'{head}:{path}'], cwd=R, capture_output=True, text=True)
    assert result.returncode == 0, (head, path, result.stderr)
    return result.stdout
compiler = rows('compiler-acceptance-cells.tsv')
schemas = rows('schema-acceptance-cells.tsv')
for cells in [compiler, schemas]:
    unique([c['cell-id'] for c in cells])
    for cell in cells:
        assert cell['responsibility'] and cell['expected-result'], cell['cell-id']
        assert cell.get('fixture-or-construction', cell.get('source-or-value-construction')), cell['cell-id']
        if cell['witness-test']:
            path = cell['witness-source'].split(':')[0]
            src = source_at(cell['evidence-head'], path)
            assert re.search(r'^\s*fn ' + re.escape(cell['witness-test']) + r'\(', src, re.M), cell
            assert cell['witness-command'].endswith(' -- --exact'), cell['cell-id']
        if cell['evidence-status'].startswith('untested'):
            assert not cell['witness-test'], ('untested cell claiming existing test', cell['cell-id'])
for file, keys in [('compiler-methods.tsv', compiler), ('frontend-apis.tsv', compiler), ('compiler-internal-entrypoints.tsv', compiler), ('schema-families.tsv', schemas)]:
    allowed = {c['cell-id'] for c in keys}
    for row in rows(file):
        linked = row['acceptance-cells'].split(';')
        assert linked and all(c in allowed for c in linked), (file, row)
        if 'remaining-acceptance-cells' in row:
            assert set(filter(None, row['remaining-acceptance-cells'].split(';'))) <= set(linked)
# The internal entrances are extra responsibilities, not fabricated public API count.
assert {r['method'] for r in rows('compiler-internal-entrypoints.tsv')} == {
    'compile_interactive_source', 'compile_resolved_root', 'compile_interactive_resolved_root'}
assert {r['qualified-owner'] for r in rows('frontend-apis.tsv')} == {'CanonicalSourceFrontend', 'CanonicalSourceProgram'}
body = (R / 'src/core/src/schema/mod.rs').read_text().split('pub enum SchemaBody {',1)[1].split('\n}\n',1)[0]
variants = re.findall(r'^    ([A-Z]\w+)(?:,|\(| \{)',body,re.M)
assert unique([r['schema'] for r in rows('schema-families.tsv')]) == set(variants)
observations = json.loads((D / 'schema-observations.json').read_text())
assert observations['audit-working-tree-test-sha256'] == hashlib.sha256(
    (R / observations['audit-working-tree-test']).read_bytes()).hexdigest()
recorded = {row['cell-id']: row for row in observations['cases']}
expected_cells = {row['cell-id']: row for row in schemas if row['cell-id'].startswith(('QT-', 'QG-', 'QN-'))}
assert set(recorded) == set(expected_cells) and len(recorded) == 31
for cell_id, row in recorded.items():
    assert row['test'] == expected_cells[cell_id]['witness-test'], cell_id
    if row['gap']:
        assert row['gap'] in expected_cells[cell_id]['related-gaps'].split(';'), cell_id
assert collections.Counter(row['strict-test-result'] for row in recorded.values()) == {'pass': 7, 'fail': 24}
assert collections.Counter(row['gap'] for row in recorded.values()) == {None: 7, 'G25': 23, 'G26': 1}
assert observations['publication-phase-passes'] == dict(collections.Counter(
    phase for row in recorded.values() for phase in row['completed-publication-phases']))
print(f'Acceptance accounting verified: {len(compiler)} compiler/frontend cells and {len(schemas)} schema cells; all method/schema links and existing exact-test symbols resolve. No tests were executed by this check.')
