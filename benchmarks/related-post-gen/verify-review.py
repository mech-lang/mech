#!/usr/bin/env python3
"""Verify evidence, its historical compiler, and any current integration inventory."""
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

packet = Path(__file__).resolve().parent
root = packet.parents[1]
manifest = json.loads((packet / 'review/manifest.json').read_text())

def verify(base, name, expected):
    path = (base / name).resolve()
    if not path.is_relative_to(base.resolve()):
        raise SystemExit(f'Invalid manifest path: {name}')
    if not path.is_file():
        raise SystemExit(f'Missing published file: {name}')
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f'Hash mismatch: {name}')

for name, item in manifest['files'].items():
    verify(packet, name, item['sha256'])
    if item.get('compression') == 'gzip':
        assert hashlib.sha256(gzip.decompress((packet/name).read_bytes())).hexdigest() == item['uncompressed_sha256'], name
# The current branch includes newer compiler work. Historical timings belong
# to the recorded Git tree, not to whatever compiler happens to be checked out.
revision = manifest['measured_compiler']
assert len(revision) == 40 and all(c in '0123456789abcdef' for c in revision)
names = list(manifest['compiler_files'])
for name in names:
    assert (root/name).resolve().is_relative_to(root.resolve()), name
objects = subprocess.run(
    ['git', 'cat-file', '--batch'], cwd=root,
    input=''.join(f'{revision}:{name}\n' for name in names).encode(),
    capture_output=True, check=True,
).stdout
at = 0
for name in names:
    end = objects.index(b'\n', at)
    header = objects[at:end].split()
    if len(header) != 3 or header[1] != b'blob':
        raise SystemExit(f'Historical compiler blob unavailable: {revision}:{name}')
    size = int(header[2])
    at = end + 1
    data = objects[at:at+size]
    assert len(data) == size and objects[at+size:at+size+1] == b'\n', name
    assert hashlib.sha256(data).hexdigest() == manifest['compiler_files'][name], name
    at += size + 1
assert at == len(objects)

integration = packet / 'integration/source-inventory.json'
current_files = 0
if integration.exists():
    current = json.loads(integration.read_text())
    for name, expected in current['files'].items():
        verify(root, name, expected)
    current_files = len(current['files'])

report = json.loads((packet / 'results/comparison-native-fixed-frame-arenas-confirmation-20261106-5000-m3.json').read_text())
assert report['summary']['mech-after-4']['median_post_json_seconds'] == 0.003140833
assert report['matched_native_reference']['median_seconds'] == 0.001651875
assert report['whole_process_reference']['variant'] == 'rust-mech-control-streaming'
print(json.dumps(dict(status='verified', evidence_files=len(manifest['files']),
                      historical_compiler_files=len(manifest['compiler_files']),
                      historical_source='recorded_git_tree',
                      current_integration_source_files=current_files,
                      measured_compiler=manifest['measured_compiler'],
                      executes_benchmark=False)))
