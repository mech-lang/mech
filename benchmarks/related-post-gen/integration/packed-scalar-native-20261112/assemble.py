#!/usr/bin/env python3
"""Archive completed observations only; no compilation or native execution."""
from pathlib import Path
import difflib
import gzip
import hashlib
import json
import subprocess
import shutil
import zipfile

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
M = Path('/private/tmp/mech-related-managed-parity-evidence-20261111')
BASE338 = 'b939e42a6cbbb6e9cd5a747d6c5ba1ba09567164'
BASE340 = '1ef5985a9c58fd2857bdf8eedc06a2b7b50ef622'
records, blobs, omitted = {}, {}, set()
BLOB_CACHE = M / 'packed-scalar-packet-blob-cache-v1'
BLOB_CACHE.mkdir(exist_ok=True)
cache_path = BLOB_CACHE / 'cohort-cache.json'
old_cache_path = D / '.assembly-cohort-cache.json'
if old_cache_path.exists() and not cache_path.exists():
    shutil.move(old_cache_path, cache_path)
if (D / 'records').exists():
    for old in (D / 'records').glob('*.gz'):
        target = BLOB_CACHE / 'records' / old.name
        target.parent.mkdir(exist_ok=True)
        if target.exists():
            assert old.read_bytes() == target.read_bytes()
            old.unlink()
        else:
            shutil.move(old, target)
    (D / 'records').rmdir()
cohort_cache = json.loads(cache_path.read_bytes()) if cache_path.exists() else {}
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def add(name, raw):
    digest = sha(raw)
    if name in records:
        assert records[name]['uncompressed_sha256'] == digest, name
        return
    if digest not in blobs:
        rel = 'records/' + digest + '.gz'
        target = BLOB_CACHE / rel
        target.parent.mkdir(exist_ok=True)
        if target.exists():
            packed = target.read_bytes()
            assert gzip.decompress(packed) == raw
        else:
            packed = gzip.compress(raw, compresslevel=9, mtime=0)
            target.write_bytes(packed)
        blobs[digest] = dict(path=rel, uncompressed_sha256=digest,
                             uncompressed_bytes=len(raw), compressed_sha256=sha(packed),
                             compressed_bytes=len(packed))
    records[name] = dict(source=name, **blobs[digest])


def evidence(path, name=None):
    path = Path(path)
    if not path.is_file():
        return
    logical_name = name or str(path.relative_to(M))
    if logical_name in cohort_cache:
        row = cohort_cache[logical_name]
        records[logical_name] = row
        blobs[row['uncompressed_sha256']] = {k: v for k, v in row.items() if k != 'source'}
        return
    if path.suffix in ('.rlib', '.rmeta', '.a', '.dylib') or any(
        part in path.parts for part in ('backend-target', 'target', '.git', '__pycache__')
    ):
        omitted.add(str(path))
        return
    with path.open('rb') as stream:
        magic = stream.read(4)
    if magic in (b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\x7fELF') and path.suffix != '.o':
        omitted.add(str(path))
        return
    add(logical_name, path.read_bytes())


def tree(name, no_source=False):
    top = M / name
    assert top.exists(), top
    for path in sorted(top.rglob('*')):
        rel = path.relative_to(top)
        if not path.is_file() or path.name == '.DS_Store':
            continue
        if any(part in rel.parts for part in ('target', 'backend-target', '.git', '__pycache__')):
            continue
        if no_source and rel.parts[0] == 'source':
            continue
        evidence(path)


def read(name):
    return json.loads((M / name).read_bytes())


def git_files(commit, names, missing=False):
    raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT, check=True,
                         input=''.join(commit + ':' + name + '\n' for name in names).encode(),
                         capture_output=True).stdout
    cursor, result = 0, {}
    for name in names:
        end = raw.index(b'\n', cursor)
        header = raw[cursor:end].split()
        cursor = end + 1
        if header[-1] == b'missing':
            assert missing, (commit, name)
            continue
        assert len(header) == 3 and header[1] == b'blob', (name, header)
        size = int(header[2])
        result[name] = raw[cursor:cursor + size]
        cursor += size
        assert raw[cursor:cursor + 1] == b'\n'
        cursor += 1
    assert cursor == len(raw)
    return result


