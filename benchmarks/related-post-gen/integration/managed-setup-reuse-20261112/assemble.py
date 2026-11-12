#!/usr/bin/env python3
"""Freeze completed evidence only. Never build or execute a native benchmark."""
from pathlib import Path
import difflib
import gzip
import hashlib
import json
import subprocess

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
M = Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
BASE = '4b6133f85ae85f858bfe29f8cc871aada58a77bb'
records, blobs, omitted = {}, {}, []


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def add(name, raw):
    key = sha(raw)
    if name in records:
        assert records[name]['uncompressed_sha256'] == key, name
        return
    if key in blobs:
        records[name] = {**blobs[key], 'source': name}
        return
    packed = gzip.compress(raw, compresslevel=9, mtime=0)
    relative = 'records/' + key + '.gz'
    target = D / relative
    target.parent.mkdir(exist_ok=True)
    if target.exists():
        assert target.read_bytes() == packed
    else:
        target.write_bytes(packed)
    row = {'source': name, 'path': relative, 'uncompressed_sha256': key,
           'uncompressed_bytes': len(raw), 'compressed_sha256': sha(packed),
           'compressed_bytes': len(packed)}
    records[name] = row
    blobs[key] = row


def evidence_file(path, name=None):
    if path.suffix in ('.rlib', '.rmeta', '.a', '.dylib'):
        omitted.append(str(path))
        return
    with path.open('rb') as f:
        magic = f.read(4)
    if magic in (b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\x7fELF') and path.suffix != '.o':
        omitted.append(str(path))
        return
    add(name or str(path.relative_to(M)), path.read_bytes())


def tree(name, no_source=False):
    top = M / name
    assert top.exists(), top
    for path in sorted(top.rglob('*')):
        if not path.is_file() or path.name == '.DS_Store':
            continue
        relative = path.relative_to(top)
        if no_source and relative.parts[0] == 'source':
            continue
        evidence_file(path)


def read(name):
    return json.loads((M / name).read_bytes())


def git_files(commit, names):
    raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT, check=True,
                         input=''.join(commit + ':' + name + '\n' for name in names).encode(),
                         capture_output=True).stdout
    cursor, result = 0, {}
    for name in names:
        end = raw.index(b'\n', cursor)
        header = raw[cursor:end].split()
        cursor = end + 1
        assert len(header) == 3 and header[1] == b'blob', (name, header)
        size = int(header[2])
        result[name] = raw[cursor:cursor + size]
        cursor += size
        assert raw[cursor:cursor + 1] == b'\n'
        cursor += 1
    assert cursor == len(raw)
    return result


for name in ('first-setup-reuse', 'new-managed-resources/resources-setup-first',
             'typed-static-replay-facts-candidate', 'worker-owned-capture-candidate',
             'static-replay-byte', 'static-replay-u32',
             'owned-capture-wide', 'owned-capture-u32',
             'compact-guarded-resource-preparation', 'rust-fresh4-control', 'IR338-publication', 'IR338-rebase-latest973'):
    tree(name)
for name in ('static-replay-compiler-v1', 'owned-capture-compiler-v1'):
    tree(name, no_source=True)
for name in ('build-gpu-round-v2.py', 'build-gpu-round-tests.py',
             'prepare-source-round.py', 'new-managed-resources/run.py',
             'new-managed-resources/run-with-compact.py',
             'guarded-owner-compiler-v1/build.json',
             'guarded-owner-compiler-v1/source-inventory.json',
             'guarded-owner-version-fixture-tests-v3/source-inventory.json',
             'typed-replay-compiler-v3/build.json',
             'typed-replay-compiler-v3/source-inventory.json',
             'prefix-compiler-v1/build.json',
             'scoped-frame-compiler-v1/broad-native-tests.json',
             'native-failure-attribution.json'):
    evidence_file(M / name)
for name in ('benchmarks/related-post-gen/compare-implementations.py',
             'benchmarks/related-post-gen/measurement.rs',
             'benchmarks/related-post-gen/experiments/check_subset3.py'):
    add('base-commit/' + name, git_files(BASE, [name])[name])
