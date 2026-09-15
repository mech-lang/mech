#!/usr/bin/env python3
"""Merge serial observational logs; later exact-case reruns replace earlier records.

This records observations and input identity. It does not certify behavior.
Run with the complete log first and optional targeted rerun logs afterward.
"""
import argparse
import collections
import hashlib
import json
import pathlib
import re

D = pathlib.Path(__file__).resolve().parent
R = D.parents[3]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('logs', nargs='+', type=pathlib.Path)
a = p.parse_args()
fixture_path = R / 'tests/fixtures/s8-replacement-audit/semantic-cases.json'
harness_path = R / 'src/runtime/tests/s8_replacement_gap_audit.rs'
fixture_bytes = fixture_path.read_bytes()
harness_bytes = harness_path.read_bytes()

def fingerprint(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
    return f'{value:016x}'

input_identity = {
    'version': 1, 'algorithm': 'fnv1a64',
    'harness_bytes': len(harness_bytes), 'harness_fingerprint': fingerprint(harness_bytes),
    'fixture_bytes': len(fixture_bytes), 'fixture_fingerprint': fingerprint(fixture_bytes),
}
records = {}
for log in a.logs:
    identity_seen = False
    for line in log.read_text().splitlines():
        match = re.search(r'(AUDIT(?:_[A-Z]+)?) (\{.*\})', line)
        if not match:
            continue
        record, raw = match.groups()
        value = json.loads(raw)
        if record == 'AUDIT_IDENTITY':
            assert value == input_identity, f'{log}: stale compiled harness or fixture identity'
            identity_seen = True
            continue
        assert identity_seen, f'{log}: observation precedes execution-time input identity'
        if record == 'AUDIT_SUMMARY':
            continue
        identity = (record, value.get('id', value.get('name', '')), value.get('route', ''))
        records[identity] = {'record': record, 'value': value, 'evidence-log': log.name}
    assert identity_seen, f'{log}: missing execution-time input identity'
fixtures = json.loads(fixture_bytes)
observed = {v['value']['id']: v['value'] for v in records.values() if v['record'] == 'AUDIT'}
assert set(observed) == {f['id'] for f in fixtures}, 'missing/extra semantic observations'
for fixture in fixtures:
    value = observed[fixture['id']]
    assert value['expected_stage'] == fixture.get('expected_rejection_stage', 'pass')
    assert value['contract_status'] == ('pass' if value['stage'] == value['expected_stage'] and (fixture.get('expected_rejection_detail') is None or value['detail'] == fixture['expected_rejection_detail']) else 'fail')
result = list(records.values())
expected_counts = {'AUDIT': len(fixtures), 'AUDIT_ROUTE': 18, 'AUDIT_GRAPH': 1,
                   'AUDIT_BROWSER': 1, 'AUDIT_CATALOG': 120, 'AUDIT_VISIBILITY': 12}
assert dict(collections.Counter(r['record'] for r in result)) == expected_counts, 'incomplete audit record census'
(D / 'observations.json').write_text(json.dumps(result, indent=2) + '\n')
metadata = {
    'production-baseline': '662d29b79df8ab05a25bbadb941a689fd5bd5aae',
    'fixture-sha256': hashlib.sha256(fixture_bytes).hexdigest(),
    'harness-sha256': hashlib.sha256(harness_bytes).hexdigest(),
    'execution-input-identity': input_identity,
    'evidence-logs': [{'name': log.name, 'sha256': hashlib.sha256(log.read_bytes()).hexdigest()} for log in a.logs],
    'records': dict(collections.Counter(r['record'] for r in result)),
    'semantic-stages': dict(collections.Counter(v['stage'] for v in observed.values())),
    'semantic-contract-comparisons': dict(collections.Counter(v['contract_status'] for v in observed.values())),
    'current-target-rejection-matches': sum(v['contract_status'] == 'pass' and bool(v.get('milestone_capability_group')) for v in observed.values()),
    'positive-target-capabilities-executed': sum(bool(v.get('positive_capability_executed')) for v in observed.values()),
    'invalid-original-positive-fixtures': [f['id'] for f in fixtures if f.get('audit_disposition') == 'invalid-original-positive'],
    'interpretation': 'Stage pass is execution, not contract certification. Negative calls can wrongly execute successfully. Invalid original positives remain historical observations, not positive semantic witnesses. Missing expected output values prove equivalence only.'
}
(D / 'observation-metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
print(json.dumps(metadata, indent=2))
