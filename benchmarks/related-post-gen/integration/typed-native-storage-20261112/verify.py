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
        assert values[0]['parallel_peak_workers'] == 4
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
config = read('first-guarded-typed/config.json')
variants = {v['name']: v for v in primary['implementations']}
assert len(variants) == 35 and len(primary['samples']) == 1085
assert primary['execution_order']['rounds'] == orders(config['implementations'], 202611120913, 31)
times = {}
paired_rows = {}
for index, order in enumerate(primary['execution_order']['rounds']):
    assert [r['variant'] for r in primary['samples'] if r['sample'] == index] == order
for row in primary['samples']:
    name = row['variant']
    assert row['exit'] == 0 and row['exact_rust_output'] is True
    exact(blob(f'first-guarded-typed/runs/{name}-{row["sample"]}.json'), row['output_sha256'])
    value = elapsed(variants[name], row['stdout'])
    close(value, row['post_json_seconds'] if variants[name]['kind'] == 'mech' else row['seconds'])
    paired_rows[row['sample'], name] = value
    times.setdefault(name, []).append(value)
    assert process(blob('first-guarded-typed/runs/' + Path(row['process_resources_raw']).name).decode()) == row['process_resources']
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
rng = random.Random(202611120914)
for comparison in paired['results']:
    values = [100 * (paired_rows[i, comparison['candidate']] / paired_rows[i, comparison['control']] - 1) for i in range(31)]
    close(statistics.median(values), comparison['paired_percent'])
    assert comparison['wins'] == sum(v < 0 for v in values)
    draws = sorted(statistics.median(rng.choices(values, k=31)) for _ in range(10000))
    for a, b in zip((draws[250], draws[9749]), comparison['confidence_95']):
        close(a, b)

prefix = 'independent-compact-key-u32-prepared'
confirmation = read(report['confirmation']['record'])
plan = read(report['confirmation']['plan'])
assert sha(blob(report['confirmation']['plan'])) == confirmation['plan_sha256']
assert plan['rounds'] == orders(plan['implementations'], plan['seed'], 13)
assert len(confirmation['samples']) == confirmation['exact_outputs'] == 351
assert confirmation['samples'] == [json.loads(line) for line in blob(prefix + '/samples.jsonl').splitlines()]
confirm_variants = {v['name']: v for v in plan['implementations']}
confirm_times = {}
confirm_pairs = {}
for i, order in enumerate(plan['rounds']):
    assert [r['variant'] for r in confirmation['samples'] if r['round'] == i] == order
for row in confirmation['samples']:
    assert row['exit'] == 0 and row['timed_out'] is False and row['exact_output'] is True
    exact(pin(prefix, row['output']))
    stdout = pin(prefix, row['stdout']).decode()
    pin(prefix, row['stderr'])
    value = elapsed(confirm_variants[row['variant']], stdout)
    if row['post_json_eligible']:
        close(value, row['post_json_seconds'])
        confirm_times.setdefault(row['variant'], []).append(value)
        confirm_pairs[row['round'], row['variant']] = value
    close(row['whole_process_nanoseconds'] / 1e9, row['whole_process_seconds'])
confirm_medians = {n: statistics.median(v) for n, v in confirm_times.items()}
for name, summary in confirmation['summary'].items():
    assert summary['samples'] == summary['exact_outputs'] == 13
    if summary['post_json_eligible']:
        close(summary['median_post_json_seconds'], confirm_medians[name])
for key in ('original_20_serial_Rust', 'current_22_eligible_Rust'):
    ref = confirmation['references'][key]
    assert min(ref['eligible_variants'], key=confirm_medians.__getitem__) == ref['variant']
    close(confirm_medians[ref['variant']], ref['median_seconds'])
for comparison in confirmation['candidate_paired_comparisons']:
    values = [100 * (confirm_pairs[i, confirmation['candidate']] / confirm_pairs[i, comparison['reference']] - 1) for i in range(13)]
    assert values == comparison['paired_percent_changes']
    close(statistics.median(values), comparison['median_paired_percent_change'])
    rng = random.Random(comparison['bootstrap_seed'])
    draws = sorted(statistics.median(rng.choices(values, k=13)) for _ in range(10000))
    for a, b in zip((draws[249], draws[9749]), comparison['bootstrap_95_interval_percent']):
        close(a, b)
