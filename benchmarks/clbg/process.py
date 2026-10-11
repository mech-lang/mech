"""Blocking child completion with a POSIX deadline, without timeout poll delays."""
import signal
import subprocess

class _Deadline(Exception):
    pass

def run_checked(argv, *, timeout, **kwargs):
    def expired(signum, frame):
        raise _Deadline()
    previous = signal.signal(signal.SIGALRM, expired)
    try:
        with subprocess.Popen(argv, **kwargs) as child:
            signal.setitimer(signal.ITIMER_REAL, timeout)
            try:
                status = child.wait()
            except _Deadline:
                child.kill()
                child.wait()
                raise subprocess.TimeoutExpired(argv, timeout) from None
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
            if status:
                raise subprocess.CalledProcessError(status, argv)
    finally:
        signal.signal(signal.SIGALRM, previous)
