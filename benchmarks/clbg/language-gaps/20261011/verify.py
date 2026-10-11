#!/usr/bin/env python3
"""Validate general bigint values/function bodies without rewriting old evidence."""
from pathlib import Path
import argparse, hashlib, json, os, subprocess, time

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ROOT = CLBG.parents[1]
OLD = CLBG / 'execution-modes/20261010'
BIN = Path('/tmp/mech-execution-modes-target/release/mech-execution-modes-compiled')
EMIT = BIN.with_name('mech-execution-modes-emit')
ENV = os.environ | {'RAYON_NUM_THREADS': '8', 'MECH_SEQUENCE_PARTITIONS': '4',
    'MECH_SEQUENCE_NARROW': '1', 'MECH_SEQUENCE_STD_MAP': '1',
    'MECH_SEQUENCE_ASYNC': '1', 'MECH_SEQUENCE_BUFFER_REUSE': '1'}

def sha(path):
    with Path(path).open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()

def save(name, value):
    (HERE / 'results').mkdir(exist_ok=True)
    (HERE / 'results' / name).write_text(json.dumps(value, indent=2) + '\n')

def fixtures():
    rows = []
    for case in json.loads((HERE / 'fixtures/cases.json').read_text()):
        for variant, mode, extra_environment in [
            ('jit', 'jit', {}),
            ('jit-no-inline', 'jit', {'MECH_CONTROL_INLINE': '0'}),
            ('resident', 'resident', {}),
        ]:
            start = time.perf_counter()
            run = subprocess.run([str(BIN), str(ROOT / case['path']), mode],
                env=ENV | extra_environment, cwd=ROOT, capture_output=True, timeout=60)
            if case['error']:
                correct = run.returncode != 0 and (case.get('compile_error') or
                    case['error_contains'].encode() in run.stderr)
            else:
                correct = run.returncode == 0 and run.stdout.decode() == case['expected']
            rows.append(dict(case=case['name'], mode=variant, correct=correct,
                seconds=time.perf_counter() - start, exit_code=run.returncode,
                stdout=run.stdout.decode(errors='replace'), stderr=run.stderr.decode(),
                source_sha256=sha(ROOT / case['path'])))
            print(case['name'], variant, 'PASS' if correct else 'FAIL', flush=True)
            if not correct:
                print(rows[-1], flush=True)
            save('operator-fixtures.json', rows)
    assert all(r['correct'] for r in rows)

def regression():
    builds = json.loads((OLD / 'builds.json').read_text())['programs']
    baseline = json.loads((CLBG / 'additional-languages/20261009-full-suite/results/timings.json').read_text())
    rows = []
    for key, original in builds.items():
        directory = HERE / 'builds/regression' / key.replace(':', '-')
        directory.mkdir(parents=True, exist_ok=True)
        environment = ENV | original['build_environment']
        run = subprocess.run([str(EMIT), original['kind'], original['source'], str(directory), *original['flags']],
            env=environment, cwd=ROOT, capture_output=True, timeout=120)
        obj = directory / ('mech-numeric.o' if original['kind'] == 'numeric' else
            'mech-grid.o' if original['kind'] == 'grid' else 'mech-' + original['kind'] + '.o')
        identical = run.returncode == 0 and sha(obj) == original['native_object_sha256']
        print(key, 'OBJECT IDENTICAL' if identical else 'OBJECT FAILED/CHANGED', flush=True)
        record = dict(key=key, source_sha256=sha(original['source']), identical_native_object=identical,
            old_native_object_sha256=original['native_object_sha256'],
            native_object_sha256=sha(obj) if obj.exists() else None,
            emitter_stderr=run.stderr.decode(), modes=[])
        bench = original['benchmark']
        inp = CLBG / ('regexredux/inputs/regexredux-input-small.txt' if bench == 'regexredux' else
            'knucleotide/inputs/knucleotide-input-1000.txt') if bench in ['regexredux', 'knucleotide', 'revcomp'] else None
        args = [] if inp else [str(baseline['workloads'][bench][0])]
        expected = baseline['programs'][bench + ':mech-control']['small_validation']['sha256']
        flags = ['--fp-contract-loop'] if bench == 'mandelbrot' and 'simd' in key else (
            ['--algebraic-powers', '--fp-contract', '--reassociate-constants', '--schedule', '--pack-state-tails'] if bench == 'nbody' else [])
        for mode in ['jit', 'resident']:
            environment = ENV | original['runtime_environment']
            handle = inp.open('rb') if inp else subprocess.DEVNULL
            try:
                run = subprocess.run([str(BIN), original['source'], mode, *args, *flags],
                    stdin=handle, env=environment, cwd=ROOT, capture_output=True, timeout=120)
            finally:
                if inp:
                    handle.close()
            digest = hashlib.sha256(run.stdout).hexdigest()
            correct = run.returncode == 0 and digest == expected
            if bench == 'nbody' and run.returncode == 0:
                energies = [float(v) for v in run.stdout.decode().split()]
                correct = len(energies) == 2 and all(abs(a-b) <= 1e-8 for a,b in zip(energies, [-0.169075164,-0.169087605]))
            record['modes'].append(dict(mode=mode, correct=correct, exit_code=run.returncode,
                stdout_sha256=digest, stdout_bytes=len(run.stdout), stderr=run.stderr.decode()))
            print(key, mode, 'PASS' if correct else 'FAIL', flush=True)
        rows.append(record)
        save('unchanged-benchmarks.json', rows)
    assert all(r['identical_native_object'] and all(m['correct'] for m in r['modes']) for r in rows)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('stage', choices=['fixtures', 'regression'])
    args = parser.parse_args()
    {'fixtures': fixtures, 'regression': regression}[args.stage]()

if __name__ == '__main__':
    main()
