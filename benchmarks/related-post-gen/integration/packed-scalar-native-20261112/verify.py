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
import zipfile

D = Path(__file__).resolve().parent
ROOT = D.parents[3]
report = json.loads((D / 'report.json').read_bytes())
manifest = json.loads((D / 'records.json').read_bytes())
container = manifest['container']
container_path = D / container['path']
assert container_path.resolve().is_relative_to(D)
container_raw = container_path.read_bytes()
assert len(container_raw) == container['bytes']
assert hashlib.sha256(container_raw).hexdigest() == container['sha256']
archive = zipfile.ZipFile(container_path)
members = archive.infolist()
assert len({member.filename for member in members}) == len(members)
assert all(member.compress_type == zipfile.ZIP_STORED for member in members)
records = {}
digests = {}
checked = set()
for row in manifest['records']:
    assert row['source'] not in records
    assert not row['path'].startswith('/') and '..' not in Path(row['path']).parts
    if row['path'] not in checked:
        packed = archive.read(row['path'])
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
assert sum(archive.getinfo(name).file_size for name in checked) == manifest['compressed_bytes']
assert {member.filename for member in members} == checked


@lru_cache(maxsize=128)
def data(path):
    return gzip.decompress(archive.read(path))


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


# Only packet files and published Git objects are read below. Historical absolute
# paths are opaque provenance labels, resolved by their archived content digest.
def pinned(value):
 raw=data(digests[value['sha256']]);assert sha(raw)==value['sha256']
 if 'bytes'in value:assert len(raw)==value['bytes']
 return raw

def ledger(name,value):
 assert value['posts']==5000 and value['sample']==0
 assert value['parallel_batches']==1 and 1<=value['parallel_peak_workers']<=4
 q=report['managed_work_qualifications'].get(name)
 if q:
  assert value['parallel_tasks']==4
  assert value['parallel_numeric_work']==q['numeric_work']
  assert value['parallel_numeric_regions']==q['numeric_regions']
  if q['exact_total_required']:assert value['work']==q['work'],name

def jstdout(stdout):return next(json.loads(line)for line in stdout.splitlines()if line.startswith('{'))
observations = {}
all_indexed = {}


def observe(cohort, name, value):
    ledger(name, value)
    result = observations.setdefault(cohort, {}).setdefault(name, dict(peak_workers={}, total_work={}))
    for field, key in [('parallel_peak_workers', 'peak_workers'), ('work', 'total_work')]:
        if field in value:
            text = str(value[field])
            result[key][text] = result[key].get(text, 0) + 1