cohorts = [
    dict(kind='primary', prefix='first-packed-native-query-gains', outputs=1860, rounds=31, variants=60),
    dict(kind='resources', prefix='new-managed-resources/resources-packed-query-first-authoritative', outputs=406, rounds=7, variants=58),
    dict(kind='primary', prefix='first-scalar-packed-query-gains-v2', outputs=1736, rounds=31, variants=56),
    dict(kind='resources', prefix='new-managed-resources/resources-scalar-packed-query-first', outputs=399, rounds=7, variants=57),
    dict(kind='confirmation', prefix='independent-compact-packed-scalar-confirmation-v1', outputs=468, rounds=13, variants=36),
]
assert sum(row['outputs'] for row in cohorts) == 4869
for row in cohorts:
    row['record'] = row['prefix'] + ('/latency.json' if row['kind'] == 'primary' else '/report.json')
    row['plan'] = row['prefix'] + ('/config.json' if row['kind'] == 'primary' else '/plan.json')
    tree(row['prefix'])
cache_path.write_text(json.dumps(records, sort_keys=True) + '\n')

compiler_labels = [
    'dense-producer-comparison-compiler-v1',
    'compact-bit-count-compiler-v3', 'compact-bit-count-tests-v4',
    'zero-extend-load-compiler-v1', 'scalar-edge-homes-compiler-v1',
    'scalar-edge-homes-compiler-v2', 'packed-popcount-compiler-v1',
    'packed-selected-homes-compiler-v1',
    'allocation-phase-census-v1/diagnostic-runtime-compiler',
    'allocation-phase-census-v2/diagnostic-runtime-compiler',
]
for name in compiler_labels:
    tree(name, no_source=True)

trees = [
    'first-scalar-packed-query-gains',
    'new-managed-resources/resources-packed-query-first',
    'compact-bit-count-candidate-v1', 'compact-bit-count-candidate-v2',
    'compact-bit-count-candidate-v3', 'compact-bit-count-candidate-v4',
    'compact-bit-count-handoff-v1', 'compact-bit-count-machine-audit-v1',
    'compact-idiom-control343-preparation-v1',
    'compact-idiom-control343-wide-byte-preparation-v1',
    'compact-idiom345-preparation-v1', 'compact-idiom348-preparation-v1',
    'compact-idiom349-preparation-v1', 'compact-composition-entries-v1',
    'compact-composition349-entries-v1', 'zero-extend-packed-load-candidate',
    'zero-extend-load-qualified-v1', 'selected-boolean-homes-candidate',
    'scalar-edge-homes-preparation-v1', 'scalar-edge-homes-qualified-v1',
    'scalar-home-clif343-audit', 'scalar-home-clif347-audit',
    'packed-popcount-preparation-v1', 'packed-popcount-qualified-v1',
    'packed-selected-homes-preparation-v1', 'packed-selected-homes-qualified-v1',
    'packed-selected-homes-static-audit-v1', 'packed-selected-homes-static-audit-v2',
    'packed-selected-homes-static-audit-v3',
    'branch-insert-key-u32-source', 'branch-insert-ir340-preparation-v1',
    'branch-insert-ir340-qualified-v1', 'branch-insert-ir343-qualified-v1',
    'branch-insert-ir346-qualified-v1',
    'packed-scalar-packet-development-v1',
]
for name in trees:
    tree(name)
if (M / 'packed-scalar-fullpath-profile-v1/complete.json').exists():
    tree('packed-scalar-fullpath-profile-v1')
if (M / 'IR349-source-publication').exists():
    tree('IR349-source-publication')
for version in (1, 2):
    top = M / ('allocation-phase-census-v' + str(version))
    for path in sorted(top.rglob('*')):
        if path.is_file() and not ('source' in path.relative_to(top).parts):
            evidence(path)
    source = top / 'source'
    if source.exists():
        for path in sorted(source.rglob('*')):
            if path.is_file():
                evidence(path)

