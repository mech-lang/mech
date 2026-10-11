#!/usr/bin/env python3
"""Build new pure-Mech prototypes and unchanged compact AOT controls."""
from pathlib import Path
import hashlib, json, os, shutil, subprocess

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ROOT = CLBG.parents[1]
OLD = CLBG / 'execution-modes/20261010'
EMIT = '/tmp/mech-execution-modes-target/release/mech-execution-modes-emit'
CARGO = '/Users/cim310/.cargo/bin/cargo'
TARGET = '/tmp/mech-execution-modes-target'

def sha(path):
    with Path(path).open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()

def main():
    prior = json.loads((OLD / 'builds.json').read_text())['programs']
    rows = []
    for benchmark, key in [('fasta', 'fasta:mech-fasta-final-compact-v2'),
                           ('pidigits', 'pidigits:mech-pi-final-compact-signed')]:
        control = prior[key]
        profiles = ['prototype', 'binary-search', 'rebuilt-smallest'] if benchmark == 'fasta' else ['prototype', 'rebuilt-smallest']
        for profile in profiles:
            directory = HERE / 'builds' / (benchmark + '-' + profile)
            directory.mkdir(parents=True, exist_ok=True)
            source = HERE / 'sources' / (benchmark + '.' + profile + '.mec') if profile in ['prototype', 'binary-search'] else Path(control['source'])
            environment = os.environ | control['build_environment'] | {
                'CARGO_TARGET_DIR': TARGET, 'C_INCLUDE_PATH': '/opt/homebrew/include',
                'LIBRARY_PATH': '/opt/homebrew/lib'}
            for name in ['Cargo.toml', 'Cargo.lock', 'build.rs', 'src/main.rs']:
                destination = directory / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(OLD / 'builds' / key.replace(':', '-') / name, destination)
            # Only package identity differs; the application remains the standard ABI CLI.
            manifest = directory / 'Cargo.toml'
            package = 'mech-language-gaps-' + benchmark + '-' + profile
            text = manifest.read_text().replace('name="' + benchmark + '"', 'name="' + package + '"')
            manifest.write_text(text)
            print('BUILD', benchmark, profile, flush=True)
            with (directory / 'build.log').open('w') as log:
                subprocess.run([EMIT, 'control', str(source), str(directory)], env=environment,
                    cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
                subprocess.run([CARGO, 'build', '--release', '--offline', '--manifest-path', str(manifest)],
                    env=environment, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
            shutil.copy2(Path(TARGET) / 'release' / package, directory / 'aot')
            rows.append(dict(benchmark=benchmark, profile=profile, binary=str(directory / 'aot'),
                binary_sha256=sha(directory / 'aot'), source=str(source), source_sha256=sha(source),
                native_object_sha256=sha(directory / 'mech-control.o'),
                old_native_object_sha256=control['native_object_sha256'],
                runtime_environment=control['runtime_environment']))
            (HERE / 'results/prototype-builds.json').write_text(json.dumps(rows, indent=2) + '\n')
    print('Built', len(rows), 'AOT binaries', flush=True)

if __name__ == '__main__':
    main()