def primary(cohort):
    prefix = cohort['prefix']
    value, config = read(cohort['record']), read(cohort['plan'])
    protocol = read(prefix + '/protocol.json')
    variants = {v['name']: v for v in value['implementations']}
    assert len(variants) == cohort['variants']
    assert len(value['samples']) == cohort['outputs']
    rounds = cohort['rounds']
    assert value['execution_order']['rounds'] == orders(config['implementations'], protocol['seed'], rounds)
    for i, order in enumerate(value['execution_order']['rounds']):
        assert [r['variant'] for r in value['samples'] if r['sample'] == i] == order
    indexed, timings = {}, {}
    for row in value['samples']:
        name = row['variant']
        assert row['exit'] == 0 and row['exact_rust_output'] is True
        exact(blob(f'{prefix}/runs/{name}-{row["sample"]}.json'), row['output_sha256'])
        seconds = elapsed(variants[name], row['stdout'])
        indexed[row['sample'], name] = seconds
        timings.setdefault(name, []).append(seconds)
        if variants[name]['kind'] == 'mech':
            observe(prefix, name, jstdout(row['stdout']))
            close(seconds, row['post_json_seconds'])
        else:
            close(seconds, row['seconds'])
        assert process(blob(prefix + '/runs/' + Path(row['process_resources_raw']).name).decode()) == row['process_resources']
    med = {name: statistics.median(rows) for name, rows in timings.items()}
    for name, variant in variants.items():
        rows = [r for r in value['samples'] if r['variant'] == name]
        summary = value['summary'][name]
        assert len(rows) == summary['samples'] == rounds and summary['all_exact_rust'] is True
        close(summary['median_seconds'], statistics.median(r['seconds'] for r in rows))
        close(summary['median_whole_process_seconds'], statistics.median(r['wall_seconds'] for r in rows))
        assert summary['process_resources_median'] == medians([r['process_resources'] for r in rows])
        if variant.get('calculation_boundary') == 'post_json_including_setup':
            close(summary['median_post_json_seconds'], med[name])
    eligible = [v['name'] for v in variants.values() if v['kind'] != 'mech' and v.get('calculation_boundary') == 'post_json_including_setup']
    assert len(eligible) == 22
    reference = min(eligible, key=med.__getitem__)
    assert reference == value['primary_reference']['variant']
    close(med[reference], value['primary_reference']['median_seconds'])
    paired = read(prefix + '/paired-comparisons.json')
    assert paired['report_sha256'] == sha(blob(cohort['record']))
    for pair in paired['comparisons']:
        changes = [100 * (indexed[i, pair['candidate']] / indexed[i, pair['control']] - 1) for i in range(rounds)]
        close(statistics.median(changes), pair['median_percent'])
        assert pair['candidate_faster_pairs'] == sum(x < 0 for x in changes)
        rng = random.Random(pair['bootstrap_seed'])
        draws = sorted(statistics.median(rng.choices(changes, k=rounds)) for _ in range(paired['bootstrap_replicates']))
        for actual, expected in zip([draws[250], draws[9749]], pair['percentile95']):
            close(actual, expected)
        assert pair['gain_established'] == (pair['percentile95'][1] < 0)
        assert pair['regression_established'] == (pair['percentile95'][0] > 0)
    run = read(prefix + '/run.json')
    assert run['exit'] == 0
    assert run['config_sha256'] == sha(blob(prefix + '/config.json'))
    assert run['expected_outputs'] == cohort['outputs']
    assert run['command'][run['command'].index('--seed') + 1] == str(protocol['seed'])
    if 'protocol_sha256' in run:
        assert run['protocol_sha256'] == sha(blob(prefix + '/protocol.json'))
    assert run['script_sha256'] == sha(blob('published-base/benchmarks/related-post-gen/compare-implementations.py'))
    all_indexed[prefix] = indexed


def resources(cohort):
    prefix = cohort['prefix']
    value, plan = read(cohort['record']), read(cohort['plan'])
    variants = {v['name']: v for v in plan['implementations']}
    assert len(variants) == cohort['variants']
    assert len(value['samples']) == value['exact_outputs'] == cohort['outputs']
    assert sha(blob(cohort['plan'])) == value['plan_sha256']
    assert plan['rounds'] == orders(plan['implementations'], plan['seed'], cohort['rounds'])
    assert value['samples'] == [json.loads(line) for line in blob(prefix + '/samples.jsonl').splitlines()]
    assert value['latency_selection_forbidden'] is True and value['whole_process_is_acceptance_gate'] is False
    for i, order in enumerate(plan['rounds']):
        assert [r['variant'] for r in value['samples'] if r['round'] == i] == order
    for row in value['samples']:
        assert row['exit'] == 0 and row['timed_out'] is False and row['exact_output'] is True
        exact(pin(prefix, row['output']))
        stdout = pin(prefix, row['stdout']).decode()
        pin(prefix, row['stderr'])
        assert process(pin(prefix, row['process_resources_raw']).decode()) == row['whole_process_counters']
        variant = variants[row['variant']]
        if variant['kind'] == 'mech':
            native = jstdout(stdout)
            assert native == row['raw_adapter_record']
            observe(prefix, variant.get('latency_control_name', variant['name'].removesuffix('-instrumented')), native)
            assert row['windows']['post_json'] == native['measurement']
            close(row['instrumented_post_json_seconds_diagnostic_only'], native['post_json_seconds'])
            for raw, label in [('restore_measurement', 'restore'), ('json_measurement', 'json'), ('admission_measurement', 'admission')]:
                if raw in native:
                    assert row['windows'][label] == native[raw]
        elif variant['instrumented']:
            native = json.loads(next(line[len('PROCESS_METRICS '):] for line in stdout.splitlines() if line.startswith('PROCESS_METRICS ')))
            assert native == row['raw_adapter_record']
            label = 'post_json' if variant['post_json_counter_eligible'] else 'excluded_processing'
            assert row['windows'][label] == native.get('processing', native)
        for window in row['windows'].values():
            assert window['allocator_accounting_underflows'] == window['allocator_lifetime_accounting_underflows'] == 0
            assert window['hardware']['instructions_retired'] > 0 and window['hardware']['cycles'] > 0
    for name, summary in value['summary'].items():
        rows = [r for r in value['samples'] if r['variant'] == name]
        assert len(rows) == summary['samples'] == cohort['rounds'] and summary['all_exact'] is True
        assert medians([r['whole_process_counters'] for r in rows]) == summary['whole_process_medians']
        for label, expected in summary['window_medians'].items():
            assert medians([r['windows'][label] for r in rows]) == expected


