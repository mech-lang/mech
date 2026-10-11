#!/usr/bin/env python3
"""Locked small/adversarial validation and full seven-map diagnostic checks."""
from collections import Counter
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import random
import subprocess

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
CLBG = SUITE.parents[1]
SOURCE = HERE / 'sources/fast.ua'
RESULTS = HERE / 'results'
WIDTHS = (1, 2, 3, 4, 6, 12, 18)
QUERIES = ('GGT', 'GGTA', 'GGTATT', 'GGTATTTTAATT', 'GGTATTTTAATTTATAGT')
ENVIRONMENT = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1',
                   VECLIB_MAXIMUM_THREADS='1')
BASE = json.loads((SUITE / 'uiua-manifest.json').read_text())
RUNTIME = Path(BASE['runtime']['path'])
LOCK = Path('/tmp/mech-clbg-timing.lock')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def parse(data):
    reading = False
    sequence = []
    for line in data.decode().splitlines():
        if line.startswith('>THREE'):
            reading = True
        elif reading and line.startswith('>'):
            break
        elif reading and not line.startswith(';'):
            sequence.append(line.upper())
    return ''.join(sequence)


def reference(data):
    sequence = parse(data)
    maps = [Counter(sequence[i:i + width]
                    for i in range(max(0, len(sequence) - width + 1)))
            for width in WIDTHS]
    output = []
    for counts in maps[:2]:
        total = sum(counts.values())
        output.extend(f'{key} {100 * count / total:.3f}\n'
                      for key, count in sorted(counts.items(),
                          key=lambda item: (-item[1], item[0])))
        output.append('\n')
    output.extend(f'{counts[query]}\t{query}\n'
                  for counts, query in zip(maps[2:], QUERIES))
    return sequence, maps, ''.join(output).encode()


def debruijn(alphabet='ACGT', width=6):
    """Arbitrary fixture containing every width-six key; not benchmark logic."""
    a = [0] * (len(alphabet) * width)
    output = []
    def visit(t, period):
        if t > width:
            if width % period == 0:
                output.extend(a[1:period + 1])
        else:
            a[t] = a[t - period]
            visit(t + 1, period)
            for digit in range(a[t - period] + 1, len(alphabet)):
                a[t] = digit
                visit(t + 1, t)
    visit(1, 1)
    text = ''.join(alphabet[i] for i in output)
    return text + text[:width - 1]


def record(name, sequence):
    return ('>ONE ignored\nGGTATTTTAATTTATAGT\n>THREE ' + name + '\n' +
            '\n'.join(sequence[i:i + 37].lower() for i in range(0, len(sequence), 37)) +
            '\n>FOUR ignored\nGGTATTTTAATTTATAGT\n').encode()


