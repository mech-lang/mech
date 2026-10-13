"""Parent owns this one fork child before any debugger API can block."""
import fcntl
import json
import os
from pathlib import Path
import signal
import time


class SuspendedChild:
    def __init__(self, executable, arguments, environment, cwd, output, common_fd):
        self.executable = str(executable)
        self.arguments = list(arguments)
        self.environment = dict(environment)
        self.cwd = str(cwd)
        self.output = Path(output)
        self.common_fd = common_fd
        self.pid = None
        self.reaped = False
        self.final_wait_status = None
        self.promoted_fd = None

    def start(self):
        assert self.pid is None and not self.output.exists()
        self.output.mkdir()
        self.promoted_fd = fcntl.fcntl(self.common_fd, fcntl.F_DUPFD_CLOEXEC, 64)
        self.pid = os.fork()
        if self.pid == 0:
            try:
                os.setsid()
                os.chdir(self.cwd)
                for number, name, flags in ((0, '/dev/null', os.O_RDONLY),
                        (1, str(self.output / 'native.stdout'), os.O_WRONLY | os.O_CREAT | os.O_EXCL),
                        (2, str(self.output / 'native.stderr'), os.O_WRONLY | os.O_CREAT | os.O_EXCL)):
                    descriptor = os.open(name, flags, 0o600)
                    os.dup2(descriptor, number)
                    if descriptor != number:
                        os.close(descriptor)
                os.set_inheritable(self.promoted_fd, True)
                # No target executable instruction, initializer or Julia method
                # can run until the debugger resumes this exact owned child.
                os.kill(os.getpid(), signal.SIGSTOP)
                os.execve(self.executable, [self.executable, *self.arguments], self.environment)
            except BaseException as error:
                os.write(2, ('Owned pre-exec child failed: ' + repr(error) + '\n').encode())
                os._exit(127)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            waited, status = os.waitpid(self.pid, os.WNOHANG | os.WUNTRACED)
            if waited:
                if os.WIFEXITED(status) or os.WIFSIGNALED(status):
                    self.reaped = True
                    self.final_wait_status = status
                    raise RuntimeError('Owned child exited before the required pre-exec stop')
                assert os.WIFSTOPPED(status) and os.WSTOPSIG(status) == signal.SIGSTOP
                assert os.getpgid(self.pid) == self.pid
                record = dict(pid=self.pid, parent_pid=os.getpid(), pgid=self.pid,
                    initial_wait_status=status, stopped_before_exec=True,
                    target_executable=self.executable, arguments=self.arguments,
                    common_mutex_fd=self.promoted_fd, parent_retains_POSIX_child_ownership=True)
                with (self.output / 'parent-owned-before-exec.json').open('x') as stream:
                    json.dump(record, stream, indent=2)
                    stream.write('\n')
                return record
            time.sleep(0.01)
        raise RuntimeError('Owned child did not reach its bounded pre-exec stop')

    def close(self, group_members):
        record = dict(pid=self.pid, only_this_parent_owned_child=True,
            cleanup_signal_sent=False, reaped=self.reaped, final_wait_status=self.final_wait_status)
        try:
            if self.pid is not None and not self.reaped:
                # A live or unreaped direct fork child keeps its PID reserved;
                # do not infer ownership from a process name or stale PID.
                waited, status = os.waitpid(self.pid, os.WNOHANG)
                if waited:
                    self.reaped = True
                    self.final_wait_status = status
                else:
                    assert os.getpgid(self.pid) == self.pid
                    try:
                        os.killpg(self.pid, signal.SIGKILL)
                        record['cleanup_signal_sent'] = True
                    except ProcessLookupError:
                        pass
                    deadline = time.monotonic() + 5
                    while time.monotonic() < deadline:
                        waited, status = os.waitpid(self.pid, os.WNOHANG)
                        if waited:
                            self.reaped = True
                            self.final_wait_status = status
                            break
                        time.sleep(0.01)
                assert self.reaped, 'Exact owned child did not exit/reap within the cleanup bound'
            record.update(reaped=self.reaped, final_wait_status=self.final_wait_status,
                no_remaining_parent_owned_native=self.pid is None or self.reaped)
            members = group_members(self.pid) if self.pid is not None else []
            record['remaining_native_group_members'] = members
            assert not members, 'Owned native leader exited with remaining group members'
            return record
        finally:
            if self.promoted_fd is not None:
                os.close(self.promoted_fd)
                self.promoted_fd = None