extra = [
    'owned-capture-compiler-v1/build.json', 'owned-capture-compiler-v1/source-inventory.json',
    'dense-worker-compiler-v2/build.json', 'dense-worker-compiler-v2/source-inventory.json',
    'typed-replay-compiler-v3/source-inventory.json',
    'comparison-chain-compiler-v3/backend-build.json',
    'comparison-chain-compiler-v3/backend-libraries.json',
    'comparison-chain-compiler-v3/vendor-full-source-inventory.json',
    'dense-producer-comparison-compiler-v1/build.json',
    'dense-producer-comparison-compiler-v1/source-inventory.json',
    'dense-producer-comparison-compiler-v1/broad-native-tests.json',
    'build-fresh-backend-tests-v2.py', 'build-packed-popcount-v1.py',
    'build-packed-selected-homes-v1.py', 'build-compact-bit-count-v3.py',
    'build-compact-bit-count-tests-v4.py', 'audit-packed-selected-homes-v3.py',
    'new-managed-resources/run-with-compact.py', 'new-managed-resources/run.py',
    'prepare-source-round.py', 'independent-managed-confirm-v7.py',
    'independent-compact-confirm-v1.py', 'native-failure-attribution.json',
]
for name in extra:
    evidence(M / name)
print('Archive trees collected: ' + str(len(records)), flush=True)

# Capture pinned inputs (historical absolute paths are provenance labels only).
seen_json = set()
pin_mismatches = []


def referenced(value):
    if isinstance(value, dict):
        path, digest = value.get('path'), value.get('sha256')
        if isinstance(path, str) and isinstance(digest, str):
            p = Path(path)
            if p.is_file():
                evidence(p, 'referenced-inputs/' + str(p).lstrip('/'))
                if str(p) not in omitted:
                    raw = p.read_bytes()
                    if sha(raw) != digest:
                        pin_mismatches.append(dict(path=str(p), expected_sha256=digest,
                                                   current_sha256=sha(raw),
                                                   expected_digest_already_archived=digest in blobs))
                        return
                    if 'bytes' in value:
                        assert len(raw) == value['bytes'], p
                    if p.suffix == '.json' and digest not in seen_json:
                        seen_json.add(digest)
                        referenced(json.loads(raw))
        for child in value.values():
            referenced(child)
    elif isinstance(value, list):
        for child in value:
            referenced(child)


for name in list(records):
    if name.endswith('.json'):
        digest = records[name]['uncompressed_sha256']
        if digest in seen_json:
            continue
        seen_json.add(digest)
        try:
            value = json.loads(gzip.decompress((BLOB_CACHE / records[name]['path']).read_bytes()))
        except (UnicodeDecodeError, json.JSONDecodeError):
            continue
        referenced(value)
for cohort in cohorts:
    value = read(cohort['record'])
    for variant in value.get('implementations', []) + read(cohort['plan']).get('implementations', []):
        for field in ('source', 'receipt', 'compiler_build', 'measurement_helper'):
            if isinstance(variant.get(field), str):
                p = Path(variant[field])
                evidence(p, 'referenced-inputs/' + str(p).lstrip('/'))
        referenced(variant.get('pins', {}))
print('Referenced inputs collected: ' + str(len(records)), flush=True)
add('pin-discovery-mismatches.json', (json.dumps(pin_mismatches, indent=2) + '\n').encode())

resource_plan = read(cohorts[1]['plan'])
for name, value in [('posts.json', resource_plan['input']), ('gold.json', resource_plan['gold'])]:
    raw = Path(value['path']).read_bytes()
    assert sha(raw) == value['sha256']
    add('publication-inputs/' + name, raw)

bases = {}
for label, commit in [('owned-capture-compiler-v1', BASE338), ('dense-worker-compiler-v2', BASE340)]:
    inv = read(label + '/source-inventory.json')
    files = git_files(commit, sorted(inv['files']))
    assert all(sha(files[name]) == digest for name, digest in inv['files'].items())
    bases[label] = dict(commit=commit, inventory_record=label + '/source-inventory.json', files=files)


