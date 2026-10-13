"""Detach an already-reviewed job without automatic retry or qualification.

The command itself must own the common timing mutex. This launcher only owns a
private lease and records the exact process exit; exit zero is not admission.
A persisted identity and one-byte handoff precede command launch. Existing job
folders are never reused. Read request/start/terminal files instead of relying
on a foreground tool session across conversation turns.
"""
import argparse
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import traceback

HERE = Path(__file__).resolve().parent
JOBS = HERE / 'owned-jobs'


def now():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def identity(pid):
    result = subprocess.run(['ps', '-p', str(pid), '-o', 'pid=,ppid=,lstart=,command='],
                            capture_output=True, text=True, check=False)
    listing = result.stdout.strip()
    return dict(pid=pid, listing=listing) if listing else None


def worker(job, lease_fd, go_fd):
    signal.signal(signal.SIGHUP, signal.SIG_IGN)
    child = None
    requested_stop = []
    def stop(signum, frame):
        requested_stop.append(signum)
        if child is not None and child.poll() is None:
            # Only the direct command created by this worker is signalled.
            child.send_signal(signum)
        else:
            raise InterruptedError('Durable worker interrupted before command launch')
    for signum in (signal.SIGTERM, signal.SIGINT):
        signal.signal(signum, stop)
    terminal = dict(status='failed-before-command', started_at=now(),
                    worker_identity=identity(os.getpid()), actual_command_started=False,
                    exit_code=None, qualification_granted=False, automatic_retry=False)
    try:
        held, named = os.fstat(lease_fd), (job / 'owner.lock').stat()
        assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
        assert os.read(go_fd, 1) == b'1', 'Persisted launch was not authorized'
        os.close(go_fd)
        go_fd = None
        request = json.loads((job / 'request.json').read_text())
        command, cwd = request['command'], request['cwd']
        assert request['schema_version'] == 1
        assert isinstance(command, list) and command and all(isinstance(x, str) and x for x in command)
        assert Path(cwd).is_absolute() and Path(cwd).is_dir()
        assert not (job / 'terminal.json').exists()
        save(job / 'worker-started.json', dict(started_at=now(), worker_identity=identity(os.getpid()),
             inherited_private_lease=True, detached_session=True,
             common_mutex_owned_by_target_command=True, command=command, cwd=cwd,
             qualification_granted=False))
        with (job / 'command.stdout').open('xb') as out, (job / 'command.stderr').open('xb') as err:
            child = subprocess.Popen(command, cwd=cwd, stdin=subprocess.DEVNULL,
                                     stdout=out, stderr=err, pass_fds=(lease_fd,))
            observed_command = identity(child.pid)
            terminal.update(actual_command_started=True, command_pid=child.pid,
                            command_identity=observed_command,
                            command_identity_observation_missing=observed_command is None)
            save(job / 'command-started.json', dict(started_at=now(),
                 command_pid=child.pid, command_identity=terminal['command_identity'],
                 command_identity_observation_missing=observed_command is None, command=command, cwd=cwd))
            code = child.wait()
        terminal.update(exit_code=code,
                        status='command-exited-zero-awaiting-report-review' if code == 0 else 'command-failed')
    except BaseException as error:
        terminal.update(error_type=type(error).__name__, error=str(error) or type(error).__name__)
        try:
            with (job / 'worker-error.log').open('x') as stream:
                stream.write(traceback.format_exc())
        except BaseException as log_error:
            terminal['error_log_write_failure'] = dict(error_type=type(log_error).__name__,
                error=str(log_error) or type(log_error).__name__)
        # Cleanup always runs even if persisting the original error failed.
        # A bookkeeping failure must not abandon a command that we just started.
        if child is not None and child.poll() is None:
            cleanup = dict(direct_owned_pid=child.pid, descendant_cleanup_performed=False,
                           target_report_cleanup_requires_review=True)
            terminal['exception_cleanup'] = cleanup
            try:
                child.terminate()
                try:
                    terminal['exit_code'] = child.wait(timeout=10)
                    cleanup['terminated_and_reaped'] = True
                except subprocess.TimeoutExpired:
                    cleanup['term_timeout_seconds'] = 10
                    child.kill()  # Popen leader still unreaped; never a reused or unknown PID.
                    terminal['exit_code'] = child.wait(timeout=10)
                    cleanup['owned_leader_killed_and_reaped'] = True
            except BaseException as cleanup_error:
                cleanup.update(error_type=type(cleanup_error).__name__,
                               error=str(cleanup_error) or type(cleanup_error).__name__,
                               owned_leader_cleanup_unconfirmed=True)
    finally:
        terminal.update(finished_at=now(), stop_signals=requested_stop)
        save(job / 'terminal.json', terminal)
        if go_fd is not None:
            os.close(go_fd)
        os.close(lease_fd)
    return 0 if terminal['exit_code'] == 0 and not terminal.get('error') else 1


def launch(name, cwd, command):
    assert name and all(c.isalnum() or c in '-_' for c in name)
    assert Path(cwd).is_absolute() and Path(cwd).is_dir()
    assert command and all(isinstance(x, str) and x for x in command)
    JOBS.mkdir(exist_ok=True)
    job = JOBS / name
    job.mkdir()  # Refuse any previous launch, including failed or interrupted jobs.
    save(job / 'request.json', dict(schema_version=1, created_at=now(), command=command,
         cwd=cwd, common_mutex_owned_by_target_command=True,
         qualification_granted=False, automatic_retry=False))
    with (job / 'owner.lock').open('a+') as lease:
        fcntl.flock(lease, fcntl.LOCK_EX | fcntl.LOCK_NB)
        go_read, go_write = os.pipe()
        try:
            with (job / 'worker.log').open('xb') as log:
                process = subprocess.Popen([sys.executable, '-B', '-u', str(Path(__file__).resolve()),
                    '--worker', str(job), '--lease-fd', str(lease.fileno()), '--go-fd', str(go_read)],
                    stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                    start_new_session=True, pass_fds=(lease.fileno(), go_read))
            os.close(go_read)
            go_read = None
            observed = identity(process.pid)
            assert observed is not None, 'Detached worker exited before handoff'
            record = dict(status='durable-job-launched', launched_at=now(), job=str(job),
                          worker_identity=observed, command=command, cwd=cwd,
                          detached_session=True, automatic_retry=False, qualification_granted=False)
            save(job / 'launch.json', record)
            assert os.write(go_write, b'1') == 1
            print(json.dumps(record), flush=True)
        finally:
            if go_read is not None:
                os.close(go_read)
            os.close(go_write)
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--job')
    parser.add_argument('--cwd')
    parser.add_argument('--command-json')
    parser.add_argument('--worker', type=Path)
    parser.add_argument('--lease-fd', type=int)
    parser.add_argument('--go-fd', type=int)
    args = parser.parse_args()
    if args.worker:
        assert args.worker.parent == JOBS and args.lease_fd is not None and args.go_fd is not None
        return worker(args.worker, args.lease_fd, args.go_fd)
    assert args.job and args.cwd and args.command_json
    return launch(args.job, args.cwd, json.loads(args.command_json))


if __name__ == '__main__':
    sys.exit(main())
