"""Measure the isolated parser with exact tree, work and diagnostics checks."""

import json
from pathlib import Path
from statistics import median
import subprocess
import sys

from stage import BASE, CHECKOUT, HERE, OUT, REFERENCE, ROOT, sha

CONTROL = REFERENCE / 'candidate'
TARGET = 'benchmarks/mech-deflate/native-profile/native-compact-20261116/program.mec'
RESULTS = HERE / 'standalone'
ROLES = ('control', 'ascii-miss')


def read(path):
    return json.loads(path.read_bytes())


def save(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def main():
    subprocess.run(['git', 'diff', '--quiet', 'f96c1d4fdb', BASE, '--', 'src/syntax'], cwd=ROOT, check=True)
    source = CHECKOUT / TARGET
    expected = read(REFERENCE / 'measured/comparison.json')['exact_parse_report']
    assert expected['source_sha256'] == sha(source)
    paths = dict(zip(ROLES, (CONTROL, OUT), strict=True))
    binaries = {}
    for role, path in paths.items():
        binary = path / 'parser'
        identity = read(path / 'build.json')
        assert sha(binary) == identity['binary_sha256']
        binaries[role] = binary
    assert '0 failed' in read(OUT / 'tests.json')['summary']
    RESULTS.mkdir()
    order = [ROLES, tuple(reversed(ROLES)), tuple(reversed(ROLES)), ROLES] * 2
    plan = {'base': BASE, 'source_sha256': sha(source), 'input': str(source),
            'orders': order, 'binaries': {role: {'path': str(p), 'sha256': sha(p)} for role, p in binaries.items()},
            'report_contract': expected, 'boundary': 'fresh-process standalone strict parser interval'}
    save(RESULTS / 'plan.json', plan)

    def trial(role, label):
        record = RESULTS / f'{label}-{role}.json'
        with (RESULTS / f'{label}-{role}.log').open('wb') as log:
            subprocess.run([binaries[role], source, record], stdout=log,
                           stderr=subprocess.STDOUT, check=True)
        measured = read(record)
        elapsed = measured.pop('parse_seconds')
        assert measured == expected, (role, measured, expected)
        return elapsed

    warmup = {name: trial(name, 'warmup') for name in ROLES}
    trials = [{name: trial(name, f'order-{i}') for name in roles}
              for i, roles in enumerate(order, 1)]
    result = {'plan_sha256': sha(RESULTS / 'plan.json'), 'warmup': warmup, 'trials': trials,
              'median_seconds': {name: median(row[name] for row in trials) for name in ROLES},
              'candidate_x': median(row[ROLES[1]] / row[ROLES[0]] for row in trials),
              'paired_save_ms': 1000 * median(row[ROLES[0]] - row[ROLES[1]] for row in trials)}
    save(RESULTS / 'result.json', result)
    print(result, flush=True)


if __name__ == '__main__':
    main()
