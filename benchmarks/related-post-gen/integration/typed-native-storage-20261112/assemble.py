#!/usr/bin/env python3
"""Freeze existing evidence only. This script never builds or runs a benchmark."""
from pathlib import Path
import gzip
import hashlib
import json
import statistics
import subprocess

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
M = Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
COMMIT = '4b6133f85ae85f858bfe29f8cc871aada58a77bb'
BASE = '6cd626c3dfb5cdac2efc3c6978949a717ada27a8'
records = {}
blobs = {}
omitted = []


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def add(name, raw):
    if name in records:
        assert records[name]['uncompressed_sha256'] == digest(raw), name
        return
    key = digest(raw)
    if key in blobs:
        records[name] = {**blobs[key], 'source': name}
        return
    packed = gzip.compress(raw, compresslevel=9, mtime=0)
    relative = 'records/' + key + '.gz'
    target = D / relative
    target.parent.mkdir(exist_ok=True)
    if not target.exists():
        target.write_bytes(packed)
    else:
        assert target.read_bytes() == packed
    row = {'source': name, 'path': relative, 'uncompressed_sha256': key,
           'uncompressed_bytes': len(raw), 'compressed_sha256': digest(packed),
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
    raw = path.read_bytes()
    # Executable/library closure is represented by immutable build/link/hash receipts.
    if path.suffix in ('.rlib', '.rmeta', '.a', '.dylib') or raw[:4] in (
            b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\x7fELF'):
        if path.suffix != '.o':
            omitted.append(str(path))
            return
    add(name or str(path.relative_to(M)), raw)


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
    cursor = 0
    result = {}
    for name in names:
        end = raw.index(b'\n', cursor)
        head = raw[cursor:end].split()
        cursor = end + 1
        assert len(head) == 3 and head[1] == b'blob', (name, head)
        size = int(head[2])
        result[name] = raw[cursor:cursor + size]
        cursor += size
        assert raw[cursor:cursor + 1] == b'\n'
        cursor += 1
    assert cursor == len(raw)
    return result


for name in (
        'first-guarded-typed', 'new-managed-resources/resources-guarded-first',
        'independent-compact-key-u32-prepared', 'fullpath-profile-preparation',
        'fullpath-profile-preparation-v2', 'typed-utf8-yield-candidate',
        'typed-relative-cap-test-fix', 'typed-replay-binding-bound-candidate',
        'guarded-dictionary-candidate', 'guarded-owner-broad-attribution',
        'direct-byte-source', 'key-u32-source', 'compact-guarded-preparation',
        'compact-guarded-resource-preparation', 'IR336-publication',
        'IR334-rebase-ee773', 'typed-fixed-byte', 'guarded-owner-wide',
        'guarded-owner-byte', 'key-u32-probe', 'rust-fresh4-control'):
    tree(name)

for name in (
        'typed-yield-compiler-v1', 'typed-yield-compiler-v2',
        'typed-yield-fixture-tests-v3', 'typed-yield-fixture-tests-v4',
        'typed-yield-fixture-tests-v5', 'typed-replay-compiler-v3',
        'guarded-owner-compiler-v1', 'guarded-owner-fixture-tests-v2',
        'guarded-owner-version-fixture-tests-v3', 'prefix-compiler-v1'):
    tree(name, no_source=True)

for name in (
        'build-engine-round.py', 'build-engine-tests-round.py',
        'build-gpu-round-v2.py', 'build-gpu-round-tests.py',
        'prepare-source-round.py', 'prepare-guarded-typed-cohort.py',
        'independent-managed-confirm-v2.py', 'new-managed-resources/run.py',
        'run-fullpath-profiles.py', 'typed-native-lowering-metadata-summary.json',
        'typed-numeric-static-summary.json', 'native-failure-attribution.json',
        'unchanged-native-failure-comparison.json', 'host-native-contracts.json'):
    evidence_file(M / name)

for name in ('benchmarks/related-post-gen/compare-implementations.py',
             'benchmarks/related-post-gen/experiments/check_subset3.py'):
    add('published-commit/' + name, git_files(COMMIT, [name])[name])
for path in sorted((ROOT / 'benchmarks/related-post-gen/fixtures').rglob('*')):
    if path.is_file():
        add('published-commit/' + str(path.relative_to(ROOT)), path.read_bytes())

primary = read('first-guarded-typed/latency.json')
resource = read('new-managed-resources/resources-guarded-first/report.json')
resource_plan = read('new-managed-resources/resources-guarded-first/plan.json')
confirm = read('independent-compact-key-u32-prepared/report.json')
confirm_plan = read('independent-compact-key-u32-prepared/plan.json')

# Retain every source/receipt referenced by the qualified rosters, without executables.
for variant in (primary['implementations'] + resource_plan['implementations'] + confirm_plan['implementations']):
    for key in ('source', 'receipt'):
        if isinstance(variant.get(key), str):
            path = Path(variant[key])
            if path.is_file():
                evidence_file(path, 'referenced-inputs/' + str(path).lstrip('/'))
    for key, pin in variant.get('pins', {}).items():
        if isinstance(pin, dict) and 'path' in pin and any(token in key for token in ('source', 'receipt', 'metadata', 'object', 'descriptor', 'bundle')):
            path = Path(pin['path'])
            if path.is_file():
                assert digest(path.read_bytes()) == pin['sha256']
                evidence_file(path, 'referenced-inputs/' + str(path).lstrip('/'))

for name, pin in [('posts.json', resource_plan['input']), ('gold.json', resource_plan['gold'])]:
    raw = Path(pin['path']).read_bytes()
    assert digest(raw) == pin['sha256']
    add('publication-inputs/' + name, raw)

publication = read('IR336-publication/source-commit.json')
assert publication['source_commit'] == COMMIT and len(publication['files']) == 23
for name, raw in git_files(COMMIT, sorted(publication['files'])).items():
    assert digest(raw) == publication['files'][name]
    add('measured-source/' + name, raw)
patch = subprocess.run(['git', 'diff', '--binary', BASE, COMMIT, '--',
                        *sorted(publication['files'])], cwd=ROOT, capture_output=True, check=True).stdout
add('measured-source/IR334-to-IR336.patch', patch)

# Full inventories are retained; only changed or overridden bytes are copied.
inventories = {}
for label in ('typed-replay-compiler-v3', 'guarded-owner-compiler-v1',
              'guarded-owner-fixture-tests-v2', 'guarded-owner-version-fixture-tests-v3'):
    inventory = read(label + '/source-inventory.json')
    inventories[label] = inventory
all_names = sorted(set.union(*(set(i['files']) for i in inventories.values())))
published = git_files(COMMIT, all_names)
inventory_overrides = {}
for label, inventory in inventories.items():
    overrides = {}
    for name, expected in inventory['files'].items():
        if digest(published[name]) == expected:
            continue
        path = M / label / 'source' / name
        if not path.is_file():
            path = M / 'typed-replay-compiler-v3/source' / name
        raw = path.read_bytes()
        assert digest(raw) == expected, (label, name)
        add('inventory-overrides/' + label + '/' + name, raw)
        overrides[name] = expected
    inventory_overrides[label] = overrides
literal = inventories['typed-replay-compiler-v3']['literal_includes']
for name, raw in git_files(COMMIT, sorted(literal)).items():
    assert digest(raw) == literal[name], name

rows = []
reference = primary['primary_reference']
serial = min((v['name'] for v in primary['implementations']
              if v['kind'] != 'mech' and v.get('calculation_boundary') == 'post_json_including_setup'
              and 'fresh4' not in v['name']),
             key=lambda n: primary['summary'][n]['median_post_json_seconds'])
for variant in primary['implementations']:
    n = variant['name']
    if variant['kind'] == 'mech' or n in (reference['variant'], serial):
        s = primary['summary'][n]
        rows.append({'variant': n, 'post_json_ms': s['median_post_json_seconds'] * 1000,
                     'times_current_fastest_Rust': s['median_post_json_seconds'] / reference['median_seconds'],
                     'whole_process_ms': s['median_whole_process_seconds'] * 1000})

report = {
    'schema': 'mech.related-post-gen.typed-native-storage-evidence.v1',
    'measured_source_commit': COMMIT, 'base_IR334_source_commit': BASE,
    'typed_IR': 336, 'work_policy': 127, 'ABI': 42, 'image': 11,
    'managed_selected': 'IR334; IR336 wide inconclusive and byte/U32 regress',
    'compact_selected': 'IR336 U32 storage; confirmed faster than current IR336 wide',
    'fastest_Rust_target_met': False, 'whole_process_is_acceptance_gate': False,
    'timed_exact_outputs': 1660,
    'primary': {'record': 'first-guarded-typed/latency.json', 'exact_outputs': 1085,
                'rounds': 31, 'variants': 35, 'seed': 202611120913,
                'bootstrap_seed': 202611120914,
                'paired': 'first-guarded-typed/paired-comparisons.json',
                'current_fastest_Rust': reference, 'rows': rows},
    'confirmation': {'record': 'independent-compact-key-u32-prepared/report.json',
                     'plan': 'independent-compact-key-u32-prepared/plan.json',
                     'exact_outputs': 351, 'rounds': 13, 'variants': 27},
    'resources': {'record': 'new-managed-resources/resources-guarded-first/report.json',
                  'plan': 'new-managed-resources/resources-guarded-first/plan.json',
                  'exact_outputs': 224, 'rounds': 7, 'variants': 32,
                  'instrumented_latency_selection_forbidden': True},
    'profiles': {'record': 'fullpath-profile-preparation-v2/completed-profiles.json',
                 'successful_runs': 2, 'iterations_per_run': 5001,
                 'diagnostic_only': True, 'included_in_timed_exact_outputs': False,
                 'prior_partial_failure_retained': 'fullpath-profile-preparation/ir334-scoped-frame/profile/receipt.json',
                 'prelaunch_schema_error_retained': 'fullpath-profile-preparation-v2/prelaunch-schema-error.json'},
    'source_closure': {'inventory_overrides': inventory_overrides,
                       'source_files': len(all_names), 'literal_includes': len(literal),
                       'overlay': publication['files'],
                       'patch': 'measured-source/IR334-to-IR336.patch'},
    'qualification': {'managed': ['typed-fixed-byte', 'guarded-owner-wide',
                                   'guarded-owner-byte', 'key-u32-probe'],
                      'compact': 'compact-guarded-preparation/qualification.json',
                      'compact_resource_adapters': 'compact-guarded-resource-preparation/qualification.json',
                      'compact_resource_qualification_executions': 96,
                      'full_posts': 5000, 'edges_per_variant': 31,
                      'broad': 'guarded-owner-broad-attribution/attribution.json',
                      'broad_counts': {'passed': 1067, 'failed': 18, 'ignored': 11},
                      'inherited_failures': 16, 'stale_version_assertions_fixed': 2,
                      'broad_rerun_after_test_only_fix': False},
    'retention': {'executables_and_libraries': 'build/link/hash receipts; not duplicated',
                  'source_snapshots': 'full inventories plus minimal changed/override bytes',
                  'outputs': 'all logical output paths retained; identical bytes stored once',
                  'initial_failures': 'retained without replacement or benchmark reruns'},
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
