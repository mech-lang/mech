"""Isolated complete-output study, prepared and root-reviewed before launch.

Supervisor, separate sampler and native process retain the same common mutex.
The frozen execute function stays unchanged; native Popen registration sends
an authoritative process-group id to the supervisor for cleanup.
"""
import argparse
import copy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
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
REPORT = HERE / 'results/full-study.json'


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix+'.tmp')
    temporary.write_text(json.dumps(value, indent=2)+'\n')
    temporary.replace(path)


def load_harness():
    # The candidate also has prepare.py; isolate the frozen harness's module.
    previous = sys.modules.pop('prepare', None)
    sys.path.insert(0, str(N))
    try:
        spec = importlib.util.spec_from_file_location('uiua_parallel_frozen_harness', N / 'run.py')
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


def prepare():
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        qualification_path = HERE / 'manifest.json'
        qualification = json.loads(qualification_path.read_text())
        assert qualification['status'] == 'prepared' and qualification['validation_status'] == 'passed'
        assert qualification['full_performance_runs'] == 0
        harness = load_harness()
        harness.PinGuard(qualification['pins']).verify()
        audit_path = N / 'source-audit-uiua.json'
        audit = json.loads(audit_path.read_text())
        assert audit['status'] == 'reviewed'
        pins = dict(qualification['pins'])
        pins.update(audit['pins'])
        for filename in [N / 'run.py', N / 'prepare.py', ROOT / 'benchmarks/clbg/compact/run.py',
                         ROOT / 'benchmarks/clbg/nbody/run.py', ROOT / 'benchmarks/clbg/array-comparison/all-20261009/run.py']:
            expected = audit['pins'][str(filename)]
            assert sha(filename) == expected, ('Frozen harness pin differs', filename)
            pins[str(filename)] = expected
        pins.update({str(qualification_path): sha(qualification_path), str(audit_path): sha(audit_path),
                     str(Path(__file__).resolve()): sha(__file__)})
        profiles = []
        for family, identifier in (('uiua-mandelbrot-v2', 'whole11'), ('uiua-mandel-active-v3', 'active14')):
            baseline_path = N / family / 'manifest.json'
            baseline = json.loads(baseline_path.read_text())
            assert sha(baseline_path) == qualification['retained']['manifests'][str(baseline_path)]
            row = copy.deepcopy(next(row for row in baseline['programs'] if row['benchmark'] == 'mandelbrot'))
            row.update(id=identifier, key='mandelbrot:uiua-parallel-study:'+identifier,
                       cwd=str(N), runtime_environment=qualification['programs'][0]['runtime_environment'],
                       role='unchanged sequential source, fresh environment-matched full control', maximum_workers=1)
            profiles.append(row)
        for family, identifier in (('whole', 'whole16-parallel'), ('active', 'active19-parallel')):
            row = copy.deepcopy(next(row for row in qualification['programs'] if row['id'] == 'mandelbrot-uiua-parallel-'+family+'-v4'))
            row.update(id=identifier, key='mandelbrot:uiua-parallel-study:'+identifier,
                       role='additional ordinary Uiua Spawn/Wait candidate')
            profiles.append(row)
        control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == 'mandelbrot'))
        assert control['loc'] == 29 and control['command'][-1] == '8'
        assert sha(control['source']) == control['source_sha256']
        assert sha(control['binary']) == control['binary_sha256']
        control.update(id='mech29-control', key='mandelbrot:uiua-parallel-study:mech29-control',
                       role='unchanged Mech fast 29 LOC / eight-worker AOT control', maximum_workers=8)
        profiles.append(control)
        pins[control['source']] = control['source_sha256']
        pins[control['binary']] = control['binary_sha256']
        identifiers = [row['id'] for row in profiles]
        orders = [dict(round=round_index+1, profiles=identifiers[round_index:]+identifiers[:round_index]) for round_index in range(3)]
        output_reference = harness.compact.HASHES['mandelbrot'][1]
        assert tuple(output_reference) == (32000015, '609262469ee6a0262ccd03932e557f745c9e7b997ad17835a02a0232a64807be')
        manifest = dict(status='prepared-awaiting-root-review', created_at=now(), samples=3, size=16000,
                        profiles=profiles, orders=orders, pins=pins, shared_timing_lock=str(LOCK),
                        required_power_profile=harness.REQUIRED, maximum_rss_mib=8192, timeout_seconds=600,
                        environment_retry_limit=3, maximum_compute_workers=8,
                        full_output_reference=dict(bytes=output_reference[0], sha256=output_reference[1]),
                        timing_boundary='Every fresh N16000 process includes runtime startup, ordinary workers, all recurrence work, packing, ordered complete PBM write, process shutdown, and an identical supervisor PID-registration pipe write. Compilation, validation and hashing are excluded. Frozen execute power/compiler/RSS/group monitors are retained.',
                        ownership='Detached supervisor, forked sampler and native child inherit the same held common mutex descriptor. Sampler and supervisor retain it through native cleanup and result persistence. Authoritative launch PGID notification allows supervisor cleanup after sampler failure; a surviving sampler completes cleanup if supervisor dies.',
                        retry_policy='Record every unqualified attempt. Only environment-only failures may retry, at most three attempts per required sample. Output, pins, timeout, resource, sampler and cleanup errors stop immediately.',
                        raw_output_policy='Frozen execute validates the complete temporary regular-file output and preserves byte count, digest, status and diagnostics in each per-attempt evidence file.',
                        chart_policy='Experimental evidence only; no cohort report/audit writes, admission, score publication or refreeze. All five profiles need three qualified samples before comparison.')
        harness.PinGuard(pins).verify()
        save(MANIFEST, manifest)
        print(json.dumps(dict(manifest=str(MANIFEST), sha256=sha(MANIFEST), runner=str(Path(__file__).resolve()),
                              runner_sha256=sha(__file__), status=manifest['status']), indent=2), flush=True)


