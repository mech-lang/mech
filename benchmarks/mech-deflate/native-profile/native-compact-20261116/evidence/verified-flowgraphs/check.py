"""Check the verifier contracts using the libraries built for the matched run."""
import json
from pathlib import Path
import subprocess

from production import HERE, OUT, save, sha


def main():
    desired = {'cranelift_codegen', 'cranelift_frontend', 'cranelift_native'}
    libraries = {}
    for line in (OUT / 'rebuild/compiler-build.jsonl').read_text().splitlines():
        if not line.startswith('{'):
            continue
        item = json.loads(line)
        name = item.get('target', {}).get('name')
        if item['reason'] == 'compiler-artifact' and name in desired:
            path, = [Path(file) for file in item['filenames'] if file.endswith('.rlib')]
            libraries[name] = path
    assert set(libraries) == desired
    dependencies, = {path.parent for path in libraries.values()}
    binary = HERE / 'check-verifier'
    source = HERE / 'check.rs'
    command = ['rustc', '+nightly-2026-03-03', '--edition=2024', '--test', '-D', 'warnings',
               '-L', f'dependency={dependencies}']
    for name, path in sorted(libraries.items()):
        command.extend(('--extern', f'{name}={path}'))
    command.extend((str(source), '-o', str(binary)))
    with (HERE / 'check-build.log').open('wb') as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    result = subprocess.run([binary, '--test-threads=1'], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (HERE / 'checks.log').write_text(result.stdout)
    assert result.returncode == 0 and '4 passed; 0 failed' in result.stdout, result.stdout
    save(HERE / 'checks.json', {'command': command, 'source_sha256': sha(source), 'binary_sha256': sha(binary),
         'libraries': {name: sha(path) for name, path in libraries.items()}, 'output': result.stdout})
    print(result.stdout, flush=True)


if __name__ == '__main__':
    main()
