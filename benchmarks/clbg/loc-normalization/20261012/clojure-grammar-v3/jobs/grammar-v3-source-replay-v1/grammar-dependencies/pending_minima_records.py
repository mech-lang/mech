"""Strict adapter for the four separately reviewed pending-minimum studies.

The caller holds the common timing mutex. This module neither executes a
program nor changes a manifest, report, producer LOC, source, or timing.
Canonical sizes/output constants below come from the frozen CLBG validators;
the new producer intentionally has no ``full_output_reference`` field.
"""
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
import re
import statistics


POWER = {'source': 'AC Power', 'low_power_mode': 0}
LOCK = '/tmp/mech-clbg-timing.lock'
COHORT = 'loc-normalization/20261012/pending-minima'
STUDIES = {
    'mandelbrot': dict(identity='mandelbrot-lua-2', language='lua', size=16000,
        source_sha256='16eacf7585991619923c8add0aa9ae0c1327aa2b5f31764208fcf27a5d52f249',
        manifest_sha256='97ea9ced57da4f707222dc898b7f9a9cc03bb505d5f89ee865754bad850f7861',
        source_suffix='compact/upstream/mandelbrot-lua-2.lua', flags=[],
        reference=(32000015, '609262469ee6a0262ccd03932e557f745c9e7b997ad17835a02a0232a64807be')),
    'spectralnorm': dict(identity='spectralnorm-node-7', language='javascript', size=5500,
        source_sha256='2dea06681168ffa46990b461ef7468f38afeb3fd955d8b3078d8918b1c5afd4a',
        manifest_sha256='679a1874d1948b800dd9f5292099b15238a2079fb47eaad32d8a08b3ab501104',
        source_suffix='compact/upstream/spectralnorm-node-7.js', flags=[],
        reference_text=b'1.274224153\n'),
    'binarytrees': dict(identity='binarytrees-ruby-2', language='ruby', size=21,
        source_sha256='d75c541ff893ccfb5b5fa399010bc60c7f14f51d1077a0f983502ffe60f05f6c',
        manifest_sha256='dd0a83144f83dea6de4b9533a1627543a16675161bbc67a341a155bdda31c86d',
        source_suffix='compact/upstream/binarytrees-ruby-2.rb', flags=['--yjit', '-W0'],
        reference=(455, '341de11a51feab3d8122b4b5d6a68b038a2d14434aa9bc2372f39300bf5f48e1')),
    'fannkuchredux': dict(identity='fannkuchredux-php-8', language='php', size=12,
        source_sha256='7931a305151ebb9d4b54ec3729931622806334296557c2153637c2ea9315b1c4',
        manifest_sha256='3e67337e67df5dceb76ce647a709a37839aa5a010c2fe8c0a368338efb94026a',
        source_suffix='compact/upstream/fannkuchredux-php-8.php',
        flags=['-n', '-d', 'memory_limit=-1', '-d', 'opcache.enable_cli=1',
               '-d', 'opcache.jit_buffer_size=64M', '-d', 'opcache.jit=tracing',
               '-d', 'short_open_tag=1'],
        reference_text=b'3968050\nPfannkuchen(12) = 65\n'),
}

# These are the exact fatal keys emitted by the frozen shared supervisor. An
# invented qualification-check map cannot substitute for these actual fields.
FATAL = ('error', 'resource_violation', 'cleanup_error', 'supervisor_cleanup',
         'supervisor_registration_error', 'sampler_failure', 'monitor_errors',
         'resource_monitor_errors', 'monitor_finish_error')
EXTRA_FAILURES = ('supervisor_failure', 'supervisor_error', 'supervisor_pin_failure',
                  'supervisor_cleanup_error', 'preparation_supervisor_failure',
                  'source_work_failure', 'full_failure')
EMPTY = ('remaining_group_members', 'monitor_errors', 'resource_monitor_errors',
         'supervisor_remaining_members')
ENVIRONMENT = ('competing_compilers', 'power_changes')


def _require(condition, message):
    if not condition:
        raise ValueError(message)


def reference(benchmark):
    """Independent whole-output oracle, never supplied by the candidate."""
    study = STUDIES[benchmark]
    if 'reference_text' in study:
        data = study['reference_text']
        count, digest = len(data), hashlib.sha256(data).hexdigest()
    else:
        count, digest = study['reference']
    return {'bytes': count, 'sha256': digest, 'reference_match': True}


