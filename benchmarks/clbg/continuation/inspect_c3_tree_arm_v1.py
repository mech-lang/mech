"""Bounded saved scored-creator evidence read under the common mutex."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
PRIOR = HERE.parent / 'c3-comparison/20261012/work-qualification/work-v2'
OUTPUT = HERE / 'jobs/c3-tree-arm-inspection-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')
print('WAIT common mutex: saved C3 tree ARM, PID ' + str(os.getpid()), flush=True)
with LOCK.open('a+') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    named, opened = LOCK.stat(), os.fstat(lock.fileno())
    assert (named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino)
    assert not OUTPUT.exists(); OUTPUT.mkdir(parents=True)
    pins = {}
    def read(path):
        assert path.stat().st_size < 4 * 1024 * 1024
        value = path.read_bytes(); pins[str(path)] = hashlib.sha256(value).hexdigest()
        return value.decode()
    report = json.loads(read(PRIOR / 'report.json'))
    assert report['status'] == 'stopped-unqualified' and report['error'] == 'Final native scored creator lacks actual malloc/calloc'
    assert len(report['attempts']) == len(report['proofs']) == 38
    directory = PRIOR / 'scored-allocation/c3-binarytrees-compact'
    receipt = json.loads(read(directory / 'final-scored-arm.json'))
    assert receipt['command'][0:2] == ['/usr/bin/otool', '-tvV']
    assert type(receipt['exit_status']) is type(receipt['final_exit_status']) is int and receipt['exit_status'] == receipt['final_exit_status'] == 0
    assert receipt['inherited_common_mutex'] is True
    text = read(directory / 'final-scored-arm.stdout')
    labels = list(re.finditer(r'^([^\n]+):\n', text, re.M))
    creators = []
    for index, start in enumerate(labels):
        if 'binarytrees' not in start[1] or not start[1].endswith('make'): continue
        end = labels[index + 1].start() if index + 1 < len(labels) else len(text)
        body = text[start.start():end]
        target = OUTPUT / ('creator-' + str(len(creators)) + '.arm')
        with target.open('x') as stream: stream.write(body)
        pins[str(target)] = hashlib.sha256(target.read_bytes()).hexdigest()
        creators.append(dict(symbol=start[1], path=str(target)))
        print(body, flush=True)
    assert len(creators) == 1, creators
    result = dict(status='saved-evidence-extracted-awaiting-call-review', creators=creators,
        actual_saved_proof_count=38, native_processes=0, source_edits=0, allocation_gate_changed=False,
        exclusive_common_mutex_held=True, actual_pid=os.getpid(), pins=pins)
    with (OUTPUT / 'inspection.json').open('x') as stream: json.dump(result, stream, indent=2); stream.write('\n')
    print(json.dumps(dict(status=result['status'], creators=creators, actual_saved_proof_count=38)), flush=True)
