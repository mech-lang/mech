#!/usr/bin/env python3
"""Offline source, timing, resource and correctness verification. No native execution."""
from functools import lru_cache
from pathlib import Path
import gzip
import hashlib
import json
import math
import random
import re
import statistics
import subprocess

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
report = json.loads((D / 'report.json').read_bytes())
manifest = json.loads((D / 'records.json').read_bytes())
records = {}
digests = {}
checked = set()
for row in manifest['records']:
    assert row['source'] not in records
    path = D / row['path']
    assert path.resolve().is_relative_to(D)
    if row['path'] not in checked:
        packed = path.read_bytes()
        assert len(packed) == row['compressed_bytes']
        assert hashlib.sha256(packed).hexdigest() == row['compressed_sha256']
        raw = gzip.decompress(packed)
        assert len(raw) == row['uncompressed_bytes']
        assert hashlib.sha256(raw).hexdigest() == row['uncompressed_sha256']
        checked.add(row['path'])
    records[row['source']] = row
    digests[row['uncompressed_sha256']] = row['path']
assert len(records) == manifest['logical_records']
assert len(checked) == manifest['unique_blobs']
assert sum((D / name).stat().st_size for name in checked) == manifest['compressed_bytes']
assert {str(p.relative_to(D)) for p in (D / 'records').glob('*.gz')} == checked


@lru_cache(maxsize=128)
def data(path):
    return gzip.decompress((D / path).read_bytes())


def blob(name):
    return data(records[name]['path'])


def read(name):
    return json.loads(blob(name))


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def close(a, b):
    assert math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12), (a, b)


def pin(prefix, value):
    raw = blob(prefix + '/' + value['path'])
    assert sha(raw) == value['sha256'] and len(raw) == value['bytes']
    return raw


gold = read('publication-inputs/gold.json')
assert len(gold) == len(read('publication-inputs/posts.json')) == 5000
gold_digest = sha(blob('publication-inputs/gold.json'))
exact_digests = set()


def exact(raw, expected_digest=gold_digest):
    digest = sha(raw)
    assert digest == expected_digest
    if digest not in exact_digests:
        assert json.loads(raw) == gold
        exact_digests.add(digest)


TIMER = re.compile(r'^Processing time \(w/o IO\):\s*([0-9]+(?:\.[0-9]+)?)\s*(ns|µs|μs|us|ms|s)\s*$', re.MULTILINE)
UNITS = {'ns': 1e-9, 'µs': 1e-6, 'μs': 1e-6, 'us': 1e-6, 'ms': 1e-3, 's': 1}


def elapsed(variant, stdout):
    if variant['kind'] == 'mech':
        values = [json.loads(line) for line in stdout.splitlines() if line.startswith('{')]
        values = [v for v in values if 'post_json_seconds' in v]
        assert len(values) == 1 and values[0]['posts'] == 5000 and values[0]['sample'] == 0
        result = values[0]['post_json_seconds']
        assert 1 <= values[0]['parallel_peak_workers'] <= 4
        assert values[0]['parallel_batches'] == 1
        if 'parallel_tasks' in values[0]:
            assert values[0]['parallel_tasks'] == 4
    else:
        values = TIMER.findall(stdout)
        assert len(values) == 1
        result = float(values[0][0]) * UNITS[values[0][1]]
    assert math.isfinite(result) and result > 0
    return result


def process(text):
    result = {}
    for line in text.splitlines():
        tokens = line.strip().split()
        if len(tokens) == 2 and tokens[0] in ('real', 'user', 'sys'):
            result[{'real': 'wall_seconds', 'user': 'user_cpu_seconds', 'sys': 'system_cpu_seconds'}[tokens[0]]] = float(tokens[1])
        elif tokens and tokens[0].isdigit():
            key = '_'.join(tokens[1:])
            key = {'maximum_resident_set_size': 'peak_resident_bytes',
                   'peak_memory_footprint': 'peak_physical_footprint_bytes',
                   'cycles_elapsed': 'cycles'}.get(key, key)
            result[key] = int(tokens[0])
    return result


