"""Independent record checks for the requested native language cohorts.

This pure adapter is preparation for publication, not chart admission. The
caller must hold the common timing lock, verify immutable manifest/source/work
pins, supply the independent canonical oracle and reconcile the complete suite.
Canonical LOC is a separate exact-source overlay; null producer LOC is retained.
"""
from copy import deepcopy
import json
import math
import statistics


LANGUAGES = frozenset({'nim', 'crystal', 'pony', 'c3'})
POWER = dict(source='AC Power', low_power_mode=0)
CHECKS = frozenset({
    'sampler_exited_normally', 'authoritative_native_group_received',
    'native_inherited_common_mutex', 'native_exited_normally',
    'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
    'no_monitor_finish_error', 'no_execute_cleanup_error',
    'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload',
    'exact_command_and_group', 'complete_output_reference_match',
    'complete_output_bytes_and_hash', 'pins_unchanged',
})
EMPTY = ('remaining_group_members', 'monitor_errors', 'resource_monitor_errors',
         'supervisor_remaining_members', 'competing_compilers', 'power_changes')
FATAL = ('error', 'error_type', 'resource_violation', 'cleanup_error',
         'supervisor_cleanup', 'sampler_failure', 'supervisor_failure',
         'supervisor_pin_failure', 'supervisor_registration_error',
         'monitor_finish_error', 'source_work_error', 'supervisor_error',
         'sampler_error', 'registration_error', 'supervisor_cleanup_error',
         'wrapper_supervisor_failure', 'wrapper_error', 'wrapper_error_type')


def require(value, message):
    if not value:
        raise ValueError(message)


def exact_integer(value, expected):
    return type(value) is int and value == expected


