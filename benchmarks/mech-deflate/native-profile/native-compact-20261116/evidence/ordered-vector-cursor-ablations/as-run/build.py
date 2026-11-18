"""Build the probe compiler and verify the emitted control with its switch off."""

from hashlib import file_digest
import json
import os
from shutil import copy2
import subprocess
import sys

from stage import BASE, HERE, Q, ROOT

CHECKOUT = Q / 'native-review-pr/checkout'
PACKET = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
TARGET = HERE.parent / 'definition-compiler/target'
OUT = HERE / 'probe-compiler'
COMPILER = OUT / 'compile_profile'
CONTROL_PLAN = HERE.parent / 'ascii-literal-miss/production/matched/plan.json'


def read(path):
    return json.loads(path.read_bytes())


def sha(path):
    with path.open('rb') as f:
        return file_digest(f, 'sha256').hexdigest()


def save(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def main():
    source_root = HERE / 'single-address' if sys.argv[1:] == ['single-address'] else HERE
    output = source_root / 'probe-compiler'
    pins = read(source_root / 'source.json')
    assert pins['base'] == BASE
    assert subprocess.check_output(['git', 'rev-parse', 'refs/heads/codex/native-compact-source-handoff'],
                                   cwd=ROOT).decode().strip() == BASE
    control = read(CONTROL_PLAN)['actors']['mech-ascii-miss']
    assert control['compile'][0] == pins['compiler_control']
    assert control['compiler_sha256'] == pins['compiler_control_sha256']
    exit_writes = {}
    for path, digest in pins['source_sha256'].items():
        prior = subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT)
        local = CHECKOUT / path
        source = source_root / 'source' / path
        assert local.read_bytes() == prior and sha(source) == digest, path
        exit_writes[local] = (prior, source)
    output.mkdir()
    command = [sys.executable, '-B', PACKET / 'rebuild.py', output / 'disabled',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
    try:
        for local, (_, source) in exit_writes.items():
            copy2(source, local)
        with (output / 'build.log').open('wb') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, check=True,
                           stdout=log, stderr=subprocess.STDOUT)
        copy2(TARGET / 'release/compile_profile', output / 'compile_profile')
    finally:
        for local, (prior, source) in exit_writes.items():
            assert local.read_bytes() in (prior, source.read_bytes()), local
            local.write_bytes(prior)
    files = {'frontend/program.mecb': 'bytecode', 'native/program.o': 'encoder',
             'native/metadata.json': 'metadata'}
    emitted = {label: sha(output / 'disabled' / name) for name, label in files.items()}
    assert emitted == control['artifact_sha256'], (emitted, control['artifact_sha256'])
    result = pins | {'compiler_sha256': sha(output / 'compile_profile'), 'command': list(map(str, command)),
                     'disabled_artifacts': emitted, 'prior_plan_sha256': sha(CONTROL_PLAN),
                     'source_patch_sha256': sha(source_root / 'cursor.patch'), 'validated': True}
    save(output / 'result.json', result)
    print('Disabled-mode bytecode, object and metadata reproduced', flush=True)


if __name__ == '__main__':
    main()
