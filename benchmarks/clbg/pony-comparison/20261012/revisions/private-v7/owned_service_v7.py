"""Additive bounded command/image service; does not mutate frozen v6 code."""
import os
from pathlib import Path
import signal
import subprocess
import time


def group_members(pgid):
    listing = subprocess.run(['/bin/ps', '-eo', 'pid,pgid,stat,comm'],
        capture_output=True, text=True, check=True, timeout=1).stdout
    rows = []
    for line in listing.splitlines()[1:]:
        fields = line.split(None, 3)
        if len(fields) == 4 and int(fields[1]) == pgid and not fields[2].startswith('Z'):
            rows.append(dict(pid=int(fields[0]), pgid=int(fields[1]),
                             state=fields[2], executable=Path(fields[3]).name))
    return rows


def kill_owned_group(pgid):
    try:
        os.killpg(pgid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    deadline = time.monotonic() + 5
    while True:
        remaining = group_members(pgid)
        if not remaining:
            return
        if time.monotonic() >= deadline:
            raise RuntimeError('Owned group remains after bounded cleanup: ' + repr(remaining))
        time.sleep(0.01)


def identity(pid):
    result = subprocess.run(['/bin/ps', '-p', str(pid), '-o', 'lstart=', '-o', 'command='],
        capture_output=True, text=True, timeout=5)
    return dict(pid=pid, process_listing=result.stdout.strip()) if result.returncode == 0 and result.stdout.strip() else None


def owned_command(preparation, guard, harness, command, environment, output, lock_fd, timeout, name):
    """File-backed output and bounded wait/cleanup; always persist this attempt."""
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / (name + '.json')
    assert not receipt.exists(), 'Never overwrite or repeat an attempt'
    detail = dict(command=command, started_at=guard.now(), inherited_common_mutex=True,
        timeout_seconds=timeout, preparation_only=True, never_chart_eligible=True,
        bounded_local_service=True, cleanup_errors=[])
    process = None
    cleanup_failed = False
    stdout_path = output / (name + '.stdout')
    stderr_path = output / (name + '.stderr')
    try:
        with stdout_path.open('xb') as stdout, stderr_path.open('xb') as stderr:
            try:
                process = subprocess.Popen(command, cwd=preparation.HERE, env=environment,
                    stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr,
                    start_new_session=True, pass_fds=(lock_fd,))
                detail['pid'] = process.pid
                detail['identity'] = identity(process.pid)
                if process.poll() is None:
                    assert os.getpgid(process.pid) == process.pid
                    assert detail['identity'] is not None, 'Owned live child identity missing'
                guard.save(output / (name + '.active.json'), detail)
                process.wait(timeout=timeout)
                detail.update(exit_status=process.returncode, finished_at=guard.now())
                assert process.returncode == 0, ('Owned command failed', name, process.returncode)
                assert not group_members(process.pid), 'Owned leader exited with a live child'
            except BaseException as error:
                detail.update(error=str(error) or type(error).__name__, error_type=type(error).__name__)
                raise
            finally:
                if process is not None:
                    try:
                        kill_owned_group(process.pid)
                    except BaseException as error:
                        cleanup_failed = True
                        detail['cleanup_errors'].append(dict(stage='group-kill',
                            error=str(error), error_type=type(error).__name__))
                    try:
                        process.wait(timeout=5)
                    except BaseException as error:
                        cleanup_failed = True
                        detail['cleanup_errors'].append(dict(stage='leader-wait',
                            error=str(error), error_type=type(error).__name__))
                        if process.poll() is None:
                            # Popen owns this unreaped leader; no discovered or
                            # foreign PID is guessed when group cleanup failed.
                            try:
                                process.kill()
                                process.wait(timeout=5)
                                detail['known_leader_fallback_reaped'] = True
                            except BaseException as fallback_error:
                                detail['cleanup_errors'].append(dict(stage='known-leader-fallback',
                                    error=str(fallback_error), error_type=type(fallback_error).__name__))
                    detail['final_exit_status'] = process.returncode
                    detail['leader_reaped'] = process.returncode is not None
                    try:
                        remaining = group_members(process.pid)
                        detail['remaining_group_members'] = remaining
                        if remaining:
                            cleanup_failed = True
                    except BaseException as error:
                        cleanup_failed = True
                        detail['cleanup_errors'].append(dict(stage='group-reconcile',
                            error=str(error), error_type=type(error).__name__))
                detail['cleanup_complete'] = process is not None and not cleanup_failed
    finally:
        detail['journal_finished_at'] = guard.now()
        preparation.new_json(receipt, detail)
        if cleanup_failed:
            raise RuntimeError('Owned cleanup failed; bounded attempt journal retained')
    return dict(stdout=stdout_path.read_bytes(), stderr=stderr_path.read_bytes(), detail=detail)


def native_images(preparation, guard, harness, binary, environment, output, lock_fd, allowed_roots):
    architecture = owned_command(preparation, guard, harness, ['/usr/bin/lipo', '-archs', str(binary)],
        environment, output, lock_fd, 30, 'architecture')['stdout'].decode().split()
    assert 'arm64' in architecture, 'Compiler/executable lacks a native arm64 slice'
    payload = owned_command(preparation, guard, harness, ['/usr/bin/otool', '-L', str(binary)],
        environment, output, lock_fd, 30, 'linked-images')['stdout'].decode()
    dependencies, pins = [], {}
    for line in payload.splitlines()[1:]:
        install = line.strip().split(' (', 1)[0]
        if install.startswith(('/usr/lib/', '/System/Library/')):
            dependencies.append(dict(install_name=install, scope='OS-image-not-hashed'))
            continue
        if install.startswith(('@loader_path/', '@executable_path/')):
            lookup = binary.parent / install.split('/', 1)[1]
        elif install.startswith('@rpath/'):
            suffix = install[len('@rpath/'):]
            candidates = [path for root in allowed_roots for path in root.rglob(suffix) if path.is_file()]
            assert len({path.resolve() for path in candidates}) == 1, ('Ambiguous/unresolved runtime image', install)
            lookup = candidates[0]
        else:
            lookup = Path(install)
        target = lookup.resolve(strict=True)
        assert any(target.is_relative_to(root.resolve()) for root in allowed_roots), ('Unreviewed runtime image', install)
        digest = preparation.sha(target)
        dependencies.append(dict(install_name=install, lookup_path=str(lookup), path=str(target), sha256=digest))
        pins[str(lookup)] = pins[str(target)] = digest
    preparation.new_json(output / 'dependency-review.json',
        dict(binary=str(binary), architectures=architecture, dependencies=dependencies))
    return pins
