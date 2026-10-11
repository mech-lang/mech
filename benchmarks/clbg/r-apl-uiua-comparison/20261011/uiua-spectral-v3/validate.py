#!/usr/bin/env python3
"""Locked small/boundary validation only; never run the full 5500 workload."""
from collections import Counter
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
CASES = (1, 2, 7, 100, 113, 129, 257)
VARIANTS = ('compact9', 'compact', 'fast')
ENVIRONMENT = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1',
                   VECLIB_MAXIMUM_THREADS='1')
BASE = json.loads((SUITE / 'uiua-manifest.json').read_text())
RUNTIME = Path(BASE['runtime']['path'])
RESULTS = HERE / 'results'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def reference(n):
    """Independent scalar binary64 formula; all 40 products recompute A."""
    def coefficient(i, j):
        s = i + j
        return 1.0 / (s * (s + 1) / 2 + i + 1)
    def forward(vector):
        return [sum(coefficient(i, j) * vector[j] for j in range(n))
                for i in range(n)]
    def transpose(vector):
        return [sum(coefficient(i, j) * vector[i] for i in range(n))
                for j in range(n)]
    vector = [1.0] * n
    for _ in range(19):
        vector = transpose(forward(vector))
    other = transpose(forward(vector))
    value = math.sqrt(sum(a * b for a, b in zip(vector, other)) /
                      sum(a * a for a in vector))
    return f'{value:.9f}\n'.encode()


def execute(source, n):
    command = [str(RUNTIME), 'run', '--no-format', str(source), str(n)]
    environment = dict(os.environ, **ENVIRONMENT)
    completed = subprocess.run(command, cwd=SUITE, env=environment,
                               capture_output=True, timeout=30)
    return command, completed


def instrument(source, variant):
    """Add side-effect-only trace prints; every original expression is retained."""
    output = []
    coefficient_binding = 'Triangle' if variant == 'compact9' else 'Sums'
    rows_expression = '⧻ .' if variant != 'fast' else '⧻ Rows .'
    for line in source.read_text().splitlines():
        name = line.split('←', 1)[0].strip()
        if name == coefficient_binding:
            line += ' &p ⊂ "COEFFICIENT-ROWS=" °⋕ ' + rows_expression
        elif name == 'Forward':
            line += ' &p "FORWARD-PRODUCT"'
        elif name == 'Transpose':
            line += ' &p "TRANSPOSE-PRODUCT"'
        output.append(line)
    return '\n'.join(output) + '\n'


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    for path, pin in BASE['runtime']['pins'].items():
        assert sha(path) == pin, path
    source_pins = {str(HERE / 'sources' / (variant + '.ua')):
                   sha(HERE / 'sources' / (variant + '.ua'))
                   for variant in VARIANTS}
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
        scope='Small/boundary output validation and instrumented work audit only; no full timings.',
        shared_timing_lock=str(LOCK), runtime=str(RUNTIME), runtime_sha256=sha(RUNTIME),
        source_pins=source_pins, runtime_environment=ENVIRONMENT,
        reference='Independent Python scalar binary64 canonical formula, all 40 products.',
        validator=str(Path(__file__).resolve()), validator_sha256=sha(__file__),
        cases=[], instrumentation=[])
    print('Waiting for shared timing lock; no validation executes before acquisition.', flush=True)
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired shared timing lock for bounded small probes.', flush=True)
        expected = {n: reference(n) for n in CASES}
        for variant in VARIANTS:
            source = HERE / 'sources' / (variant + '.ua')
            for n in CASES:
                command, completed = execute(source, n)
                stem = RESULTS / f'{variant}-{n}'
                stem.with_suffix('.stdout').write_bytes(completed.stdout)
                stem.with_suffix('.stderr').write_bytes(completed.stderr)
                record = dict(variant=variant, n=n, source=str(source),
                    source_sha256=sha(source), command=command, cwd=str(SUITE),
                    runtime_environment=ENVIRONMENT, returncode=completed.returncode,
                    bytes=len(completed.stdout), sha256=hashlib.sha256(completed.stdout).hexdigest(),
                    expected=expected[n].decode(), expected_sha256=hashlib.sha256(expected[n]).hexdigest(),
                    reference_match=completed.returncode == 0 and completed.stdout == expected[n],
                    stderr=completed.stderr.decode(), stdout_file=str(stem.with_suffix('.stdout')))
                report['cases'].append(record)
                print(variant, n, 'PASS' if record['reference_match'] else 'FAIL',
                      repr(completed.stdout), completed.stderr.decode(), flush=True)
                assert record['reference_match'], record
            traced = RESULTS / (variant + '-instrumented.ua')
            traced.write_text(instrument(source, variant))
            for n in (2, 129, 257):
                command, completed = execute(traced, n)
                lines = completed.stdout.decode().splitlines()
                rows = [int(line.split('=', 1)[1]) for line in lines
                        if line.startswith('COEFFICIENT-ROWS=')]
                counts = Counter(lines)
                blocks = 1 if variant != 'fast' else math.ceil(n / 128)
                expected_rows = ([n] if variant != 'fast' else
                                 [min(128, n - start) for start in range(0, n, 128)]) * 40
                assert completed.returncode == 0, completed.stderr.decode()
                assert counts['FORWARD-PRODUCT'] == counts['TRANSPOSE-PRODUCT'] == 20
                assert rows == expected_rows and len(rows) == 40 * blocks
                assert sum(rows) == 40 * n
                assert lines[-1].encode() + b'\n' == expected[n]
                trace_path = RESULTS / f'{variant}-work-{n}.stdout'
                trace_path.write_bytes(completed.stdout)
                report['instrumentation'].append(dict(variant=variant, n=n,
                    original_source=str(source), original_source_sha256=sha(source),
                    instrumented_source=str(traced), instrumented_source_sha256=sha(traced),
                    command=command, cwd=str(SUITE), runtime_environment=ENVIRONMENT,
                    transformation=f'Append trace-only prints to {"Triangle" if variant == "compact9" else "Sums"}, Forward and Transpose; original expressions retained.',
                    forward_products=20, transpose_products=20, coefficient_blocks=len(rows),
                    coefficient_rows=rows, coefficient_elements=sum(rows) * n,
                    expected_coefficient_elements=40 * n * n,
                    maximum_block_rows=max(rows), no_cached_coefficient_matrix=True,
                    reference_match=True, stdout_file=str(trace_path), stdout_sha256=sha(trace_path)))
                print(variant, n, 'WORK PASS', len(rows), 'blocks,', sum(rows) * n,
                      'coefficient elements', flush=True)
    assert source_pins == {path: sha(path) for path in source_pins}
    report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
                  passed=len(report['cases']), work_checks_passed=len(report['instrumentation']))
    (RESULTS / 'validation.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')


if __name__ == '__main__':
    main()
