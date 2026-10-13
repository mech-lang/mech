"""Crystal full studies using the frozen shared twenty-gate native sampler.

Each sealed benchmark has two unchanged qualified Crystal forms and a freshly
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
GATES = {'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'native_exited_normally', 'no_execution_error', 'no_wrapper_integrity_error', 'no_resource_violation',
    'no_resource_monitor_errors', 'no_compiler_power_monitor_errors', 'no_monitor_finish_error',
    'no_execute_cleanup_error', 'no_remaining_execute_group', 'no_supervisor_cleanup',
    'no_supervisor_remaining_group', 'valid_environment', 'actual_full_workload', 'exact_command_and_group',
    'complete_output_reference_match', 'complete_output_bytes_and_hash', 'pins_unchanged'}
SOURCE_FAILURE_SAFETY = ('sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'no_wrapper_integrity_error', 'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
    'no_monitor_finish_error', 'no_execute_cleanup_error', 'no_remaining_execute_group',
    'no_supervisor_cleanup', 'no_supervisor_remaining_group', 'valid_environment',
    'actual_full_workload', 'exact_command_and_group', 'pins_unchanged')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def configure(benchmark, revision):
    assert benchmark in BENCHMARKS
    assert revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in revision)
    assert sha(GUARD) == GUARD_SHA, 'Frozen sampler changed'
    spec = importlib.util.spec_from_file_location('crystal_full_frozen_sampler', GUARD)
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
    candidates = [row for row in rows if row['language'] == 'crystal']
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
    """Only actual source failures with independently proven complete ownership."""
    checks = attempt.get('qualification_checks', {})
    return (type(checks) is dict and set(checks) == GATES
        and all(type(value) is bool for value in checks.values())
        and guard.benchmark_failure(attempt)
        and all(checks.get(key) is True for key in SOURCE_FAILURE_SAFETY))


def run(benchmark, revision, digest):
    print('WAIT common mutex: Crystal full-study initial review', benchmark, flush=True)
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
            canonical_LOC_status='pending separate immutable count overlay')
        guard.save(guard.REPORT, report)
        try:
            for ordered in manifest['orders']:
                for identifier in ordered['profiles']:
                    if rows[identifier].get('full_failure'):
                        continue
                    print('WAIT common mutex: Crystal full', benchmark, ordered['round'], identifier, flush=True)
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
                            if isolated_profile_failure(guard, attempt):
                                row['full_failure'] = {key: attempt.get(key) for key in (
                                    'attempt', 'error', 'error_type', 'seconds', 'exit_status',
                                    'stdout_bytes', 'stdout_sha256', 'stderr', 'resource_violation')}
                                row['measurement_status'] = 'failed-full-source-unscored'
                            else:
                                row['measurement_status'] = 'stopped-unqualified'
                        guard.save(guard.REPORT, report)
                        assert attempt['qualified'] or row.get('full_failure'), 'Actual sample integrity/environment failed; preserved without retry'
            assert len(report['attempts']) <= 9
            assert all(len(row['samples_seconds']) == 3 or row.get('full_failure') for row in rows.values())
            control = next(row for row in rows.values() if row['language'] == 'mech')
            for row in rows.values():
                row['time_x_mech'] = (row['median_seconds'] / control['median_seconds']) if not row.get('full_failure') and not control.get('full_failure') else None
                row['matched_mech_control'] = control['id']
                row['matched_mech_source_sha256'] = control['source_sha256']
            with LOCK.open('a+') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX)
                guard.verify(manifest['pins'])
                report.update(status='complete-with-source-failures' if any(row.get('full_failure') for row in rows.values()) else 'complete',
                              finished_at=guard.now(), pins_unchanged_at_end=True)
                guard.save(guard.REPORT, report)
        except BaseException as error:
            report.update(status='stopped-unqualified', finished_at=guard.now(), active=None,
                          error=str(error) or type(error).__name__, error_type=type(error).__name__)
            guard.save(guard.REPORT, report)
            raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('Crystal full supervisor received signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    parser.add_argument('--benchmark', choices=BENCHMARKS, required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--manifest-sha256', required=True)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--run', action='store_true')
    mode.add_argument('--detach', action='store_true')
    args = parser.parse_args()
    if args.detach:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            guard = configure(args.benchmark, args.revision)
            review(guard, args.benchmark, args.manifest_sha256)
            with (guard.OUT / 'launch-owner.lock').open('a+') as owner:
                fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
                assert not guard.REPORT.exists() and not (guard.OUT / 'launch.json').exists()
                command = [sys.executable, '-u', str(Path(__file__).resolve()), '--run',
                    '--benchmark', args.benchmark, '--revision', args.revision,
                    '--manifest-sha256', args.manifest_sha256]
                with (guard.OUT / 'supervisor.log').open('xb') as log:
                    process = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                        stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                receipt = dict(pid=process.pid, identity=guard.identity(process.pid),
                    command=command, launched_at=guard.now(), manifest_sha256=args.manifest_sha256,
                    shared_timing_lock=str(LOCK), waiting_child_not_joined_under_mutex=True)
                with (guard.OUT / 'launch.json').open('x') as stream:
                    json.dump(receipt, stream, indent=2)
                    stream.write('\n')
                print(json.dumps(receipt, indent=2), flush=True)
        return
    run(args.benchmark, args.revision, args.manifest_sha256)


if __name__ == '__main__':
    main()
