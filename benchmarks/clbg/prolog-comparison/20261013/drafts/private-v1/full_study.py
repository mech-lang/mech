"""Prolog full studies using the frozen shared twenty-gate native sampler.

Each sealed benchmark has two unchanged qualified Prolog forms and a freshly
measured Mech AOT control, three rotated rounds, and complete output validation.
Existing reports are never resumed or replaced. Publication is a separate step.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import statistics
import subprocess
import sys

HERE = Path(__file__).resolve().parents[2]
CLBG = HERE.parents[1]
SUITE = CLBG / 'r-apl-uiua-comparison/20261011'
GUARD = SUITE / 'uiua-fann-compact-v5/controller_rev2/remaining_owner.py'
GUARD_SHA = '55581d56609803243b3be264e6a2e37229be06789c22788bbf8cdb6ff6828544'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BENCHMARKS = ('regexredux', 'revcomp', 'pidigits', 'spectralnorm', 'mandelbrot',
              'fasta', 'knucleotide', 'binarytrees', 'nbody', 'fannkuchredux')
GATES = {
    'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged',
}



def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def configure(benchmark, revision):
    assert benchmark in BENCHMARKS
    assert revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in revision)
    assert sha(GUARD) == GUARD_SHA, 'Frozen sampler changed'
    spec = importlib.util.spec_from_file_location('prolog_full_frozen_sampler', GUARD)
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    guard.OUT = HERE / 'studies' / revision / benchmark
    guard.REPORT = guard.OUT / 'report.json'
    return guard


def review(guard, benchmark, digest):
    path = guard.OUT / 'manifest.json'
    assert sha(path) == digest, 'Reviewed manifest changed'
    manifest = json.loads(path.read_text())
    assert manifest['status'] == 'prepared-awaiting-root-review'
    assert manifest['benchmark'] == benchmark and manifest['samples'] == 3
    assert manifest['canonical_full_workload'] is True
    assert manifest['chart_admission'] is False and manifest['no_automatic_retry'] is True
    harness = guard.load_harness()
    assert manifest['size'] == harness.WORK[benchmark][1]
    assert manifest['shared_timing_lock'] == str(LOCK)
    rows = manifest['profiles']
    assert len(rows) == 3 and len({row['id'] for row in rows}) == 3
    assert {row['benchmark'] for row in rows} == {benchmark}
    candidates = [row for row in rows if row['language'] == 'prolog']
    controls = [row for row in rows if row['language'] == 'mech']
    assert len(candidates) == 2 and len(controls) == 1
    assert {row['variant'] for row in candidates} == {'compact', 'optimized'}
    assert all(row['required_work_review']['status'] == 'passed' for row in candidates)
    assert all(row['maximum_workers'] <= 8 for row in rows)
    ids = {row['id'] for row in rows}
    assert [order['round'] for order in manifest['orders']] == [1, 2, 3]
    for order in manifest['orders']:
        assert len(order['profiles']) == 3 and set(order['profiles']) == ids
    assert manifest['pins'][str(Path(__file__).resolve())] == sha(__file__)
    assert manifest['pins'][str(GUARD)] == GUARD_SHA
    guard.verify(manifest['pins'])
    return manifest, harness


def isolated_profile_failure(guard, attempt):
    """A genuine runtime/output/RSS failure with independently proven clean ownership."""
    safe = ('sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
        'no_wrapper_integrity_error', 'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
        'no_monitor_finish_error', 'no_execute_cleanup_error', 'no_remaining_execute_group',
        'no_supervisor_cleanup', 'no_supervisor_remaining_group', 'valid_environment',
        'actual_full_workload', 'exact_command_and_group', 'pins_unchanged')
    checks = attempt.get('qualification_checks', {})
    return (isinstance(checks, dict) and set(checks) == GATES and
            all(type(value) is bool for value in checks.values()) and
            guard.benchmark_failure(attempt) and all(checks[key] is True for key in safe))


def run(benchmark, revision, digest):
    print('WAIT common mutex: Prolog full-study initial review', benchmark, flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        guard = configure(benchmark, revision)
        manifest, harness = review(guard, benchmark, digest)
    harness.install_interrupt_handlers()
    with (guard.OUT / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not guard.REPORT.exists(), 'No duplicate launch or implicit resume'
        rows = {row['id']: copy.deepcopy(row) for row in manifest['profiles']}
        for row in rows.values():
            row.update(samples_seconds=[], sample_validation=[], attempts=[], measurement_status='pending')
        report = dict(status='running', started_at=guard.now(), benchmark=benchmark,
            manifest=str(guard.OUT / 'manifest.json'), manifest_sha256=digest,
            sample_count=3, profiles=rows, orders=manifest['orders'], attempts=[], active=None,
            supervisor=guard.identity(os.getpid()), shared_timing_lock=str(LOCK),
            full_workload_n=manifest['size'], chart_admission=False, no_automatic_retry=True,
            canonical_LOC_status='pending separate immutable count overlay', isolated_failed_profiles=[],
            skipped_failed_profile_slots=[])
        guard.save(guard.REPORT, report)
        try:
            for ordered in manifest['orders']:
                for identifier in ordered['profiles']:
                    if rows[identifier]['measurement_status'] == 'failed-full-workload':
                        report['skipped_failed_profile_slots'].append(dict(round=ordered['round'], id=identifier,
                            reason='Preserved actual earlier failure; no automatic retry'))
                        guard.save(guard.REPORT, report)
                        continue
                    print('WAIT common mutex: Prolog full', benchmark, ordered['round'], identifier, flush=True)
                    with LOCK.open('a+') as lock:
                        fcntl.flock(lock, fcntl.LOCK_EX)
                        guard.verify(manifest['pins'])
                        harness.compact.wait_for_environment(harness.REQUIRED)
                        print('FULL', benchmark, ordered['round'], identifier, manifest['size'], flush=True)
                        try:
                            attempt = guard.one_attempt(harness, rows[identifier], manifest, report,
                                                        ordered['round'], lock.fileno())
                        except BaseException as error:
                            index = len(report['attempts']) + 1
                            raw = guard.OUT / 'attempts' / f'{index:03d}-{identifier}.json'
                            supervised = raw.with_suffix('.supervisor.json')
                            source = supervised if supervised.exists() else raw
                            attempt = json.loads(source.read_text()) if source.exists() else {}
                            attempt.update(attempt=index, round=ordered['round'], id=identifier,
                                qualified=False, full_performance_run=True, chart_admission=False,
                                wrapper_supervisor_failure=True, wrapper_error=str(error) or type(error).__name__,
                                wrapper_error_type=type(error).__name__)
                            guard.save(raw.with_suffix('.wrapper.json'), attempt)
                            report['attempts'].append(attempt)
                            report['active'] = None
                            rows[identifier]['attempts'].append(index)
                            rows[identifier]['measurement_status'] = 'stopped-unqualified'
                            guard.save(guard.REPORT, report)
                            raise
                        attempt.update(full_performance_run=True, chart_admission=False)
                        report['attempts'].append(attempt)
                        report['active'] = None
                        row = rows[identifier]
                        row['attempts'].append(attempt['attempt'])
                        if attempt['qualified']:
                            row['samples_seconds'].append(attempt['seconds'])
                            row['sample_validation'].append(attempt['validation'])
                            row['median_seconds'] = statistics.median(row['samples_seconds'])
                            row['measurement_status'] = 'measured' if len(row['samples_seconds']) == 3 else 'partial'
                            if len(row['samples_seconds']) > 1:
                                row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
                        else:
                            row['measurement_status'] = 'stopped-unqualified'
                        guard.save(guard.REPORT, report)
                        if not attempt['qualified']:
                            assert isolated_profile_failure(guard, attempt), 'Integrity/environment/ownership failure; preserved for inspection'
                            row['measurement_status'] = 'failed-full-workload'
                            row['full_failure'] = {key: attempt.get(key) for key in
                                ('error', 'error_type', 'seconds', 'exit_status', 'attempt', 'finished_at',
                                 'stdout_bytes', 'stdout_sha256', 'stderr', 'resource_violation')}
                            row.pop('median_seconds', None)
                            row.pop('sample_stdev_seconds', None)
                            report['isolated_failed_profiles'].append(identifier)
                            guard.save(guard.REPORT, report)
                            print('RETAINED runtime failure; continue independent profiles', identifier, attempt.get('error'), flush=True)
            assert len(report['attempts']) + len(report['skipped_failed_profile_slots']) == 9
            assert all(len(row['samples_seconds']) == 3 or row['measurement_status'] == 'failed-full-workload'
                       for row in rows.values())
            control = next(row for row in rows.values() if row['language'] == 'mech')
            for row in rows.values():
                row['time_x_mech'] = (row['median_seconds'] / control['median_seconds']
                    if row['measurement_status'] == control['measurement_status'] == 'measured' else None)
                row['matched_mech_control'] = control['id']
                row['matched_mech_source_sha256'] = control['source_sha256']
            with LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                guard.verify(manifest['pins'])
                report.update(status='complete' if not report['isolated_failed_profiles'] else 'complete-with-unqualified-profiles',
                    finished_at=guard.now(), pins_unchanged_at_end=True,
                    all_profiles_fully_measured=not report['isolated_failed_profiles'])
                guard.save(guard.REPORT, report)
        except BaseException as error:
            report.update(status='stopped-unqualified', finished_at=guard.now(), active=None,
                          error=str(error) or type(error).__name__, error_type=type(error).__name__)
            guard.save(guard.REPORT, report)
            raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Prolog full supervisor received signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    parser.add_argument('--benchmark', choices=BENCHMARKS, required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--manifest-sha256', required=True)
    parser.add_argument('--run', action='store_true', required=True)
    args = parser.parse_args()
    run(args.benchmark, args.revision, args.manifest_sha256)


if __name__ == '__main__':
    main()
