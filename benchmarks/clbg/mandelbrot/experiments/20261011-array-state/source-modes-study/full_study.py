"""Separate guarded canonical Mech JIT/interpreted study and same-source AOT controls.

The frozen comparison execute function supplies its normal timing, complete
output validator and monitors. An IPC notification records the launched native
process group for the independent supervisor. This notification's single pipe
write is included in fresh-process JIT/AOT wall time and occurs before the
interpreted loaded-execution boundary. Preparation never launches full samples.
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
E = HERE.parent
ROOT = E.parents[4]
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


def load_preparation():
    spec = importlib.util.spec_from_file_location('canonical_mode_preparation', HERE / 'prepare.py')
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
        assert not MANIFEST.exists(), 'Existing sealed study requires explicit review; no overwrite'
        validation_path = HERE / 'qualification.json'
        validation = json.loads(validation_path.read_text())
        assert validation['status'] == 'passed' and validation['pins_unchanged_at_end']
        assert validation['checks'] == 96 and validation['complete_pbms'] is True
        assert validation['supervision_probes']['status'] == 'passed' and validation['supervision_probes']['checks'] == 6
        assert all(row['qualified'] is True and row['never_chart_eligible'] is True
                   for row in validation['supervision_probes']['attempts'])
        verify(validation['pins'])
        compiler_path = Path(validation['compiler_manifest'])
        assert sha(compiler_path) == validation['compiler_manifest_sha256']
        compiler_manifest = json.loads(compiler_path.read_text())
        verify(compiler_manifest['source_pins'])
        expected_small = {(row['id'], width, workers) for row in validation['profiles']
                          for width in validation['widths'] for workers in (1, 8)}
        actual_small = {(row['id'], row['size'], row['workers']) for row in validation['images']}
        assert len(validation['images']) == len(expected_small) == 96 and actual_small == expected_small
        assert all(row['qualified'] is True and row['validation']['reference_match'] is True
                   and row['supervised_native_group']['inherited_common_mutex'] is True
                   for row in validation['images'])
        audit_path = N / 'source-audit-uiua.json'
        audit = json.loads(audit_path.read_text())
        profiles = copy.deepcopy(validation['profiles'])
        assert {row['id'] for row in profiles} == {'scalar11-jit', 'scalar11-resident', 'row16-jit', 'row16-resident'}
        for row in profiles:
            row['role'] = 'canonical source mode, freshly qualified generic executor'
            row['state_layout'] = '1x3 numeric state' if row['loc'] == 11 else '2x8 numeric state plus scalar iteration'
        aot_manifest_path, aot_report_path = E / 'full-study/manifest.json', E / 'full-study/report.json'
        aot_manifest = json.loads(aot_manifest_path.read_text())
        aot_report = json.loads(aot_report_path.read_text())
        assert aot_report['status'] == 'complete' and aot_report['manifest_sha256'] == sha(aot_manifest_path)
        assert aot_report['pins_unchanged_at_end'] is True
        verify(aot_manifest['pins'])
        for original_id, new_id in [('scalar11', 'scalar11-aot-control'), ('five16-rows-loop-fma', 'row16-aot-control')]:
            row = copy.deepcopy(next(item for item in aot_manifest['profiles'] if item['id'] == original_id))
            assert len(aot_report['profiles'][original_id]['samples_seconds']) == 3
            source_mode = next(item for item in profiles if item['loc'] == row['loc'])
            assert row['source'] == source_mode['source'] and row['source_sha256'] == source_mode['source_sha256']
            assert sha(row['binary']) == row['binary_sha256']
            row.update(id=new_id, key='mandelbrot:canonical-source-modes:' + new_id, mode='aot-control',
                       timing_boundary='Fresh-process elapsed including startup, complete PBM output and shutdown; unchanged canonical-source AOT binary from the completed array-state study.',
                       maximum_workers=8, role='unchanged same-source AOT control, freshly measured in these rounds')
            profiles.append(row)
        pins = dict(validation['pins'])
        pins.update(compiler_manifest['source_pins'])
        for filename, digest in aot_manifest['pins'].items():
            assert filename not in pins or pins[filename] == digest, ('Canonical and AOT compiler provenance differ', filename)
            pins[filename] = digest
        for tool in ('rustc', 'cargo'):
            entry = compiler_manifest[tool]
            assert sha(entry['path']) == entry['sha256']
            pins[entry['path']] = entry['sha256']
        pins.update({str(validation_path): sha(validation_path), str(compiler_path): sha(compiler_path),
                     str(audit_path): sha(audit_path), str(Path(__file__)): sha(__file__),
                     str(HERE / 'prepare.py'): sha(HERE / 'prepare.py'),
                     str(aot_manifest_path): sha(aot_manifest_path), str(aot_report_path): sha(aot_report_path)})
        for row in profiles:
            pins[row['source']] = row['source_sha256']
            pins[row['binary']] = row['binary_sha256']
        for image in validation['images']:
            assert sha(image['output']) == image['validation']['sha256']
            pins[image['output']] = image['validation']['sha256']
        for filename in [N / 'run.py', N / 'prepare.py', ROOT / 'benchmarks/clbg/compact/run.py',
                         ROOT / 'benchmarks/clbg/nbody/run.py', ROOT / 'benchmarks/clbg/array-comparison/all-20261009/run.py']:
            expected = audit['pins'][str(filename)]
            assert sha(filename) == expected, ('Frozen harness pin differs', filename)
            pins[str(filename)] = expected
        harness = load_harness()
        order = []
        rng = random.Random(20261012)
        for round_index in range(3):
            identifiers = [row['id'] for row in profiles]
            rng.shuffle(identifiers)
            order.append(dict(round=round_index + 1, profiles=identifiers))
        manifest = dict(status='prepared-awaiting-independent-review', created_at=now(), samples=3, size=16000, workers=8,
                        source_metric='LOC only; one independent authored binding/statement per line',
                        profiles=profiles, orders=order, pins=pins, shared_timing_lock=str(LOCK),
                        required_power_profile=harness.REQUIRED, maximum_rss_mib=10240, timeout_seconds=14400,
                        full_output_reference=dict(bytes=harness.compact.HASHES['mandelbrot'][1][0],
                                                   sha256=harness.compact.HASHES['mandelbrot'][1][1]),
                        timing_boundary='Three fresh N16000 processes per profile, maximum eight native workers, full PBM to regular-file sinks. JIT and AOT score complete fresh-process elapsed time. Interpreted scores kernel_seconds after preparation, including image computation, PBM writing and flush; preparation and whole-process times are retained separately. Frozen execute output validator and power/compiler/RSS/group monitors retained. Implementation builds, pins and output hashing excluded. Supervisor registration is in JIT/AOT wall time and before the interpreted loaded-execution timer.',
                        source_timing_gate='Exactly one finite nonnegative prepare_seconds and positive kernel_seconds TIMING record for JIT/interpreted; their sum must fit measured wall time. These records never replace JIT whole-process time.',
                        compiler_manifest=str(compiler_path), compiler_manifest_sha256=sha(compiler_path),
                        qualification=str(validation_path), qualification_sha256=sha(validation_path),
                        unchanged_aot_provenance=dict(manifest=str(aot_manifest_path), manifest_sha256=sha(aot_manifest_path), report=str(aot_report_path), report_sha256=sha(aot_report_path)),
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
                event = dict(type='native-group-started', pid=process.pid, pgid=process.pid, command=command)
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
        result = harness.execute(row, True, manifest['timeout_seconds'], manifest['maximum_rss_mib'])
        result.update(canonical_full_workload=manifest['size'] == 16000, never_chart_eligible=manifest['size'] != 16000)
        result['wall_seconds'] = result['seconds']
        if row['mode'] in ('jit', 'resident'):
            timing = load_preparation().source_timing(result, row['mode'])
            result.update(timing=timing, prepare_seconds=timing['prepare_seconds'],
                          kernel_seconds=timing['kernel_seconds'], seconds=timing['reported_seconds'],
                          valid_source_timing=True)
        else:
            result['valid_source_timing'] = True
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
                                              no_supervisor_remaining_group=not result.get('supervisor_remaining_members'),
                                              valid_environment=result.get('valid_environment') is True,
                                              valid_source_timing=result.get('valid_source_timing') is True,
                                              actual_full_workload=result.get('full_workload') is True and result.get('workload_n') == manifest['size'],
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
                      profiles={key: dict(loc=row['loc'], mode=row['mode'], state_layout=row['state_layout'],
                                          timing_boundary=row['timing_boundary'], samples_seconds=[], sample_details=[]) for key, row in rows.items()})
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
                            report['profiles'][identifier]['sample_details'].append({key: attempt[key] for key in
                                ('seconds', 'wall_seconds', 'prepare_seconds', 'kernel_seconds', 'validation', 'supervised_native_group') if key in attempt})
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
