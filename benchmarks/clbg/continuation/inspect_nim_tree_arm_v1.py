"""One saved-evidence read under the common mutex; no native execution."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent / 'nim-comparison/20261012'
PRIOR = ROOT / 'work-qualification/work-v3'
OUTPUT = HERE / 'jobs/nim-tree-arm-inspection-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')


def sha(value): return hashlib.sha256(value).hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2); stream.write('\n')


print('WAIT common mutex: saved Nim compact tree ARM, PID ' + str(os.getpid()), flush=True)
with LOCK.open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    named, opened = LOCK.stat(), os.fstat(lock.fileno())
    assert (named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino)
    assert not OUTPUT.exists()
    OUTPUT.mkdir(parents=True)
    pins = {}
    def read(path):
        assert path.stat().st_size <= 1024 * 1024, 'Bounded saved evidence read only'
        value = path.read_bytes(); pins[str(path)] = sha(value)
        return value.decode()
    report = json.loads(read(PRIOR / 'report.json'))
    assert report['status'] == 'stopped-unqualified' and report['attempts'] == [] and report['tree_proofs'] == []
    assert report['error'] == 'Final ARM path must allocate before any conditional/unconditional control or return'
    directory = PRIOR / 'nim-binarytrees-compact'
    generated = read(directory / 'scored-creator.c')
    names = set(re.findall(r'\b(make__\w+)\(', generated))
    assert len(names) == 1
    name = names.pop()
    text = read(directory / 'scored-binary/disassembly.stdout')
    receipt = json.loads(read(directory / 'scored-binary/disassembly.json'))
    assert receipt['exit_status'] == receipt['final_exit_status'] == 0 and receipt['inherited_common_mutex'] is True
    assert receipt['command'][0:2] == ['/usr/bin/otool', '-tvV']
    marker = '_' + name + ':'
    assert text.count(marker) == 1
    after = text.split(marker, 1)[1]
    boundary = re.search(r'(?m)^_[A-Za-z][A-Za-z0-9_$]*:\s*$', after)
    body = after[:boundary.start()] if boundary else after
    target = OUTPUT / 'compact-scored-creator.arm'
    with target.open('x') as stream: stream.write(marker + body)
    pins[str(target)] = sha(target.read_bytes())
    pins[str(Path(__file__).resolve())] = sha(Path(__file__).read_bytes())
    result = dict(status='saved-evidence-extracted-awaiting-path-review', creator_symbol=name,
        prior_failed_report=str(PRIOR / 'report.json'), disassembly_receipt=receipt,
        creator=str(target), creator_bytes=target.stat().st_size, pins=pins,
        actual_pid=os.getpid(), exclusive_common_mutex_held=True,
        native_processes=0, benchmark_replays=0, source_edits=0, allocation_gate_changed=False,
        completed_at=datetime.now(timezone.utc).isoformat())
    save(OUTPUT / 'inspection.json', result)
    print(json.dumps(dict(status=result['status'], creator=str(target), creator_symbol=name, bytes=result['creator_bytes'])), flush=True)
    print(marker + body, flush=True)