def confirmation_pair(pair, candidate, indexed, rounds):
    changes = [100 * (indexed[i, candidate] / indexed[i, pair['reference']] - 1) for i in range(rounds)]
    assert changes == pair['paired_percent_changes']
    close(statistics.median(changes), pair['median_paired_percent_change'])
    assert pair['candidate_faster_pairs'] == sum(x < 0 for x in changes)
    rng = random.Random(pair['bootstrap_seed'])
    draws = sorted(statistics.median(rng.choices(changes, k=rounds)) for _ in range(pair['bootstrap_draws']))
    for actual, expected in zip([draws[249], draws[9749]], pair['bootstrap_95_interval_percent']):
        close(actual, expected)


def confirmation(cohort):
    prefix = cohort['prefix']
    value, plan = read(cohort['record']), read(cohort['plan'])
    variants = {v['name']: v for v in plan['implementations']}
    rounds = cohort['rounds']
    assert len(variants) == cohort['variants']
    assert len(value['samples']) == value['exact_outputs'] == value['expected_outputs'] == cohort['outputs']
    assert sha(blob(cohort['plan'])) == value['plan_sha256']
    assert plan['rounds'] == orders(plan['implementations'], plan['seed'], rounds)
    assert value['same_machine_independent_orchestration'] is True and value['whole_process_is_acceptance_gate'] is False
    assert value['samples'] == [json.loads(line) for line in blob(prefix + '/samples.jsonl').splitlines()]
    for i, order in enumerate(plan['rounds']):
        assert [r['variant'] for r in value['samples'] if r['round'] == i] == order
    indexed = {}
    for row in value['samples']:
        assert row['exit'] == 0 and row['timed_out'] is False and row['exact_output'] is True
        exact(pin(prefix, row['output']))
        stdout = pin(prefix, row['stdout']).decode()
        pin(prefix, row['stderr'])
        name = row['variant']
        seconds = elapsed(variants[name], stdout)
        close(row['whole_process_seconds'], row['whole_process_nanoseconds'] / 1e9)
        if variants[name]['kind'] == 'mech':
            observe(prefix, name, jstdout(stdout))
        if row['post_json_eligible']:
            close(seconds, row['post_json_seconds'])
            indexed[row['round'], name] = seconds
        else:
            assert row['post_json_seconds'] is None
    for name, summary in value['summary'].items():
        rows = [r for r in value['samples'] if r['variant'] == name]
        assert len(rows) == summary['samples'] == summary['exact_outputs'] == rounds
        close(summary['median_whole_process_seconds'], statistics.median(r['whole_process_seconds'] for r in rows))
        if summary['post_json_eligible']:
            close(summary['median_post_json_seconds'], statistics.median(r['post_json_seconds'] for r in rows))
    for label, reference in value['references'].items():
        metric = 'median_whole_process_seconds' if label.startswith('whole_process') else 'median_post_json_seconds'
        winner = min(reference['eligible_variants'], key=lambda n: value['summary'][n][metric])
        assert winner == reference['variant']
        close(reference['median_seconds'], value['summary'][winner][metric])
    for pair in value['candidate_paired_comparisons']:
        confirmation_pair(pair, value['candidate'], indexed, rounds)
    additional = read(prefix + '/additional-paired-comparisons.json')
    assert additional['report_sha256'] == sha(blob(cohort['record']))
    for pair in additional['comparisons']:
        confirmation_pair(pair, pair['candidate'], indexed, rounds)
    assert value['target_met_against_current_fastest_Rust'] is False
    all_indexed[prefix] = indexed


for name, qualification in report['managed_work_qualifications'].items():
    raw = blob(qualification['record'])
    assert sha(raw) == qualification['sha256']
    receipt = json.loads(raw)
    assert receipt['exit'] == 0
    ledger(name, jstdout(receipt['stdout']))
assert [name for name, q in report['managed_work_qualifications'].items() if not q['exact_total_required']] == ['mech-managed-typed-fixed-byte']
for cohort in report['cohorts']:
    globals()[cohort['kind']](cohort)