def medians(rows):
    result = {}
    for key in sorted(set.intersection(*(set(r) for r in rows))):
        values = [r[key] for r in rows]
        if all(isinstance(v, dict) for v in values):
            result[key] = medians(values)
        elif all(type(v) in (int, float) for v in values):
            result[key] = statistics.median(values)
    return result


def orders(variants, seed, rounds):
    rng = random.Random(seed)
    result = []
    for _ in range(rounds):
        names = [v['name'] for v in variants]
        rng.shuffle(names)
        result.append(names)
    return result


primary = read(report['primary']['record'])
config = read('first-setup-reuse/config.json')
variants = {v['name']: v for v in primary['implementations']}
assert len(variants) == 39 and len(primary['samples']) == 1209
assert primary['execution_order']['rounds'] == orders(config['implementations'], 202611120953, 31)
managed_work = {}
for name, qualification in report['primary']['managed_work_qualifications'].items():
    qualified_raw = blob(qualification['record'])
    assert sha(qualified_raw) == qualification['sha256']
    qualified = json.loads(qualified_raw)
    assert qualified['exit'] == 0
    value = next(json.loads(line) for line in qualified['stdout'].splitlines() if line.startswith('{'))
    assert value['posts'] == 5000 and value['parallel_tasks'] == 4 and value['parallel_batches'] == 1
    assert value['work'] == qualification['work']
    assert value['parallel_numeric_work'] == qualification['parallel_numeric_work']
    assert value['parallel_numeric_regions'] == qualification['parallel_numeric_regions']
    managed_work[name] = qualification
peak_histograms = {name: {} for name in managed_work}
work_histograms = {name: {} for name in managed_work}
assert [name for name, q in managed_work.items() if not q['exact_total_required']] == ['mech-managed-typed-fixed-byte']
times = {}
paired_rows = {}
for index, order in enumerate(primary['execution_order']['rounds']):
    assert [r['variant'] for r in primary['samples'] if r['sample'] == index] == order
for row in primary['samples']:
    name = row['variant']
    assert row['exit'] == 0 and row['exact_rust_output'] is True
    exact(blob(f'first-setup-reuse/runs/{name}-{row["sample"]}.json'), row['output_sha256'])
    value = elapsed(variants[name], row['stdout'])
    if name in managed_work:
        record = next(json.loads(line) for line in row['stdout'].splitlines() if line.startswith('{'))
        assert record['parallel_tasks'] == 4 and record['parallel_batches'] == 1
        if managed_work[name]['exact_total_required']:
            assert record['work'] == managed_work[name]['work']
        assert record['parallel_numeric_work'] == managed_work[name]['parallel_numeric_work']
        assert record['parallel_numeric_regions'] == managed_work[name]['parallel_numeric_regions']
        bill = str(record['work'])
        work_histograms[name][bill] = work_histograms[name].get(bill, 0) + 1
        peak = str(record['parallel_peak_workers'])
        peak_histograms[name][peak] = peak_histograms[name].get(peak, 0) + 1
    close(value, row['post_json_seconds'] if variants[name]['kind'] == 'mech' else row['seconds'])
    paired_rows[row['sample'], name] = value
    times.setdefault(name, []).append(value)
    assert process(blob('first-setup-reuse/runs/' + Path(row['process_resources_raw']).name).decode()) == row['process_resources']
assert peak_histograms == report['primary']['managed_observed_peak_worker_histograms']
assert work_histograms == report['primary']['managed_total_work_histograms']
assert all(sum(histogram.values()) == 31 for histogram in peak_histograms.values())
timings = {name: statistics.median(values) for name, values in times.items()}
for name, variant in variants.items():
    rows = [r for r in primary['samples'] if r['variant'] == name]
    summary = primary['summary'][name]
    assert summary['samples'] == 31 and summary['all_exact_rust'] is True
    close(summary['median_seconds'], statistics.median(r['seconds'] for r in rows))
    close(summary['median_whole_process_seconds'], statistics.median(r['wall_seconds'] for r in rows))
    assert summary['process_resources_median'] == medians([r['process_resources'] for r in rows])
    if variant.get('calculation_boundary') == 'post_json_including_setup':
        close(summary['median_post_json_seconds'], timings[name])
