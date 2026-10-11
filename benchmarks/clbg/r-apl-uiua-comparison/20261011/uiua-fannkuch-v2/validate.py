#!/usr/bin/env python3
"""Locked bounded output, permutation and exact prefix-reversal audit; never n=12! work."""
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import random
import subprocess

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
RESULTS = HERE / 'results'
VARIANTS = ('compact', 'fast')
CASES = (1, 2, 3, 4, 5, 6, 7, 8)
ENVIRONMENT = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')
BASE = json.loads((SUITE / 'uiua-manifest.json').read_text())
RUNTIME = Path(BASE['runtime']['path'])


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def permutations(n):
    """Independent incremental canonical-order enumeration, not factoradic unranking."""
    p = list(range(n))
    count = [0] * n
    r = n
    while True:
        while r != 1:
            count[r-1] = r
            r -= 1
        yield p.copy()
        while True:
            if r == n:
                return
            p[:r+1] = p[1:r+1] + p[:1]
            count[r] -= 1
            if count[r] > 0:
                break
            r += 1


def flip_count(p):
    p = p.copy()
    result = 0
    while p[0]:
        p[:p[0]+1] = reversed(p[:p[0]+1])
        result += 1
    return result


def reference(n):
    ps = list(permutations(n))
    assert len(ps) == math.factorial(n)
    counts = [flip_count(p) for p in ps]
    checksum = sum(c * (-1 if rank % 2 else 1) for rank, c in enumerate(counts))
    return ps, counts, f'{checksum}\nPfannkuchen({n}) = {max(counts)}\n'.encode()


def execute(source, n):
    command = [str(RUNTIME), 'run', '--no-format', str(source), str(n)]
    done = subprocess.run(command, cwd=SUITE, env=dict(os.environ, **ENVIRONMENT), capture_output=True, timeout=120)
    return command, done


def instrument(source):
    lines = []
    for line in source.read_text().splitlines():
        if line.startswith('Flips    ←'):
            lines.extend(['Flips ← (', '  &p ⊂ "INITIAL=" json .',
                          '  ' + line.split('←', 1)[1].strip(), '  &p ⊂ "COUNTS=" json .', ')'])
        elif line.startswith('FlipStep ←'):
            lines.extend(['FlipStep ← (', '  ' + line.split('←', 1)[1].strip(),
                          '  &p ⊂ "STEP=" json .', ')'])
        else:
            lines.append(line)
    return '\n'.join(lines) + '\n'


def validate_trace(done, n, expected_ps, expected_counts, expected_output):
    assert done.returncode == 0, done.stderr.decode()
    visited = []
    final_counts = []
    previous = None
    counts = None
    blocks = []
    steps = 0
    flips = 0
    canonical_output = []
    for line in done.stdout.decode().splitlines(keepends=True):
        if line.startswith('INITIAL='):
            assert previous is None
            previous = json.loads(line[len('INITIAL='):])
            counts = [0] * len(previous)
            assert 0 < len(previous) <= 1024
            assert all(len(p) == n and sorted(p) == list(range(n)) for p in previous)
            visited.extend(previous)
            blocks.append(len(previous))
        elif line.startswith('STEP='):
            assert previous is not None
            current, observed_counts = json.loads(line[len('STEP='):])
            assert len(current) == len(previous)
            expected_next = []
            for i, p in enumerate(previous):
                q = p.copy()
                if p[0] != 0:
                    q[:p[0]+1] = reversed(p[:p[0]+1])
                    counts[i] += 1
                    flips += 1
                expected_next.append(q)
            assert current == expected_next
            assert observed_counts == counts
            previous = current
            steps += 1
        elif line.startswith('COUNTS='):
            assert previous is not None and all(p[0] == 0 for p in previous)
            observed_counts = json.loads(line[len('COUNTS='):])
            assert observed_counts == counts
            final_counts.extend(observed_counts)
            previous = None
            counts = None
        else:
            canonical_output.append(line)
    assert previous is None
    assert visited == expected_ps, 'Every rank must occur once in exact canonical order.'
    assert final_counts == expected_counts
    assert ''.join(canonical_output).encode() == expected_output
    assert flips == sum(expected_counts)
    expected_blocks = [min(1024, math.factorial(n)-start) for start in range(0, math.factorial(n), 1024)]
    assert blocks == expected_blocks
    return dict(permutations=len(visited), unique_permutations=len({tuple(p) for p in visited}),
        canonical_rank_order=True, exact_permutation_rows=True, every_prefix_reversal_verified=True,
        actual_prefix_reversals=flips, expected_prefix_reversals=sum(expected_counts),
        block_sizes=blocks, block_step_calls=steps, all_finished_lanes_unchanged=True,
        all_counts_equal_independent_scalar=True, reference_match=True)


def persist(source, n, command, done, stem):
    out = RESULTS / (stem + '.stdout')
    err = RESULTS / (stem + '.stderr')
    out.write_bytes(done.stdout)
    err.write_bytes(done.stderr)
    return dict(source=str(source), source_sha256=sha(source), n=n, command=command,
        cwd=str(SUITE), runtime_environment=ENVIRONMENT, returncode=done.returncode,
        bytes=len(done.stdout), sha256=sha(out), stdout_file=str(out), stderr_file=str(err),
        stderr_sha256=sha(err))