assert sum(row['outputs'] for row in report['cohorts']) == report['timed_resource_exact_outputs'] == 4869
for table in report['timing_tables']:
    value = read(table['cohort'] + ('/latency.json' if table['cohort'].startswith('first-') else '/report.json'))
    reference = value.get('primary_reference', value.get('references', {}).get('current_22_eligible_Rust'))
    assert table['current_fastest_Rust'] == reference
    for row in table['rows']:
        summary = value['summary'][row['variant']]
        seconds = summary.get('median_post_json_seconds')
        if seconds is None:
            assert row['post_json_ms'] is row['times_current_fastest_Rust'] is None
        else:
            close(row['post_json_ms'], seconds * 1000)
            close(row['times_current_fastest_Rust'], seconds / reference['median_seconds'])
        close(row['whole_process_ms'], summary['median_whole_process_seconds'] * 1000)
for table in report['resource_tables']:
    value = read(table['cohort'] + '/report.json')
    for row in table['rows']:
        summary = value['summary'][row['variant']]
        assert row['post_json_window'] == summary['window_medians'].get('post_json')
        assert row['whole_process'] == summary['whole_process_medians']
last = read('first-scalar-packed-query-gains-v2/latency.json')
confirm = read('independent-compact-packed-scalar-confirmation-v1/report.json')
close(report['selection']['compact349_u32_primary_multiple'], last['summary']['mech-ir349-compact-key-u32-4']['median_post_json_seconds'] / last['primary_reference']['median_seconds'])
close(report['selection']['compact349_u32_confirmation_multiple'], confirm['summary']['mech-ir349-compact-key-u32-4']['median_post_json_seconds'] / confirm['references']['current_22_eligible_Rust']['median_seconds'])
paired = read('first-scalar-packed-query-gains-v2/paired-comparisons.json')['comparisons']
extra = read('independent-compact-packed-scalar-confirmation-v1/additional-paired-comparisons.json')['comparisons']
for candidate, control in [('mech-ir349-compact-key-u32-4', 'mech-ir336-compact-key-u32-4'), ('mech-ir349-compact-maintained-wide-4', 'mech-ir348-compact-maintained-wide-4')]:
    assert next(row for row in paired if row['candidate'] == candidate and row['control'] == control)['percentile95'][1] < 0
    matches = [row for row in extra if row['candidate'] == candidate and row['reference'] == control]
    if not matches:
        matches = [row for row in confirm['candidate_paired_comparisons'] if confirm['candidate'] == candidate and row['reference'] == control]
    assert len(matches) == 1 and matches[0]['bootstrap_95_interval_percent'][1] < 0
parent = next(row for row in paired if row['candidate'] == 'mech-ir349-compact-key-u32-4' and row['control'] == 'mech-ir348-compact-key-u32-4')
assert parent['percentile95'][0] <= 0 <= parent['percentile95'][1]


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
            posts = read('published-base/benchmarks/related-post-gen/fixtures/posts-' + name + '.json')
            indices = read('published-base/benchmarks/related-post-gen/fixtures/expected-' + name + '.json')['related_indices']
            expected = [{'_id': p['_id'], 'tags': p['tags'], 'related': [posts[i] for i in neighbors]} for p, neighbors in zip(posts, indices)]
            assert read(prefix + '/actual-' + name + '.json') == expected


qualification_prefixes = {str(Path(row['record']).parent) for row in report['qualifications']}
edge_prefixes = [name.removesuffix('/edge-fixtures.json') for name in records
                 if name.endswith('/edge-fixtures.json') and any(name.startswith(prefix + '/') for prefix in qualification_prefixes)
                 and not name.startswith('referenced-inputs/')]
assert len(edge_prefixes) == 58
for prefix in edge_prefixes:
    edge_set(prefix)
