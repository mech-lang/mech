"""Separate actual N12 fannkuch study; prepare and root-review before launch.

Uses the unchanged frozen execute harness. Supervisor, sampler and native
process retain the common mutex through cleanup; no scaled scores are admitted.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import random
import selectors
import signal
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
N = HERE.parent
ROOT = N.parents[3]
OUT = HERE / 'full-study'
LOCK = Path('/tmp/mech-clbg-timing.lock')
MANIFEST = OUT / 'manifest.json'
REPORT = OUT / 'report.json'
QUALIFICATION = HERE / 'manifest.json'
QUALIFICATION_SHA = '41462f55b1c1f6c800d08cca9ba9ba46cb5e3c6efd7f0ff3d21cfde514b70b7c'


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(value, indent=2) + '\n')
    temporary.replace(path)


def load_harness():
    previous = sys.modules.pop('prepare', None)
    sys.path.insert(0, str(N))
    try:
        spec = importlib.util.spec_from_file_location('fann_parallel_frozen_harness', N / 'run.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    finally:
        sys.path.pop(0)
        if previous is not None:
            sys.modules['prepare'] = previous
        else:
            sys.modules.pop('prepare', None)


def identity(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'lstart=', '-o', 'command='], capture_output=True, text=True)
    return dict(pid=pid, process_listing=result.stdout.strip()) if result.returncode == 0 and result.stdout.strip() else None


def verify(pins):
    for filename, digest in pins.items():
        assert sha(filename) == digest, ('Pinned file changed', filename)


def prepare():
    global OUT, REPORT
    with LOCK.open('a+') as lock:
        print('Waiting for shared mutex for bounded Fann runner qualification and sealing.', flush=True)
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired common mutex.', flush=True)
        assert not MANIFEST.exists(), 'Existing sealed study must not be replaced'
        assert sha(QUALIFICATION) == QUALIFICATION_SHA, 'Sealed candidate preparation changed'
        qualification = json.loads(QUALIFICATION.read_text())
        assert qualification['status'] == 'prepared-validated' and qualification['full_performance_runs'] == 0
        assert qualification['validation_evidence']['counts'] == dict(outputs_passed=128, baseline_checks_passed=8,
            traces_passed=10, worker_gates_passed=22, range_checks_passed=36, matrix_checks_passed=18)
        verify(qualification['pins'])
        audit_path = N / 'source-audit-uiua.json'
        audit = json.loads(audit_path.read_text())
        assert audit['status'] == 'reviewed'
        pins = dict(qualification['pins'])
        for filename in [N / 'run.py', N / 'prepare.py', ROOT / 'benchmarks/clbg/compact/run.py',
                         ROOT / 'benchmarks/clbg/nbody/run.py', ROOT / 'benchmarks/clbg/array-comparison/all-20261009/run.py']:
            expected = audit['pins'][str(filename)]
            assert sha(filename) == expected, ('Frozen harness pin differs', filename)
            pins[str(filename)] = expected
        pins.update({str(QUALIFICATION): QUALIFICATION_SHA, str(audit_path): sha(audit_path),
                     str(Path(__file__).resolve()): sha(__file__), str(HERE / 'STUDY.md'): sha(HERE / 'STUDY.md'),
                     str(HERE / 'MIGRATION-PLAN.md'): sha(HERE / 'MIGRATION-PLAN.md'),
                     str(Path(sys.executable).resolve()): sha(Path(sys.executable).resolve())})
        profiles = []
        for original in qualification['programs']:
            row = {key: copy.deepcopy(original[key]) for key in
                   ('benchmark', 'language', 'source', 'source_sha256', 'command', 'runtime_environment', 'loc', 'maximum_workers')}
            identity_name = 'uiua-' + original['source_profile']
            row.update(id=identity_name, key='fannkuchredux:parallel-study:' + identity_name, cwd=str(ROOT),
                       role='sealed composed compact20/fast21 ordinary Uiua global-rank worker candidate',
                       timing_boundary='Fresh-process elapsed including Uiua startup, source compilation, explicit workers, all 12! permutations and actual prefix flips, ordered checksum/maximum write and shutdown.')
            profiles.append(row)
        control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == 'fannkuchredux'))
        assert control['loc'] == 59 and control['command'] == [control['binary'], '{n}']
        assert control['source_sha256'] == 'b8a2ce6509db36487e42bb3ded0e054b4cde743e05cb05c94f8f3c6e45780ebe'
        assert control['binary_sha256'] == '3d3599f1f174915251d6323592ea81bf3dc7ca57f0f07b8f310cd6825393e004'
        control.update(id='mech59-aot-control', key='fannkuchredux:parallel-study:mech59-aot-control',
                       role='unchanged Mech fast 59 LOC AOT control, freshly measured in these rounds', maximum_workers=8,
                       runtime_environment=dict(control['runtime_environment'], RAYON_NUM_THREADS='8'),
                       timing_boundary='Fresh-process elapsed including startup, all 12! permutations and actual prefix flips, checksum/maximum write and shutdown.')
        profiles.append(control)
        pins[control['source']] = control['source_sha256']
        pins[control['binary']] = control['binary_sha256']
        verify(pins)
        harness = load_harness()
        harness.install_interrupt_handlers()
        harness.compact.wait_for_environment(harness.REQUIRED)
        # Exercise each actual sampler/PGID/lock path at canonical small N7.
        # Probe artifacts remain outside the eventual full report/attempt paths.
        old_out, old_report = OUT, REPORT
        OUT = HERE / 'runner-qualification'
        REPORT = OUT / 'report.json'
        assert not REPORT.exists(), 'Existing bounded runner evidence must not be overwritten'
        small = b'228\nPfannkuchen(7) = 16\n'
        probe_manifest = dict(size=7, canonical_full_workload=False, maximum_compute_workers=8, pins=pins,
                              timeout_seconds=120, maximum_rss_mib=2048,
                              full_output_reference=dict(bytes=len(small), sha256=hashlib.sha256(small).hexdigest()))
        probe_report = dict(status='running', canonical_full_workload=False, never_chart_eligible=True,
                            candidate_manifest_sha256=QUALIFICATION_SHA, attempts=[], active=None)
        save(REPORT, probe_report)
        try:
            for row in profiles:
                attempt = one_attempt(harness, row, probe_manifest, probe_report, 0, lock.fileno())
                attempt.update(canonical_full_workload=False, never_chart_eligible=True)
                probe_report['attempts'].append(attempt)
                probe_report['active'] = None
                save(REPORT, probe_report)
                assert attempt['qualified'], ('Bounded runner qualification failed', row['id'], attempt)
            probe_report.update(status='passed', finished_at=now(), checks=len(profiles), pins_unchanged_at_end=True)
            save(REPORT, probe_report)
            probe_report_path = REPORT
            probe_paths = [REPORT] + [Path(row['result_path']) for row in probe_report['attempts']]
        except BaseException as error:
            probe_report.update(status='failed', finished_at=now(), error=str(error), error_type=type(error).__name__)
            save(REPORT, probe_report)
            raise
        finally:
            OUT, REPORT = old_out, old_report
        pins.update({str(path): sha(path) for path in probe_paths})
        identifiers = [row['id'] for row in profiles]
        rng = random.Random(20261012)
        orders = []
        for index in range(3):
            ordered = identifiers.copy()
            rng.shuffle(ordered)
            orders.append(dict(round=index + 1, profiles=ordered))
        full = b'3968050\nPfannkuchen(12) = 65\n'
        manifest = dict(status='prepared-awaiting-root-review', created_at=now(), samples=3, size=12,
                        canonical_full_workload=True, full_permutations=479001600, maximum_compute_workers=8,
                        profiles=profiles, orders=orders, pins=pins, shared_timing_lock=str(LOCK),
                        required_power_profile=harness.REQUIRED, maximum_rss_mib=8192, timeout_seconds=14400,
                        full_output_reference=dict(bytes=len(full), sha256=hashlib.sha256(full).hexdigest(),
                                                   checksum=3968050, maximum=65),
                        candidate_manifest=str(QUALIFICATION), candidate_manifest_sha256=QUALIFICATION_SHA,
                        runner_qualification=str(probe_report_path), runner_qualification_sha256=sha(probe_report_path),
                        timing_boundary='Three actual fresh N12 processes per profile. All 479001600 global ranks are materialized, every actual prefix reversal finishes, checksum uses global parity. Whole-process timing includes startup, source compilation where applicable, explicit worker startup/join, complete output and shutdown. Hashing, qualification and pin checks are excluded.',
                        ownership='Supervisor, forked sampler and native child inherit and retain the common mutex through cleanup and native exit. Authoritative native PGID registration closes the ownership gap; late notifications are drained before cleanup and lock release.',
                        failure_policy='Fatal output, pin, timeout, resource, monitor, sampler, native status or cleanup failure stops the study and retains all evidence. No automatic retries or scaled performance scores.',
                        raw_output_policy='Frozen execute validates the complete temporary regular-file checksum/maximum output; full byte count, SHA, status and diagnostics are retained.',
                        chart_policy='Separate experimental evidence only. No frozen suite source/audit/report writes, migration, chart admission or publication.')
        verify(pins)
        save(MANIFEST, manifest)
        print(json.dumps(dict(manifest=str(MANIFEST), sha256=sha(MANIFEST), runner=str(Path(__file__).resolve()),
                              runner_sha256=sha(__file__), status=manifest['status'], profiles=len(profiles), full_processes=9), indent=2), flush=True)


def sampler(harness, row, write_fd, result_path, manifest, lock_fd):
    os.setsid()
    harness.install_interrupt_handlers()
    original = harness.subprocess.Popen
    result = None
    command = [part.replace('{n}', str(manifest['size'])) for part in row['command']]

    def recorded_popen(*args, **kwargs):
        argv = args[0] if args else kwargs.get('args')
        native_command = list(argv) == command and kwargs.get('start_new_session')
        if native_command:
            kwargs['pass_fds'] = tuple(sorted(set(kwargs.get('pass_fds', ())) | {lock_fd}))
        process = original(*args, **kwargs)
        try:
            if native_command:
                event = dict(type='native-group-started', pid=process.pid, pgid=process.pid, command=command, inherited_common_mutex=True)
                os.write(write_fd, (json.dumps(event) + '\n').encode())
        except BrokenPipeError:
            # The sampler/native process both retain the inherited common lock
            # even if the supervisor died. Complete execute cleanup/persistence.
            pass
        except BaseException:
            if native_command:
                # execute has not received the Popen result during registration.
                harness.kill_group(process.pid)
                process.wait()
            raise
        return process

    harness.subprocess.Popen = recorded_popen
    try:
        verify(manifest['pins'])
        result = harness.execute(row, manifest['canonical_full_workload'], manifest['timeout_seconds'], manifest['maximum_rss_mib'])
        verify(manifest['pins'])
        result['pins_unchanged_at_end'] = True
        save(result_path, result)
        os._exit(0)
    except BaseException as error:
        evidence = dict(result or {})
        if evidence.get('error'):
            evidence['execution_failure_before_sampler_error'] = {key: evidence.get(key) for key in ('error', 'error_type')}
        evidence.update(error=str(error), error_type=type(error).__name__, sampler_failure=True)
        save(result_path, evidence)
        os._exit(1)


def one_attempt(harness, row, manifest, report, round_index, lock_fd):
    index = len(report['attempts']) + 1
    result_path = OUT / 'attempts' / f'{index:03d}-{row["id"]}.json'
    read_fd, write_fd = os.pipe()
    os.set_blocking(read_fd, False)
    poll = selectors.DefaultSelector()
    poll.register(read_fd, selectors.EVENT_READ)
    buffer, native, reaped, child, result = b'', None, False, None, None

    def read_events(persist):
        nonlocal buffer, native
        while True:
            try:
                data = os.read(read_fd, 4096)
            except BlockingIOError:
                break
            if not data:
                try:
                    poll.unregister(read_fd)
                except KeyError:
                    pass
                break
            buffer += data
            while b'\n' in buffer:
                line, buffer = buffer.split(b'\n', 1)
                event = json.loads(line)
                if event['type'] == 'native-group-started':
                    native = event
                    if persist:
                        report['active']['native'] = dict(event, identity=identity(event['pid']))
                        save(REPORT, report)

    try:
        child = os.fork()
        if child == 0:
            try:
                os.close(read_fd)
                sampler(harness, row, write_fd, result_path, manifest, lock_fd)
            except BaseException as error:
                save(result_path, dict(error=str(error), error_type=type(error).__name__, sampler_failure=True))
            finally:
                os._exit(1)
        os.close(write_fd)
        write_fd = None
        report['active'] = dict(attempt=index, round=round_index, id=row['id'], sampler_pid=child,
                                sampler_identity=identity(child), started_at=now(), result_path=str(result_path), sampler_inherited_common_mutex=True)
        save(REPORT, report)
        while True:
            if poll.select(0.1):
                read_events(True)
            pid, status = os.waitpid(child, os.WNOHANG)
            if pid:
                reaped = True
                read_events(True)
                break
        result = json.loads(result_path.read_text()) if result_path.exists() else dict(error='Sampler exited without result', sampler_failure=True)
        result.update(round=round_index, attempt=index, sampler_wait_status=status, result_path=str(result_path))
        if native:
            result['supervised_native_group'] = native
            members = harness.group_members(native['pgid'])
            result['supervisor_remaining_members'] = members
            if members:
                harness.kill_group(native['pgid'])
                result.update(error='Native group remained after sampler exit; cleaned before releasing lock', supervisor_cleanup=True)
        verify(manifest['pins'])
        result['qualification_checks'] = dict(sampler_exited_normally=status == 0,
                                              authoritative_native_group_received=native is not None,
                                              native_inherited_common_mutex=native is not None and native.get('inherited_common_mutex') is True,
                                              native_exited_normally=result.get('exit_status') == 0,
                                              no_execution_error=not result.get('error'),
                                              no_resource_violation=not result.get('resource_violation'),
                                              no_resource_monitor_errors=not result.get('resource_monitor_errors'),
                                              no_compiler_power_monitor_errors=not result.get('monitor_errors'),
                                              no_monitor_finish_error=not result.get('monitor_finish_error'),
                                              no_execute_cleanup_error=not result.get('cleanup_error'),
                                              no_remaining_execute_group=not result.get('remaining_group_members'),
                                              no_supervisor_cleanup=not result.get('supervisor_cleanup'),
                                              no_supervisor_remaining_group=not result.get('supervisor_remaining_members'),
                                              valid_environment=result.get('valid_environment') is True,
                                              actual_full_workload=result.get('full_workload') is manifest['canonical_full_workload'] and result.get('workload_n') == manifest['size'],
                                              exact_command_and_group=native is not None and native['command'] == result.get('command') == [part.replace('{n}', str(manifest['size'])) for part in row['command']],
                                              complete_output_reference_match=result.get('validation', {}).get('reference_match') is True,
                                              complete_output_bytes_and_hash=(result.get('stdout_bytes'), result.get('stdout_sha256')) == (manifest['full_output_reference']['bytes'], manifest['full_output_reference']['sha256']),
                                              pins_unchanged=result.get('pins_unchanged_at_end') is True)
        result['qualified'] = all(result['qualification_checks'].values())
        return result
    finally:
        if child and not reaped:
            try:
                os.kill(child, signal.SIGTERM)
            except ProcessLookupError:
                pass
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                try:
                    pid, _ = os.waitpid(child, os.WNOHANG)
                except ChildProcessError:
                    reaped = True
                    break
                if pid:
                    reaped = True
                    break
                time.sleep(0.05)
            if not reaped:
                try:
                    os.kill(child, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                try:
                    os.waitpid(child, 0)
                except ChildProcessError:
                    pass
        # Always drain and clean ownership, including abnormal sampler exits
        # already reaped before reaching this finally block.
        read_events(False)
        if native:
            harness.kill_group(native['pgid'])
        poll.close()
        os.close(read_fd)
        if write_fd is not None:
            os.close(write_fd)


def run(manifest_digest):
    with LOCK.open('a+') as initial_lock:
        fcntl.flock(initial_lock, fcntl.LOCK_EX)
        assert sha(MANIFEST) == manifest_digest, 'Reviewed study manifest changed'
        manifest = json.loads(MANIFEST.read_text())
        verify(manifest['pins'])
        harness = load_harness()
    harness.install_interrupt_handlers()
    rows = {row['id']: row for row in manifest['profiles']}
    with (OUT / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not REPORT.exists(), 'Existing study report needs explicit review; no automatic resumption'
        report = dict(status='running', created_at=now(), manifest=str(MANIFEST), manifest_sha256=manifest_digest,
                      supervisor=identity(os.getpid()), orders=manifest['orders'], attempts=[], active=None,
                      profiles={key: dict(loc=row['loc'], role=row['role'], timing_boundary=row['timing_boundary'], samples_seconds=[], sample_details=[]) for key, row in rows.items()})
        save(REPORT, report)
        try:
            for ordered in manifest['orders']:
                for identifier in ordered['profiles']:
                    with LOCK.open('a+') as lock:
                        fcntl.flock(lock, fcntl.LOCK_EX)
                        harness.compact.wait_for_environment(harness.REQUIRED)
                        verify(manifest['pins'])
                        print('FULL', ordered['round'], identifier, manifest['size'], manifest['maximum_compute_workers'], flush=True)
                        attempt = one_attempt(harness, rows[identifier], manifest, report, ordered['round'], lock.fileno())
                        report['attempts'].append(attempt)
                        report['active'] = None
                        if attempt['qualified']:
                            report['profiles'][identifier]['samples_seconds'].append(attempt['seconds'])
                            report['profiles'][identifier]['sample_details'].append({key: attempt[key] for key in
                                ('seconds', 'validation', 'supervised_native_group') if key in attempt})
                        save(REPORT, report)
                        if not attempt['qualified']:
                            raise RuntimeError('Full sample failed qualification; evidence retained and study stopped')
            for profile in report['profiles'].values():
                assert len(profile['samples_seconds']) == len(profile['sample_details']) == 3
                profile['median_seconds'] = statistics.median(profile['samples_seconds'])
                profile['sample_stdev_seconds'] = statistics.stdev(profile['samples_seconds'])
            report.update(status='complete', finished_at=now(), pins_unchanged_at_end=True)
        except BaseException as error:
            report.update(status='stopped', error=str(error), error_type=type(error).__name__, finished_at=now())
            raise
        finally:
            save(REPORT, report)


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--prepare', action='store_true')
    mode.add_argument('--run', action='store_true')
    mode.add_argument('--detach', action='store_true')
    parser.add_argument('--manifest-sha256')
    args = parser.parse_args()
    if args.prepare:
        prepare()
    elif args.run:
        assert args.manifest_sha256, 'Reviewed manifest hash required'
        run(args.manifest_sha256)
    else:
        with LOCK.open('a+') as launch_lock:
            fcntl.flock(launch_lock, fcntl.LOCK_EX)
            assert args.manifest_sha256 and sha(MANIFEST) == args.manifest_sha256
            manifest = json.loads(MANIFEST.read_text())
            verify(manifest['pins'])
            assert not REPORT.exists() and not (OUT / 'launch.json').exists(), 'Existing supervisor/report must not be relaunched'
            process = None
            try:
                with (OUT / 'supervisor.log').open('ab') as log:
                    process = subprocess.Popen([sys.executable, '-u', str(Path(__file__).resolve()), '--run', '--manifest-sha256', args.manifest_sha256],
                                               stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                launch = dict(pid=process.pid, identity=identity(process.pid), manifest_sha256=args.manifest_sha256, launched_at=now())
                save(OUT / 'launch.json', launch)
            except BaseException:
                if process is not None:
                    process.terminate()
                    process.wait()
                raise
        print(json.dumps(launch, indent=2), flush=True)


if __name__ == '__main__':
    main()
