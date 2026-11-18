"""Bound actor-slot variation for the exact runtime artifacts in both orders."""

from contextlib import redirect_stdout
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys
from time import time_ns

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
OUT = HERE / 'fixed-worker-diagnostic'
RECIPE = Q / 'compare-library.py'
FIXTURES = Q / 'fixtures.json'
PEERS = Q / 'pinned-peers/pins.json'
FRAME = ROADMAP / 'frame-copy-v2/bound/worker-checks/shared-frame-copies'
SCORER = ROADMAP / 'scorer-vector-latch/latch-kept-root-layout/bound'
CASES = ('320-overlap', 'prefix-natural-4096', 'prefix-natural-8192',
         'prefix-periodic-1024', 'prefix-random-1024', 'random-1024')
ACTORS = ('mech-control', 'mech-experiment', 'roc', 'libdeflate')
BASE = '3cde720cdf8b24d6bc4f1818988479d8eb5ae960'
MODES = {'a-a': (FRAME, FRAME), 'a-b': (FRAME, SCORER)}


def read(path):
    return json.loads(path.read_bytes())


def sha(path):
    with Path(path).open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def record(path, data):
    with path.open('x') as out:
        json.dump(data, out, indent=2)
        out.write('\n')


def capture():
    thermal = subprocess.run(['/usr/bin/pmset', '-g', 'therm'], capture_output=True, text=True, check=False)
    return {'utc_ns': time_ns(), 'load_average': os.getloadavg(), 'thermal': thermal.stdout,
            'thermal_exit': thermal.returncode}


def pin(folder):
    linkage = read(folder / 'runtime.json')
    obj = folder / 'program.o'
    assert linkage['link'][linkage['linked_encoder']['argument_index']] == f'link-arg={obj}'
    proof = {'worker_sha256': sha(folder / 'mech-worker'), 'metadata_sha256': sha(folder / 'metadata.json'),
             'object_sha256': sha(obj), 'path': str(folder.relative_to(Q))}
    assert all(linkage[key] == value for key, value in proof.items() if key.endswith('sha256'))
    return proof


def main():
    assert tuple(sys.argv[1:]) == ()
    OUT.mkdir()
    worker_pins = {'frame-copy': pin(FRAME), 'bottom-tested': pin(SCORER)}
    peers = read(PEERS)
    assert sha(FIXTURES) == peers['input_inventory_sha256']
    assert set(CASES) <= {row['name'] for row in read(FIXTURES)}
    for actor in ('roc', 'libdeflate'):
        peer = peers['actors'][actor]
        assert sha(peer['command'][0]) == peer['binary_sha256']
    source = RECIPE.read_text()
    marker = "selected = {case['name'] for case in cases}"
    assert source.count(marker) == 1
    scoped = OUT / 'scoped-timer.py'
    scoped.write_text(source.replace(marker, f'selected = set({CASES!r})'))
    record(OUT / 'plan.json', {'base': BASE, 'cases': CASES, 'workers': worker_pins,
        'raw_timer_sha256': sha(RECIPE), 'scoped_timer_sha256': sha(scoped),
        'inventory_sha256': sha(FIXTURES), 'peer_pins_sha256': sha(PEERS),
        'orders': ['a-a-forward', 'a-b-forward', 'a-b-reverse', 'a-a-reverse'],
        'blocks_per_case': 24, 'slots': {k: [str(x.relative_to(Q)) for x in v] for k,v in MODES.items()},
        'interpretation': 'The duplicate frame-copy worker establishes slot drift. A/B repeats the exact prior scorer.'})
    rows = []
    for mode, order in (('a-a', 'forward'), ('a-b', 'forward'), ('a-b', 'reverse'), ('a-a', 'reverse')):
        run = f'short-visit-fixed-{mode}-{order}'
        assert not (Q / run).exists()
        before = capture()
        roles = MODES[mode]
        assert pin(roles[0]) == worker_pins['frame-copy']
        assert pin(roles[1]) == worker_pins['frame-copy' if mode == 'a-a' else 'bottom-tested']
        sys.argv = [str(RECIPE), run, *map(str, roles)] + (['reverse'] if order == 'reverse' else [])
        with (OUT / (mode + '-' + order + '.log')).open('x') as log, redirect_stdout(log):
            exec(compile(scoped.read_text(), str(scoped), 'exec'),
                 {'__name__': '__main__', '__file__': str(RECIPE)})
        after = capture()
        outcome = Q / run
        complete = read(outcome / 'complete.json')
        assert complete['cases'] == len(CASES) and complete['balanced_blocks_per_case'] == 24
        assert complete['all_outputs_independently_inflated']
        summary = {row['case']: row for row in read(outcome / 'summary.json')}
        assert set(summary) == set(CASES), set(summary)
        ratios = {name: values['paired_x_libdeflate'] for name, values in summary.items()}
        row = {'role': mode, 'order': order, 'run': run, 'before': before, 'after': after, 'ratios': ratios,
               'completion': complete, 'summary_sha256': sha(outcome / 'summary.json')}
        rows.append(row)
        print(mode, order, [(n, round(v['mech-control'], 6), round(v['mech-experiment'], 6))
                           for n, v in ratios.items() if n in ('320-overlap', 'prefix-natural-8192')], flush=True)
    record(OUT / 'summary.json', {'plan_sha256': sha(OUT / 'plan.json'), 'rows': rows})


if __name__ == '__main__':
    main()
