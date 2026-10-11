#!/usr/bin/env python3
"""Small/adversarial exact-output checks, exclusively under the shared timing lock."""
from pathlib import Path
import argparse
import fcntl
import hashlib
import json
import os
import random
import re
import subprocess
import time

HERE = Path(__file__).resolve().parent
G = HERE.parent
C = G.parents[1]
RUNTIME = G / '.build/toolchains/gnu-apl-local/bin/apl'
BENIGN_STDERR = re.compile(rb'\s*(?:WARNING: File pipe-to_client still open - closing it\s*)?\Z')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def reference(data):
    mapping = bytes.maketrans(b'ACBDGHKMNSRUTWVYacbdghkmnsrutwvy',
                            b'TGVHCDMKNSYAAWBRTGVHCDMKNSYAAWBR')
    records = []
    for line in data.splitlines():
        if line.startswith(b'>'):
            records.append([line.replace(b'\r', b''), []])
        elif records and not line.startswith(b';'):
            records[-1][1].append(line.replace(b'\r', b''))
    out = []
    for header, parts in records:
        sequence = b''.join(parts).translate(mapping)[::-1]
        out.append(header + b'\n')
        out.extend(sequence[i:i + 60] + b'\n' for i in range(0, len(sequence), 60))
    return b''.join(out)


def cases():
    canonical = (C / 'knucleotide/inputs/knucleotide-input-1000.txt').read_bytes()
    expected = reference(canonical)
    assert len(expected) == 10245
    assert sha(expected) == '549dcdcc7df2685145e6cbdfd682469bd9c12ff1beec93c270f8ba447acd4855'
    yield 'canonical-small', canonical
    yield 'all-IUPAC-empty-record', b'>one\nACBDGHKMNSRUTWVYacbdghkmnsrutwvy\n>empty\n>two\nAACCGGTT\n'
    yield 'CRLF-comments-no-final-newline', b'>one\r\naCgTu\r\n;ignored\r\nMrWsYkVhDbN'
    yield 'empty-input', b''
    yield 'missing-header', b'ACGT\n;comment\n'
    yield 'header-without-final-newline', b'>empty'
    yield 'consecutive-empty-records', b'>one\n>two\n>three'
    rng = random.Random(20261011)
    alphabet = b'ACBDGHKMNSRUTWVYacbdghkmnsrutwvy'
    dna = bytes(rng.choice(alphabet) for _ in range(491581))
    for length in [1, 59, 60, 61, 245759, 245760, 245761, 491581]:
        yield 'reverse-chunk-length-' + str(length), b'>raw sequence\n' + dna[:length] + b'\n'
    # Force record/comment markers around exact bulk-read boundaries.
    for position in [245759, 245760, 245761]:
        prefix = b'>first\n'
        body = b'A' * (position - len(prefix) - 1)
        yield 'header-boundary-' + str(position), prefix + body + b'\n>second\r\nacGT\n>empty\n'
    yield 'multi-chunk-comment', b'>one\nAC\n;' + b'>ignored' * 70000 + b'\nGt\n'
    yield 'multi-chunk-header', b'>' + b'header' * 90000 + b'\nACgt\n'
    yield 'comment-ends-at-chunk-boundary', b'>one\n;' + b'x' * (245760 - 7) + b'\nACGT\n'
    yield 'header-ends-at-chunk-boundary', b'>' + b'h' * (245760 - 2) + b'\nACGT\n'


def main():
    results = HERE / 'results'
    results.mkdir(exist_ok=True)
    rows = []
    paths = {variant: HERE / 'sources' / f'revcomp.{variant}.apl'
             for variant in ('compact', 'optimized')}
    initial_pins = {str(path): sha(path.read_bytes()) for path in
                    [RUNTIME, HERE / 'run-apl-scratch.sh', G / 'run-apl.sh', *paths.values()]}
    for case, data in cases():
        expected = reference(data)
        for variant, path in paths.items():
            command = [str(HERE / 'run-apl-scratch.sh'), '--cc',
                       '8' if variant == 'optimized' else '1', '-f', str(path)]
            started = time.monotonic()
            process = subprocess.run(command, input=data, capture_output=True,
                                     cwd=G, env=os.environ | {'LC_ALL': 'C'}, timeout=90)
            passed = (process.returncode == 0 and process.stdout == expected
                      and BENIGN_STDERR.fullmatch(process.stderr) is not None)
            row = dict(benchmark='revcomp', variant=variant, case=case,
                       command=command, n=None, input_bytes=len(data), input_sha256=sha(data),
                       stdout_bytes=len(process.stdout), stdout_sha256=sha(process.stdout),
                       expected_bytes=len(expected), expected_stdout_sha256=sha(expected),
                       returncode=process.returncode, stderr=process.stderr.decode(),
                       validation_seconds=time.monotonic() - started,
                       reference_match=passed, passed=passed)
            rows.append(row)
            if case == 'canonical-small' or not passed:
                (results / f'{case}.{variant}.stdout').write_bytes(process.stdout)
                (results / f'{case}.{variant}.stderr').write_bytes(process.stderr)
            print(case, variant, 'PASS' if passed else 'FAIL', flush=True)
            if not passed:
                print('actual', repr(process.stdout[:300]), 'expected', repr(expected[:300]),
                      'stderr', process.stderr.decode()[:1000], flush=True)
                break
        if not all(row['passed'] for row in rows):
            break
    assert all(sha(Path(path).read_bytes()) == digest for path, digest in initial_pins.items())
    report = dict(passed=all(row['passed'] for row in rows), rows=rows, pins=initial_pins,
                  boundary_lengths=[245760, 262143],
                  protocol='Exact complete-output validation only; shared exclusive timing lock; no canonical full timings.')
    (results / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
    assert report['passed']


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--wait-for-lock', action='store_true')
    args = parser.parse_args()
    with open('/tmp/mech-clbg-timing.lock', 'a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | (0 if args.wait_for_lock else fcntl.LOCK_NB))
        except BlockingIOError:
            raise SystemExit('Shared timing lock is busy; no validation process was started.')
        main()
