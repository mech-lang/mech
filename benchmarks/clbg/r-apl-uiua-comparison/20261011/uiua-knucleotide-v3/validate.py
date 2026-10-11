#!/usr/bin/env python3
"""Mutex-serialized small outputs, complete maps and bounded-work checks."""
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
RESULTS = HERE / 'results'
LOCK = Path('/tmp/mech-clbg-timing.lock')
BASE = json.loads((SUITE / 'uiua-manifest.json').read_text())
RUNTIME = Path(BASE['runtime']['path'])
ENVIRONMENT = dict(LC_ALL='C', OMP_NUM_THREADS='1', OPENBLAS_NUM_THREADS='1',
                   VECLIB_MAXIMUM_THREADS='1')
WIDTHS = (1, 2, 3, 4, 6, 12, 18)
PROFILES = ('compact', 'bounded', 'native-group')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def reference_module():
    path = SUITE / 'uiua-knucleotide-v2/validate.py'
    spec = importlib.util.spec_from_file_location('knucleotide_v2_reference', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def instrument(text, profile):
    maps = 'Maps     ← ≡(□Count) [1 2 3 4 6 12 18]'
    assert text.count(maps) == 1
    text = text.replace(maps, maps + '\n≡(&p ⊂ "MAP=" json ◇) Maps')
    if profile != 'compact':
        windows = 'ChunkWindows ← ⧈∘ ⊃K.Width ChunkText'
        assert text.count(windows) == 1
        text = text.replace(windows, windows +
            ' &p $"WINDOWS=_ _ _" ⊃(K.Width|K.Start|ChunkSize) .')
    if profile == 'native-group':
        classes = 'ClassCounts ← ⊃(⊕⊢)(⊕⧻) ⊸⊛'
        assert text.count(classes) == 1
        text = text.replace(classes,
            'ClassCounts ← ⊙⊸(&p ⊂ "WEIGHTSUM=" °⋕ /+) ⊸(&p ⊂ "DISTINCT=" °⋕ ⧻) ⊃(⊕⊢)(⊕⧻) ⊸⊛')
    return text


def execute(source, data):
    command = [str(RUNTIME), 'run', '--no-format', str(source)]
    done = subprocess.run(command, cwd=SUITE, env=dict(os.environ, **ENVIRONMENT),
                          input=data, capture_output=True, timeout=300)
    return command, done


def recorded_run(source, data, stem):
    command, done = execute(source, data)
    stdout = RESULTS / (stem + '.stdout')
    stderr = RESULTS / (stem + '.stderr')
    stdout.write_bytes(done.stdout)
    stderr.write_bytes(done.stderr)
    detail = dict(command=command, source=str(source), source_sha256=sha(source),
                  cwd=str(SUITE), runtime_environment=ENVIRONMENT,
                  returncode=done.returncode, stdout_file=str(stdout),
                  stdout_sha256=sha(stdout), stderr_file=str(stderr),
                  stderr_sha256=sha(stderr), stderr=done.stderr.decode())
    return done, detail


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    inputs = RESULTS / 'inputs'
    inputs.mkdir(exist_ok=True)
    derivation = json.loads((HERE / 'derivation.json').read_text())
    for path, digest in BASE['runtime']['pins'].items():
        assert sha(path) == digest, path
    report = dict(status='running', created_at=datetime.now(timezone.utc).isoformat(),
                  scope='Small/adversarial output, complete seven maps and work checks; no full timings.',
                  runtime=str(RUNTIME), runtime_sha256=sha(RUNTIME),
                  runtime_environment=ENVIRONMENT, shared_timing_lock=str(LOCK),
                  validator=str(Path(__file__).resolve()), validator_sha256=sha(__file__),
                  reference_validator=str(SUITE / 'uiua-knucleotide-v2/validate.py'),
                  reference_validator_sha256=sha(SUITE / 'uiua-knucleotide-v2/validate.py'),
                  derivation=derivation, original_counterexamples=[], cases=[], instrumentation=[])
    print('Waiting for shared timing lock.', flush=True)
    with LOCK.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print('Acquired shared timing lock.', flush=True)
        reference = reference_module()
        cases = reference.fixtures() + [
            ('eof-comment-exact-counterexample', b'>THREE\nACGT\n;ACGT'),
            ('eof-empty-comment', b'>THREE\nACGT\n;'),
            ('eof-comment-crlf-query', b'>THREE eof\r\naCgT\r\n;GGTATTTTAATTTATAGT'),
            ('consecutive-empty-comments', b'>THREE\n;\nACGT\n;\r\nGGTATT\n;'),
            ('no-final-newline-sequence', b'>THREE last\nggtattttaatttatagtacgt'),
            ('unterminated-target-header', b'>THREE'),
            ('comment-embedded-header-marker', b'>THREE\nACGT\n;>IGNORED\nTGCA\n'),
            ('comments-with-target-header-crlf', b'>ONE\r\n;>THREE ignored\r\nACGT\r\n>THREE\r\nGGTATT\r\n;>FOUR ignored\r\nACGT\r\n>FOUR\r\nGGGG\r\n'),
            ('empty-three-short-headers', b'>ONE\nA\n>THREE\n>\nGGT\n'),
        ]
        for example_name in ('eof-comment-exact-counterexample', 'comment-embedded-header-marker'):
            example = next(data for name,data in cases if name == example_name)
            _, _, example_expected = reference.reference(example)
            for row in derivation:
                original = Path(row['original_source'])
                done, detail = recorded_run(original, example, row['profile'] + '.original-' + example_name)
                assert done.returncode == 0 and done.stdout != example_expected, detail
                detail.update(profile=row['profile'], case=example_name,
                              input_bytes=example.decode(), expected=example_expected.decode(),
                              observed=done.stdout.decode(), reference_match=False,
                              confirmed_parser_defect=True)
                report['original_counterexamples'].append(detail)
        for name, data in cases:
            input_path = inputs / (name + '.fasta')
            input_path.write_bytes(data)
            sequence, expected_maps, expected = reference.reference(data)
            expected_windows = [[width, start, min(65536, max(0, len(sequence) - width + 1) - start)]
                                for width in WIDTHS
                                for start in range(0, max(0, len(sequence) - width + 1), 65536)]
            for profile in PROFILES:
                row = next(row for row in derivation if row['profile'] == profile)
                source = Path(row['source'])
                assert sha(source) == row['source_sha256']
                done, detail = recorded_run(source, data, profile + '.' + name)
                assert done.returncode == 0 and done.stdout == expected, detail
                detail.update(profile=profile, case=name, input_file=str(input_path),
                              input_sha256=sha(input_path), sequence_length=len(sequence),
                              expected=expected.decode(), reference_match=True)
                report['cases'].append(detail)
                traced = RESULTS / (profile + '.instrumented.ua')
                traced.write_text(instrument(source.read_text(), profile))
                done, work = recorded_run(traced, data, profile + '.' + name + '.work')
                assert done.returncode == 0, work
                lines = done.stdout.decode().splitlines(keepends=True)
                observed_maps = [json.loads(line[4:]) for line in lines if line.startswith('MAP=')]
                observed_maps = [{} if value == [] else value for value in observed_maps]
                assert observed_maps == [dict(counts) for counts in expected_maps], (profile, name)
                clean_output = ''.join(line for line in lines
                                       if not line.startswith(('MAP=', 'WINDOWS=', 'WEIGHTSUM=', 'DISTINCT='))).encode()
                assert clean_output == expected, (profile, name)
                windows = [list(map(int, line[8:].split())) for line in lines if line.startswith('WINDOWS=')]
                weights = [int(line[10:]) for line in lines if line.startswith('WEIGHTSUM=')]
                distinct = [int(line[9:]) for line in lines if line.startswith('DISTINCT=')]
                if profile != 'compact':
                    assert windows == expected_windows, (profile, name, windows, expected_windows)
                if profile == 'native-group':
                    assert weights == [size for _, _, size in expected_windows], (profile, name)
                    assert distinct == [len({sequence[i:i + width] for i in range(start, start + size)})
                                        for width, start, size in expected_windows], (profile, name)
                work.update(profile=profile, case=name, input_file=str(input_path),
                            input_sha256=sha(input_path), original_source=str(source),
                            original_source_sha256=sha(source), widths=list(WIDTHS),
                            sequence_length=len(sequence), chunks=windows,
                            grouped_weight_sums=weights, distinct_keys_merged_per_chunk=distinct,
                            complete_map_cardinalities=[len(counts) for counts in expected_maps],
                            complete_map_totals=[sum(counts.values()) for counts in expected_maps],
                            expected_windows_by_width={str(width): max(0, len(sequence) - width + 1) for width in WIDTHS},
                            complete_seven_maps_match=True, complete_output_match=True,
                            every_window_once=True,
                            work_evidence='Every complete map key/count checked; compact uses whole-record native stencil, bounded variants additionally trace exact start partition.',
                            trace_only=True)
                report['instrumentation'].append(work)
                print(profile, name, 'OUTPUT/MAPS/WORK PASS', flush=True)
        for row in derivation:
            assert sha(row['original_source']) == row['original_source_sha256']
            assert sha(row['source']) == row['source_sha256']
    report.update(status='passed', finished_at=datetime.now(timezone.utc).isoformat(),
                  passed=len(report['cases']), work_checks_passed=len(report['instrumentation']),
                  fixtures_per_profile=len(cases), profiles=list(PROFILES))
    (RESULTS / 'validation.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')


if __name__ == '__main__':
    main()
