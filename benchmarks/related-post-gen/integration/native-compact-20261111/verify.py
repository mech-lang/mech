#!/usr/bin/env python3
"""Verify published records and independently reconstruct the Rust comparison.

Uses only Python's standard library. Does not execute benchmark programs.
"""

from collections import defaultdict
from pathlib import Path
import gzip
import hashlib
import json
import math
import statistics
import subprocess

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
records = {}
count = 0


def digest(data):
    return hashlib.sha256(data).hexdigest()


for manifest_name in ('records.json', 'records-ir320.json'):
    manifest = json.loads((D / manifest_name).read_text())
    for record in manifest['records']:
        path = D / record['path']
        assert path.resolve().is_relative_to(D), path
        compressed = path.read_bytes()
        assert digest(compressed) == record['compressed_sha256'], path
        data = gzip.decompress(compressed)
        assert digest(data) == record['uncompressed_sha256'], path
        assert len(data) == record['uncompressed_bytes'], path
        if manifest_name == 'records-ir320.json':
            assert len(compressed) == record['compressed_bytes'], path
            records[record['source']] = data
        count += 1


def read(name):
    return json.loads(records[name])


def close(actual, expected):
    assert math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-12), (actual, expected)


report = json.loads((D / 'latest.json').read_text())
gold_sha = '470263449577b9b386d47cf81a5b7ed6ec4155eabc8bb31709f55e2e3634f6fc'
assert digest(records[f'publication-inputs/{gold_sha}.output.json']) == gold_sha
assert digest(records['publication-inputs/posts.json']) == report['input_sha256']
source = ROOT / report['benchmark_source']
assert digest(source.read_bytes()) == report['source_sha256']

lat = read('first-v30/latency.json')
res = read('first-v30/resources.json')
ind = read('independent-v30/report.json')
eligible = lat['primary_reference']['eligible_variants']
candidate = 'mech-ir320-compact-native-4'
all_exact = 0


def reconstruct(cohort, expected_count, independent=False):
    global all_exact
    samples = cohort['samples']
    assert len(samples) == expected_count
    times = defaultdict(list)
    for sample in samples:
        assert sample['exit'] == 0
        assert sample['exact_full_output' if independent else 'exact_rust_output']
        assert sample['output_sha256'] == gold_sha
        seconds = sample.get('post_json_seconds', sample.get('seconds'))
        if seconds is not None:
            times[sample['variant']].append(seconds)
        all_exact += 1
    return {name: statistics.median(values) for name, values in times.items()}


medians = reconstruct(lat, 744)
reconstruct(res, 175)
independent = reconstruct(ind, 312, independent=True)
fastest = min(eligible, key=medians.__getitem__)
fastest_independent = min(eligible, key=independent.__getitem__)
assert fastest == lat['primary_reference']['variant']
assert fastest_independent == ind['fastest_eligible_native']
close(medians[fastest], lat['primary_reference']['median_seconds'])

for row in report['rows']:
    name = row['implementation']
    close(row['post_json_ms'], medians[name] * 1000)
    close(row['times_fastest_eligible_Rust'], medians[name] / medians[fastest])
    samples = [s for s in lat['samples'] if s['variant'] == name]
    close(row['whole_process_ms'], statistics.median(s['wall_seconds'] for s in samples) * 1000)
    resource_name = name if name == candidate else name + '-instrumented'
    resources = [s for s in res['samples'] if s['variant'] == resource_name]
    assert len(resources) == 7
    observed = row['resources']
    for field, raw in (
        ('allocation_requests', 'allocation_requests'),
        ('requested_allocator_bytes', 'requested_allocation_bytes'),
        ('peak_additional_requested_allocator_heap_bytes', 'peak_additional_live_requested_heap_bytes'),
    ):
        close(observed[field], statistics.median(s['measurement'][raw] for s in resources))
    for field, raw in (('instructions', 'instructions_retired'), ('cycles', 'cycles')):
        close(observed[field], statistics.median(s['measurement']['hardware'][raw] for s in resources))
    assert all(s['measurement']['allocator_accounting_underflows'] == 0 for s in resources)
    assert all(s['measurement']['allocator_lifetime_accounting_underflows'] == 0 for s in resources)

ratio = medians[candidate] / medians[fastest]
independent_ratio = independent[candidate] / independent[fastest_independent]
assert ratio <= 1 and independent_ratio <= 1
close(independent_ratio, report['independent_confirmation']['times_fastest_eligible_Rust'])
close(independent[candidate] * 1000, report['independent_confirmation']['candidate_ms'])
close(independent[fastest_independent] * 1000, report['independent_confirmation']['Rust_ms'])

for name, expected in (('first-v25/latency.json', 744), ('first-v25/resources.json', 175),
                       ('first-v27/latency.json', 744), ('first-v27/resources.json', 175)):
    reconstruct(read(name), expected)

output_index = read('publication-inputs/output-content-index.json')['outputs']
assert len(output_index) == all_exact
assert all(r['output_sha256'] == gold_sha for r in output_index)

for artifact in ('original', 'decoded'):
    measured = records[f'exact-source-v30/{artifact}.o']
    assert measured == records[f'exact-source-v31/{artifact}.o']
    assert digest(measured) == report['native_size']['Object_sha256']
    assert read(f'qualification-v30/{artifact}-5000-exact.json')['exact']
    edges = read(f'qualification-v30/{artifact}-edges.json')
    assert edges['exit'] == 0 and '"cases": 31' in edges['stdout']
    imports = read(f'qualification-v30/{artifact}-imports.json')
    assert imports['exit'] == 0
    assert set(imports['stdout'].split()) == {'_' + n for n in report['implementation']['native_Object_imports']}

contracts = read('final-v31-contracts.json')
assert contracts['exit'] == 0 and contracts['passed'] == 589
assert contracts['all583_prior_names_retained'] and not contracts['missing']
assert read('format-check-v31.json')['exit'] == 0
inventory = read('compiler-v31/source-inventory.json')
# This packet describes a completed historical compiler build. Subsequent
# compiler work must not invalidate its evidence or be represented as measured.
source_commit = report['compiler_source_commit']
requests = sorted(inventory['files'])
batch = subprocess.run(
    ['git', 'cat-file', '--batch'], cwd=ROOT,
    input=''.join(f'{source_commit}:{name}\n' for name in requests).encode(),
    capture_output=True, check=True,
).stdout
offset = 0
for name in requests:
    end = batch.index(b'\n', offset)
    header = batch[offset:end].split()
    assert len(header) == 3 and header[1] == b'blob', (name, header)
    length = int(header[2])
    offset = end + 1
    data = batch[offset:offset + length]
    assert digest(data) == inventory['files'][name], name
    offset += length
    assert batch[offset:offset + 1] == b'\n', name
    offset += 1
assert offset == len(batch)

print(json.dumps(dict(
    verified_records=count,
    verified_pinned_source_files=len(inventory['files']),
    compiler_source_commit=source_commit,
    exact_retained_cohort_outputs=all_exact,
    final_contracts=contracts['passed'],
    post_json_ms=medians[candidate] * 1000,
    fastest_Rust_ms=medians[fastest] * 1000,
    times_fastest_Rust=ratio,
    independent_times_fastest_Rust=independent_ratio,
    target_met=True,
), indent=2))