qualification_full = []
for entry in report['qualifications']:
    value = read(entry['record'])
    prefix = str(Path(entry['record']).parent)
    if entry['kind'] == 'managed':
        assert len(value['results']) * 32 == entry['outputs']
        for row in value['results']:
            assert row['full_records'] == 5000 and row['exact_gold'] is True
            exact(pinned(row['output']))
            qualification_full.append(row['output']['sha256'])
            audit = json.loads(pinned(row['link_audit']))
            assert audit['status'] == 'passed'
            assert audit['every_native_symbol_has_exact_object_relative_address'] is True
            assert audit['all_text_equal_except_independently_verified_relocation_fields'] is True
            receipt = json.loads(pinned(row['build_receipt']))
            assert receipt['exit'] == 0
            assert receipt['executable_sha256'] == row['executable']['sha256'] == audit['executable']['sha256']
    else:
        assert value['status'] == 'passed' and value['correctness_executions'] == entry['outputs']
        assert len(value['records']) * 32 == entry['outputs']
        for row in value['records']:
            assert row.get('full5000_exact', row.get('full_exact')) is True and row['edge_cases'] == 31
            label = row['label']
            form = row.get('form')
            output = prefix + '/' + label + ('/' + form + '.output.json' if form else '/output.json')
            exact(blob(output))
            qualification_full.append(sha(blob(output)))
            audit = json.loads(pinned(row['link_audit']))
            assert audit['status'] == 'passed'
            assert json.loads(pinned(row['build_receipt']))['exit'] == 0
assert len(qualification_full) == 58
assert len(qualification_full) * 32 == sum(row['outputs'] for row in report['qualifications']) == report['qualification_exact_outputs_separate'] == 1856


def counts(value):
    found = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', value['stdout'])
    assert len(found) == 1
    return tuple(map(int, found[0]))


failures = lambda value: sorted(re.findall(r'^    (native_artifact::[^\n]+)$', value['stdout'], re.MULTILINE))
baseline = read('dense-producer-comparison-compiler-v1/broad-native-tests.json')
broad_records = {
    'zero-extend-load-compiler-v1/broad-native-tests.json': 1092,
    'scalar-edge-homes-compiler-v2/broad-native.json': 1093,
    'packed-popcount-compiler-v1/broad-native-tests.json': 1094,
    'packed-selected-homes-compiler-v1/broad-native-tests.json': 1099,
}
for name, passed in broad_records.items():
    value = read(name)
    if 'stdout' not in value:
        value = dict(value, stdout=blob(str(Path(name).parent / Path(value['stdout_path']).name)).decode())
    assert value['exit'] == 101
    assert counts(value) == (passed, 16, 11), (name, counts(value))
    assert '35 filtered out' in value['stdout']
    assert len(failures(value)) == 16 and failures(value) == failures(baseline)
assert read('scalar-edge-homes-compiler-v1/mech_gpu.build.json')['exit'] != 0
assert read('compact-bit-count-compiler-v3/bit-count-tests.json')
assert read('packed-popcount-compiler-v1/initial-focused-wrapper-failure.json')
assert read('packed-popcount-compiler-v1/initial-single-line-name-parser-summary.json')
assert read('packed-selected-homes-static-audit-v1/failure.json')
assert read('packed-selected-homes-static-audit-v2/failure.json')


def stdout_records(value):
    if isinstance(value, dict):
        if isinstance(value.get('stdout'), str):
            yield value
        for child in value.values():
            yield from stdout_records(child)
    elif isinstance(value, list):
        for child in value:
            yield from stdout_records(child)


for label, expected in [('packed-popcount-compiler-v1', 52), ('packed-selected-homes-compiler-v1', 70)]:
    raw = blob(label + '/focused-contracts.json')
    focused = list(stdout_records(json.loads(raw)))
    assert focused and all(row['exit'] == 0 for row in focused)
    # One successful test prints multi-line native JSON between its name and
    # its final "ok". Check every command's all-pass summary and the complete
    # started-name set rather than requiring name and result on one line.
    assert all(counts(row)[1:] == (0, 0) for row in focused)
    starts = [name for row in focused for name in re.findall(r'\btest (native_artifact::\S+) \.\.\.', row['stdout'])]
    assert len(starts) == sum(counts(row)[0] for row in focused)
    names = set(starts)
    distinct = read(label + '/focused-distinct-tests.json')
    assert names == set(distinct['passed']) and len(names) == distinct['count'] == expected
assert read('branch-insert-key-u32-source/receipt.json')['bounded_source_semantics_checks'] == 6336

