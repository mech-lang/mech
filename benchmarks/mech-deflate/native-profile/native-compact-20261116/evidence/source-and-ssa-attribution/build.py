"""Build the opt-in diagnostic from the exact measured compiler source."""

from hashlib import file_digest
import json
import os
import subprocess
import sys

from stage import CHECKOUT, HEAD, HERE, PACKAGE, Q, REPO, digest, git

TARGET = HERE.parent / 'definition-compiler/target'
OUTPUT = HERE / 'compiler'
COMPILER = OUTPUT / 'compile_profile'
MATCHED = HERE.parent / 'ssa-local-and-first/matched-both/plan.json'
REFERENCE = 'mech-ssa-first-definition'


def read(path):
    return json.loads(path.read_bytes())


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def sha(path):
    with path.open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def main():
    subprocess.run(['git', 'merge-base', '--is-ancestor', HEAD,
                    'refs/heads/codex/native-compact-source-handoff'], cwd=REPO, check=True)
    pins = read(HERE / 'source.json')
    assert pins['base'] == HEAD
    overlay = {}
    for rel, row in pins['sources'].items():
        path = CHECKOUT / rel
        prior = path.read_bytes()
        updated = (HERE / 'source' / rel).read_bytes()
        assert prior == git('show', f'{HEAD}:{rel}')
        assert digest(prior) == row['original'] and digest(updated) == row['attribution']
        overlay[path] = prior, updated
    OUTPUT.mkdir(exist_ok=False)
    command = [sys.executable, '-B', CHECKOUT / PACKAGE / 'rebuild.py', OUTPUT / 'disabled',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    try:
        for file, (_, candidate) in overlay.items():
            file.write_bytes(candidate)
        with (OUTPUT / 'build.log').open('x') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, check=True,
                           stdout=log, stderr=subprocess.STDOUT)
        subprocess.run(['/bin/cp', '-c', TARGET / 'release/compile_profile', COMPILER], check=True)
    finally:
        for file, (original, candidate) in overlay.items():
            assert file.read_bytes() in (original, candidate), file
            file.write_bytes(original)
    expected = read(MATCHED)['actors'][REFERENCE]['artifact_sha256']
    files = {'bytecode': 'frontend/program.mecb', 'encoder': 'native/program.o',
             'metadata': 'native/metadata.json'}
    actual = {part: sha(OUTPUT / 'disabled' / name) for part, name in files.items()}
    assert actual == expected
    save(OUTPUT / 'build.json', {'base': HEAD, 'sources': pins['sources'], 'artifacts': actual,
        'compiler_sha256': sha(COMPILER), 'patch_sha256': sha(HERE / 'instrumentation.patch'),
        'command': list(map(str, command))})
    print('Disabled diagnostic reproduces all three compiler artifacts.', flush=True)


if __name__ == '__main__':
    main()
