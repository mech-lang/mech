"""New-controller-only same-description lock promotion and native observation."""
import fcntl
import os
from pathlib import Path
import threading


class HighDescriptorLock:
    def __init__(self, path):
        self.path = Path(path)

    def __str__(self):
        return str(self.path)

    def open(self, mode):
        assert mode == 'a+'
        duplicate = None
        with self.path.open(mode) as original:
            try:
                duplicate = fcntl.fcntl(original.fileno(), fcntl.F_DUPFD_CLOEXEC, 64)
                expected, observed = os.fstat(original.fileno()), os.fstat(duplicate)
                path = self.path.stat()
                assert duplicate >= 64
                assert (observed.st_dev, observed.st_ino) == (expected.st_dev, expected.st_ino) == (path.st_dev, path.st_ino)
                result = os.fdopen(duplicate, mode)
                duplicate = None
                return result
            finally:
                if duplicate is not None:
                    os.close(duplicate)
        # The low alias always closes; the returned high descriptor is the sole
        # controller alias of that same open description, with no lock keeper.


def reconcile(payload, pid, descriptor, state, path):
    fields = payload.decode().splitlines()
    assert [field for field in fields if field.startswith('p')] == ['p' + str(pid)]
    assert [field for field in fields if field.startswith('f')] == ['f' + str(descriptor)]
    devices = [int(field[1:], 16) for field in fields if field.startswith('D')]
    inodes = [int(field[1:]) for field in fields if field.startswith('i')]
    names = [Path(field[1:]).resolve() for field in fields if field.startswith('n')]
    assert devices == [state.st_dev] and inodes == [state.st_ino]
    assert names == [Path(path).resolve()]
    return dict(status='passed', native_pid=pid, descriptor=descriptor,
                device=state.st_dev, inode=state.st_ino, path=str(Path(path).resolve()))


class NativeObservation:
    def __init__(self, guard, helper, harness, report, row, n, lock_fd, output, path):
        self.stop = threading.Event()
        self.proof = None
        self.error = None
        self.state = os.fstat(lock_fd)
        assert lock_fd >= 64
        expected = [part.replace('{n}', str(n)) for part in row['command']]
        runtime = row['binary']

        def observe():
            try:
                while not self.stop.wait(0.005):
                    native = (report.get('active') or {}).get('native')
                    if native is None:
                        continue
                    current = guard.identity(native['pid'])
                    if current is None or runtime not in current['process_listing']:
                        continue  # Only actual GNU APL, after the shell exec.
                    assert native['command'] == expected and native['pid'] == native['pgid']
                    assert native['inherited_common_mutex'] is True
                    command = ['/usr/sbin/lsof', '-nP', '-a', '-p', str(native['pid']),
                               '-d', str(lock_fd), '-FpfDin']
                    result = helper.owned_command(guard, harness, command, dict(os.environ), output,
                                                  lock_fd, 5, 'actual-native-lock')
                    self.proof = dict(reconcile(result['stdout'], native['pid'], lock_fd, self.state, path),
                        runtime_identity=current, actual_command=expected,
                        observation_receipt=str(output / 'actual-native-lock.json'),
                        observation_stdout=str(output / 'actual-native-lock.stdout'),
                        native_after_launcher_exec=True, same_open_description_promoted=True,
                        preparation_only=True, never_chart_eligible=True)
                    return
            except BaseException as error:
                self.error = error

        self.thread = threading.Thread(target=observe, name='actual-apl-lock-observer')
        self.thread.start()

    def finish(self):
        self.stop.set()
        self.thread.join(timeout=12)
        assert not self.thread.is_alive(), 'Owned descriptor observer did not finish cleanup'
        if self.error is not None:
            raise self.error
        assert self.proof is not None, 'Actual largest bounded GNU APL lock was not observed'
        return self.proof