machine_audit = read('packed-selected-homes-static-audit-v3/audit.json')
assert len(machine_audit['queries']) == 6
for row in machine_audit['queries']:
    assembly = pinned(row['assembly']).decode()
    instructions = re.findall(r'^[ \t]*([0-9a-f]+):[ \t]+(\S+)(?:[ \t]+(.*))?$', assembly, re.MULTILINE)
    assert len(instructions) == row['static_instructions']
    assert len(instructions) * 4 == row['code_bytes']
    addresses = [int(pc, 16) for pc, _, _ in instructions]
    assert addresses == list(range(addresses[0], addresses[0] + row['code_bytes'], 4))
    assert sum(op == 'ccmp' for _, op, _ in instructions) == row['ccmp']
    assert sum(op == 'cnt.8b' for _, op, _ in instructions) == row['cnt8b'] == 2
    assert sum(op == 'addv.8b' for _, op, _ in instructions) == row['addv8b'] == 2
    metadata = json.loads(pinned(row['metadata']))
    header = 1628 if row['variant'] == 'key-u32' else 1587
    function = next(function for function in metadata['compiled_numeric_functions'] if function['header'] == header)
    assert function['code_bytes'] == row['code_bytes'] and function['stack_bytes'] == row['stack_bytes']
    for field in ('selected_home_constant_operands', 'selected_home_merge_parameters'):
        assert function['cleanup'].get(field, 0) == row.get(field, 0)
    assert row['successful_backedge_helpers'] == 0
    if row['ir'] == 349:
        expected = [[1716, 8, 1163], [1716, 8, 1163]] if row['variant'] == 'key-u32' else []
        assert row['producer_element_read_guards'] == expected

# Reconstruct all candidate/test/diagnostic source only from published Git objects.
import difflib


def git_files(commit, names, missing=False):
    raw = subprocess.run(['git', 'cat-file', '--batch'], cwd=ROOT, check=True, capture_output=True,
                         input=''.join(commit + ':' + name + '\n' for name in names).encode()).stdout
    cursor, result = 0, {}
    for name in names:
        end = raw.index(b'\n', cursor)
        header = raw[cursor:end].split()
        cursor = end + 1
        if header[-1] == b'missing':
            assert missing, (commit, name)
            continue
        assert len(header) == 3 and header[1] == b'blob'
        size = int(header[2])
        result[name] = raw[cursor:cursor + size]
        cursor += size
        assert raw[cursor:cursor + 1] == b'\n'
        cursor += 1
    assert cursor == len(raw)
    return result


closure, bases, reconstructed, builds = report['source_closure'], {}, {}, {}
for label, base in closure['bases'].items():
    inventory = read(base['inventory_record'])
    source = git_files(base['commit'], sorted(inventory['files']))
    assert all(sha(source[name]) == digest for name, digest in inventory['files'].items())
    bases[label] = source
for label, entry in closure['compilers'].items():
    inventory = read(entry['inventory_record'])
    assert sha(json.dumps(inventory['files'], sort_keys=True).encode()) == inventory['sha256']
    base, source, parts = bases[entry['base']], {}, []
    for name, digest in inventory['files'].items():
        raw = blob('source-overrides/' + label + '/' + name) if name in entry['overrides'] else base[name]
        assert sha(raw) == digest, (label, name)
        source[name] = raw
    assert len(source) == entry['source_count']
    assert {name: sha(raw) for name, raw in source.items() if base.get(name) != raw} == entry['overrides']
    for name in entry['overrides']:
        parts.append('diff --git a/' + name + ' b/' + name + '\n')
        parts.extend(difflib.unified_diff(base.get(name, b'').decode().splitlines(True), source[name].decode().splitlines(True), fromfile='a/' + name if name in base else '/dev/null', tofile='b/' + name))
    assert ''.join(parts).encode() == blob(entry['patch_record'])
    reconstructed[label] = source
    if label + '/build.json' in records:
        build = read(label + '/build.json')
        assert build['source_sha256'] == inventory['sha256']
        if 'stages' in build:
            assert len(build['stages']) == 12 and all(stage['exit'] == 0 for stage in build['stages'])
        builds[label] = build
vendor = read(closure['vendor_inventory'])
assert len(vendor) == closure['vendor_files'] == 246
base_vendor = git_files(closure['vendor_base_commit'], sorted(vendor), missing=True)
for name, digest in vendor.items():
    raw = blob('vendor-overrides/' + name) if name in closure['vendor_overrides'] else base_vendor[name]
    assert sha(raw) == digest
    for label, source in reconstructed.items():
        if not label.startswith('allocation-phase') and name in source:
            assert source[name] == raw