for path in sorted((ROOT / 'benchmarks/related-post-gen/fixtures').rglob('*')):
    if path.is_file():
        name = str(path.relative_to(ROOT))
        add('base-commit/' + name, git_files(BASE, [name])[name])

primary = read('first-setup-reuse/latency.json')
resources = read('new-managed-resources/resources-setup-first/report.json')
resource_plan = read('new-managed-resources/resources-setup-first/plan.json')


def referenced(value):
    if isinstance(value, dict):
        if isinstance(value.get('path'), str) and 'sha256' in value:
            path = Path(value['path'])
            if path.is_file():
                # Libraries/executables remain represented by their exact hash receipts.
                evidence_file(path, 'referenced-inputs/' + str(path).lstrip('/'))
                if str(path) not in omitted:
                    assert sha(path.read_bytes()) == value['sha256']
        for child in value.values():
            referenced(child)
    elif isinstance(value, list):
        for child in value:
            referenced(child)


for variant in primary['implementations'] + resource_plan['implementations']:
    for key in ('source', 'receipt', 'compiler_build', 'measurement_helper'):
        if isinstance(variant.get(key), str):
            path = Path(variant[key])
            if path.is_file():
                evidence_file(path, 'referenced-inputs/' + str(path).lstrip('/'))
    referenced(variant.get('pins', {}))
referenced(resource_plan)
for name, pin in [('posts.json', resource_plan['input']), ('gold.json', resource_plan['gold'])]:
    raw = Path(pin['path']).read_bytes()
    assert sha(raw) == pin['sha256']
    add('publication-inputs/' + name, raw)

# No live source is used as measurement authority. Derive the complete minimal
# overlay from the two immutable build snapshots and the published IR336 base.
base_inventory = read('guarded-owner-version-fixture-tests-v3/source-inventory.json')
base = git_files(BASE, sorted(base_inventory['files']))
assert all(sha(base[name]) == expected for name, expected in base_inventory['files'].items())
inventories = {label: read(label + '/source-inventory.json') for label in
               ('static-replay-compiler-v1', 'owned-capture-compiler-v1')}
overrides = {}
for label, inventory in inventories.items():
    current = {}
    for name, expected in inventory['files'].items():
        if name in base and sha(base[name]) == expected:
            continue
        raw = (M / label / 'source' / name).read_bytes()
        assert sha(raw) == expected, (label, name)
        add('inventory-overrides/' + label + '/' + name, raw)
        current[name] = expected
    overrides[label] = current
final = overrides['owned-capture-compiler-v1']
assert len(final) == 13
patch_parts = []
for name, expected in sorted(final.items()):
    raw = (M / 'owned-capture-compiler-v1/source' / name).read_bytes()
    add('measured-source/' + name, raw)
    patch_parts.append('diff --git a/' + name + ' b/' + name + '\n')
    old = base.get(name, b'')
    patch_parts.extend(difflib.unified_diff(old.decode().splitlines(True), raw.decode().splitlines(True),
                       fromfile='a/' + name if name in base else '/dev/null', tofile='b/' + name))
patch = ''.join(patch_parts).encode()
add('measured-source/IR336-to-IR338.patch', patch)
source_overlay = {'base_commit': BASE, 'files': final,
                  'new_files': sorted(set(final) - set(base)),
                  'IR337_source_inventory': 'static-replay-compiler-v1/source-inventory.json',
                  'IR338_source_inventory': 'owned-capture-compiler-v1/source-inventory.json',
                  'source_files_IR337': 2816, 'source_files_IR338': 2817,
                  'patch_sha256': sha(patch), 'benchmark_selected': False}
add('measured-source/overlay.json', (json.dumps(source_overlay, indent=2) + '\n').encode())
literal = read('typed-replay-compiler-v3/source-inventory.json')['literal_includes']
for name, raw in git_files(BASE, sorted(literal)).items():
    assert sha(raw) == literal[name]