eligible = [v['name'] for v in variants.values() if v['kind'] != 'mech' and v.get('calculation_boundary') == 'post_json_including_setup']
assert len(eligible) == 22
reference = min(eligible, key=timings.__getitem__)
assert reference == primary['primary_reference']['variant'] == report['primary']['current_fastest_Rust']['variant']
close(timings[reference], primary['primary_reference']['median_seconds'])
for row in report['primary']['rows']:
    name = row['variant']
    close(row['post_json_ms'], timings[name] * 1000)
    close(row['times_current_fastest_Rust'], timings[name] / timings[reference])
paired = read(report['primary']['paired'])
rng = random.Random(202611120954)
for comparison in paired['results']:
    values = [100 * (paired_rows[i, comparison['candidate']] / paired_rows[i, comparison['control']] - 1) for i in range(31)]
    close(statistics.median(values), comparison['paired_percent'])
    assert comparison['wins'] == sum(v < 0 for v in values)
    draws = sorted(statistics.median(rng.choices(values, k=31)) for _ in range(10000))
    for a, b in zip((draws[250], draws[9749]), comparison['confidence_95']):
        close(a, b)

prefix = 'new-managed-resources/resources-setup-first'
resources = read(report['resources']['record'])
plan = read(report['resources']['plan'])
assert sha(blob(report['resources']['plan'])) == resources['plan_sha256']
assert plan['rounds'] == orders(plan['implementations'], plan['seed'], 7)
assert len(resources['samples']) == resources['exact_outputs'] == 259
assert resources['samples'] == [json.loads(line) for line in blob(prefix + '/samples.jsonl').splitlines()]
assert resources['latency_selection_forbidden'] is True and resources['whole_process_is_acceptance_gate'] is False
resource_variants = {v['name']: v for v in plan['implementations']}
for i, order in enumerate(plan['rounds']):
    assert [r['variant'] for r in resources['samples'] if r['round'] == i] == order
for row in resources['samples']:
    assert row['exit'] == 0 and row['timed_out'] is False and row['exact_output'] is True
    exact(pin(prefix, row['output']))
    stdout = pin(prefix, row['stdout']).decode()
    pin(prefix, row['stderr'])
    assert process(pin(prefix, row['process_resources_raw']).decode()) == row['whole_process_counters']
    variant = resource_variants[row['variant']]
    if variant['kind'] == 'mech':
        value = json.loads(next(line for line in stdout.splitlines() if line.startswith('{')))
        assert value == row['raw_adapter_record']
        control_name = variant.get('latency_control_name', variant['name'].removesuffix('-instrumented'))
        if control_name in managed_work:
            assert managed_work[control_name]['exact_total_required'] is True
            assert value['work'] == managed_work[control_name]['work']
            assert value['parallel_numeric_work'] == managed_work[control_name]['parallel_numeric_work']
            assert value['parallel_numeric_regions'] == managed_work[control_name]['parallel_numeric_regions']
            assert value['parallel_tasks'] == 4 and value['parallel_batches'] == 1
            assert 1 <= value['parallel_peak_workers'] <= 4
        assert row['windows']['post_json'] == value['measurement']
        close(row['instrumented_post_json_seconds_diagnostic_only'], value['post_json_seconds'])
        for raw, label in [('restore_measurement', 'restore'), ('json_measurement', 'json'), ('admission_measurement', 'admission')]:
            if raw in value:
                assert row['windows'][label] == value[raw]
    elif variant['instrumented']:
        value = json.loads(next(line[len('PROCESS_METRICS '):] for line in stdout.splitlines() if line.startswith('PROCESS_METRICS ')))
        assert value == row['raw_adapter_record']
        label = 'post_json' if variant['post_json_counter_eligible'] else 'excluded_processing'
        assert row['windows'][label] == value.get('processing', value)
    for window in row['windows'].values():
        assert window['allocator_accounting_underflows'] == window['allocator_lifetime_accounting_underflows'] == 0
        assert window['hardware']['instructions_retired'] > 0 and window['hardware']['cycles'] > 0