assert confirmation['target_met_against_current_fastest_Rust'] is False

prefix = 'new-managed-resources/resources-guarded-first'
resources = read(report['resources']['record'])
plan = read(report['resources']['plan'])
assert sha(blob(report['resources']['plan'])) == resources['plan_sha256']
assert plan['rounds'] == orders(plan['implementations'], plan['seed'], 7)
assert len(resources['samples']) == resources['exact_outputs'] == 224
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


assert counts(read('typed-replay-compiler-v3/typed-fixed-contracts.json')) == (6, 0, 0)
assert counts(read('typed-replay-compiler-v3/engine-call-regression-tests.json')) == (131, 0, 0)
contracts = read('guarded-owner-fixture-tests-v2/guarded-typed-contracts.json')
for item in contracts:
    assert item['exit'] == 0
assert counts(next(v for v in contracts if v['filter'] == 'guarded_dictionary')) == (6, 0, 0)
assert counts(next(v for v in contracts if v['filter'] == 'utf8_checked')) == (6, 0, 0)
broad = read('guarded-owner-fixture-tests-v2/broad-native-tests.json')
assert counts(broad) == (1067, 18, 11)
attribution = read('guarded-owner-broad-attribution/attribution.json')
assert len(attribution['retained_failures']) == 16 and len(attribution['new_failures']) == 2
assert attribution['prior_failed_names_equal_attributed_baseline'] is True
baseline = read('native-failure-attribution.json')
assert baseline['unchanged_baseline_reproduces_all_failures'] is True
assert sorted(attribution['retained_failures']) == sorted(baseline['failed_names'])
failure_lines = re.findall(r'^    (native_artifact::[^\n]+)$', broad['stdout'], re.MULTILINE)
assert sorted(failure_lines) == sorted(attribution['retained_failures'] + [r['name'] for r in attribution['new_failures']])
assert attribution['broad_rerun'] is False
assert counts(read('guarded-owner-version-fixture-tests-v3/focused-huffman-tests.json')) == (2, 0, 0)
initial_finite = read('typed-yield-fixture-tests-v5/finite-boundary-diagnostic.json')
assert initial_finite['exit'] == 101 and counts(initial_finite) == (0, 1, 0)


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
            posts = read('published-commit/benchmarks/related-post-gen/fixtures/posts-' + name + '.json')
            indices = read('published-commit/benchmarks/related-post-gen/fixtures/expected-' + name + '.json')['related_indices']
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

compact = read(report['qualification']['compact'])
assert len(compact['records']) == 6
for row in compact['records']:
    assert row['compiler_IR'] == 336 and row['full5000_exact'] is True
    assert row['maintained_edge_cases'] == 31 and row['fullinput_workers'] == 4
    label, form = row['label'], row['form']
    exact(blob('compact-guarded-preparation/' + label + '/' + form + '.output.json'))
    edge_set('compact-guarded-preparation/' + label + '/' + form + '.edges')
    for key in ('source', 'object', 'metadata', 'coordinate_include', 'link_audit', 'build_receipt'):
        assert row[key]['sha256'] in digests

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

profiles = read(report['profiles']['record'])
assert profiles['status'] == 'complete' and profiles['diagnostic_only'] is True
assert len(profiles['drivers']) == 2
for driver in profiles['drivers']:
    assert driver['exit'] == driver['sampler_exit'] == 0
    assert driver['exact5000'] is driver['native_objects_unchanged'] is driver['diagnostic_only'] is True
    value = driver['measurement']
    assert value['iterations'] == 5001 and value['posts'] == 5000 and value['timing_claim'] is False
    assert value['fresh_inputs_and_ledger_each_iteration'] is True
    assert value['fresh_worker_tasks_each_iteration'] == 4
    assert value['includes_json_decode_each_iteration'] is value['includes_iteration_result_and_input_destruction'] is True
    assert sum(value['observed_peak_worker_histogram'].values()) == 5001
    exact(blob('fullpath-profile-preparation-v2/' + driver['label'] + '/final-output.json'))
    raw = blob('fullpath-profile-preparation-v2/' + driver['label'] + '/profile/stdout.txt').decode()
    assert json.loads(next(line for line in raw.splitlines() if line.startswith('{'))) == value
