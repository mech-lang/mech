"""Build an adapter envelope from immutable formatter evidence under common FD.

No source formatting, application execution, imports of candidate modules or
dataset writes. Call from a separately reviewed integration controller.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path

LOCK = Path('/tmp/mech-clbg-timing.lock')


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def load(qualification_path, qualification_sha256, terminal_path, terminal_sha256, common_fd):
    # Reconcile the exact caller descriptor; no separately opened nested lock.
    # Reasserting the same open file description is nonblocking and leaves it
    # held for the caller's entire integration transaction.
    actual, named = os.fstat(common_fd), LOCK.stat()
    require((actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino), 'Wrong caller common mutex')
    fcntl.flock(common_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    qualification_path, terminal_path = Path(qualification_path).resolve(), Path(terminal_path).resolve()
    require(sha(qualification_path) == qualification_sha256 and sha(terminal_path) == terminal_sha256,
            'Root-reviewed formatter evidence changed')
    q = json.loads(qualification_path.read_bytes())
    terminal = json.loads(terminal_path.read_bytes())
    require(q['status'] == terminal['status'] == 'completed-formatted-loc-measurement', 'Formatter stage did not complete')
    require(terminal['body_error'] is None and terminal['integrity_errors'] == [] and
            terminal['available_bytes_not_fully_pinned'] is False and terminal['inherited_common_mutex'] is True and
            terminal['chart_admission'] is False and type(terminal['actual_application_processes']) is int and
            terminal['actual_application_processes'] == 0, 'Formatter terminal closure failed')
    require(terminal['qualification'] == str(qualification_path) and
            terminal['qualification_sha256'] == qualification_sha256 and terminal['verified_pins'] == q['pins'],
            'Terminal does not bind exact qualification bytes/pins')
    for path, digest in q['pins'].items():
        require(sha(path) == digest, 'Formatter source/output/receipt changed: ' + path)
    q['artifact'] = dict(path=str(qualification_path), sha256=qualification_sha256)
    q['terminal_artifact'] = dict(path=str(terminal_path), sha256=terminal_sha256)
    q['terminal'] = terminal
    q['receipts'] = {}
    for path in q['pins']:
        if Path(path).suffix == '.json':
            q['receipts'][path] = json.loads(Path(path).read_bytes())
    # Retain input receipt closure exactly. The saved qualification is never
    # rewritten and the terminal does not pretend to self-pin its own bytes.
    q['provenance'] = {str(qualification_path): qualification_sha256, str(terminal_path): terminal_sha256}
    return q
