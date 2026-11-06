#!/usr/bin/env python3
"""Verify published evidence and the unchanged measured compiler source."""
import gzip
import hashlib
import json
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
for name, expected in manifest['compiler_files'].items():
    verify(root, name, expected)

report = json.loads((packet / 'results/comparison-native-fixed-frame-arenas-confirmation-20261106-5000-m3.json').read_text())
assert report['summary']['mech-after-4']['median_post_json_seconds'] == 0.003140833
assert report['matched_native_reference']['median_seconds'] == 0.001651875
assert report['whole_process_reference']['variant'] == 'rust-mech-control-streaming'
print(json.dumps(dict(status='verified', evidence_files=len(manifest['files']),
                      unchanged_compiler_files=len(manifest['compiler_files']),
                      measured_compiler=manifest['measured_compiler'],
                      executes_benchmark=False)))