partial = read(report['profiles']['prior_partial_failure_retained'])
assert partial['exit'] != 0
assert read('fullpath-profile-preparation-v2/ir334-scoped-frame/profile/offline-finalization.json')['native_exit'] == 0

commit = report.get('published_source_equivalent_commit', report['measured_source_commit'])


def git_files(commit, names):
    raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT, check=True, capture_output=True,
                         input=''.join(commit + ':' + name + '\n' for name in names).encode()).stdout
    cursor = 0
    result = {}
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
inventory = read('guarded-owner-version-fixture-tests-v3/source-inventory.json')
published = git_files(commit, sorted(inventory['files']))
assert len(published) == closure['source_files'] == 2814
for name, expected in inventory['files'].items():
    assert sha(published[name]) == expected, name
assert len(closure['overlay']) == 23
base = git_files(report['base_IR334_source_commit'], sorted(inventory['files']))
assert {name: sha(raw) for name, raw in published.items() if raw != base[name]} == closure['overlay']
for name, expected in closure['overlay'].items():
    assert sha(blob('measured-source/' + name)) == expected == sha(published[name])
for label, overrides in closure['inventory_overrides'].items():
    current = read(label + '/source-inventory.json')
    assert sha(json.dumps(current['files'], sort_keys=True).encode()) == current['sha256']
    for name, expected in current['files'].items():
        raw = blob('inventory-overrides/' + label + '/' + name) if name in overrides else published[name]
        assert sha(raw) == expected, (label, name)
literal = read('typed-replay-compiler-v3/source-inventory.json')['literal_includes']
assert len(literal) == closure['literal_includes'] == 164
for name, raw in git_files(commit, sorted(literal)).items():
    assert sha(raw) == literal[name]
patch = subprocess.run(['git', 'diff', '--binary', report['base_IR334_source_commit'], commit,
                        '--', *sorted(closure['overlay'])], cwd=ROOT, capture_output=True, check=True).stdout
assert patch == blob(closure['patch'])
engine = read('typed-replay-compiler-v3/build.json')
gpu = read('guarded-owner-compiler-v1/build.json')
assert len(engine['stages']) == len(gpu['stages']) == 12
assert set(engine['rebuild']) == {'mech_engine', 'mech_compute', 'mech_stdlib', 'mech_runtime', 'mech_gpu'}
old = {s['crate']: s for s in engine['stages']}
for stage in engine['stages'] + gpu['stages']:
    assert stage['exit'] == 0
prefix_build = {s['crate']: s for s in read('prefix-compiler-v1/build.json')['stages']}
for stage in engine['stages']:
    if stage.get('reused'):
        assert stage['configuration_unchanged'] is True
        for key in ('command', 'environment', 'library_sha256', 'source_sha256'):
            assert stage[key] == prefix_build[stage['crate']][key]
for stage in gpu['stages']:
    if stage['crate'] != 'mech_gpu':
        assert stage['library_sha256'] == old[stage['crate']]['library_sha256']
for label in ('guarded-owner-wide', 'guarded-owner-byte', 'key-u32-probe'):
    for kind in ('aot', 'metrics'):
        receipt = read(label + '/runner-' + kind + '.build.json')
        assert receipt['compiler_build_sha256'] == sha(blob('guarded-owner-compiler-v1/build.json'))
        libraries = {s['crate']: s['library_sha256'] for s in gpu['stages']}
        for name, value in receipt['libraries'].items():
            if name.startswith('mech_'):
                assert libraries[name] == value['sha256']
assert report['timed_exact_outputs'] == 1085 + 224 + 351 == 1660
print(json.dumps({'verified_records': len(records), 'unique_blobs': len(checked),
                  'compressed_bytes': manifest['compressed_bytes'], 'measured_source_commit': commit,
                  'source_inventory_entries': 2814, 'literal_include_entries': 164,
                  'primary_exact_outputs': 1085, 'independent_exact_outputs': 351,
                  'resource_exact_outputs': 224, 'managed_qualifications': 4,
                  'compact_original_decoded_qualifications': 6, 'edges_per_variant': 31,
                  'compact_resource_qualification_executions': 96,
                  'managed_IR334_still_selected': True, 'compact_U32_gain_confirmed': True,
                  'fastest_Rust_target_met': False, 'native_executions_by_verifier': 0}, indent=2))