for name, summary in resources['summary'].items():
    rows = [r for r in resources['samples'] if r['variant'] == name]
    assert len(rows) == summary['samples'] == 7 and summary['all_exact'] is True
    assert medians([r['whole_process_counters'] for r in rows]) == summary['whole_process_medians']
    for window, value in summary['window_medians'].items():
        assert medians([r['windows'][window] for r in rows]) == value

def counts(value):
    result = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', value['stdout'])
    assert len(result) == 1
    return tuple(map(int, result[0]))


focused = read(report['qualification']['focused'])
assert len(focused) == 8
assert [counts(value) for value in focused] == [(2, 0, 0), (18, 0, 0), (4, 0, 0), (1, 0, 0), (5, 0, 0), (6, 0, 0), (2, 0, 0), (12, 0, 0)]
assert all(value['exit'] == 0 for value in focused)
passed_names = [name for value in focused for name in re.findall(r'^test (\S+) \.\.\. ok$', value['stdout'], re.MULTILINE)]
assert len(passed_names) == len(set(passed_names)) == report['qualification']['focused_passed'] == 50
compact_test = read(report['qualification']['compact_typed'])
assert compact_test['exit'] == 0 and counts(compact_test) == (1, 0, 0)
assert re.findall(r'^test (\S+) \.\.\. ok$', compact_test['stdout'], re.MULTILINE)[0] not in passed_names
broad = read(report['qualification']['broad'])
baseline = read(report['qualification']['baseline_broad'])
assert broad['exit'] == baseline['exit'] == 101
assert counts(broad) == (1079, 16, 11) and counts(baseline) == (1056, 16, 11)
assert '35 filtered out' in broad['stdout']
failure_names = lambda value: sorted(re.findall(r'^    (native_artifact::[^\n]+)$', value['stdout'], re.MULTILINE))
attribution = read(report['qualification']['broad_attribution'])
assert failure_names(broad) == failure_names(baseline) == sorted(attribution['failures'])
assert len(attribution['failures']) == 16
assert sha(blob(report['qualification']['broad'])) == attribution['current_report_sha256']
assert read('native-failure-attribution.json')['unchanged_baseline_reproduces_all_failures'] is True


def edge_set(prefix):
    edges = read(prefix + '/edge-fixtures.json')
    assert len(edges) == 31
    for edge in edges:
        assert edge['exit'] == 0
        name = edge['fixture']
        if edge.get('exact_rust_output'):
            assert read(prefix + '/edges/' + name + '/actual.json') == read(prefix + '/edges/' + name + '/related_posts_rust.json')
        else:
            assert edge['exact_expected_output'] is True
            posts = read('base-commit/benchmarks/related-post-gen/fixtures/posts-' + name + '.json')
            indices = read('base-commit/benchmarks/related-post-gen/fixtures/expected-' + name + '.json')['related_indices']
            expected = [{'_id': p['_id'], 'tags': p['tags'], 'related': [posts[i] for i in neighbors]} for p, neighbors in zip(posts, indices)]
            assert read(prefix + '/actual-' + name + '.json') == expected


for label in report['qualification']['managed']:
    exact(blob(label + '/output.json'))
    edge_set(label + '/edges')
    audit = read(label + '/same-object-audit.json')
    assert audit['status'] == 'passed'
    assert audit['every_native_symbol_has_exact_object_relative_address'] is True
    assert audit['all_text_equal_except_independently_verified_relocation_fields'] is True
    assert sha(blob(label + '/build/object.o')) == audit['object']['sha256']
    for kind in ('aot', 'metrics'):
        receipt = read(label + '/runner-' + kind + '.build.json')
        assert receipt['exit'] == 0
        if kind == 'aot':
            assert receipt['executable_sha256'] == audit['executable']['sha256']
        for value in receipt['bundle'].values():
            assert value['sha256'] in digests

