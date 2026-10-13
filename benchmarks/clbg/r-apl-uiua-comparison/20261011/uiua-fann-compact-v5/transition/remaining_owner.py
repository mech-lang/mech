"""Draft durable continuation for unchanged remaining R/APL report selections.

No execution is authorized until the post-boundary manifest receives review.
Uses the frozen executor/validators and inherited-mutex native/sampler guard.
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
CANDIDATE = HERE.parent
N = CANDIDATE.parent
ROOT = N.parents[3]
OUT = HERE / 'remaining-owner'
LOCK = Path('/tmp/mech-clbg-timing.lock')
MANIFEST = OUT / 'manifest.json'
REPORT = None
TRANSITION = HERE / 'transition.json'
STAGES = [('r', 'fannkuchredux'), ('apl', 'mandelbrot'), ('apl', 'fasta'), ('apl', 'knucleotide'),
          ('apl', 'binarytrees'), ('apl', 'nbody'), ('apl', 'fannkuchredux')]


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
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert not MANIFEST.exists(), 'Do not replace an existing continuation manifest'
        transition = json.loads(TRANSITION.read_text())
        assert transition['status'] == 'maintenance-complete-new-owner-awaiting-review'
        mapping_path = Path(transition['mapping'])
        mapping = json.loads(mapping_path.read_text())
        assert sha(mapping_path) == transition['mapping_sha256']
        plan_path = HERE / 'plan.json'
        plan = json.loads(plan_path.read_text())
        assert sha(plan_path) == transition['plan_sha256']
        verify(plan['pins'])
        assert identity(plan['stage_owner_identity']['pid']) is None
        assert identity(plan['coordinator_identity']['pid']) is None
        harness = load_harness()
        pins = dict(plan['pins'])
        pins.update({str(Path(__file__).resolve()): sha(__file__), str(HERE / 'REMAINING.md'): sha(HERE / 'REMAINING.md'),
                     str(TRANSITION): sha(TRANSITION), str(mapping_path): sha(mapping_path), str(plan_path): sha(plan_path)})
        reports, audits, snapshots = {}, {}, {}
        OUT.mkdir(parents=True, exist_ok=True)
        for language in ('r', 'apl'):
            audit_path = Path(mapping['continuing_audits'][language]['path'])
            assert sha(audit_path) == mapping['continuing_audits'][language]['sha256']
            audit = json.loads(audit_path.read_text())
            report_path = Path(mapping['continuing_reports'][language])
            report = json.loads(report_path.read_text())
            assert report['active'] is None if 'active' in report else True
            assert report['source_audit_sha256'] == sha(audit_path) and report['sample_count'] == 3
            pins.update(audit['pins'])
            pins[str(audit_path)] = sha(audit_path)
            snapshot = OUT / (language + '-before.json')
            with snapshot.open('xb') as stream:
                stream.write(report_path.read_bytes())
            snapshots[language] = dict(path=str(snapshot), sha256=sha(snapshot))
            pins[str(snapshot)] = sha(snapshot)
            reports[language] = str(report_path)
            audits[language] = dict(path=str(audit_path), sha256=sha(audit_path))
        verify(pins)
        manifest = dict(status='prepared-awaiting-independent-review', created_at=now(), pins=pins,
                        stages=[dict(language=language, benchmark=benchmark) for language, benchmark in STAGES],
                        reports=reports, audits=audits, before_reports=snapshots, samples=3,
                        old_owner_identities=[plan['stage_owner_identity'], plan['coordinator_identity']],
                        owner_leases=[str(N / 'results/coordinator-v4' / ('owner-' + language + '.lock')) for language in ('r', 'apl')],
                        retained_uiua_report=mapping['retained_uiua_report'], shared_timing_lock=str(LOCK),
                        timeout_seconds=14400, maximum_rss_mib=10240,
                        native_policy='Every actual command uses the frozen execute implementation; forked sampler and native command inherit the common mutex. Normal sampler/native exits, exact command, complete frozen validation, unchanged pins and error-free resource/compiler/power/cleanup gates required.',
                        timing_policy='Fresh-process elapsed includes runtime/input/work/output/shutdown and identical PGID registration. Pin verification and output hashing remain excluded. Keep every older completed sample and exact declared command/environment; no score scaling.',
                        failure_policy='Every attempt persists. Fatal or environmental qualification failure stops without automatic retry or duplicate launch; source failure is recorded only when frozen execute records a real benchmark error.',
                        mutation_policy='Only existing R/APL timing reports continue under their original leases. No Uiua report/audit/source/manifest/harness edit.')
        save(MANIFEST, manifest)
        print(json.dumps(dict(manifest=str(MANIFEST), sha256=sha(MANIFEST), runner_sha256=sha(__file__)), indent=2), flush=True)


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
                                sampler_identity=identity(child), started_at=now(), result_path=str(result_path), sampler_inherited_common_mutex=True, key=row['key'], sample=round_index)
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
                                              complete_output_bytes_and_hash=(manifest['full_output_reference'] is None or (result.get('stdout_bytes'), result.get('stdout_sha256')) == (manifest['full_output_reference']['bytes'], manifest['full_output_reference']['sha256'])),
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


def execute_remaining(digest):
    global REPORT
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert sha(MANIFEST) == digest, 'Reviewed continuation manifest changed'
        manifest = json.loads(MANIFEST.read_text())
        verify(manifest['pins'])
        assert all(identity(row['pid']) is None for row in manifest['old_owner_identities'])
        harness = load_harness()
    harness.install_interrupt_handlers()
    leases = []
    try:
        for filename in manifest['owner_leases']:
            lease = Path(filename).open('a+')
            fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
            leases.append(lease)
        assert not (OUT / 'status.json').exists(), 'Existing continuation requires explicit recovery review'
        states = {}
        for language, filename in manifest['reports'].items():
            assert sha(filename) == manifest['before_reports'][language]['sha256'], 'Original report changed before reviewed continuation'
            report = json.loads(Path(filename).read_text())
            assert not report.get('active')
            states[language] = report
        status = dict(status='running', supervisor=identity(os.getpid()), manifest_sha256=digest, stages=[], started_at=now())
        save(OUT / 'status.json', status)
        for stage in manifest['stages']:
            language, benchmark = stage['language'], stage['benchmark']
            report = states[language]
            REPORT = Path(manifest['reports'][language])
            stage_record = dict(stage, started_at=now())
            status['stages'].append(stage_record)
            for sample in range(3):
                rows = [row for row in report['programs'].values() if row['benchmark'] == benchmark and not row.get('timing_alias_of')]
                random.Random(20261011 + sample * 1000 + sum(map(ord, language + benchmark))).shuffle(rows)
                order = [row['key'] for row in rows]
                rounds = report['orders'].setdefault(benchmark, [])
                if len(rounds) <= sample:
                    rounds.append(order)
                else:
                    assert rounds[sample] == order, 'Existing sample order changed'
                for row in rows:
                    if len(row['samples_seconds']) > sample or row.get('full_failure'):
                        continue
                    with LOCK.open('a+') as lock:
                        fcntl.flock(lock, fcntl.LOCK_EX)
                        verify(manifest['pins'])
                        harness.compact.wait_for_environment(harness.REQUIRED)
                        reference = harness.compact.HASHES.get(benchmark)
                        if benchmark == 'fannkuchredux':
                            data = b'3968050\nPfannkuchen(12) = 65\n'
                            full_reference = dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
                        elif benchmark == 'nbody':
                            # Frozen validate checks both complete energy tokens
                            # with the original1e-8 numerical tolerance.
                            full_reference = None
                        else:
                            assert reference, ('Missing complete output oracle', benchmark)
                            full_reference = dict(bytes=reference[1][0], sha256=reference[1][1])
                        sample_manifest = dict(manifest, size=harness.WORK[benchmark][1], canonical_full_workload=True,
                            full_output_reference=full_reference,
                            timeout_seconds=min(manifest['timeout_seconds'], row.get('recommended_timeout_seconds', manifest['timeout_seconds'])))
                        print('FULL REMAINING', language, benchmark, sample + 1, row['id'], flush=True)
                        attempt = one_attempt(harness, row, sample_manifest, report, sample + 1, lock.fileno())
                        number = len(report['attempts']) + 1
                        attempt.update(sample=sample + 1, attempt_number=number, continuation_manifest_sha256=digest)
                        report['attempts'].append(attempt)
                        row['attempts'].append(number)
                        report.pop('active', None)
                        if not attempt['qualified']:
                            if attempt.get('error') and attempt.get('valid_environment') is True:
                                row['full_failure'] = {key: attempt.get(key) for key in ('error', 'error_type', 'seconds', 'exit_status', 'attempt_number', 'finished_at', 'stdout_bytes', 'stdout_sha256', 'stderr', 'resource_violation')}
                                row['measurement_status'] = 'failed'
                            else:
                                row['measurement_status'] = 'continuation-stopped-unqualified'
                            save(REPORT, report)
                            raise RuntimeError('Remaining actual sample failed qualification; evidence retained and no retry')
                        row['samples_seconds'].append(attempt['seconds'])
                        row['sample_validation'].append(attempt['validation'])
                        row.setdefault('full_validation', attempt['validation'])
                        row['median_seconds'] = statistics.median(row['samples_seconds'])
                        row['measurement_status'] = 'measured' if len(row['samples_seconds']) == 3 else 'partial'
                        if len(row['samples_seconds']) >= 2:
                            row['sample_stdev_seconds'] = statistics.stdev(row['samples_seconds'])
                        harness.copy_aliases(report)
                        report['updated_at'] = now()
                        save(REPORT, report)
            group = [row for row in report['programs'].values() if row['benchmark'] == benchmark]
            if all(len(row['samples_seconds']) >= 3 or row.get('full_failure') for row in group) and benchmark not in report['completed_benchmarks']:
                report['completed_benchmarks'].append(benchmark)
            terminal = all(len(row['samples_seconds']) >= 3 or row.get('full_failure') for row in report['programs'].values())
            report['status'] = ('complete-with-failures' if any(row.get('full_failure') for row in report['programs'].values()) else 'complete') if terminal else 'partial'
            report['updated_at'] = now()
            save(REPORT, report)
            stage_record.update(status='complete', finished_at=now())
            save(OUT / 'status.json', status)
        verify(manifest['pins'])
        status.update(status='complete', finished_at=now(), pins_unchanged_at_end=True)
        save(OUT / 'status.json', status)
    except BaseException as error:
        if 'status' in locals():
            status.update(status='stopped', error=str(error), error_type=type(error).__name__, finished_at=now())
            save(OUT / 'status.json', status)
        raise
    finally:
        for lease in leases:
            lease.close()


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
        assert args.manifest_sha256, 'Exact independently reviewed manifest SHA required'
        execute_remaining(args.manifest_sha256)
    else:
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert args.manifest_sha256 and sha(MANIFEST) == args.manifest_sha256
            manifest = json.loads(MANIFEST.read_text())
            verify(manifest['pins'])
            assert not (OUT / 'launch.json').exists() and not (OUT / 'status.json').exists()
            process = None
            try:
                with (OUT / 'supervisor.log').open('ab') as log:
                    process = subprocess.Popen([sys.executable, '-u', str(Path(__file__).resolve()), '--run', '--manifest-sha256', args.manifest_sha256], stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                launch = dict(pid=process.pid, identity=identity(process.pid), manifest_sha256=args.manifest_sha256, launched_at=now())
                with (OUT / 'launch.json').open('x') as stream:
                    json.dump(launch, stream, indent=2)
                    stream.write('\n')
            except BaseException:
                if process is not None:
                    process.terminate()
                    process.wait()
                raise
        print(json.dumps(launch, indent=2), flush=True)


if __name__ == '__main__':
    main()