def fixtures():
    result = [('canonical-small', (CLBG / 'knucleotide/inputs/knucleotide-input-1000.txt').read_bytes())]
    result.extend([
        ('lexical-ties-stop', b'>ONE ignore\nACGT\n>THREE ties\nAAAAACCCCGGGGTTTT\n>FOUR stop\nGGGG\n'),
        ('short-window', b'>THREE short\nA\n'),
        ('empty-record', b'>THREE empty\n>FOUR\nAGG\n'),
        ('missing-record', b'>ONE\nACGT\n'),
        ('mixed-case-comments-crlf', b'>THREE crlf\r\naCgT\r\n;ignored comment\r\nGGTATT\r\n'),
    ])
    rng = random.Random(20261011)
    arbitrary = ''.join(rng.choice('ACGT') for _ in range(10003)) + QUERIES[-1]
    result.append(('arbitrary-all-widths-growth', record('arbitrary', arbitrary)))
    diverse = debruijn() + QUERIES[-1]
    result.append(('all-six-base-keys-collision-growth', record('all-six-base-keys', diverse)))
    for length in (65535, 65536, 65537, 65553, 65554, 131091):
        text = (diverse * ((length + len(diverse) - 1) // len(diverse)))[:length]
        result.append((f'chunk-boundary-{length}', record('boundary', text)))
    return result


def instrument(source):
    text = source.read_text()
    text = text.replace('ChunkWindows ← ⧈∘ ⊃K.Width ChunkText',
        'ChunkWindows ← ⧈∘ ⊃K.Width ChunkText &p $"WINDOWS=_ _ _" ⊃(K.Width|K.Start|ChunkSize) .')
    text = text.replace('ClassCounts ← ⊃(⊕⊢)(⊕⧻) ⊸⊛',
        'ClassCounts ← ⊙⊸(&p ⊂ "WEIGHTSUM=" °⋕ /+) ⊸(&p ⊂ "DISTINCT=" °⋕ ⧻) ⊃(⊕⊢)(⊕⧻) ⊸⊛')
    text = text.replace('Maps     ← ≡(□Count) [1 2 3 4 6 12 18]',
        'Maps     ← ≡(□Count) [1 2 3 4 6 12 18]\n≡(&p ⊂ "MAP=" json ◇) Maps')
    return text


def execute(source, data):
    command = [str(RUNTIME), 'run', '--no-format', str(source)]
    completed = subprocess.run(command, cwd=SUITE, env=dict(os.environ, **ENVIRONMENT),
                               input=data, capture_output=True, timeout=60)
    return command, completed


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    inputs = RESULTS / 'inputs'
    inputs.mkdir(exist_ok=True)
    for path, digest in BASE['runtime']['pins'].items():
        assert sha(path) == digest, path
    original_sha = sha(SOURCE)
    traced = RESULTS / 'instrumented.ua'
    traced.write_text(instrument(SOURCE))
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
        scope='Small/adversarial output and complete seven-map work checks only; no full workload timings.',
        source=str(SOURCE), source_sha256=original_sha, runtime=str(RUNTIME),
        runtime_sha256=sha(RUNTIME), runtime_environment=ENVIRONMENT,
        shared_timing_lock=str(LOCK), validator=str(Path(__file__).resolve()),
        validator_sha256=sha(__file__), instrumented_source=str(traced),
        instrumented_source_sha256=sha(traced),
        instrumentation_transformation='Retain every original expression; append trace-only prints for chunk width/start/count, grouped distinct keys and weight sum, then serialize every complete map separately. Traced sources are never timed.',
        cases=[], instrumentation=[])
    print('Waiting for shared timing lock; all reference/probe work occurs after acquisition.', flush=True)
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired shared timing lock.', flush=True)
        for name, data in fixtures():
            input_path = inputs / (name + '.fasta')
            input_path.write_bytes(data)
            sequence, maps, expected = reference(data)
            command, completed = execute(SOURCE, data)
            stdout = RESULTS / (name + '.stdout')
            stderr = RESULTS / (name + '.stderr')
            stdout.write_bytes(completed.stdout)
            stderr.write_bytes(completed.stderr)
            case = dict(case=name, n=1000 if name == 'canonical-small' else None,
                source=str(SOURCE), source_sha256=original_sha, command=command,
                cwd=str(SUITE), runtime_environment=ENVIRONMENT,
                input_file=str(input_path), input_sha256=sha(input_path),
                sequence_length=len(sequence), returncode=completed.returncode,
                expected=expected.decode(), expected_sha256=hashlib.sha256(expected).hexdigest(),
                bytes=len(completed.stdout), sha256=sha(stdout), stdout_file=str(stdout),
                stderr_file=str(stderr), stderr=completed.stderr.decode(),
                reference_match=completed.returncode == 0 and completed.stdout == expected)
            report['cases'].append(case)
            print(name, 'OUTPUT PASS' if case['reference_match'] else 'OUTPUT FAIL', flush=True)
            assert case['reference_match'], case
            command, completed = execute(traced, data)
            trace_stdout = RESULTS / (name + '.work.stdout')
            trace_stderr = RESULTS / (name + '.work.stderr')
            trace_stdout.write_bytes(completed.stdout)
            trace_stderr.write_bytes(completed.stderr)
            lines = completed.stdout.decode().splitlines(keepends=True)
            window_records = [list(map(int, line[8:].split()))
                              for line in lines if line.startswith('WINDOWS=')]
            expected_windows = [[width, start, min(65536, max(0, len(sequence) - width + 1) - start)]
                               for width in WIDTHS
                               for start in range(0, max(0, len(sequence) - width + 1), 65536)]
            counts = [int(line[10:]) for line in lines if line.startswith('WEIGHTSUM=')]
            distinct = [int(line[9:]) for line in lines if line.startswith('DISTINCT=')]
            expected_distinct = [len({sequence[i:i + width] for i in range(start, start + size)})
                                 for width, start, size in expected_windows]
            observed_maps = [json.loads(line[4:]) for line in lines if line.startswith('MAP=')]
            observed_maps = [{} if value == [] else value for value in observed_maps]
            clean_output = ''.join(line for line in lines
                if not line.startswith(('WINDOWS=', 'WEIGHTSUM=', 'DISTINCT=', 'MAP='))).encode()
            assert completed.returncode == 0, completed.stderr.decode()
            assert window_records == expected_windows, (name, window_records, expected_windows)
            assert counts == [size for _, _, size in expected_windows], (name, counts)
            assert distinct == expected_distinct, (name, distinct, expected_distinct)
            assert observed_maps == [dict(counts) for counts in maps], name
            assert clean_output == expected, (name, clean_output, expected)
            work = dict(case=name, command=command, cwd=str(SUITE),
                runtime_environment=ENVIRONMENT, input_file=str(input_path), input_sha256=sha(input_path),
                source=str(traced), source_sha256=sha(traced), original_source_sha256=original_sha,
                sequence_length=len(sequence), widths=list(WIDTHS),
                chunks=window_records, maximum_chunk_starts=max([size for _, _, size in window_records], default=0),
                grouped_weight_sums=counts, distinct_keys_merged_per_chunk=distinct,
                total_windows_by_width={str(width): sum(size for w, _, size in window_records if w == width)
                                        for width in WIDTHS},
                expected_windows_by_width={str(width): max(0, len(sequence) - width + 1) for width in WIDTHS},
                complete_map_cardinalities=[len(counts) for counts in maps],
                complete_map_totals=[sum(counts.values()) for counts in maps],
                complete_seven_maps_match=True, every_window_once=True,
                weighted_merges_match=True, complete_output_match=True,
                stdout_file=str(trace_stdout), stdout_sha256=sha(trace_stdout),
                stderr_file=str(trace_stderr), stderr_sha256=sha(trace_stderr))
            report['instrumentation'].append(work)
            print(name, 'MAP/WORK PASS', sum(work['total_windows_by_width'].values()),
                  'windows;', sum(distinct), 'weighted merges;', work['complete_map_cardinalities'], flush=True)
    assert sha(SOURCE) == original_sha
    report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
                  passed=len(report['cases']), work_checks_passed=len(report['instrumentation']))
    (RESULTS / 'validation.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')


if __name__ == '__main__':
    main()