literal = read(closure['literal_inventory'])['literal_includes']
assert len(literal) == closure['literal_includes'] == 164
for name, raw in git_files(closure['vendor_base_commit'], sorted(literal)).items():
    assert sha(raw) == literal[name]
fresh = read('comparison-chain-compiler-v3/backend-libraries.json')
parent = read('owned-capture-compiler-v1/build.json')
parent_libs = {stage['crate']: stage for stage in parent['stages']}


def externs(command):
    return dict(command[i + 1].split('=', 1) for i, item in enumerate(command[:-1]) if item == '--extern')


for label, build in builds.items():
    if 'stages' not in build:
        continue
    gpu = next(stage for stage in build['stages'] if stage['crate'] == 'mech_gpu')
    for stage in build['stages']:
        if stage['crate'] != 'mech_gpu':
            assert stage['library_sha256'] == parent_libs[stage['crate']]['library_sha256']
    if not label.startswith('allocation-phase'):
        assert build['backend_libraries'] == fresh
        actual = externs(gpu['command'])
        for name in ['cranelift_codegen', 'cranelift_frontend', 'cranelift_jit', 'cranelift_module', 'cranelift_native', 'cranelift_object']:
            assert actual[name] == fresh[name]['path']
for label in ['zero-extend-load-compiler-v1', 'scalar-edge-homes-compiler-v2', 'packed-popcount-compiler-v1', 'packed-selected-homes-compiler-v1']:
    value = read(label + '/tests.build.json')
    assert value['exit'] == 0
    actual = externs(value['command'])
    baseline_externs = externs(next(stage for stage in parent['stages'] if stage['crate'] == 'mech_gpu')['command'])
    for name in ['cranelift_codegen', 'cranelift_frontend', 'cranelift_jit', 'cranelift_module', 'cranelift_native', 'cranelift_object']:
        expected = baseline_externs[name] if label == 'zero-extend-load-compiler-v1' else fresh[name]['path']
        assert actual[name] == expected

optional_alias_checked = False
alias_record = 'IR349-source-publication/source-commit.json'
if alias_record in records:
    alias = read(alias_record)
    assert alias['source_copy_result']['exact_source_hash'] == read('packed-selected-homes-compiler-v1/source-inventory.json')['sha256']
    present = subprocess.run(['git', 'cat-file', '-e', alias['commit'] + '^{commit}'], cwd=ROOT, capture_output=True)
    if present.returncode == 0:
        candidate = reconstructed['packed-selected-homes-compiler-v1']
        assert git_files(alias['commit'], sorted(candidate)) == candidate
        optional_alias_checked = True

census_outputs = 0
for version in (1, 2):
    prefix = 'allocation-phase-census-v' + str(version)
    value = read(prefix + '/report.json')
    assert value['outputs_exact'] == 14
    assert value['timing_claim'] is False
    assert value['phase_sums_all_exact'] is True and value['all_underflow_overflow_zero'] is True and value['all_work_exact'] is True
    assert len(value['runs']) == 14
    for run in value['runs']:
        receipt = json.loads(pinned(run))
        assert receipt['exit'] == 0 and receipt['full5000_exact'] is True and receipt['phase_sums_equal_total'] is True
        exact(pinned(receipt['output']))
        pinned(receipt['stdout'])
        pinned(receipt['stderr'])
        native = receipt['report']
        assert native['iterations'] == 1 and native['native_work_per_iteration'] == 37134118305
        assert native['fresh_inputs_and_ledger_each_iteration'] is True and native['fresh_worker_tasks_each_iteration'] == 4
        census = native['allocation_census']
        assert census['underflow'] == census['overflow'] == 0
        additive = ('alloc_calls', 'alloc_zeroed_calls', 'realloc_calls', 'dealloc_calls',
                    'allocation_requests', 'requested_bytes', 'successful_issued_bytes',
                    'retired_layout_bytes', 'allocation_failures')
        for field in additive:
            assert sum(phase['counts'][field] for phase in census['phases']) == census['total'][field]
        for phase in census['phases']:
            counts_ = phase['counts']
            assert counts_['allocation_requests'] == sum(counts_[field] for field in ('alloc_calls', 'alloc_zeroed_calls', 'realloc_calls'))
            assert counts_['allocation_failures'] == 0
        phase_ids = {4, 5, 6, 7, 8, 16, 17, 18, 19, 20, 21, 22}
        if version == 2:
            phase_ids.update(range(32, 51))
            phase_ids.update(range(80, 98))
        post = {field: sum(phase['counts'][field] for phase in census['phases'] if phase['phase'] in phase_ids) for field in additive}
        assert post == receipt['post_json_counts']
        assert post['allocation_requests'] == 725 and post['requested_bytes'] == 5198427
        assert post['dealloc_calls'] == 5634 and post['retired_layout_bytes'] == 4476443
        census_outputs += 1
