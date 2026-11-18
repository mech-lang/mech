"""Sample warm compression by the pinned workers without measuring peer ratios."""

from collections import Counter
from hashlib import file_digest
import json
import os
from pathlib import Path
import re
from statistics import median
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
GROUP = 'prefix-natural-4096'
REFERENCE = Q / 'current-roadmap/frame-copy-v2/bound/worker-checks/shared-frame-copies'
CANDIDATE = HERE / 'latch-kept-root-layout/bound'
PEERS = Q / 'pinned-peers/pins.json'
INVENTORY = Q / 'fixtures.json'
OUTPUT = HERE / 'runtime-sample'
FRAME = re.compile(r"^([ !:+|]+?)(\d+) (.+?)  \(in [^)]+\)")


def read(path):
    return json.loads(path.read_bytes())


def sha(path):
    with Path(path).open('rb') as src:
        return file_digest(src, 'sha256').hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def bracket(names, role):
    if role == 'libdeflate':
        if any('deflate_compress' in name and 'decompress' not in name for name in names):
            return 'libdeflate compression'
    else:
        if any('mech_compact_' in name for name in names):
            if any(re.search(r'mech_compact_numeric_3009(?!\d)', name) for name in names):
                return 'ordered scorer'
            if any(re.search(r'mech_compact_numeric_475(?!\d)', name) for name in names):
                return 'match collection'
            if any(re.search(r'mech_compact_numeric_\d+', name) for name in names):
                return 'other numeric phase'
            return 'checked encoder and runtime'
    if any('decompress' in name or 'miniz_oxide' in name for name in names):
        return 'independent validation'
    return 'other worker and idle'


def classify(location, role):
    stack = []
    counts = Counter()
    leaves = Counter()

    def close_frame():
        depth, count, name, children = stack[-1]
        assert count >= children, (depth, name, count, children)
        if count > children:
            category = bracket([frame[2] for frame in stack], role)
            counts[category] += count - children
            leaves[category, name] += count - children
        stack.pop()

    active = False
    for line in location.read_text().splitlines():
        if line == 'Call graph:':
            active = True
        if line.startswith(('Total number in stack', 'Sort by top of stack', 'Binary Images:')):
            break
        match = FRAME.match(line) if active else None
        if match is None:
            continue
        depth = len(match[1])
        while stack and stack[-1][0] >= depth:
            close_frame()
        count = int(match[2])
        if stack:
            stack[-1][3] += count
        stack.append([depth, count, match[3], 0])
    while stack:
        close_frame()
    assert counts and any(value > 0 for key, value in counts.items()
                          if key in ('libdeflate compression', 'ordered scorer', 'match collection'))
    return {'sample_counts': dict(counts), 'leaves': [[*key, n] for key, n in leaves.most_common(30)]}


def acquire(args, label, role, case, index):
    stderr = OUTPUT / (label + '-worker.stderr')
    raw = OUTPUT / (label + '.txt')
    with stderr.open('wb') as errors:
        worker = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors)
        try:
            def send(text):
                worker.stdin.write(text.encode())
                worker.stdin.flush()

            def line():
                return worker.stdout.readline().decode().strip()

            assert line() == 'SESSION_READY'
            send(f'C\t{index}\t0\n')
            assert line() == 'CASE_READY\t' + case['name']
            send('W')
            assert line().startswith('WARMUP_DONE\t' + case['name'] + '\t')
            sampler = ['/usr/bin/sample', str(worker.pid), '3', '1', '-file', str(raw)]
            log = OUTPUT / (label + '-sample.log')
            with log.open('wb') as sr:
                sampling = subprocess.Popen(sampler, stdout=sr, stderr=subprocess.STDOUT)
                assert sampling.pid != worker.pid
                send('W' * 25)
                warmups = [line() for _ in range(25)]
                assert all(value.startswith('WARMUP_DONE\t' + case['name'] + '\t') for value in warmups)
                assert sampling.wait(timeout=12) == 0, log.read_text()
            send('NN')
            outcomes = [json.loads(line()) for _ in range(2)]
            assert all(row['case'] == case['name'] and row['independent_inflation']
                       and row['input_alias_unchanged'] and row['borrowed_output_alias_unchanged'] for row in outcomes)
            assert line() == 'CASE_DONE\t' + case['name']
            send('Q\n')
            assert worker.wait(timeout=10) == 0
        finally:
            if worker.poll() is None:
                worker.terminate()
                worker.wait(timeout=10)
    return {'role': role, 'raw_sha256': sha(raw), 'warmup_count': len(warmups),
            'checks': outcomes, **classify(raw, role)}


def main():
    assert sys.argv[1:] == []
    pins = read(PEERS)
    assert sha(INVENTORY) == pins['input_inventory_sha256']
    lib = pins['actors']['libdeflate']
    actors = {'libdeflate': lib['command']}
    assert sha(lib['command'][0]) == lib['binary_sha256']
    for role, root in (('current', REFERENCE), ('bottom-tested', CANDIDATE)):
        file = root / 'mech-worker'
        receipt = read(root / 'runtime.json')
        assert sha(file) == receipt['worker_sha256']
        metadata = root / 'metadata.json'
        assert sha(metadata) == receipt['metadata_sha256']
        actors[role] = list(map(str, (file, metadata, INVENTORY)))
    cases = read(INVENTORY)
    (index, case), = [(i, row) for i, row in enumerate(cases) if row['name'] == GROUP]
    assert sha(case['input']) == case['input_sha256']
    OUTPUT.mkdir()
    schedule = list(actors) + list(reversed(actors))
    write(OUTPUT / 'plan.json', {'source_case': case, 'schedule': schedule, 'sample_interval_ms': 1,
                                 'sample_duration_seconds': 3, 'actors': actors,
                                 'worker_sha256': {key: sha(values[0]) for key, values in actors.items()},
                                 'fixture_inventory_sha256': sha(INVENTORY), 'untimed': True})
    results = [acquire(actors[role], 'round-' + str(index + 1), role, case, cases.index(case))
               for index, role in enumerate(schedule)]
    medians = {}
    for role in actors:
        values = [row['sample_counts'] for row in results if row['role'] == role]
        keys = sorted(set().union(*values))
        medians[role] = {name: median(entry.get(name, 0) for entry in values) for name in keys}
    write(OUTPUT / 'result.json', {'plan_sha256': sha(OUTPUT / 'plan.json'), 'captures': results,
                                   'median_samples': medians, 'untimed': True})
    print(medians, flush=True)


if __name__ == '__main__':
    main()