managed_work_qualifications = {}
peak_histograms = {}
work_histograms = {}
for variant in primary['implementations']:
    if not variant['name'].startswith('mech-managed-'):
        continue
    name = variant['name']
    path = Path(variant['receipt']).parent / 'qualify-5000.json'
    raw = path.read_bytes()
    record_name = 'control-qualifications/' + name + '/qualify-5000.json'
    add(record_name, raw)
    qualified = json.loads(raw)
    assert qualified['exit'] == 0
    value = next(json.loads(line) for line in qualified['stdout'].splitlines() if line.startswith('{'))
    assert value['posts'] == 5000 and value['parallel_tasks'] == 4 and value['parallel_batches'] == 1
    managed_work_qualifications[name] = {'record': record_name, 'sha256': sha(raw), 'work': value['work'], 'parallel_numeric_work': value['parallel_numeric_work'], 'parallel_numeric_regions': value['parallel_numeric_regions'], 'exact_total_required': name != 'mech-managed-typed-fixed-byte'}
    histogram = {}
    bills = {}
    for sample in primary['samples']:
        if sample['variant'] != name:
            continue
        value = next(json.loads(line) for line in sample['stdout'].splitlines() if line.startswith('{'))
        assert value['parallel_tasks'] == 4 and value['parallel_batches'] == 1
        assert value['parallel_numeric_work'] == managed_work_qualifications[name]['parallel_numeric_work']
        assert value['parallel_numeric_regions'] == managed_work_qualifications[name]['parallel_numeric_regions']
        if managed_work_qualifications[name]['exact_total_required']:
            assert value['work'] == managed_work_qualifications[name]['work']
        bills[str(value['work'])] = bills.get(str(value['work']), 0) + 1
        peak = str(value['parallel_peak_workers'])
        assert 1 <= int(peak) <= 4
        histogram[peak] = histogram.get(peak, 0) + 1
    peak_histograms[name] = histogram
    work_histograms[name] = bills

rows = []
reference = primary['primary_reference']
serial = min((v['name'] for v in primary['implementations']
              if v['kind'] != 'mech' and v.get('calculation_boundary') == 'post_json_including_setup'
              and 'fresh4' not in v['name']),
             key=lambda n: primary['summary'][n]['median_post_json_seconds'])
for variant in primary['implementations']:
    name = variant['name']
    if variant['kind'] == 'mech' or name in (reference['variant'], serial):
        value = primary['summary'][name]
        rows.append({'variant': name, 'post_json_ms': value['median_post_json_seconds'] * 1000,
                     'times_current_fastest_Rust': value['median_post_json_seconds'] / reference['median_seconds'],
                     'whole_process_ms': value['median_whole_process_seconds'] * 1000})
resource_rows = []
for name, summary in resources['summary'].items():
    if name.startswith('mech-') or 'fresh4' in name:
        value = summary['window_medians']['post_json']
        resource_rows.append({'variant': name,
            'instructions_retired': value['hardware']['instructions_retired'],
            'cycles': value['hardware']['cycles'],
            'allocation_requests': value['allocation_requests'],
            'requested_allocation_bytes': value['requested_allocation_bytes'],
            'peak_additional_live_requested_heap_bytes': value['peak_additional_live_requested_heap_bytes'],
            'whole_process_peak_resident_bytes': summary['whole_process_medians']['peak_resident_bytes']})

