#!/usr/bin/env python3
"""Check retained bytes, pinned sources and complete post-JSON comparisons."""
from pathlib import Path
import gzip
import hashlib
import json
import math
import statistics
import subprocess

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
report = json.loads((D / 'report.json').read_text())
manifest = json.loads((D / 'records.json').read_text())
records = {}
checked = set()
for record in manifest['records']:
    path = D / record['path']
    assert path.resolve().is_relative_to(D)
    if record['path'] not in checked:
        packed = path.read_bytes()
        assert hashlib.sha256(packed).hexdigest() == record['compressed_sha256']
        assert len(packed) == record['compressed_bytes']
        data = gzip.decompress(packed)
        assert hashlib.sha256(data).hexdigest() == record['uncompressed_sha256']
        assert len(data) == record['uncompressed_bytes']
        checked.add(record['path'])
    records[record['source']] = record


def blob(name):
    return gzip.decompress((D / records[name]['path']).read_bytes())


def read(name):
    return json.loads(blob(name))


def close(a, b):
    assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12), (a, b)


def post_json(sample):
    if 'post_json_seconds' in sample:
        return sample['post_json_seconds']
    if sample['variant'].startswith('mech'):
        for line in sample['stdout'].splitlines():
            try:
                record = json.loads(line)
            except ValueError:
                continue
            if 'post_json_seconds' in record:
                return record['post_json_seconds']
    return sample['seconds']


gold = blob('publication-inputs/gold.json')
assert hashlib.sha256(gold).hexdigest() == report['gold_sha256']
assert len(json.loads(gold)) == 5000
assert hashlib.sha256(blob('publication-inputs/posts.json')).hexdigest() == report['input_sha256']
latency = read(report['primary_cohort'])
resources = read(report['resource_cohort'])
for cohort, expected in [(latency, report['latency_exact_outputs']),
                         (resources, report['resource_exact_outputs'])]:
    assert len(cohort['samples']) == expected
    assert all(s['exit'] == 0 and s['exact_rust_output'] and
               s['output_sha256'] == report['gold_sha256'] for s in cohort['samples'])
medians = {
    name: statistics.median(post_json(s) for s in latency['samples'] if s['variant'] == name)
    for name in latency['summary']
}
fastest = min(latency['primary_reference']['eligible_variants'], key=medians.__getitem__)
assert fastest == report['Rust_reference']
for row in report['rows']:
    name = row['implementation']
    close(row['post_json_ms'], medians[name] * 1000)
    close(row['times_fastest_Rust'], medians[name] / medians[fastest])
    close(row['whole_process_ms'], statistics.median(
        s['wall_seconds'] for s in latency['samples'] if s['variant'] == name) * 1000)
    if 'resources' in row:
        resource_name = name if name == 'mech-ir320-compact-native-4' else name + '-instrumented'
        samples = [s for s in resources['samples'] if s['variant'] == resource_name]
        assert len(samples) == 7
        for field in ['allocation_requests', 'requested_allocation_bytes',
                      'peak_additional_live_requested_heap_bytes']:
            close(row['resources'][field], statistics.median(s['measurement'][field] for s in samples))
        for field in ['instructions_retired', 'cycles']:
            close(row['resources']['hardware'][field], statistics.median(
                s['measurement']['hardware'][field] for s in samples))
        assert all(s['measurement']['allocator_accounting_underflows'] == 0 and
                   s['measurement']['allocator_lifetime_accounting_underflows'] == 0 for s in samples)

assert blob('baseline/build/object.o') == blob('host-v2-4/build/object.o')
assert hashlib.sha256(blob('host-v2-4/build/object.o')).hexdigest() == report['native_Object_sha256']
assert read('host-v2-4/qualify-5000.json')['exit'] == 0
assert read('host-v2-4/qualify-edges.json')['exit'] == 0
assert '"cases": 31' in read('host-v2-4/qualify-edges.json')['stdout']
assert read('host-v2-core-contracts.json')['exit'] == 0
assert '397 passed' in read('host-v2-core-contracts.json')['stdout']
assert read('native-failure-attribution.json')['unchanged_baseline_reproduces_all_failures']


def verify_source(commit, files):
    names = sorted(files)
    result = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT,
                            input=''.join(f'{commit}:{name}\n' for name in names).encode(),
                            capture_output=True, check=True).stdout
    offset = 0
    for name in names:
        end = result.index(b'\n', offset)
        header = result[offset:end].split()
        assert len(header) == 3 and header[1] == b'blob', (name, header)
        length = int(header[2])
        offset = end + 1
        assert hashlib.sha256(result[offset:offset + length]).hexdigest() == files[name], name
        offset += length
        assert result[offset:offset + 1] == b'\n'
        offset += 1
    assert offset == len(result)
    return len(names)


source_count = verify_source(report['compiler_source_commit'],
                             read(report['production_source_inventory'])['files'])
baseline_count = verify_source(report['baseline_source_commit'],
                               read('coherent-baseline/source-inventory.json')['files'])
verify_source(report['compiler_source_commit'], {
    report['selected_adapter']: report['selected_adapter_sha256'],
    report['benchmark_source']: report['benchmark_source_sha256'],
})
ratio = medians[report['candidate']] / medians[fastest]
assert report['target_met'] == (ratio <= 1)
print(json.dumps(dict(verified_records=len(records), unique_blobs=len(checked),
                     candidate_source_files=source_count, baseline_source_files=baseline_count,
                     exact_primary_outputs=len(latency['samples']),
                     exact_resource_outputs=len(resources['samples']),
                     managed_ms=medians[report['candidate']] * 1000,
                     fastest_Rust_ms=medians[fastest] * 1000,
                     times_fastest_Rust=ratio, managed_target_met=ratio <= 1), indent=2))