assert census_outputs == report['allocation_census_exact_outputs_separate'] == 28

profile_checked = False
if 'packed-scalar-fullpath-profile-v1/analysis.json' in records:
    analysis = read('packed-scalar-fullpath-profile-v1/analysis.json')
    assert analysis['timing_claim'] is False and analysis['iterations'] == 5001
    assert analysis['exact_Object_and_library_closure_match_ordinary'] is True
    assert analysis['work_matches_ordinary'] == report['managed_work_qualifications']['mech-managed-packed-selected-homes-u32']['work'] == 37134118305
    assert analysis['native_nonwait_stack_weight'] == 4228
    assert analysis['generated_self_stack_weight'] == 3039
    assert analysis['query_self_stack_weight'] == 1240
    assert analysis['escrow_path_weight'] == 219 and analysis['allocator_weight'] == 59
    for key in ('complete', 'recipe', 'stack_summary', 'PC_map'):
        pinned(analysis[key])
    complete = read('packed-scalar-fullpath-profile-v1/complete.json')
    assert complete['timing_claim'] is False and complete['all_processes_terminal'] is True
    for run in complete['runs']:
        assert run['exit'] == run['sampler_exit'] == 0
        assert run['diagnostic_only'] is True and run['nonquiet_concurrent_build_test_load'] is True
        measurement = run['measurement']
        assert measurement['iterations'] == 5001 and measurement['fresh_worker_tasks_each_iteration'] == 4
        assert measurement['native_work_per_iteration'] == 37134118305
        assert measurement['native_work_all_iterations'] == 37134118305 * 5001
        assert sum(measurement['observed_peak_worker_histogram'].values()) == 5001
        assert set(measurement['observed_peak_worker_histogram']) <= set('1234')
    for name in records:
        if name.startswith('packed-scalar-fullpath-profile-v1/') and name.endswith('/final-output.json'):
            exact(blob(name))
    profile_checked = True

omitted = set(json.loads((D / 'omitted-binaries.json').read_bytes()))


def verify_pins(value):
    if isinstance(value, dict):
        if isinstance(value.get('path'), str) and isinstance(value.get('sha256'), str):
            if value['sha256'] in digests:
                pinned(value)
            else:
                assert value['path'] in omitted, value['path']
        for child in value.values():
            verify_pins(child)
    elif isinstance(value, list):
        for child in value:
            verify_pins(child)


for cohort in report['cohorts']:
    verify_pins(read(cohort['plan']))
mismatches = read(report['pin_discovery_mismatches_record'])
assert mismatches
for mismatch in mismatches:
    assert mismatch['path'].endswith('/allocation-phase-census-v2/HANDOFF.json')
    assert mismatch['expected_sha256'] in digests
handoff_name = 'allocation-phase-census-v2/handoff.json'
if handoff_name not in records:
    handoff_name = 'allocation-phase-census-v2/HANDOFF.json'
verify_pins(read(handoff_name))
verify_pins(read('allocation-phase-census-v2/wrapper-recovery.json'))
assert report['selection']['new_managed_selected'] is False
assert report['selection']['managed_selected_checkpoint_IR'] == 334
assert report['selection']['fastest_Rust_target_met'] is False
assert sha(blob('packet-report.json')) == sha((D / 'report.json').read_bytes())
print(json.dumps(dict(verified_records=len(records), unique_blobs=len(checked), compressed_bytes=manifest['compressed_bytes'],
                      timed_resource_exact_outputs=4869, qualification_exact_outputs=1856, census_exact_diagnostic_outputs=28,
                      reconstructed_compiler_snapshots=len(reconstructed), vendor_files=246, literal_includes=164,
                      optional_published_IR349_alias_checked=optional_alias_checked, fresh_IR349_profile_checked=profile_checked,
                      broad_passed=broad_records, inherited_failures=16, native_executions_by_verifier=0,
                      source_override_counts={key: len(value['overrides']) for key, value in closure['compilers'].items()},
                      observed_histograms=observations), indent=2))
