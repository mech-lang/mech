"""Run focused verifier checks using the candidate compiler's Rust libraries."""
from hashlib import sha256
import json
from pathlib import Path
import subprocess

here = Path(__file__).resolve().parent
candidate = here / 'candidate'


def main():
    binary = here / 'check-verifier'
    source = here / 'check.rs'
    libraries = {}
    desired = {'cranelift_codegen', 'cranelift_frontend', 'cranelift_native'}
    for line in (candidate / 'rebuild/compiler-build.jsonl').read_text().splitlines():
        if not line.startswith('{'):
            continue
        row = json.loads(line)
        if row['reason'] == 'compiler-artifact' and row['target']['name'] in desired:
            name = row['target']['name']
            assert name not in libraries
            libraries[name], = [Path(file) for file in row['filenames'] if file.endswith('.rlib')]
    assert libraries.keys() == desired
    dependency, = set(p.parent for p in libraries.values())
    args = ['rustc', '+nightly-2026-03-03', '--edition=2024', '--test', '-D', 'warnings',
            '-L', f'dependency={dependency}']
    for name, path in sorted(libraries.items()):
        args.extend(('--extern', f'{name}={path}'))
    args.extend((str(source), '-o', str(binary)))
    with (here / 'check-build.log').open('w') as log:
        subprocess.run(args, stdout=log, stderr=subprocess.STDOUT, check=True)
    result = subprocess.check_output([binary, '--test-threads=1'], text=True)
    assert '2 passed; 0 failed' in result
    print(result, flush=True)
    (here / 'check-receipt.json').write_text(json.dumps({'build': args,
        'code': sha256(source.read_bytes()).hexdigest(), 'binary': sha256(binary.read_bytes()).hexdigest(),
        'test_output': result}, indent=2) + '\n')


if __name__ == '__main__':
    main()