def sampler(harness, row, write_fd, result_path, manifest, lock_fd):
    os.setsid()
    harness.install_interrupt_handlers()
    original = harness.subprocess.Popen
    command = [part.replace('{n}', str(manifest['size'])) for part in row['command']]

    def recorded_popen(*args, **kwargs):
        argv = args[0] if args else kwargs.get('args')
        native = list(argv) == command and kwargs.get('start_new_session')
        if native:
            kwargs['pass_fds'] = tuple(set(kwargs.get('pass_fds', ())) | {lock_fd})
        process = original(*args, **kwargs)
        if native:
            try:
                event = dict(type='native-group-started', pid=process.pid, pgid=process.pid, command=command,
                             inherited_common_mutex=True)
                os.write(write_fd, (json.dumps(event)+'\n').encode())
            except BrokenPipeError:
                pass
            except BaseException:
                # Until this wrapper returns, frozen execute has no proc to
                # clean. A catchable interrupt in registration therefore
                # belongs to the wrapper that already knows the native PID.
                harness.kill_group(process.pid)
                process.wait()
                raise
        return process

    harness.subprocess.Popen = recorded_popen
    try:
        guard = harness.PinGuard(manifest['pins'])
        guard.verify()
        result = harness.execute(row, True, manifest['timeout_seconds'], manifest['maximum_rss_mib'])
        guard.verify()
        result['pins_unchanged_at_end'] = True
        save(result_path, result)
        os._exit(0)
    except BaseException as error:
        result = locals().get('result', {})
        result.update(error=str(error), error_type=type(error).__name__, sampler_failure=True)
        save(result_path, result)
        os._exit(1)