resource_qualification = read(report['qualification']['compact_resource_adapters'])
assert resource_qualification['latency_claim'] is resource_qualification['resource_claim'] is False
assert resource_qualification['correctness_executions'] == 96
assert len(resource_qualification['records']) == 3
for row in resource_qualification['records']:
    assert row['full5000_exact'] is row['measurement_shape_valid'] is True
    assert row['native_payload_counters_identical_to_ordinary'] is True
    assert row['allocator_accounting_underflows'] == 0
    assert row['edge_cases_exact'] == 31 and row['fullinput_workers'] == 4
    exact(blob('compact-guarded-resource-preparation/' + row['label'] + '/output.json'))
    edge_set('compact-guarded-resource-preparation/' + row['label'] + '/edges')
    for key in ('source', 'native_object', 'coordinates', 'native_link_audit', 'edge_receipt'):
        assert row[key]['sha256'] in digests
provenance = read('compact-guarded-resource-preparation/provenance-audit.json')
assert provenance['status'] == 'passed' and len(provenance['proofs']) == 3
for row in provenance['proofs']:
    assert row['libraries_identical_to_ordinary'] is row['native_bundle_identical_to_ordinary'] is True
    assert row['ordinary_executable_unchanged'] is row['instrumentation_inverse_matches_exact_ordinary_source'] is True

for instrumented in (False, True):
    prefix = 'rust-fresh4-control/' + ('metrics/' if instrumented else '')
    qualification = read(prefix + 'qualification-summary.json')
    for control in qualification['controls']:
        folder = prefix + control['label'] + '/'
        assert sha(blob(folder + 'src/main.rs')) == control['source_sha256']
        assert control['exact_full5000'] is True and control['unit_tests_passed'] == 5
        assert counts(read(folder + 'tests.json')) == (5, 0, 0)
        exact(blob(folder + 'full5000-fresh4-work/related_posts_rust_mech.json'))
        if not instrumented:
            assert control['exact_edges'] == 31 and control['full5000_fresh_query_workers'] == 4

# Source closure: verify the immutable whole-source inventories against published
# IR336 plus only their exact changed bytes. No live working tree authority.
import difflib


def git_files(commit, names):
    raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT, check=True, capture_output=True,
                         input=''.join(commit + ':' + name + '\n' for name in names).encode()).stdout
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


closure = report['source_closure']
base_commit = report['base_IR336_source_commit']
base_inventory = read('guarded-owner-version-fixture-tests-v3/source-inventory.json')
base_source = report['base_IR336_source_equivalent_commit']
base = git_files(base_source, sorted(base_inventory['files']))
assert len(base) == closure['base_source_files'] == 2814
for name, expected in base_inventory['files'].items():
    assert sha(base[name]) == expected, name
inventories, reconstructed = {}, {}
for label, expected_count in [('static-replay-compiler-v1', 2816), ('owned-capture-compiler-v1', 2817)]:
    inventory = read(label + '/source-inventory.json')
    assert len(inventory['files']) == expected_count
    assert sha(json.dumps(inventory['files'], sort_keys=True).encode()) == inventory['sha256']
    overrides = closure['inventory_overrides'][label]
    current = {}
    for name, expected in inventory['files'].items():
        raw = blob('inventory-overrides/' + label + '/' + name) if name in overrides else base[name]
        assert sha(raw) == expected, (label, name)
        current[name] = raw
    assert {name: sha(raw) for name, raw in current.items() if base.get(name) != raw} == overrides
    inventories[label], reconstructed[label] = inventory, current
final = reconstructed['owned-capture-compiler-v1']
assert len(closure['overlay']) == 13
assert {name: sha(raw) for name, raw in final.items() if base.get(name) != raw} == closure['overlay']
for name, expected in closure['overlay'].items():
    assert sha(blob('measured-source/' + name)) == expected == sha(final[name])
patch_parts = []
for name in sorted(closure['overlay']):
    patch_parts.append('diff --git a/' + name + ' b/' + name + '\n')
    patch_parts.extend(difflib.unified_diff(base.get(name, b'').decode().splitlines(True), final[name].decode().splitlines(True),
                       fromfile='a/' + name if name in base else '/dev/null', tofile='b/' + name))
