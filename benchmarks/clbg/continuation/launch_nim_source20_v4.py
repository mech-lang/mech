"""Capture the completed failed terminal digest and enter the reviewed Nim repair."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
FAILED = CLBG / 'loc-normalization/20261012/nim-source-replay-v1/jobs/source20-repaired-v3'
TARGET = HERE / 'nim-source20-entry-repair-v2/replay.py'
OUTPUT = HERE / 'jobs/nim-source20-v4-reviewed-dependency.json'
with Path('/tmp/mech-clbg-timing.lock').open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    stopped = FAILED / 'stopped-report.json'
    with stopped.open('rb') as stream:
        assert hashlib.file_digest(stream, 'sha256').hexdigest() == 'edda317536087182ca63ca543d007feb7610c5105419aa63657f940f31516f31'
    terminal = FAILED / 'terminal-pins.json'
    with terminal.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    value = json.loads(terminal.read_text())
    assert value['status'] == 'failed-stop'
    assert value['pins'][str(stopped)] == 'edda317536087182ca63ca543d007feb7610c5105419aa63657f940f31516f31'
    with TARGET.open('rb') as stream:
        target_sha = hashlib.file_digest(stream, 'sha256').hexdigest()
    command = [sys.executable, '-B', '-u', str(TARGET), '--revision', 'source20-entry-repaired-v4',
        '--failed-terminal-sha256', digest]
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT.open('x') as stream:
        json.dump(dict(status='completed-failed-terminal-digest-capture',
            actual_failed_terminal=str(terminal), actual_failed_terminal_sha256=digest,
            target=str(TARGET), target_sha256=target_sha, command=command,
            common_mutex_held=True, new_fixture_runs=0, native_subjects=0), stream, indent=2)
        stream.write('\n')
# Close the capture's descriptor before the original runner reacquires it.
os.execv(sys.executable, command)