def primitive_source(source, operation, data=None):
    keep = [line for line in source.read_text().splitlines()
            if line.startswith(('N ', 'FlipIndices ', 'FlipPermutations '))]
    if operation == 'indices':
        keep.append('&p json FlipIndices ⇡N')
    else:
        keep.extend(['P ← ≡°□ °json ' + json.dumps(json.dumps(data)), '&p json FlipPermutations P'])
    return '\n'.join(keep) + '\n'


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    source_pins = {str(HERE / 'sources' / (variant + '.ua')): sha(HERE / 'sources' / (variant + '.ua')) for variant in VARIANTS}
    for path, digest in BASE['runtime']['pins'].items():
        assert sha(path) == digest, path
    originals = {str(SUITE / 'sources/uiua/fannkuchredux' / (variant + '.ua')):
                 sha(SUITE / 'sources/uiua/fannkuchredux' / (variant + '.ua')) for variant in VARIANTS}
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
        scope='Bounded small output/primitive/permutation/prefix-reversal validation only. Maximum complete benchmark n=8; no full n=12 timings.',
        shared_timing_lock=str(LOCK), source_pins=source_pins, original_source_pins=originals,
        runtime=str(RUNTIME), runtime_sha256=sha(RUNTIME), runtime_environment=ENVIRONMENT,
        validator=str(Path(__file__).resolve()), validator_sha256=sha(__file__),
        reference='Independent incremental canonical permutation enumeration and scalar prefix reversals.',
        cases=[], instrumentation=[], primitive_cases=[])
    print('Waiting for common mutex; no Uiua/reference probes before acquisition.', flush=True)
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired common mutex for bounded validation.', flush=True)
        expected = {n: reference(n) for n in CASES}
        for variant in VARIANTS:
            source = HERE / 'sources' / (variant + '.ua')
            for n in CASES:
                command, done = execute(source, n)
                row = persist(source, n, command, done, f'{variant}-{n}')
                row.update(variant=variant, case='canonical-small' if n == 7 else f'independent-scalar-{n}',
                    expected_sha256=hashlib.sha256(expected[n][2]).hexdigest(),
                    reference_match=done.returncode == 0 and done.stdout == expected[n][2])
                report['cases'].append(row)
                assert row['reference_match'], row
                print(variant, n, 'OUTPUT PASS', flush=True)
            traced = RESULTS / (variant + '-instrumented.ua')
            traced.write_text(instrument(source))
            for n in (2, 6, 7, 8):
                command, done = execute(traced, n)
                row = persist(traced, n, command, done, f'{variant}-work-{n}')
                row.update(variant=variant, original_source=str(source), original_source_sha256=sha(source),
                    instrumented_source=str(traced), instrumented_source_sha256=sha(traced),
                    transformation='Wrap original Flips and FlipStep expressions unchanged in trace-only input/result prints. Instrumented sources never timed.')
                row.update(validate_trace(done, n, *expected[n]))
                report['instrumentation'].append(row)
                print(variant, n, 'WORK PASS', row['permutations'], row['actual_prefix_reversals'], flush=True)
        fast = HERE / 'sources/fast.ua'
        for n in (1, 2, 3, 7, 12):
            source = RESULTS / f'indices-{n}.ua'
            source.write_text(primitive_source(fast, 'indices'))
            command, done = execute(source, n)
            expected_indices = [[first-column if column <= first else column for column in range(n)] for first in range(n)]
            assert done.returncode == 0 and json.loads(done.stdout) == expected_indices, done.stderr.decode()
            row = persist(source, n, command, done, f'indices-{n}')
            row.update(operation='native-indices', reference_match=True, all_first_positions=n, columns=n,
                no_permutation_enumeration=True)
            report['primitive_cases'].append(row)
        rng = random.Random(20261011)
        for size in (1, 17, 256, 1024):
            n = 12
            data = [rng.sample(range(n), n) for _ in range(size)]
            source = RESULTS / f'gather-{size}.ua'
            source.write_text(primitive_source(fast, 'gather', data))
            command, done = execute(source, n)
            transformed = [list(reversed(p[:p[0]+1])) + p[p[0]+1:] for p in data]
            assert done.returncode == 0 and json.loads(done.stdout) == transformed, done.stderr.decode()
            row = persist(source, n, command, done, f'gather-{size}')
            row.update(operation='native-flat-gather', reference_match=True, permutations=size,
                no_permutation_enumeration=True, arbitrary_permutations=True)
            report['primitive_cases'].append(row)
        assert source_pins == {path: sha(path) for path in source_pins}
        assert originals == {path: sha(path) for path in originals}
    report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
        passed=len(report['cases']), work_checks_passed=len(report['instrumentation']),
        primitive_checks_passed=len(report['primitive_cases']), full_performance_runs=0)
    (RESULTS / 'validation.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
    print('All checks passed:', report['passed'], 'outputs,', report['work_checks_passed'],
          'work traces,', report['primitive_checks_passed'], 'primitive checks.', flush=True)


if __name__ == '__main__':
    main()
