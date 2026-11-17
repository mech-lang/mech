"""Compile the caller-address experiment and verify its disabled encoder."""
from difflib import unified_diff
import os
from pathlib import Path
from shutil import copy2
import subprocess
import sys

from emit import CHECKOUT, HERE, PIN, PLAN, Q, ROOT, ROADMAP, HORIZONTAL, REVERSE, emit, read, save, sha

SOURCE = Path('hosts/gpu/src/native_artifact/program')
PATHS = [SOURCE / name for name in (
    'codegen/cfg_cleanup/composed_reads.rs', 'codegen/cfg_cleanup/mod.rs',
    'compact/phase_function.rs', 'compact/phases.rs',
)]
PACKET = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
TARGET = ROADMAP / 'definition-compiler/target'
OUT = HERE / 'address-compiler'
FLAG = {'MECH_COMPOSE_CALLER_ADDRESS_WORDS': '1'}
NO_STALE_PROFILE = ('MECH_COMPACT_ROOT_TAIL_PROFILE',)


def build():
    OUT.mkdir()
    reference = read(PLAN)['actors']['mech-verified-flowgraph']
    patch = []
    tracked = {}
    for path in PATHS:
        old = subprocess.check_output(['git', 'show', f'{PIN}:{path}'], cwd=ROOT).decode()
        current = (CHECKOUT / path).read_text()
        assert old != current
        patch.extend(unified_diff(old.splitlines(keepends=True), current.splitlines(keepends=True), f'a/{path}', f'b/{path}'))
        tracked[str(path)] = sha(CHECKOUT / path)
    (OUT / 'address-words.patch').write_text(''.join(patch))
    command = [sys.executable, '-B', PACKET / 'rebuild.py', OUT / 'rebuild',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
    with (OUT / 'build.log').open('wb') as log:
        subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    compiler = OUT / 'compile_profile'
    copy2(TARGET / 'release/compile_profile', compiler)
    pins = reference['artifact_sha256']
    for path, label in [('native/program.o', 'encoder'), ('native/metadata.json', 'metadata'),
                        ('frontend/program.mecb', 'bytecode')]:
        assert sha(OUT / 'rebuild' / path) == pins[label], path
    assert all(sha(CHECKOUT / key) == value for key, value in tracked.items())
    save(OUT / 'build.json', {'source_pin': PIN, 'compiler_sha256': sha(compiler), 'source_sha256': tracked,
                             'build_command': list(map(str, command)), 'disabled_artifacts': pins,
                             'patch_sha256': sha(OUT / 'address-words.patch')})
    print('disabled encoder and metadata reproduced', sha(compiler), flush=True)


if __name__ == '__main__':
    for step in sys.argv[1:]:
        if step == 'build':
            build()
        elif step == 'emit':
            compiler = OUT / 'compile_profile'
            assert sha(compiler) == read(OUT / 'build.json')['compiler_sha256']
            emit('caller-addresses', HORIZONTAL | REVERSE | FLAG, compiler, drop_flags=NO_STALE_PROFILE)
        else:
            raise ValueError(step)
