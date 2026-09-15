#!/usr/bin/env python3
"""Check exact extraction ownership/provenance; never treat it as a semantic seal."""
import collections
import csv
import hashlib
import json
from pathlib import Path
import subprocess

D = Path(__file__).resolve().parent
R = D.parents[3]

def git(*args):
    return subprocess.check_output(['git', *args], cwd=R)

def git_text(*args):
    return git(*args).decode().strip()

def sha(data):
    return hashlib.sha256(data).hexdigest()

result = json.loads((D / 'extraction-results.json').read_text())
ledger = D / result['ledger']
assert sha(ledger.read_bytes()) == result['ledger-sha256']
with ledger.open() as file:
    rows = list(csv.DictReader(file, delimiter='\t'))
by_path = {row['path']: row for row in rows}
base = result['extraction-base']
frozen = result['frozen-production-reference']
assert len(rows) == len(by_path) == 91
assert set(by_path) == set(git_text('diff', '--name-only', base, frozen).splitlines())
actual = collections.defaultdict(list)
previous = base
for entry in result['slices']:
    head = entry['head']
    assert entry['base'] == previous
    subprocess.run(['git', 'merge-base', '--is-ancestor', previous, head], cwd=R, check=True)
    paths = git_text('diff', '--name-only', previous, head).splitlines()
    assert paths == entry['changed-paths'] and len(paths) == entry['changed-path-count']
    for path in paths:
        actual[path].append(entry['slice'])
        expected_hash = sha(git('diff', '--binary', previous, head, '--', path))
        hashes = dict(part.split(':', 1) for part in by_path[path]['actual-patch-sha256s'].split(';'))
        assert hashes[entry['slice']] == expected_hash
    for check in entry['validation']:
        assert check['head'] == head
        if check['status'] == 'completed-pass' and check['kind'] == 'cargo-test':
            assert check['completed-targets']
            assert all(target['passed'] > 0 and target['failed'] == 0 for target in check['completed-targets'])
        if check['kind'] == 'cargo-check':
            assert check['tests-executed'] is False
    if 'provenance-check' in entry:
        proof = entry['provenance-check']
        assert proof['head'] == head
        assert sha((R / proof['manifest']).read_bytes()) == proof['manifest-sha256']
    previous = head
assert previous == result['extracted-head']
for path, row in by_path.items():
    assert row['actual-slices'].split(';') == actual[path]
    assert row['extraction-pr'] == row['actual-slices']
    assert not row['remaining-owner'] and not row['remaining-patch-sha256']
    assert row['at-extracted-head-status'] == 'equals-frozen'
    assert row['frozen-blob'] == git_text('rev-parse', frozen + ':' + path)
    assert row['at-extracted-head-blob'] == row['frozen-blob']
assert result['remaining-path-count'] == 0
assert git_text('rev-parse', previous + '^{tree}') == git_text('rev-parse', frozen + '^{tree}')
assert result['extracted-tree-oid'] == result['frozen-tree-oid']
print('Verified 91 paths across 11 exact extraction slices; all per-path patches and final frozen-tree equality match. Cargo records are named-head evidence, not a milestone seal.')
