"""Complete small reference-output checks; never full benchmark timing."""
from pathlib import Path
import hashlib
import importlib.util
import json
import subprocess

D = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('small_reference', D/'support/smalltalk/validate.py')
refs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(refs)
PARAMETERS = dict(nbody=1000, mandelbrot=32, knucleotide=1000,
                  spectralnorm=30, revcomp=1000, fannkuchredux=7,
                  fasta=1000, binarytrees=10, regexredux=1000, pidigits=103)
CHECKS = D/'support/toit/checks'
CHECKS.mkdir(exist_ok=True)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def check(row, n, raw, expected, label):
    command = [part.replace('{n}', str(n)) for part in row['command']]
    before = sha(Path(row['binary']).read_bytes())
    proc = subprocess.run(command, input=raw, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, timeout=120)
    prefix = CHECKS/(row['id']+'-'+label)
    prefix.with_suffix('.stdout').write_bytes(proc.stdout)
    prefix.with_suffix('.stderr').write_bytes(proc.stderr)
    prefix.with_suffix('.reference').write_bytes(expected)
    assert proc.returncode == 0, (row['id'], proc.returncode, proc.stderr.decode(errors='replace'))
    assert proc.stdout == expected, (row['id'], label, proc.stdout[:600], expected[:600])
    assert before == sha(Path(row['binary']).read_bytes()) == row['binary_sha256']
    return dict(reference_match=True, n=n, stdin_bytes=len(raw),
                stdin_sha256=sha(raw), bytes=len(expected), sha256=sha(expected),
                output=str(prefix.with_suffix('.stdout')), binary_unchanged=True)


def main():
    path = D/'toit_manifest.json'
    manifest = json.loads(path.read_text())
    failed = []
    for row in manifest['programs']:
        benchmark = row['benchmark']
        n = PARAMETERS[benchmark]
        raw = refs.fasta(n) if benchmark in ('knucleotide', 'revcomp', 'regexredux') else b''
        expected = refs.reference(benchmark, n, raw)
        try:
            row['small_validation'] = check(row, n, raw, expected, 'canonical')
            print('PASS', row['id'], flush=True)
        except (AssertionError, subprocess.TimeoutExpired) as error:
            failed.append(row['id'])
            row['small_validation'] = dict(reference_match=False, error=str(error))
            print('FAIL', str(error), flush=True)
        path.write_text(json.dumps(manifest, indent=2)+'\n')
    if failed:
        raise RuntimeError('Failed: '+', '.join(failed))
    manifest['status'] = 'prepared'
    path.write_text(json.dumps(manifest, indent=2)+'\n')
    print('COMPLETE', len(manifest['programs']))


if __name__ == '__main__':
    main()
