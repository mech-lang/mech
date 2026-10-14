"""Load exact formatter receipt classes while retaining every source byte pin.

The frozen overlay's receipt_binding calls identify these receipt references.
A filename extension does not classify an upstream compiler input as a receipt.
No native commands, source rewriting, dataset writes or candidate imports.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re

LOCK = Path('/tmp/mech-clbg-timing.lock')


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def receipt_paths(q):
    """Select only actual schema references consumed by frozen overlay.py."""
    selected = {}

    def add(record, label):
        require(type(record) is dict, 'Missing required receipt descriptor: ' + label)
        path, digest = record.get('path'), record.get('sha256')
        require(type(path) is str and bool(path.strip()) and Path(path).is_absolute() and
                '..' not in Path(path).parts and type(digest) is str and
                re.fullmatch('[a-f0-9]{64}', digest) is not None and q['pins'].get(path) == digest,
                'Required receipt is not exactly pinned: ' + label)
        require(path not in selected or selected[path] == digest, 'Conflicting receipt references: ' + path)
        selected[path] = digest

    require(type(q.get('rows')) is dict and bool(q['rows']), 'Formatter profile rows required')
    for key, row in q['rows'].items():
        require(type(row) is dict and row.get('key') == key, 'Malformed formatter profile')
        inventory = row.get('compiler_inventory')
        if inventory is not None:
            require(type(inventory) is dict, 'Malformed compiler inventory descriptor')
            add(dict(path=inventory.get('evidence_path'), sha256=inventory.get('evidence_sha256')),
                key + ' compiler inventory')
        measurement = row.get('measurement_evidence')
        if measurement is not None:
            add(measurement, key + ' measured execution')
        files = row.get('files')
        require(type(files) is list and bool(files), 'Whole authored file bundle required')
        for item in files:
            require(type(item) is dict, 'Malformed formatter file')
            if item.get('status') == 'qualified':
                formatter, counter = item.get('formatter_evidence'), item.get('counter_evidence')
                require(type(formatter) is dict and type(counter) is dict, 'Missing native formatter/count evidence')
                add(formatter.get('version_receipt'), key + ' formatter version')
                add(formatter.get('receipt'), key + ' formatter evidence')
                add(counter.get('receipt'), key + ' physical count')
    require(bool(selected), 'No actual receipt references supplied')
    return selected


def load(qualification_path, qualification_sha256, terminal_path, terminal_sha256, common_fd):
    # Reassert the caller's same descriptor without opening a nested mutex.
    actual, named = os.fstat(common_fd), LOCK.stat()
    require((actual.st_dev, actual.st_ino) == (named.st_dev, named.st_ino) and actual.st_ino == 79859134,
            'Wrong caller common mutex')
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
    require(type(q['pins']) is dict and bool(q['pins']), 'Complete original pins required')
    for path, digest in q['pins'].items():
        require(type(path) is str and Path(path).is_absolute() and '..' not in Path(path).parts and
                type(digest) is str and re.fullmatch('[a-f0-9]{64}', digest) is not None and sha(path) == digest,
                'Formatter source/output/receipt changed: ' + str(path))
    selected = receipt_paths(q)
    q['artifact'] = dict(path=str(qualification_path), sha256=qualification_sha256)
    q['terminal_artifact'] = dict(path=str(terminal_path), sha256=terminal_sha256)
    q['terminal'] = terminal
    # No parse-error suppression. Every required receipt must decode normally.
    q['receipts'] = {path: json.loads(Path(path).read_bytes()) for path in selected}
    q['provenance'] = {str(qualification_path): qualification_sha256, str(terminal_path): terminal_sha256}
    return q