def source_bytes(label, inventory, name, digest):
    paths = [M / label / 'source' / name]
    if name.startswith('vendor/'):
        paths.append(M / 'comparison-chain-compiler-v3/source' / name)
    if name.startswith('hosts/gpu/') and inventory.get('gpu_snapshot'):
        paths.append(Path(inventory['gpu_snapshot']) / name.removeprefix('hosts/gpu/'))
    if name.startswith('vendor/') and inventory.get('vendor_snapshot'):
        paths.append(Path(inventory['vendor_snapshot']) / name.removeprefix('vendor/'))
    for path in paths:
        if path.is_file() and sha(path.read_bytes()) == digest:
            return path.read_bytes()
    raise AssertionError((label, name, digest, paths))


closures = {}
for label in compiler_labels:
    inv = read(label + '/source-inventory.json')
    _, base_name = min((sum(name not in base['files'] or sha(base['files'][name]) != digest
                           for name, digest in inv['files'].items()), key)
                      for key, base in bases.items())
    base, overrides, parts = bases[base_name], {}, []
    for name, digest in inv['files'].items():
        if name in base['files'] and sha(base['files'][name]) == digest:
            continue
        raw = source_bytes(label, inv, name, digest)
        add('source-overrides/' + label + '/' + name, raw)
        overrides[name] = digest
        parts.append('diff --git a/' + name + ' b/' + name + '\n')
        parts.extend(difflib.unified_diff(base['files'].get(name, b'').decode().splitlines(True),
                                        raw.decode().splitlines(True),
                                        fromfile='a/' + name if name in base['files'] else '/dev/null',
                                        tofile='b/' + name))
    patch = ''.join(parts).encode()
    patch_record = 'source-overrides/' + label + '/candidate.patch'
    add(patch_record, patch)
    closures[label] = dict(base=base_name, base_commit=base['commit'],
                           inventory_record=label + '/source-inventory.json',
                           overrides=overrides, patch_record=patch_record,
                           source_count=len(inv['files']))

vendor = read('comparison-chain-compiler-v3/vendor-full-source-inventory.json')
assert len(vendor) == 246
vendor_base = git_files(BASE338, sorted(vendor), missing=True)
vendor_overrides = {}
vendor_root = M / 'comparison-chain-compiler-v3/source'
for name, digest in vendor.items():
    if name in vendor_base and sha(vendor_base[name]) == digest:
        continue
    raw = (vendor_root / name).read_bytes()
    assert sha(raw) == digest
    add('vendor-overrides/' + name, raw)
    vendor_overrides[name] = digest
for label in compiler_labels:
    path = M / label / 'vendor-full-source-inventory.json'
    if path.exists():
        assert json.loads(path.read_bytes()) == vendor, label

literal = read('typed-replay-compiler-v3/source-inventory.json')['literal_includes']
for name, raw in git_files(BASE338, sorted(literal)).items():
    assert sha(raw) == literal[name]
for name in ['benchmarks/related-post-gen/compare-implementations.py',
             'benchmarks/related-post-gen/measurement.rs',
             'benchmarks/related-post-gen/experiments/check_subset3.py']:
    add('published-base/' + name, git_files(BASE340, [name])[name])
for path in sorted((ROOT / 'benchmarks/related-post-gen/fixtures').rglob('*')):
    if path.is_file():
        name = str(path.relative_to(ROOT))
        add('published-base/' + name, git_files(BASE340, [name])[name])

qualifications = []
for label in ['compact-idiom-control343-preparation-v1',
              'compact-idiom-control343-wide-byte-preparation-v1',
              'compact-idiom345-preparation-v1', 'compact-idiom348-preparation-v1',
              'compact-idiom349-preparation-v1']:
    for suffix in ('', '/resources'):
        name = label + suffix + '/qualification.json'
        value = read(name)
        qualifications.append(dict(record=name, kind='compact', outputs=value['correctness_executions']))