assert ''.join(patch_parts).encode() == blob(closure['patch'])
overlay = read(closure['overlay_record'])
assert overlay['base_commit'] == base_commit and overlay['files'] == closure['overlay']
assert overlay['new_files'] == sorted(set(final) - set(base))
assert len(overlay['new_files']) == 3 and overlay['benchmark_selected'] is False
assert overlay['patch_sha256'] == sha(blob(closure['patch']))
assert git_files(report['published_source_equivalent_commit'], sorted(final)) == final
# Original pre-rebase objects may be absent in a fresh checkout. The required
# published aliases above still validate every source byte against the frozen
# inventories; verify additional historical objects too when present.
for commit, expected in ((base_commit, base), (report['measured_source_commit'], final)):
    present = subprocess.run(['git', 'cat-file', '-e', commit + '^{commit}'], cwd=ROOT, capture_output=True).returncode == 0
    if present:
        assert git_files(commit, sorted(expected)) == expected
rebase = read('IR338-rebase-latest973/receipt.json')
assert rebase['status'] == 'pass' and rebase['exact_source_files'] == 2817
assert rebase['production_code_changes_from_base'] == 0
assert rebase['measured_source_commit'] == report['measured_source_commit']
assert rebase['published_equivalent_source_commit'] == report['published_source_equivalent_commit']
literal = read('typed-replay-compiler-v3/source-inventory.json')['literal_includes']
assert len(literal) == closure['literal_includes'] == 164
for name, raw in git_files(base_source, sorted(literal)).items():
    assert sha(raw) == literal[name]
for label, IR in [('static-replay-compiler-v1', 337), ('owned-capture-compiler-v1', 338)]:
    identity = reconstructed[label]['hosts/gpu/src/native_artifact/program/image/identity.rs'].decode()
    assert re.search(r'NATIVE_PROGRAM_TYPED_IR_VERSION\s*:\s*u64\s*=\s*' + str(IR) + r'\s*;', identity)

# Candidate development receipts are historical observations; their early
# scratch-only descriptions remain unchanged after later build/test receipts.
for label, field in [('typed-static-replay-facts-candidate', 'source_pins'), ('worker-owned-capture-candidate', 'after_pins')]:
    audit = read(label + '/audit.json')
    assert sha(blob(label + '/candidate.patch')) == audit['patch_sha256']
    for name, expected in audit[field].items():
        assert sha(final[name]) == expected

engine = read('typed-replay-compiler-v3/build.json')
base_gpu = read('guarded-owner-compiler-v1/build.json')
prefix = {s['crate']: s for s in read('prefix-compiler-v1/build.json')['stages']}
assert len(engine['stages']) == len(base_gpu['stages']) == 12
assert set(engine['rebuild']) == {'mech_engine', 'mech_compute', 'mech_stdlib', 'mech_runtime', 'mech_gpu'}
for stage in engine['stages']:
    assert stage['exit'] == 0
    if stage.get('reused'):
        assert stage['configuration_unchanged'] is True
        for key in ('command', 'environment', 'library_sha256', 'source_sha256'):
            assert stage[key] == prefix[stage['crate']][key]
engine_stages = {s['crate']: s for s in engine['stages']}
for stage in base_gpu['stages']:
    assert stage['exit'] == 0
    if stage['crate'] != 'mech_gpu':
        assert stage['library_sha256'] == engine_stages[stage['crate']]['library_sha256']
base_stages = {s['crate']: s for s in base_gpu['stages']}
builds = {}
for label in ('static-replay-compiler-v1', 'owned-capture-compiler-v1'):
    build = read(label + '/build.json')
    assert len(build['stages']) == 12
    assert build['configuration_provenance']['non_gpu_sources_verified_unchanged'] is True
    assert build['source_sha256'] == inventories[label]['sha256']
    for stage in build['stages']:
        assert stage['exit'] == 0
        if stage['crate'] != 'mech_gpu':
            assert stage['library_sha256'] == base_stages[stage['crate']]['library_sha256']
            for field in ('command', 'environment', 'source_sha256'):
                assert stage[field] == base_stages[stage['crate']][field]
    builds[label] = build