def one_attempt(harness, row, manifest, report, round_index, lock_fd):
    index = len(report['attempts'])+1
    result_path = OUT / 'attempts' / f'{index:03d}-{row["id"]}.json'
    read_fd, write_fd = os.pipe()
    child = os.fork()
    if child == 0:
        os.close(read_fd)
        sampler(harness, row, write_fd, result_path, manifest, lock_fd)
    poll = None
    buffer, native, reaped = b'', None, False
    try:
        os.close(write_fd)
        os.set_blocking(read_fd, False)
        poll = selectors.DefaultSelector()
        poll.register(read_fd, selectors.EVENT_READ)
        report['active'] = dict(attempt=index, round=round_index, id=row['id'], sampler_pid=child,
                                sampler_identity=identity(child), started_at=now(), result_path=str(result_path),
                                sampler_inherited_common_mutex=True)
        save(REPORT, report)
        while True:
            for key, _ in poll.select(0.1):
                data = os.read(key.fd, 4096)
                if not data:
                    poll.unregister(key.fd)
                buffer += data
                while b'\n' in buffer:
                    line, buffer = buffer.split(b'\n', 1)
                    event = json.loads(line)
                    if event['type'] == 'native-group-started':
                        native = event
                        report['active']['native'] = dict(event, identity=identity(event['pid']))
                        save(REPORT, report)
            pid, status = os.waitpid(child, os.WNOHANG)
            if pid:
                reaped = True
                break
        # A short-lived process can exit before the final launch notification
        # is consumed. Preserve its authoritative PGID before inspecting the
        # result or releasing any cleanup ownership.
        while True:
            try:
                data = os.read(read_fd, 4096)
            except BlockingIOError:
                break
            if not data:
                break
            buffer += data
        while b'\n' in buffer:
            line, buffer = buffer.split(b'\n', 1)
            event = json.loads(line)
            if event['type'] == 'native-group-started':
                native = event
                report['active']['native'] = dict(event, identity=identity(event['pid']))
                save(REPORT, report)
        result = json.loads(result_path.read_text()) if result_path.exists() else dict(error='Sampler exited without result', sampler_failure=True)
        result.update(round=round_index, attempt=index, sampler_wait_status=status, result_path=str(result_path))
        if native:
            result['supervised_native_group'] = native
            members = harness.group_members(native['pgid'])
            result['supervisor_remaining_members'] = members
            if members:
                harness.kill_group(native['pgid'])
                result.update(error='Native group remained after sampler exit; cleaned before releasing lock', supervisor_cleanup=True)
        elif not result.get('error'):
            result.update(error='Sampler supplied no authoritative native launch notification', supervisor_registration_error=True)
        harness.PinGuard(manifest['pins']).verify()
        fatal_fields = ('error', 'resource_violation', 'cleanup_error', 'supervisor_cleanup',
                        'supervisor_registration_error', 'sampler_failure', 'monitor_errors',
                        'resource_monitor_errors', 'monitor_finish_error')
        fatal = [name for name in fatal_fields if result.get(name)]
        if status != 0:
            fatal.append('sampler_wait_status')
        result['fatal_qualification_fields'] = fatal
        success = not fatal and result.get('validation', {}).get('reference_match') is True and result.get('pins_unchanged_at_end') is True
        environmental_change = bool(result.get('competing_compilers') or result.get('power_changes')
                                    or result.get('power_at_start') != harness.REQUIRED
                                    or result.get('power_at_end') != harness.REQUIRED)
        result['qualified'] = success and result.get('valid_environment') is True
        result['retryable_environment_only'] = success and result.get('valid_environment') is False and environmental_change
        save(result_path, result)
        return result
    finally:
        if not reaped:
            try:
                os.kill(child, signal.SIGTERM)
            except ProcessLookupError:
                pass
            deadline = time.monotonic()+8
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
        # Cleanup is required even if waitpid already reaped the sampler;
        # interruption can arrive before its final launch event is consumed.
        os.set_blocking(read_fd, False)
        while True:
            try:
                data = os.read(read_fd, 4096)
            except BlockingIOError:
                break
            if not data:
                break
            buffer += data
        while b'\n' in buffer:
            line, buffer = buffer.split(b'\n', 1)
            event = json.loads(line)
            if event['type'] == 'native-group-started':
                native = event
        if native:
            harness.kill_group(native['pgid'])
        if poll is not None:
            poll.close()
        os.close(read_fd)


def run(manifest_digest):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        assert sha(MANIFEST) == manifest_digest, 'Reviewed study manifest changed'
        manifest = json.loads(MANIFEST.read_text())
        harness = load_harness()
        harness.PinGuard(manifest['pins']).verify()
    harness.install_interrupt_handlers()
    rows = {row['id']: row for row in manifest['profiles']}
    with (OUT / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        assert not REPORT.exists(), 'Existing study report needs explicit review; no automatic resumption'
        report = dict(status='running', created_at=now(), manifest=str(MANIFEST), manifest_sha256=manifest_digest,
                      supervisor=identity(os.getpid()), orders=manifest['orders'], attempts=[], active=None,
                      profiles={key: dict(loc=row['loc'], role=row['role'], samples_seconds=[]) for key, row in rows.items()})
        save(REPORT, report)
        try:
            for ordered in manifest['orders']:
                for identifier in ordered['profiles']:
                    for retry in range(manifest['environment_retry_limit']):
                        with LOCK.open('a+') as lock:
                            fcntl.flock(lock, fcntl.LOCK_EX)
                            harness.compact.wait_for_environment(harness.REQUIRED)
                            harness.PinGuard(manifest['pins']).verify()
                            print('FULL', ordered['round'], identifier, manifest['size'], flush=True)
                            attempt = one_attempt(harness, rows[identifier], manifest, report, ordered['round'], lock.fileno())
                            attempt['environment_attempt'] = retry+1
                            report['attempts'].append(attempt)
                            report['active'] = None
                            if attempt['qualified']:
                                report['profiles'][identifier]['samples_seconds'].append(attempt['seconds'])
                            save(REPORT, report)
                            if attempt['qualified']:
                                break
                            if not attempt['retryable_environment_only']:
                                raise RuntimeError('Sample failed output/pin/resource/process qualification; evidence retained and study stopped')
                    else:
                        raise RuntimeError('Environment-only retry limit exhausted; unqualified evidence retained')
            for profile in report['profiles'].values():
                assert len(profile['samples_seconds']) == 3
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
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            assert args.manifest_sha256 and sha(MANIFEST) == args.manifest_sha256
            with (OUT / 'supervisor.log').open('ab') as log:
                process = subprocess.Popen([sys.executable, '-u', str(Path(__file__).resolve()), '--run', '--manifest-sha256', args.manifest_sha256],
                                           stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            launch = dict(pid=process.pid, identity=identity(process.pid), manifest_sha256=args.manifest_sha256, launched_at=now())
            save(OUT / 'launch.json', launch)
            print(json.dumps(launch, indent=2), flush=True)


if __name__ == '__main__':
    main()
