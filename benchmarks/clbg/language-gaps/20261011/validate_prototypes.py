#!/usr/bin/env python3
"""Check complete prototype output at boundary sizes in all execution paths."""
from pathlib import Path
import hashlib, json, os, subprocess

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ROOT = CLBG.parents[1]
DRIVER = '/tmp/mech-execution-modes-target/release/mech-execution-modes-compiled'

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    old = json.loads((CLBG / 'execution-modes/20261010/builds.json').read_text())['programs']
    built = json.loads((HERE / 'results/prototype-builds.json').read_text())
    assert len(built) == 5
    rows = []
    for bench, key, sizes in [
        ('fasta', 'fasta:mech-fasta-final-compact-v2', [1, 2, 5, 6, 12, 20, 59, 60, 61, 1000]),
        ('pidigits', 'pidigits:mech-pi-final-compact-signed', [1, 2, 9, 10, 11, 19, 20, 21, 30, 1000]),
    ]:
        control = old[key]
        environment = os.environ | control['runtime_environment']
        candidates = [(row['profile'] + '-aot', [row['binary']], {}) for row in built if row['benchmark'] == bench]
        for source_profile in ['prototype', 'binary-search'] if bench == 'fasta' else ['prototype']:
            source = str(HERE / 'sources' / (bench + '.' + source_profile + '.mec'))
            candidates += [(source_profile + '-' + variant, [DRIVER, source, mode], extra) for variant, mode, extra in [
                ('jit', 'jit', {}), ('jit-no-inline', 'jit', {'MECH_CONTROL_INLINE': '0'}), ('interpreted', 'resident', {})]]
        for n in sizes:
            reference = subprocess.run([control['binary'], str(n)], env=environment, capture_output=True, check=True, timeout=60).stdout
            for profile, command, extra in candidates:
                run = subprocess.run([*command, str(n)], env=environment | extra, cwd=ROOT, capture_output=True, timeout=60)
                passed = run.returncode == 0 and run.stdout == reference
                rows.append(dict(benchmark=bench, profile=profile, n=n, correct=passed,
                    bytes=len(run.stdout), sha256=digest(run.stdout), reference_sha256=digest(reference),
                    exit_code=run.returncode, stderr=run.stderr.decode(errors='replace')))
                (HERE / 'results/prototype-validation.json').write_text(json.dumps(rows, indent=2) + '\n')
                if not passed:
                    print(rows[-1], flush=True)
                    raise RuntimeError('prototype output mismatch')
            print(bench, n, f'PASS ({len(candidates)} execution variants)', flush=True)
    print(len(rows), 'complete-output checks passed', flush=True)

if __name__ == '__main__':
    main()
