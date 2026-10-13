"""Unexecuted bounded owned lsof observer for the actual JVM mutex descriptor."""
import json
import os
from pathlib import Path
import subprocess
import threading
import signal

LOCK = Path('/tmp/mech-clbg-timing.lock')


def new_json(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def parse_files(text):
    records, current, file = [], None, None
    for line in text.splitlines():
        if line.startswith('p'):
            current, file = dict(pid=int(line[1:]), files=[]), None
            records.append(current)
        elif current is not None and line.startswith('c'):
            current['command'] = line[1:]
        elif current is not None and line.startswith('f'):
            # lsof may append a mode character, for example 64u.
            digits = ''.join(char for char in line[1:] if char.isdigit())
            file = dict(fd=int(digits)) if digits else {}
            current['files'].append(file)
        elif file is not None and line.startswith('n'):
            file['path'] = line[1:]
        elif file is not None and line.startswith('D'):
            file['device'] = int(line[1:], 0)
        elif file is not None and line.startswith('i'):
            file['inode'] = int(line[1:])
    return records


def install(harness, command, descriptor, evidence_dir):
    """Patch only while a single reviewed direct native command is executing.

    The actual Popen handle supplies group ownership. All observers use new
    known groups, bounded drains/reaps, and exact restoration by the caller.
    """
    original_execute, original_popen = harness.execute, harness.subprocess.Popen
    identity = os.fstat(descriptor)
    proof = dict(descriptor=descriptor, common_device_inode=[identity.st_dev, identity.st_ino],
                 native_observed=False, observations=[], errors=[], reviewed_command=command)
    stop = threading.Event()
    worker = None
    owned_observers = []
    directory = Path(evidence_dir)
    directory.mkdir(parents=True, exist_ok=False)

    def kill_owned(process):
        # Only an actual unreaped Popen leader of our new session may be
        # signalled. Group membership queries themselves have bounded owners.
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

    def owned_members(pgid, prefix):
        argv = ['/bin/ps', '-eo', 'pid,pgid,stat,comm']
        process = None
        stdout, stderr = b'', b''
        detail = dict(command=argv, query_pgid=pgid, inherited_common_mutex=True, timeout_seconds=5)
        try:
            process = original_popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, start_new_session=True, pass_fds=(descriptor,))
            owned_observers.append(process)
            detail['owned_pid'] = process.pid
            stdout, stderr = process.communicate(timeout=5)
            assert process.returncode == 0, 'Bounded owned group query failed'
            result = []
            for line in stdout.decode().splitlines()[1:]:
                fields = line.split(None, 3)
                if len(fields) == 4 and int(fields[1]) == pgid and not fields[2].startswith('Z'):
                    result.append(dict(pid=int(fields[0]), pgid=int(fields[1]), state=fields[2], executable=fields[3]))
            detail['actual_members'] = result
            return result
        except BaseException as error:
            detail.update(error=str(error), error_type=type(error).__name__)
            if isinstance(error, subprocess.TimeoutExpired):
                stdout, stderr = error.output or b'', error.stderr or b''
                prefix.with_suffix('.timeout.stdout').write_bytes(stdout)
                prefix.with_suffix('.timeout.stderr').write_bytes(stderr)
            raise
        finally:
            cleanup = []
            if process is not None:
                try:
                    kill_owned(process)
                    process.wait(timeout=5)
                    detail['owned_query_reaped'] = True
                except BaseException as error:
                    cleanup.append(dict(stage='bounded-owned-query-reap', error=str(error)))
                try:
                    stdout, stderr = process.communicate(timeout=5)
                except BaseException as error:
                    cleanup.append(dict(stage='bounded-owned-query-drain', error=str(error)))
                    if isinstance(error, subprocess.TimeoutExpired):
                        prefix.with_suffix('.partial.stdout').write_bytes(error.output or b'')
                        prefix.with_suffix('.partial.stderr').write_bytes(error.stderr or b'')
                detail['exit_status'] = process.returncode
                for stream in (process.stdout, process.stderr):
                    if stream is not None:
                        stream.close()
            detail['cleanup_errors'] = cleanup
            prefix.with_suffix('.stdout').write_bytes(stdout)
            prefix.with_suffix('.stderr').write_bytes(stderr)
            new_json(prefix.with_suffix('.json'), detail)
            assert not cleanup, ('Owned group query cleanup failed', cleanup)

    def stop_native(native):
        if native.poll() is None:
            try:
                harness.kill_group(native.pid)  # Captured still-live Popen group only.
            except BaseException as error:
                proof['errors'].append(dict(error='Known owned native cleanup failed: ' + str(error), error_type=type(error).__name__))

    def observe(native):
        while not stop.is_set() and native.poll() is None:
            observer = None
            stdout, stderr = b'', b''
            prefix = directory / ('%04d-lsof' % (len(proof['observations']) + len(proof['errors']) + 1))
            argv = ['/usr/sbin/lsof', '-n', '-P', '-a', '-p', str(native.pid),
                    '-d', str(descriptor), '-FpcfnDi']
            detail = dict(command=argv, native_pid=native.pid, native_pgid=native.pid,
                          inherited_common_mutex=True, timeout_seconds=5, preparation_only=True)
            try:
                observer = original_popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE, start_new_session=True, pass_fds=(descriptor,))
                owned_observers.append(observer)
                detail['owned_observer_pid'] = observer.pid
                try:
                    stdout, stderr = observer.communicate(timeout=5)
                except subprocess.TimeoutExpired as error:
                    stdout, stderr = error.output or b'', error.stderr or b''
                    prefix.with_suffix('.timeout.stdout').write_bytes(stdout)
                    prefix.with_suffix('.timeout.stderr').write_bytes(stderr)
                    detail.update(error=str(error), error_type=type(error).__name__)
                    raise
                records = parse_files(stdout.decode(errors='replace'))
                matches = [file for row in records if row['pid'] == native.pid and row.get('command') == 'java'
                    for file in row['files'] if file.get('fd') == descriptor and
                    file.get('path') in (str(LOCK), str(LOCK.resolve())) and
                    [file.get('device'), file.get('inode')] == proof['common_device_inode']]
                if len(matches) == 1 and observer.returncode == 0:
                    proof['native_observed'] = True
                    proof['observations'].append(dict(native_pid=native.pid, native_pgid=native.pid,
                        inherited_fd=descriptor, native_command=command, actual_files=records,
                        receipt=str(prefix.with_suffix('.json'))))
                elif native.poll() is None:
                    raise RuntimeError('Actual live JVM common descriptor identity not proved by retained lsof bytes')
            except BaseException as error:
                detail.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
                proof['errors'].append(dict(error=detail['error'], error_type=detail['error_type'], receipt=str(prefix.with_suffix('.json'))))
                stop_native(native)
            finally:
                if observer is not None:
                    cleanup = []
                    try:
                        kill_owned(observer)
                        observer.wait(timeout=5)
                        detail['owned_observer_reaped'] = True
                    except BaseException as error:
                        cleanup.append(dict(stage='bounded-known-owned-observer-reap', error=str(error)))
                        if observer.poll() is None:
                            try:
                                observer.kill()
                                observer.wait(timeout=5)
                                detail['owned_observer_reaped'] = True
                            except BaseException as second:
                                cleanup.append(dict(stage='live-owned-observer-final-reap', error=str(second)))
                    try:
                        complete_stdout, complete_stderr = observer.communicate(timeout=5)
                        stdout, stderr = complete_stdout, complete_stderr
                    except BaseException as error:
                        cleanup.append(dict(stage='bounded-owned-observer-output-drain', error=str(error)))
                        if isinstance(error, subprocess.TimeoutExpired):
                            prefix.with_suffix('.partial.stdout').write_bytes(error.output or b'')
                            prefix.with_suffix('.partial.stderr').write_bytes(error.stderr or b'')
                    try:
                        remaining = owned_members(observer.pid, prefix.with_name(prefix.name + '-group-list'))
                    except BaseException as error:
                        remaining = ['unverified-owned-group']
                        cleanup.append(dict(stage='owned-observer-group-reconciliation', error=str(error)))
                    detail.update(exit_status=observer.returncode, remaining_group=remaining, cleanup_errors=cleanup)
                    if remaining or cleanup:
                        proof['errors'].append(dict(error='Bounded owned JVM FD observer cleanup failed', receipt=str(prefix.with_suffix('.json'))))
                        stop_native(native)
                    if observer.stdout is not None:
                        observer.stdout.close()
                    if observer.stderr is not None:
                        observer.stderr.close()
                prefix.with_suffix('.stdout').write_bytes(stdout)
                prefix.with_suffix('.stderr').write_bytes(stderr)
                new_json(prefix.with_suffix('.json'), detail)
            if proof['errors']:
                return
            if proof['native_observed'] is True:
                return  # Exactly one actual native descriptor observation.
            if stop.wait(.2):
                return

    def observed_worker(native):
        try:
            observe(native)
        except BaseException as error:
            # Even an evidence-write/thread-finalizer error must reach the
            # child's actual sampler record, never become an ignored thread.
            proof['errors'].append(dict(error='Owned FD observer thread failed: ' + str(error),
                error_type=type(error).__name__))
            stop_native(native)

    def observed_popen(*args, **kwargs):
        nonlocal worker
        value = original_popen(*args, **kwargs)
        argv = args[0] if args else kwargs.get('args')
        if list(argv) == command and kwargs.get('start_new_session') is True:
            try:
                assert worker is None, 'Only one exact reviewed native subject per observer scope'
                assert descriptor in kwargs.get('pass_fds', ()), 'Actual direct Java Popen must inherit the held common descriptor'
                proof['actual_native_pass_fds_contains_common_descriptor'] = True
                worker = threading.Thread(target=observed_worker, args=(value,), daemon=True)
                worker.start()
            except BaseException:
                # execute has not received this actual owned Popen handle yet.
                stop_native(value)
                value.wait(timeout=5)
                raise
        return value

    def observed_execute(*args, **kwargs):
        try:
            result = original_execute(*args, **kwargs)
        finally:
            stop.set()
            if worker is not None:
                worker.join(timeout=30)
                if worker.is_alive():
                    proof['errors'].append(dict(error='Owned JVM FD observer did not finish within its bounded join'))
                    for owned in owned_observers:
                        try:
                            kill_owned(owned)
                            owned.wait(timeout=5)
                        except BaseException as error:
                            proof['errors'].append(dict(error='Known FD observer final reap failed: ' + str(error)))
                    worker.join(timeout=10)
        result['bounded_mutex_fd_evidence'] = proof
        if proof['errors'] or proof['native_observed'] is not True:
            result['supervisor_error'] = 'Actual native JVM inherited common mutex descriptor/observer cleanup unproved'
        return result

    harness.subprocess.Popen, harness.execute = observed_popen, observed_execute
    def restore():
        stop.set()
        if worker is not None:
            worker.join(timeout=30)
        harness.subprocess.Popen, harness.execute = original_popen, original_execute
        assert worker is None or not worker.is_alive(), 'Descriptor observer remains live after restoration'
    return proof, restore
