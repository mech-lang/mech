"""Separate guarded Mech array-state study. Preparation does not run samples.

The frozen comparison execute function supplies its normal timing, complete
output validator and monitors. An IPC notification records the launched native
process group for the independent supervisor. This notification's single pipe
write is included identically in every profile's measured boundary.
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
ROOT = HERE.parents[4]
N = ROOT / 'benchmarks/clbg/r-apl-uiua-comparison/20261011'
OUT = HERE / 'full-study'
LOCK = Path('/tmp/mech-clbg-timing.lock')
MANIFEST = OUT / 'manifest.json'
REPORT = OUT / 'report.json'


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
    sys.path.insert(0, str(N))
    spec = importlib.util.spec_from_file_location('array_state_frozen_harness', N / 'run.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def identity(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'lstart=', '-o', 'command='], capture_output=True, text=True)
    return dict(pid=pid, process_listing=result.stdout.strip()) if result.returncode == 0 and result.stdout.strip() else None


def verify(pins):
    for filename, digest in pins.items():
        assert sha(filename) == digest, ('Pinned file changed', filename)


def prepare():
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        validation_path = HERE / 'validation-current/report.json'
        validation = json.loads(validation_path.read_text())
        assert validation['status'] == 'passed' and validation['pins_unchanged_at_end']
        verify(validation['pins'])
        compiler_manifest = json.loads(Path(validation['driver_manifest']).read_text())
        assert compiler_manifest['driver_sha256'] == validation['driver_sha256']
        verify(compiler_manifest['source_pins'])
        audit_path = N / 'source-audit-uiua.json'
        audit = json.loads(audit_path.read_text())
        frozen29 = next(row for row in audit['controls'] if row['benchmark'] == 'mandelbrot')
        assert sha(frozen29['source']) == frozen29['source_sha256']
        assert sha(frozen29['binary']) == frozen29['binary_sha256']
        frozen29 = copy.deepcopy(frozen29)
        frozen29.update(id='frozen29-audited', key='mandelbrot:array-state:frozen29-audited',
                        simd=True, loop_fma=True, state_layout='separate 8x1 real/imaginary arrays plus scalar iteration',
                        role='immutable published native control')
        profiles = []
        selected = [('scalar11', '1x3 numeric state', 'new canonical compact source'),
                    ('five16-loop-fma', '8x2 numeric state plus scalar iteration', 'new canonical five-step source'),
                    ('five16-rows-loop-fma', '2x8 numeric state plus scalar iteration', 'new canonical five-step row source'),
                    ('frozen20', 'separate scalar real/imaginary/iteration state', 'unchanged compact source, freshly compiled control')]
        for identifier, layout, role in selected:
            row = copy.deepcopy(next(row for row in validation['profiles'] if row['id'] == identifier))
            row.update(key='mandelbrot:array-state:' + identifier, benchmark='mandelbrot', language='mech',
                       binary=row['executable'], command=[row['executable'], '{n}', '8'], cwd=str(ROOT),
                       runtime_environment={}, state_layout=layout, role=role)
            profiles.append(row)
        profiles.append(frozen29)
        pins = dict(validation['pins'])
        pins.update(compiler_manifest['source_pins'])
        for tool in ('rustc', 'cargo'):
            entry = compiler_manifest[tool]
            assert sha(entry['path']) == entry['sha256']
            pins[entry['path']] = entry['sha256']
        pins.update({str(validation_path): sha(validation_path), str(audit_path): sha(audit_path),
                     frozen29['source']: frozen29['source_sha256'], frozen29['binary']: frozen29['binary_sha256'],
                     str(Path(__file__)): sha(__file__)})
        for filename in [N / 'run.py', N / 'prepare.py', ROOT / 'benchmarks/clbg/compact/run.py',
                         ROOT / 'benchmarks/clbg/nbody/run.py', ROOT / 'benchmarks/clbg/array-comparison/all-20261009/run.py']:
            expected = audit['pins'][str(filename)]
            assert sha(filename) == expected, ('Frozen harness pin differs', filename)
            pins[str(filename)] = expected
        for name in ('five-rows.o', 'frozen-fast.o', 'five-rows.c', 'frozen-fast.c'):
            filename = HERE / 'validation' / name
            pins[str(filename)] = sha(filename)
        harness = load_harness()
        order = []
        rng = random.Random(20261011)
        for round_index in range(3):
            identifiers = [row['id'] for row in profiles]
            rng.shuffle(identifiers)
            order.append(dict(round=round_index + 1, profiles=identifiers))
        manifest = dict(status='prepared-awaiting-root-review', created_at=now(), samples=3, size=16000, workers=8,
                        source_metric='LOC only; one independent authored binding/statement per line',
                        profiles=profiles, orders=order, pins=pins, shared_timing_lock=str(LOCK),
                        required_power_profile=harness.REQUIRED, maximum_rss_mib=10240, timeout_seconds=600,
                        full_output_reference=dict(bytes=harness.compact.HASHES['mandelbrot'][1][0],
                                                   sha256=harness.compact.HASHES['mandelbrot'][1][1]),
                        timing_boundary='Three fresh N16000 processes per profile, eight native workers, complete PBM to regular-file sinks, startup/work/flush/shutdown included. Frozen execute validator and power/compiler/RSS/group monitors retained. Compilation, pins and output hashing excluded. The supervisor PID-registration pipe write is included in every measured profile.',
                        ownership='Independent supervisor and per-sample sampler process sessions; both retain the common lock descriptor. The native benchmark command also explicitly inherits this descriptor through pass_fds, closing the launch-to-PGID-notification crash window. Supervisor receives authoritative native PGID before waiting and cleans it before lock release. A surviving sampler/native process continues holding the common lock through cleanup/exit.',
                        raw_output_policy='Frozen execute hashes/validates each complete regular-file output then closes its temporary sink; full byte count, digest, status and diagnostics retained in each attempt.',
                        chart_policy='Separate experimental study; no frozen report/source/audit changes and no chart promotion from this file.')
        verify(pins)
        save(MANIFEST, manifest)
        print(json.dumps(dict(manifest=str(MANIFEST), sha256=sha(MANIFEST), status=manifest['status']), indent=2), flush=True)


def sampler(harness, row, write_fd, result_path, manifest, lock_fd):
    os.setsid()
    harness.install_interrupt_handlers()
    original = harness.subprocess.Popen
    command = [part.replace('{n}', str(manifest['size'])) for part in row['command']]

    def recorded_popen(*args, **kwargs):
        argv = args[0] if args else kwargs.get('args')
        native_command = list(argv) == command and kwargs.get('start_new_session')
        if native_command:
            kwargs['pass_fds'] = tuple(sorted(set(kwargs.get('pass_fds', ())) | {lock_fd}))
        process = original(*args, **kwargs)
        if native_command:
            try:
                event = dict(type='native-group-started', pid=process.pid, pgid=process.pid, command=command)
                os.write(write_fd, (json.dumps(event) + '\n').encode())
            except BrokenPipeError:
                # The sampler retains the inherited common lock even if its
                # supervisor died. Complete execute cleanup/result persistence.
                pass
            except BaseException:
                # execute has not yet received the Popen result, so it cannot
                # clean this group if registration itself is interrupted.
                harness.kill_group(process.pid)
                process.wait()
                raise
        return process

    harness.subprocess.Popen = recorded_popen
    try:
        verify(manifest['pins'])
        result = harness.execute(row, True, manifest['timeout_seconds'], manifest['maximum_rss_mib'])
        verify(manifest['pins'])
        result['pins_unchanged_at_end'] = True
        save(result_path, result)
        os._exit(0)
    except BaseException as error:
        save(result_path, dict(error=str(error), error_type=type(error).__name__, sampler_failure=True))
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
                                sampler_identity=identity(child), started_at=now(), result_path=str(result_path))
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
                                              native_exited_normally=result.get('exit_status') == 0,
                                              no_execution_error=not result.get('error'),
                                              no_resource_violation=not result.get('resource_violation'),
                                              no_resource_monitor_errors=not result.get('resource_monitor_errors'),
                                              no_compiler_power_monitor_errors=not result.get('monitor_errors'),
                                              no_monitor_finish_error=not result.get('monitor_finish_error'),
                                              no_execute_cleanup_error=not result.get('cleanup_error'),
                                              no_remaining_execute_group=not result.get('remaining_group_members'),
                                              no_supervisor_cleanup=not result.get('supervisor_cleanup'),
                                              valid_environment=result.get('valid_environment') is True,
                                              complete_output_reference_match=result.get('validation', {}).get('reference_match') is True,
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
                      profiles={key: dict(loc=row['loc'], state_layout=row['state_layout'], samples_seconds=[]) for key, row in rows.items()})
        save(REPORT, report)
        try:
            for ordered in manifest['orders']:
                for identifier in ordered['profiles']:
                    with LOCK.open('a+') as lock:
                        fcntl.flock(lock, fcntl.LOCK_EX)
                        harness.compact.wait_for_environment(harness.REQUIRED)
                        verify(manifest['pins'])
                        print('FULL', ordered['round'], identifier, manifest['size'], manifest['workers'], flush=True)
                        attempt = one_attempt(harness, rows[identifier], manifest, report, ordered['round'], lock.fileno())
                        report['attempts'].append(attempt)
                        report['active'] = None
                        if attempt['qualified']:
                            report['profiles'][identifier]['samples_seconds'].append(attempt['seconds'])
                        save(REPORT, report)
                        if not attempt['qualified']:
                            raise RuntimeError('Full sample failed qualification; evidence retained and study stopped')
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
        with LOCK.open('a+') as launch_lock:
            fcntl.flock(launch_lock, fcntl.LOCK_EX)
            assert args.manifest_sha256 and sha(MANIFEST) == args.manifest_sha256
            manifest = json.loads(MANIFEST.read_text())
            verify(manifest['pins'])
            with (OUT / 'supervisor.log').open('ab') as log:
                process = subprocess.Popen([sys.executable, '-u', str(Path(__file__).resolve()), '--run', '--manifest-sha256', args.manifest_sha256],
                                           stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            launch = dict(pid=process.pid, identity=identity(process.pid), manifest_sha256=args.manifest_sha256, launched_at=now())
            save(OUT / 'launch.json', launch)
        print(json.dumps(launch, indent=2), flush=True)


if __name__ == '__main__':
    main()