for label in ['zero-extend-load-qualified-v1', 'scalar-edge-homes-qualified-v1',
              'packed-popcount-qualified-v1', 'packed-selected-homes-qualified-v1',
              'branch-insert-ir340-qualified-v1', 'branch-insert-ir343-qualified-v1',
              'branch-insert-ir346-qualified-v1']:
    value = read(label + '/qualification.json')
    qualifications.append(dict(record=label + '/qualification.json', kind='managed',
                               outputs=len(value['results']) * 32))
assert sum(row['outputs'] for row in qualifications) == 1856

# Exact matching managed per-variant ledgers; retain the historical IR335 variation.
work = {}
for cohort in cohorts:
    for variant in read(cohort['plan']).get('implementations', []):
        name = variant['name'].removesuffix('-instrumented')
        if not name.startswith('mech-managed-') or name in work:
            continue
        path = Path(variant.get('qualification', {}).get('qualify-5000.json',
                    str(Path(variant.get('receipt', '.')).parent / 'qualify-5000.json')))
        if not path.is_file():
            continue
        raw = path.read_bytes()
        value = json.loads(raw)
        assert value['exit'] == 0
        obj = next(json.loads(line) for line in value['stdout'].splitlines() if line.startswith('{'))
        record = 'control-qualifications/' + name + '/full.json'
        add(record, raw)
        work[name] = dict(record=record, sha256=sha(raw), work=obj['work'],
                          numeric_work=obj['parallel_numeric_work'],
                          numeric_regions=obj['parallel_numeric_regions'],
                          exact_total_required=name != 'mech-managed-typed-fixed-byte')

report = dict(
    schema='mech.related-post-gen.packed-scalar-native-evidence.v1',
    scope='Completed IR345–349 candidates, three source-ladder attempts, allocation census diagnostics. No IR350 or later candidate.',
    timed_resource_exact_outputs=4869, qualification_exact_outputs_separate=1856,
    allocation_census_exact_outputs_separate=28, cohorts=cohorts,
    qualifications=qualifications, managed_work_qualifications=work,
    source_closure=dict(bases={key: {k: v for k, v in base.items() if k != 'files'}
                               for key, base in bases.items()},
                        compilers=closures,
                        vendor_inventory='comparison-chain-compiler-v3/vendor-full-source-inventory.json',
                        vendor_base_commit=BASE338, vendor_overrides=vendor_overrides,
                        vendor_files=246, literal_inventory='typed-replay-compiler-v3/source-inventory.json',
                        literal_includes=len(literal)),
    selection=dict(managed_selected_checkpoint_IR=334, new_managed_selected=False,
                   fastest_Rust_target_met=False, whole_process_is_acceptance_gate=False,
                   compact349_u32_primary_ms=1.247625, compact349_u32_primary_Rust_ms=1.23675,
                   compact349_u32_primary_multiple=1.247625 / 1.23675,
                   compact349_u32_confirmation_multiple=read('independent-compact-packed-scalar-confirmation-v1/report.json')['summary']['mech-ir349-compact-key-u32-4']['median_post_json_seconds'] / read('independent-compact-packed-scalar-confirmation-v1/report.json')['references']['current_22_eligible_Rust']['median_seconds'],
                   compact349_vs336_U32='Confirmed paired gain in primary and independent confirmation; still slower than current fastest Rust.',
                   compact349_vs348_wide='Confirmed paired gain in primary and independent confirmation.',
                   compact349_vs348_U32='Primary interval crosses zero; no gain confirmed across both cohorts.',
                   compact345_vs343_U32='Independent confirmation interval crosses zero.',
                   compact348_vs345_U32='Independent confirmation interval crosses zero.',
                   source_ladder='All three current compiler forms regress prospectively; unselected.'),
    observations=dict(all_retained=True, tasks=4, batches=1, observed_peak_workers='1..4; no filtering',
                      historical_IR335_work_variation='Unselected typed-fixed-byte total work varies; exact numeric bill retained, cause unattributed.',
                      resource_latency_selection_forbidden=True,
                      compact_contract='Separate unmetered contract, checked arithmetic and indexing; native payload accounting is distinct from tracked Rust heap/RSS.',
                      static_counts='Whole native query including cold code; no dynamic spill or latency claim.',
                      allocation_census='Diagnostic allocation requests and issued/retired layout bytes, not resident memory or latency acceptance. Deallocation attribution follows retirement phase.'),
    retention=dict(no_native_execution_by_assembler=True,
                   binaries='Omitted executables/libraries retain immutable hash/build/link receipts; Object files retained.',
                   published_dependencies_only=[BASE338, BASE340]))
