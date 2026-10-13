"""Bind the reviewed frozen ready packet, then enter its one fixture/build job."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
EXPECTED = {
    'status': 'reviewed-completed-native-language-subset',
    'ready_languages': ['crystal'], 'future_requested_languages': ['elixir', 'kotlin'], 'unresolved': [],
    'languages': {'crystal': {
        'suite': {'path': str(CLBG/'crystal-comparison/20261012/reconciliation/full-v1-suite-v2.json'),
            'sha256': '386356315c9fc5c680e99077fa70fe52db130c369fe74f7e0aa301e5f7ea6490'},
        'canonical_loc': {'path': str(CLBG/'crystal-comparison/20261012/source-counts/corrected-qualified-overlay-v2/qualified-loc-overlay.json'),
            'sha256': 'e22140622a5154a1e3b9512580eb469c8c078beda4a2ae9812bf429e51e089e2'},
        'metadata': {'path': str(HERE/'publication-crystal-metadata-v2/qualified/actual-metadata-v2/metadata.json'),
            'sha256': '63b93da6f941ae4ea276d6e595d29b438ba64e315b29f461874a0edbedfa1a1f'}}}}

print('WAIT common mutex: bind reviewed current Crystal chart packet', flush=True)
with LOCK.open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    held, named = os.fstat(lock.fileno()), LOCK.stat()
    assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
    packet = HERE/'crystal-ready-publication-v1.json'
    assert not packet.stat().st_mode & 0o222
    raw = packet.read_bytes()
    assert json.loads(raw) == EXPECTED, 'Reviewed packet descriptors changed'
    digest = hashlib.sha256(raw).hexdigest()
    script = HERE/'qualify_native_ready_v1.py'
    assert not script.stat().st_mode & 0o222
    output = HERE/'jobs/native-ready-v1'
    assert not output.exists(), 'Do not retry or overwrite this exact qualification/build'
    argv = [PYTHON, '-E', '-S', '-B', '-u', str(script), '--output', str(output),
        '--packet', str(packet), '--packet-sha256', digest,
        '--input-data', str(CLBG/'overview/explorer/dist/data.json')]
    journal = HERE/'jobs/crystal-ready-current-entry-v1'
    journal.mkdir(parents=True, exist_ok=False)
    (journal/'handoff.json').write_text(json.dumps(dict(status='entering-reviewed-fixture-and-candidate-build',
        command=argv, packet_sha256=digest, common_mutex_held=True,
        native_processes=0, chart_admission=False, automatic_retry=False), indent=2)+'\n')
# The same durable PID/lease enters the original worker, which reacquires and
# independently checks the exact packet and all source/runtime prerequisites.
os.execv(PYTHON, argv)