report = {
 'schema': 'mech.related-post-gen.managed-setup-reuse-evidence.v1',
 'base_IR336_source_commit': BASE,
 'measured_source_commit': read('IR338-publication/source-commit.json')['source_commit'],
 'published_source_equivalent_commit': read('IR338-rebase-latest973/receipt.json')['published_equivalent_source_commit'],
 'candidate_IRs': [337, 338], 'work_policy': 127, 'ABI': 42, 'image': 11,
 'managed_selected': 'IR334 scoped-frame; no established managed latency gain from IR337/338',
 'new_candidates_selected': False,
 'fastest_Rust_target_met': False, 'whole_process_is_acceptance_gate': False,
 'timed_exact_outputs': 1468,
 'primary': {'record': 'first-setup-reuse/latency.json', 'exact_outputs': 1209,
             'rounds': 31, 'variants': 39, 'seed': 202611120953,
             'bootstrap_seed': 202611120954, 'paired': 'first-setup-reuse/paired-comparisons.json',
             'current_fastest_Rust': reference, 'rows': rows,
             'managed_work_qualifications': managed_work_qualifications,
             'managed_observed_peak_worker_histograms': peak_histograms,
             'managed_total_work_histograms': work_histograms,
             'historical_IR335_total_work_variation': 'Unattributed. Historical unselected IR335 control only; numeric worker bill exact, all total bills and same-compiler qualification comparator retained. No resampling or filtering.',
             'fresh_managed_worker_tasks': 4, 'all_scheduling_observations_retained': True,
             'independent_confirmation': 'Not run: no new managed latency gain established'},
 'resources': {'record': 'new-managed-resources/resources-setup-first/report.json',
               'plan': 'new-managed-resources/resources-setup-first/plan.json',
               'exact_outputs': 259, 'rounds': 7, 'variants': 37, 'seed': 202611120955,
               'instrumented_latency_selection_forbidden': True, 'rows': resource_rows,
               'compact_contract': 'Compact/unmetered; arithmetic and indexing checked. Distinct from managed contract.',
               'native_heap_scope': 'Existing compact native payload/mmap statistics remain separate from tracked Rust heap and OS counters; no blind summation'},
 'source_closure': {'base_source_files': 2814, 'source_files_IR337': 2816, 'source_files_IR338': 2817,
                    'literal_includes': 164, 'inventory_overrides': overrides,
                    'overlay': final, 'patch': 'measured-source/IR336-to-IR338.patch',
                    'overlay_record': 'measured-source/overlay.json'},
 'qualification': {'managed': ['static-replay-byte', 'static-replay-u32', 'owned-capture-wide', 'owned-capture-u32'],
                   'full_posts': 5000, 'edges_per_variant': 31, 'fresh_workers': 4,
                   'focused': 'owned-capture-compiler-v1/focused-contracts.json', 'focused_passed': 50,
                   'compact_typed': 'owned-capture-compiler-v1/compact-typed-contracts.json', 'compact_typed_passed': 1,
                   'broad': 'owned-capture-compiler-v1/broad-native-tests.json',
                   'broad_counts': {'passed': 1079, 'failed': 16, 'ignored': 11, 'filtered': 35},
                   'broad_attribution': 'owned-capture-compiler-v1/broad-native-failure-attribution.json',
                   'baseline_broad': 'scoped-frame-compiler-v1/broad-native-tests.json',
                   'all_16_failures_inherited': True,
                   'compact_resource_adapters': 'compact-guarded-resource-preparation/qualification.json',
                   'compact_resource_correctness_executions': 96,
                   'compact_qualifications_are_prior_cohort_and_not_new_outputs': True},
 'retention': {'executables_and_libraries': 'Immutable build/link/hash receipts; not duplicated',
               'source_snapshots': 'Complete inventories with minimal changed/override bytes over published IR336',
               'outputs': 'All logical output paths retained; identical bytes stored once',
               'native_executions_by_assembler': 0,
               'prior_packet': 'typed-native-storage-20261112; unchanged'},
}
(D / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
manifest = {'schema': 'mech.related-post-gen.deduplicated-records.v1',
            'logical_records': len(records), 'unique_blobs': len(blobs),
            'compressed_bytes': sum(v['compressed_bytes'] for v in blobs.values()),
            'uncompressed_logical_bytes': sum(v['uncompressed_bytes'] for v in records.values()),
            'records': [records[name] for name in sorted(records)]}
(D / 'records.json').write_text(json.dumps(manifest, indent=2) + '\n')
(D / 'omitted-binaries.json').write_text(json.dumps(sorted(set(omitted)), indent=2) + '\n')
print(json.dumps({k: v for k, v in manifest.items() if k != 'records'}, indent=2))