report['optional_published_IR349_alias'] = '7a6686c6790b0d8e0a282a5619bb884f8ffe08d0'
report['pin_discovery_mismatches_record'] = 'pin-discovery-mismatches.json'
report['test_backend_provenance'] = dict(
    IR346='Standalone focused/broad test executable links the historical backend family; production compiler uses V3. Do not claim standalone tests are backend-coherent with production.',
    IR347_IR348_IR349='Focused/broad test executables use exact V3 backend family. IR348/349 composed coverage includes unsigned-load/capture/geometry contracts; it does not retroactively change standalone IR346 provenance.',
    census='Exact IR340 ordinary Object; separate diagnostic GPU runtime overlay and original IR340 backend.')
report['timing_tables'] = []
report['resource_tables'] = []
for cohort in cohorts:
    value = read(cohort['record'])
    if cohort['kind'] == 'resources':
        report['resource_tables'].append(dict(cohort=cohort['prefix'],
            rows=[dict(variant=name, post_json_window=summary['window_medians'].get('post_json'),
                       whole_process=summary['whole_process_medians'])
                  for name, summary in value['summary'].items()]))
        continue
    plan = read(cohort['plan'])
    variants = {row['name']: row for row in value.get('implementations', plan['implementations'])}
    reference = value['primary_reference'] if cohort['kind'] == 'primary' else value['references']['current_22_eligible_Rust']
    rows = []
    for name, summary in value['summary'].items():
        seconds = summary.get('median_post_json_seconds')
        rows.append(dict(variant=name, post_json_ms=seconds * 1000 if seconds is not None else None,
                         times_current_fastest_Rust=seconds / reference['median_seconds'] if seconds is not None else None,
                         whole_process_ms=summary['median_whole_process_seconds'] * 1000,
                         code_bytes=variants[name].get('code_bytes'), stack_bytes=variants[name].get('stack_bytes'),
                         executable_file_bytes=variants[name].get('executable_file_bytes'),
                         whole_process_resources=summary.get('process_resources_median')))
    report['timing_tables'].append(dict(cohort=cohort['prefix'], current_fastest_Rust=reference, rows=rows))
(D / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
add('packet-report.json', (D / 'report.json').read_bytes())
(D / 'omitted-binaries.json').write_text(json.dumps(sorted(omitted), indent=2) + '\n')
used = {row['uncompressed_sha256'] for row in records.values()}
for path in (BLOB_CACHE / 'records').glob('*.gz'):
    if path.stem not in used:
        path.unlink()
manifest = dict(schema='mech.related-post-gen.deduplicated-records.v1',
                logical_records=len(records), unique_blobs=len(blobs),
                compressed_bytes=sum(row['compressed_bytes'] for row in blobs.values()),
                records=[records[name] for name in sorted(records)])
container_path = D / 'records.zip'
with zipfile.ZipFile(container_path, 'w', compression=zipfile.ZIP_STORED) as archive:
    for path in sorted({row['path'] for row in records.values()}):
        info = zipfile.ZipInfo(path, date_time=(1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_STORED
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        archive.writestr(info, (BLOB_CACHE / path).read_bytes())
container = container_path.read_bytes()
manifest['container'] = dict(path='records.zip', format='ZIP_STORED gzip members',
                             sha256=sha(container), bytes=len(container))
(D / 'records.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps({key: value for key, value in manifest.items() if key != 'records'}))