def _support_pins(value):
    """Reconstruct the frozen fingerprint's public support identities."""
    result = {}
    if isinstance(value, dict):
        if value.get('path') and isinstance(value.get('sha256'), str):
            result[value['path']] = value['sha256']
        for name, child in value.items():
            if name in ('pins', 'libraries', 'pin_files', 'support_pins') and isinstance(child, dict):
                for filename, digest in child.items():
                    if isinstance(digest, str) and len(digest) == 64:
                        result[str(Path(filename).absolute())] = digest
                        result[str(Path(filename).resolve())] = digest
            if isinstance(child, (dict, list)):
                result.update(_support_pins(child))
    elif isinstance(value, list):
        for child in value:
            result.update(_support_pins(child))
    return result


def _fingerprint(row):
    return dict(source_sha256=row['source_sha256'], binary_sha256=row.get('binary_sha256'),
        command=[part.replace(row['source'], '{source}') for part in row['command']],
        cwd=row['cwd'], environment=row.get('runtime_environment', {}), support=_support_pins(row))


def _read_raw(path):
    return json.loads(Path(path).read_text())


def _verify_pin(pins, path, expected, digest):
    _require(isinstance(path, str) and Path(path).is_absolute(), 'Pinned identity requires an absolute path')
    _require(isinstance(expected, str) and re.fullmatch(r'[0-9a-f]{64}', expected), 'Malformed source/runtime pin')
    _require(pins.get(path) == expected and digest(path) == expected, 'Frozen source/runtime identity changed: ' + path)


def _row_checks(row, study, benchmark, pins, digest):
    _require(row['benchmark'] == benchmark and row['loc'] is None, 'Benchmark or retained null producer LOC differs')
    _require(type(row['maximum_workers']) is int and 1 <= row['maximum_workers'] <= 8, 'Worker cap differs')
    _require(isinstance(row['command'], list) and all(isinstance(part, str) for part in row['command']), 'Malformed command')
    _require(Path(row['cwd']).is_absolute(), 'Missing explicit working directory')
    _verify_pin(pins, row['source'], row['source_sha256'], digest)
    if row['id'] == study['identity']:
        _require(row['language'] == study['language'] and row['key'] == benchmark + ':' + row['id'], 'Listed candidate identity differs')
        _require(row['source_sha256'] == study['source_sha256'], 'New candidate differs from the reviewed listed source')
        original = row['original_source']
        _require(original.endswith('/' + study['source_suffix']), 'Original listed source path differs')
        _verify_pin(pins, original, study['source_sha256'], digest)
        runtime = row['runtime']
        _verify_pin(pins, runtime['path'], runtime['sha256'], digest)
        _require(row['command'] == [runtime['path']] + study['flags'] + [row['source'], '{n}'], 'Reviewed source/runtime flags differ')
        _require(row['maximum_workers'] == 1, 'Listed sequential candidate worker cap differs')
        _require(row['algorithm']['strict'] is True and row['algorithm']['native_benchmark_kernel'] is False, 'Required-work policy differs')
        _require(row['required_work_review']['status'] == 'passed', 'Required work was not reviewed')
        _require(row['required_work_review']['source_sha256'] == row['source_sha256'], 'Work review describes a different source')
        _require(row['small_validation']['reference_match'] is True, 'Canonical small output did not pass')
        _require(row['adversarial_validation'] and all(a['qualified'] is True and a['full_workload'] is False for a in row['adversarial_validation']), 'Bounded output qualification is missing')
    else:
        _require(row['id'] == 'mech-aot-control' and row['language'] == 'mech', 'Unexpected control profile')
        _require(row['key'] == benchmark + ':loc-minimum-study:mech-control', 'Exact fresh control key differs')
        _verify_pin(pins, row['binary'], row['binary_sha256'], digest)
        _require(row['command'][0] == row['binary'] and '{n}' in row['command'], 'Control executable/full size differs')


