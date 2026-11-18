"""Build the guard closure probe with the current compiler recipe."""

from hashlib import file_digest
import json
import os
from pathlib import Path
from shutil import copy2
import subprocess
import sys

from stage import BASE, FLAG, HERE, Q, ROOT

CHECKOUT = Q / 'native-review-pr/checkout'
PLAN = HERE.parent / 'ascii-literal-miss/production/matched/plan.json'
PROBE = HERE / 'probe-compiler'
COMPILER = PROBE / 'compile_profile'
TARGET = HERE.parent / 'definition-compiler/target'
BENCH = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'


def read(path):
    return json.loads(path.read_bytes())


def sha(path):
    with Path(path).open('rb') as src:
        return file_digest(src, 'sha256').hexdigest()


def save(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def main():
    source = read(HERE / 'source.json')
    plan = read(PLAN)['actors']['mech-ascii-miss']
    assert source['base'] == BASE and source['flag'] == FLAG
    assert subprocess.check_output(['git', 'rev-parse', 'refs/heads/codex/native-compact-source-handoff'],
                                   cwd=ROOT).decode().strip() == BASE
    edits = {}
    for path, digest in source['source_sha256'].items():
        prior = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT)
        local = CHECKOUT / path
        replaced = HERE / 'source' / path
        assert prior == local.read_bytes() and sha(replaced) == digest
        edits[local] = (prior, replaced)
    PROBE.mkdir()
    command = [sys.executable, '-B', BENCH / 'rebuild.py', PROBE / 'disabled',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    clean = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    try:
        for local, (_, replacement) in edits.items():
            copy2(replacement, local)
        with (PROBE / 'build.log').open('wb') as out:
            subprocess.run(command, env=clean | {'CARGO_NET_OFFLINE': 'true'},
                           stdout=out, stderr=subprocess.STDOUT, check=True)
        copy2(TARGET / 'release/compile_profile', COMPILER)
    finally:
        for local, (prior, replacement) in edits.items():
            assert local.read_bytes() in (prior, replacement.read_bytes())
            local.write_bytes(prior)
    roles = {'encoder': 'native/program.o', 'metadata': 'native/metadata.json',
             'bytecode': 'frontend/program.mecb'}
    observed = {key: sha(PROBE / 'disabled' / name) for key, name in roles.items()}
    assert observed == plan['artifact_sha256']
    result = source | {'compiler_sha256': sha(COMPILER), 'disabled_artifacts': observed,
                       'plan_sha256': sha(PLAN), 'patch_sha256': sha(HERE / 'guard-index.patch'),
                       'build_command': list(map(str, command))}
    save(PROBE / 'result.json', result)
    print('Guard probe with optimisation off reproduced all three artifacts', flush=True)


if __name__ == '__main__':
    main()