assert read('owned-capture-compiler-v1/tests.build.json')['exit'] == 0
assert read('owned-capture-compiler-v1/tests.build.json')['source_sha256'] == inventories['owned-capture-compiler-v1']['sha256']
for label in report['qualification']['managed']:
    compiler = 'static-replay-compiler-v1' if label.startswith('static-replay') else 'owned-capture-compiler-v1'
    libraries = {s['crate']: s['library_sha256'] for s in builds[compiler]['stages']}
    for kind in ('aot', 'metrics'):
        receipt = read(label + '/runner-' + kind + '.build.json')
        assert receipt['compiler_build_sha256'] == sha(blob(compiler + '/build.json'))
        assert receipt['compiler_source_sha256'] == inventories[compiler]['sha256']
        for name, value in receipt['libraries'].items():
            if name.startswith('mech_'):
                assert libraries[name] == value['sha256']

# Verify every retained plan pin, including nested managed/compact bundles.
# Binaries are deliberately omitted; their exact digests remain in receipts.
def verify_pins(value):
    if isinstance(value, dict):
        if isinstance(value.get('path'), str) and 'sha256' in value:
            name = 'referenced-inputs/' + value['path'].lstrip('/')
            if name in records:
                raw = blob(name)
                assert sha(raw) == value['sha256']
                if 'bytes' in value:
                    assert len(raw) == value['bytes']
            else:
                assert value['path'] in json.loads((D / 'omitted-binaries.json').read_bytes()), value['path']
        for child in value.values():
            verify_pins(child)
    elif isinstance(value, list):
        for child in value:
            verify_pins(child)

verify_pins(plan)
assert sha(blob('new-managed-resources/run-with-compact.py')) == plan['script']['sha256']
assert sha(blob('first-setup-reuse/latency.json')) == plan['latency_authority']['sha256']
run = read('first-setup-reuse/run.json')
assert run['exit'] == 0
assert sha(blob('first-setup-reuse/config.json')) == run['config_sha256']
assert sha(blob('first-setup-reuse/protocol.json')) == run['protocol_sha256']
assert sha(blob('base-commit/benchmarks/related-post-gen/compare-implementations.py')) == run['script_sha256']
for row in report['resources']['rows']:
    summary = resources['summary'][row['variant']]
    window = summary['window_medians']['post_json']
    for key in ('instructions_retired', 'cycles'):
        assert row[key] == window['hardware'][key]
    for key in ('allocation_requests', 'requested_allocation_bytes', 'peak_additional_live_requested_heap_bytes'):
        assert row[key] == window[key]
    assert row['whole_process_peak_resident_bytes'] == summary['whole_process_medians']['peak_resident_bytes']
compact_rows = [row for row in resources['samples'] if row['variant'].startswith('mech-ir336-compact-')]
assert len(compact_rows) == 21
for row in compact_rows:
    native = row['raw_adapter_record']
    assert native['native_payload_requests'] == 23
    assert native['native_requested_payload_bytes'] > 0 and native['native_root_peak_payload_bytes'] > 0
    assert native['parallel_peak_workers'] == 4 and native['parallel_batches'] == 1
    assert resource_variants[row['variant']]['ordinary_managed_contract'] is False
assert report['timed_exact_outputs'] == 1209 + 259 == 1468
assert report['new_candidates_selected'] is report['fastest_Rust_target_met'] is False
print(json.dumps({'verified_records': len(records), 'unique_blobs': len(checked),
                  'compressed_bytes': manifest['compressed_bytes'], 'base_IR336_source_commit': base_commit,
                  'IR337_source_inventory_entries': 2816, 'IR338_source_inventory_entries': 2817,
                  'literal_include_entries': 164, 'source_overlay_files': 13,
                  'primary_exact_outputs': 1209, 'resource_exact_outputs': 259,
                  'new_timed_exact_outputs': 1468, 'managed_qualifications': 4, 'edges_per_variant': 31,
                  'prior_compact_resource_qualification_executions': 96,
                  'focused_passes': 50, 'compact_typed_passes': 1,
                  'broad_passed': 1079, 'broad_failures': 16, 'all_broad_failures_inherited': True,
                  'managed_IR334_still_selected': True, 'new_managed_gain_established': False,
                  'fastest_Rust_target_met': False, 'native_executions_by_verifier': 0}, indent=2))