def validate_study(manifest, report, *, manifest_sha256, benchmark, digest):
    """Return exact rows and archived environment exclusions; fail closed.

    This adapter is specific to the four root-reviewed six-process manifests.
    A partial report or any workload/output/process failure contributes no row.
    """
    _require(benchmark in STUDIES, 'Unknown pending-minimum benchmark')
    study = STUDIES[benchmark]
    _require(manifest_sha256 == study['manifest_sha256'], 'Manifest is not the exact independently reviewed study')
    _require(manifest['status'] == 'prepared-awaiting-root-review' and manifest['benchmark'] == benchmark, 'Unexpected producer manifest')
    _require(manifest['samples'] == 3 and type(manifest['samples']) is int, 'Full sample count differs')
    _require(manifest['size'] == study['size'] and type(manifest['size']) is int, 'Canonical full workload differs')
    _require(manifest['required_power_profile'] == POWER and manifest['shared_timing_lock'] == LOCK, 'Power/mutex policy differs')
    _require(manifest['maximum_compute_workers'] == 8 and manifest['maximum_rss_mib'] == 8192 and manifest['timeout_seconds'] == 14400, 'Reviewed resource limits differ')
    _require(manifest['environment_retry_limit'] == 3 and manifest['chart_admitted'] is False, 'Retry or pending publication policy differs')
    _require('full_output_reference' not in manifest, 'Do not synthesize a different producer schema')
    pins = manifest['pins']
    _require(isinstance(pins, dict) and bool(pins), 'Frozen study pins are missing')
    for path, expected in pins.items():
        _verify_pin(pins, path, expected, digest)
    expected_ids = [study['identity'], 'mech-aot-control']
    rows = {row['id']: row for row in manifest['profiles']}
    _require(len(manifest['profiles']) == len(rows) == 2 and set(rows) == set(expected_ids), 'Profile inventory is not the exact two reviewed programs')
    expected_orders = [dict(round=i + 1, profiles=expected_ids[i % 2:] + expected_ids[:i % 2]) for i in range(3)]
    _require(manifest['orders'] == expected_orders and report['orders'] == expected_orders, 'Actual full process order differs')
    for row in rows.values():
        _row_checks(row, study, benchmark, pins, digest)
    _require(report['status'] == 'complete' and report['active'] is None, 'Incomplete report cannot contribute timing rows')
    if 'canonical_full_workload' in report:
        _require(report['canonical_full_workload'] is True, 'Report is an explicitly bounded workload')
    if 'never_chart_eligible' in report:
        _require(report['never_chart_eligible'] is False, 'Report explicitly excludes chart eligibility')
    _require(report['manifest_sha256'] == manifest_sha256 and report['pins_unchanged_at_end'] is True, 'Completed report pin identity differs')
    _require(not any(report.get(field) for field in ('error', 'error_type', 'full_failure')), 'Completed report retained a fatal failure')
    manifest_path = report['manifest']
    _require(manifest_path.endswith('/' + COHORT + '/' + benchmark + '/full-study/manifest.json'), 'Report refers to another study manifest')
    _require(digest(manifest_path) == manifest_sha256, 'Recorded manifest file changed')
    _require(set(report['profiles']) == set(rows), 'Summary profile inventory differs')
    expected_output = reference(benchmark)
    queue = [(ordered['round'], identity) for ordered in expected_orders for identity in ordered['profiles']]
    qualified = {identity: [] for identity in rows}
    discarded, position, retries = [], 0, 0
    for number, attempt in enumerate(report['attempts'], 1):
        _require(position < len(queue), 'Extra full record after all required samples')
        round_index, identity = queue[position]
        row = rows[identity]
        _require(type(attempt['attempt']) is int and attempt['attempt'] == number and attempt['round'] == round_index, 'Actual attempt/round ordering differs')
        for field in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd'):
            _require(attempt[field] == row[field], 'Actual profile identity differs: ' + field)
        command = [part.replace('{n}', str(study['size'])) for part in row['command']]
        _require(attempt['command'] == command and attempt['runtime_environment'] == row.get('runtime_environment', {}), 'Actual command/environment differs')
        _require(json.loads(attempt['execution_fingerprint']) == _fingerprint(row), 'Actual source/binary/runtime fingerprint differs')
        _require(attempt['full_workload'] is True and type(attempt['workload_n']) is int and attempt['workload_n'] == study['size'], 'Bounded/scaled work cannot be a full record')
        if 'canonical_full_workload' in attempt:
            _require(attempt['canonical_full_workload'] is True, 'Explicit probe marker cannot be admitted')
        if 'never_chart_eligible' in attempt:
            _require(attempt['never_chart_eligible'] is False, 'Explicit unscored marker cannot be admitted')
        _require(not any(attempt.get(field) for field in ('preparation_only', 'primitive_or_ownership_contract')), 'Preparation-only execution cannot be admitted')
        _require(attempt['sampler_wait_status'] == attempt['exit_status'] == 0 and type(attempt['sampler_wait_status']) is int and type(attempt['exit_status']) is int, 'Sampler/native process did not exit normally')
        _require(attempt['pins_unchanged_at_end'] is True and attempt['fatal_qualification_fields'] == [], 'Raw integrity/qualification failed')
        _require(not any(attempt.get(field) for field in FATAL + EXTRA_FAILURES), 'Execution/ownership/work failure cannot be discarded as environment-only')
        _require(all(attempt[field] == [] for field in EMPTY), 'Process cleanup or resource/compiler monitor failed')
        _require(attempt['maximum_rss_mib'] == 8192 and attempt['timeout_seconds'] == 14400, 'Actual resource policy differs')
        peak = attempt['peak_process_group_rss_kib']
        _require(type(peak) is int and 0 <= peak <= 8192 * 1024, 'Measured group RSS exceeded its actual cap')
        _require(attempt['validation'] == expected_output and attempt['stdout_bytes'] == expected_output['bytes'] and attempt['stdout_sha256'] == expected_output['sha256'], 'Complete output differs from the independent full oracle')
        native = attempt['supervised_native_group']
        _require(native['type'] == 'native-group-started' and type(native['pid']) is int and native['pid'] > 0 and native['pid'] == native['pgid'], 'Authoritative native ownership is missing')
        _require(native['command'] == command and native['inherited_common_mutex'] is True, 'Native child did not retain the exact command/mutex')
        seconds = attempt['seconds']
        _require(type(seconds) in (int, float) and math.isfinite(seconds) and seconds > 0, 'Invalid actual elapsed time')
        raw_path = attempt['result_path']
        _require(raw_path.endswith('/attempts/' + f'{number:03d}-{identity}.json'), 'Actual raw record path differs')
        raw = _read_raw(raw_path)
        _require(dict(raw, environment_attempt=attempt['environment_attempt']) == attempt, 'Summary changed the original saved execution record')
        if attempt['qualified'] is not True:
            _require(attempt['qualified'] is False and attempt['valid_environment'] is False and attempt['retryable_environment_only'] is True, 'Unqualified nonenvironment record cannot be retried/admitted')
            _require(any(attempt[field] for field in ENVIRONMENT) or attempt['power_at_start'] != POWER or attempt['power_at_end'] != POWER, 'Environmental exclusion has no actual evidence')
            retries += 1
            _require(retries < 3 and attempt['environment_attempt'] == retries, 'Environment retry count differs/exhausted')
            discarded.append(deepcopy(attempt))
            continue
        _require(attempt['valid_environment'] is True and attempt['retryable_environment_only'] is False, 'Qualified record has inconsistent environment gates')
        _require(all(attempt[field] == [] for field in ENVIRONMENT) and attempt['power_at_start'] == attempt['power_at_end'] == POWER, 'Qualified sample overlaps competing load/power change')
        _require(attempt['environment_attempt'] == retries + 1, 'Successful retry numbering differs')
        qualified[identity].append(attempt)
        retries, position = 0, position + 1
    _require(position == len(queue) and retries == 0, 'Three actual full samples per program are missing')
    result = {}
    for identity, row in rows.items():
        summary = report['profiles'][identity]
        samples = [attempt['seconds'] for attempt in qualified[identity]]
        median, stdev = statistics.median(samples), statistics.stdev(samples)
        _require(len(samples) == 3 and summary['loc'] is None and summary['role'] == row['role'], 'Actual sample count/producer metadata differs')
        _require(summary['samples_seconds'] == samples and summary['median_seconds'] == median and summary['sample_stdev_seconds'] == stdev, 'Summary statistic does not reconcile to actual samples')
        result[identity] = dict(deepcopy(row), samples_seconds=samples, median_seconds=median,
            sample_stdev_seconds=stdev, sample_validation=[deepcopy(a['validation']) for a in qualified[identity]],
            sample_execution_metadata=deepcopy(qualified[identity]))
    return result, discarded
