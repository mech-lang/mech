"""Source-only JVM full-study controller; exactly reviewed manifest required.

No detach helper. A caller may launch direct --run controllers, whose subject
processes serialize behind the same common mutex. No native attempt is queued
until bounded and actual source-work prerequisites have qualified and sealed.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import statistics

HERE = Path(__file__).resolve().parents[2]
CLBG = HERE.parents[1]
GUARD = CLBG / 'r-apl-uiua-comparison/20261011/uiua-fann-compact-v5/controller_rev2/remaining_owner.py'
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
SOURCE_FAILURE_SAFETY = {'sampler_exited_normally', 'authoritative_native_group_received', 'native_inherited_common_mutex',
    'no_wrapper_integrity_error', 'no_resource_monitor_errors', 'no_compiler_power_monitor_errors',
    'no_monitor_finish_error', 'no_execute_cleanup_error', 'no_remaining_execute_group',
    'no_supervisor_cleanup', 'no_supervisor_remaining_group', 'valid_environment',
    'actual_full_workload', 'exact_command_and_group', 'pins_unchanged'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def isolated_profile_failure(guard, attempt):
    checks = attempt.get('qualification_checks')
    return (type(checks) is dict and set(checks) == GATES and
        all(type(value) is bool for value in checks.values()) and guard.benchmark_failure(attempt) and
        all(checks[name] is True for name in SOURCE_FAILURE_SAFETY))


def exact_success(attempt):
    checks = attempt.get('qualification_checks')
    assert type(checks) is dict and set(checks) == GATES
    assert all(type(value) is bool and value is True for value in checks.values())
    assert attempt['qualified'] is True and attempt['full_workload'] is True
    assert type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int
    assert attempt['exit_status'] == attempt['sampler_wait_status'] == 0
    assert attempt['validation']['reference_match'] is True and attempt['pins_unchanged_at_end'] is True


def review(guard, benchmark, revision, digest):
    path = HERE / 'studies' / revision / benchmark / 'manifest.json'
    assert sha(path) == digest
    manifest = json.loads(path.read_text())
    assert manifest['status'] == 'prepared-awaiting-root-review' and manifest['benchmark'] == benchmark
    assert manifest['samples'] == 3 and manifest['canonical_full_workload'] is True
    assert manifest['shared_timing_lock'] == str(LOCK)
    assert manifest['chart_admission'] is False and manifest['no_automatic_retry'] is True
    assert manifest['maximum_compute_workers'] == 8
    guard.verify(manifest['pins'])
    harness = guard.load_harness()
    assert manifest['size'] == harness.WORK[benchmark][1]
    rows = manifest['profiles']
    assert len(rows) == len({row['id'] for row in rows}) == 3
    candidates = [row for row in rows if row['language'] == 'clojure']
    controls = [row for row in rows if row['language'] == 'mech']
    assert len(candidates) == 2 and len(controls) == 1
    assert {row['variant'] for row in candidates} == {'compact', 'optimized'}
    assert all(row['benchmark'] == benchmark and row['maximum_workers'] <= 8 for row in rows)
    for row in candidates:
        assert row['required_work_review']['status'] == 'passed'
        assert row['compiled_input_inventory'] and row['compiled_artifacts'] and row['source_bundle']
        assert row['runtime_classpath'] and row['runtime_classpath'] == row['command'][row['command'].index('-cp') + 1].split(os.pathsep)
        assert all(Path(value).suffix == '.jar' for value in row['runtime_classpath'][1:])
        assert Path(row['runtime_classpath'][0]).name == row['id']
    identifiers = [row['id'] for row in rows]
    assert manifest['orders'] == [dict(round=i+1, profiles=identifiers[i:]+identifiers[:i]) for i in range(3)]
    assert manifest['pins'][str(Path(__file__).resolve())] == sha(__file__)
    return manifest, harness


def preserve_wrapper(guard, report, row, ordered, error):
    index = len(report['attempts']) + 1
    raw = guard.OUT / 'attempts' / f'{index:03d}-{row["id"]}.json'
    supervised = raw.with_suffix('.supervisor.json')
    source = supervised if supervised.exists() else raw
    attempt = json.loads(source.read_text()) if source.exists() else {}
    attempt.update(attempt=index, round=ordered['round'], id=row['id'], qualified=False,
        full_performance_run=True, chart_admission=False, wrapper_supervisor_failure=True,
        wrapper_error=str(error) or type(error).__name__, wrapper_error_type=type(error).__name__)
    if source.exists():
        attempt.update(actual_execution_result_path=str(source), actual_execution_result_sha256=sha(source))
    wrapper = raw.with_suffix('.wrapper.json')
    assert not wrapper.exists(), 'Raw/wrapper evidence cannot be overwritten'
    wrapper.parent.mkdir(parents=True, exist_ok=True)
    with wrapper.open('x') as stream:
        json.dump(attempt, stream, indent=2)
        stream.write('\n')
    report['attempts'].append(attempt)
    report['active'] = None
    row['attempts'].append(index)
    row['measurement_status'] = 'stopped-unqualified'
    guard.save(guard.REPORT, report)


def configure(output):
    assert sha(GUARD) == GUARD_SHA
    guard = module('clojure_frozen_full_sampler', GUARD)
    guard.OUT, guard.REPORT = output, output / 'report.json'
    return guard


def run(benchmark, revision, digest):
    output = HERE / 'studies' / revision / benchmark
    with (output / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not (output / 'report.json').exists(), 'No duplicate launch or implicit resume'
        print('WAIT common mutex: Clojure full initial review', benchmark, flush=True)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            guard = configure(output)
            manifest, harness = review(guard, benchmark, revision, digest)
        harness.install_interrupt_handlers()
        rows = {row['id']:dict(copy.deepcopy(row), samples_seconds=[], sample_validation=[],
            attempts=[], measurement_status='pending') for row in manifest['profiles']}
        report = dict(status='running', started_at=guard.now(), benchmark=benchmark,
            manifest=str(output / 'manifest.json'), manifest_sha256=digest, sample_count=3,
            profiles=rows, orders=manifest['orders'], attempts=[], skipped_planned_slots=[], active=None,
            supervisor=guard.identity(os.getpid()), shared_timing_lock=str(LOCK),
            full_workload_n=manifest['size'], chart_admission=False, no_automatic_retry=True,
            canonical_LOC_status='pending separate immutable count overlay')
        guard.save(guard.REPORT, report)
        try:
            for ordered in manifest['orders']:
                for identifier in ordered['profiles']:
                    row = rows[identifier]
                    if row.get('full_failure'):
                        report['skipped_planned_slots'].append(dict(round=ordered['round'], id=identifier,
                            source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'],
                            reason='prior actual verified source failure retained unscored',
                            failed_attempt=row['full_failure']['attempt']))
                        guard.save(guard.REPORT, report)
                        continue
                    print('WAIT common mutex: Clojure full', benchmark, ordered['round'], identifier, flush=True)
                    with LOCK.open('a+') as lock:
                        fcntl.flock(lock, fcntl.LOCK_EX)
                        guard.verify(manifest['pins'])
                        harness.compact.wait_for_environment(harness.REQUIRED)
                        print('FULL', benchmark, ordered['round'], identifier, manifest['size'], flush=True)
                        try:
                            attempt = guard.one_attempt(harness, row, manifest, report, ordered['round'], lock.fileno())
                        except BaseException as error:
                            preserve_wrapper(guard, report, row, ordered, error)
                            raise
                        attempt.update(full_performance_run=True, chart_admission=False)
                        report['attempts'].append(attempt)
                        report['active'] = None
                        row['attempts'].append(attempt['attempt'])
                        if attempt['qualified'] is True:
                            exact_success(attempt)
                            row['samples_seconds'].append(attempt['seconds'])
                            row['sample_validation'].append(attempt['validation'])
                            row['median_seconds'] = statistics.median(row['samples_seconds'])
                            row['measurement_status'] = 'measured' if len(row['samples_seconds']) == 3 else 'partial'
                            if len(row['samples_seconds']) > 1:
                                row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
                        elif isolated_profile_failure(guard, attempt):
                            row['full_failure'] = copy.deepcopy(attempt)
                            row['measurement_status'] = 'failed-full-source-unscored'
                        else:
                            row['measurement_status'] = 'stopped-unqualified'
                        guard.save(guard.REPORT, report)
                        assert attempt['qualified'] is True or row.get('full_failure'), 'Integrity/environment/ownership failure stops'
            assert len(report['attempts']) + len(report['skipped_planned_slots']) == 9
            slots = [(attempt['round'], attempt['id']) for attempt in report['attempts']] + [
                (slot['round'], slot['id']) for slot in report['skipped_planned_slots']]
            assert len(slots) == len(set(slots)) == 9
            assert all(len(row['samples_seconds']) == 3 or row.get('full_failure') for row in rows.values())
            control = next(row for row in rows.values() if row['language'] == 'mech')
            for row in rows.values():
                row['time_x_mech'] = row['median_seconds'] / control['median_seconds'] if not row.get('full_failure') and not control.get('full_failure') else None
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
        raise KeyboardInterrupt('Clojure full controller interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    parser.add_argument('--run', action='store_true', required=True)
    parser.add_argument('--benchmark', choices=BENCHMARKS, required=True)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--manifest-sha256', required=True)
    args = parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', args.revision)
    run(args.benchmark, args.revision, args.manifest_sha256)


if __name__ == '__main__':
    main()