def validate_benchmark(manifest, report, manifest_digest, *, language,
                       canonical_n, reference, fingerprint, load_evidence,
                       attempt_offset=0):
    """Reconcile nine actual records, rejecting partial or failed profiles.

Pony's suite ledger uses global attempt numbers. Its caller supplies a checked
per-benchmark view plus its real ordinal offset, after checking the entire global
ledger. Evidence paths refer to the sampler's separate supervisor artifact.
No completion flag or precomputed median substitutes for actual records.
"""
    require(language in LANGUAGES, 'Unexpected publication language')
    benchmark = manifest['benchmark']
    require(report['status'] == 'complete' and report.get('active') is None,
            'Partial or failed studies cannot supply publication records')
    require(report['manifest_sha256'] == manifest_digest and
            report['pins_unchanged_at_end'] is True, 'Different or changed manifest')
    require(not any(report.get(k) for k in FATAL), 'Report retains a fatal failure')
    require(exact_integer(manifest['samples'], 3) and
            exact_integer(manifest['size'], canonical_n) and
            manifest['canonical_full_workload'] is True, 'Canonical full work required')
    require(manifest['required_power_profile'] == POWER and
            manifest['shared_timing_lock'] == '/tmp/mech-clbg-timing.lock',
            'Measurement environment changed')
    require(exact_integer(manifest['maximum_compute_workers'], 8), 'Worker policy changed')
    require(manifest.get('never_chart_eligible') is not True and
            manifest.get('report_only') is not True, 'Report-only source cannot be published')
    require(exact_integer(attempt_offset, attempt_offset) and attempt_offset >= 0,
            'Actual ledger offset required')
    rows = {r['id']: r for r in manifest['profiles']}
    require(len(rows) == len(manifest['profiles']) == 3, 'Exact three-profile inventory required')
    candidates = [r for r in rows.values() if r['language'] == language]
    controls = [r for r in rows.values() if r['language'] == 'mech']
    require(len(candidates) == 2 and len(controls) == 1 and
            {r['variant'] for r in candidates} == {'compact', 'optimized'},
            'Both complete candidate forms and one fresh control required')
    require(all(r['benchmark'] == benchmark for r in rows.values()), 'Mixed benchmark inventory')
    require(set(report['profiles']) == set(rows), 'Summary profile inventory changed')
    require(report['orders'] == manifest['orders'] and len(manifest['orders']) == 3,
            'Recorded order changed')
    queue = []
    for number, order in enumerate(manifest['orders'], 1):
        require(exact_integer(order['round'], number) and len(order['profiles']) == 3 and
                set(order['profiles']) == set(rows), 'Missing or duplicate profile in a round')
        queue.extend((number, identity) for identity in order['profiles'])
    require(len(report['attempts']) == len(queue) == 9, 'Nine actual full executions required')
    accepted = {identity: [] for identity in rows}
    for number, (attempt, (round_index, identity)) in enumerate(zip(report['attempts'], queue), 1):
        row = rows[identity]
        require(exact_integer(attempt['attempt'], attempt_offset + number) and
                exact_integer(attempt['round'], round_index), 'Actual ledger ordering changed')
        for field in ('id', 'key', 'benchmark', 'language', 'source_sha256', 'cwd'):
            require(attempt[field] == row[field], 'Actual identity changed: ' + field)
        command = [part.replace('{n}', str(canonical_n)) for part in row['command']]
        require(attempt['command'] == command and
                attempt['runtime_environment'] == row.get('runtime_environment', {}),
                'Actual command/environment changed')
        require(json.loads(attempt['execution_fingerprint']) == fingerprint(row),
                'Execution fingerprint changed')
        require(attempt['qualified'] is True and attempt['valid_environment'] is True and
                attempt['pins_unchanged_at_end'] is True, 'Unqualified native execution')
        require(exact_integer(attempt['exit_status'], 0) and
                exact_integer(attempt['sampler_wait_status'], 0), 'Native/sampler did not exit normally')
        require(attempt['full_workload'] is True and
                exact_integer(attempt['workload_n'], canonical_n) and
                not attempt.get('preparation_only'), 'Bounded/probe result cannot be scored')
        require(set(attempt['qualification_checks']) == CHECKS and
                all(value is True for value in attempt['qualification_checks'].values()),
                'Complete literal-true ownership/output/environment gates required')
        require(all(attempt[field] == [] for field in EMPTY) and
                not any(attempt.get(field) for field in FATAL), 'Retained process/monitor/source failure')
        require(attempt['power_at_start'] == attempt['power_at_end'] == POWER,
                'Power profile changed during measurement')
        require(attempt['maximum_rss_mib'] == manifest['maximum_rss_mib'] and
                attempt['timeout_seconds'] == manifest['timeout_seconds'], 'Resource policy changed')
        peak = attempt['peak_process_group_rss_kib']
        require(type(peak) is int and 0 <= peak <= manifest['maximum_rss_mib'] * 1024,
                'Actual process group exceeded memory bound')
        native = attempt['supervised_native_group']
        require(native['type'] == 'native-group-started' and type(native['pid']) is int and
                type(native['pgid']) is int and
                native['pid'] > 0 and native['pid'] == native['pgid'] and
                native['command'] == command and native['inherited_common_mutex'] is True,
                'Exact authoritative native process ownership required')
        validation = attempt['validation']
        require(validation['reference_match'] is True and
                validation['bytes'] == attempt['stdout_bytes'] and
                validation['sha256'] == attempt['stdout_sha256'], 'Complete output evidence changed')
        if benchmark == 'nbody':
            require(len(validation['energies']) == len(reference['energies']) == 2 and
                    validation['tolerance'] == reference['tolerance'], 'Energy oracle shape/tolerance changed')
            require(all(math.isfinite(v) and abs(v-e) <= reference['tolerance']
                        for v, e in zip(validation['energies'], reference['energies'])),
                    'Full energy oracle failed')
        else:
            require(validation['bytes'] == reference['bytes'] and
                    validation['sha256'] == reference['sha256'] and
                    manifest['full_output_reference'] == reference, 'Whole canonical output differs')
        elapsed = attempt['seconds']
        require(type(elapsed) in (int, float) and math.isfinite(elapsed) and elapsed > 0,
                'Invalid actual elapsed time')
        saved = load_evidence(attempt['supervisor_result_path'])
        raw = load_evidence(attempt['result_path'])
        require(all(key in saved and saved[key] == value for key, value in raw.items()),
                'Supervisor changed the original sampler execution evidence')
        # Nim/Crystal add these policy markers after the sampler saves its
        # supervisor record. They may never overwrite any native evidence.
        normalized = deepcopy(attempt)
        for field, expected in (('full_performance_run', True), ('chart_admission', False)):
            if field in normalized and field not in saved:
                require(normalized[field] is expected, 'Wrapper policy marker changed')
                normalized.pop(field)
        require(normalized == saved, 'Report changed the saved supervisor evidence')
        accepted[identity].append(attempt)
    result = {}
    for identity, row in rows.items():
        samples = [a['seconds'] for a in accepted[identity]]
        summary = report['profiles'][identity]
        require(len(samples) == 3 and summary['samples_seconds'] == samples and
                summary.get('loc') == row.get('loc'), 'Source/sample summary changed')
        require(summary['median_seconds'] == statistics.median(samples) and
                summary['sample_stdev_seconds'] == statistics.stdev(samples), 'Statistics do not reconcile')
        require(not summary.get('full_failure') and
                summary['measurement_status'] == 'measured', 'Failed profile cannot contribute a median')
        result[identity] = dict(deepcopy(row), samples_seconds=samples,
            median_seconds=statistics.median(samples), sample_stdev_seconds=statistics.stdev(samples),
            sample_validation=[deepcopy(a['validation']) for a in accepted[identity]],
            sample_execution_metadata=deepcopy(accepted[identity]),
            publication_status='records-reconciled-awaiting-qualified-canonical-LOC',
            eligible_for_chart=False)
    control = result[controls[0]['id']]
    for row in result.values():
        row['time_ratio_to_matched_mech'] = row['median_seconds'] / control['median_seconds']
    return result
