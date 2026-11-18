"""Build the cost-length probe with the current LLVM-free compiler recipe."""

import os
from pathlib import Path
import subprocess
import sys

from stage import BASE, BRANCH, CHECKOUT, HERE, PACKET, ROADMAP, git, read, sha, write

OUT = HERE / 'compiler'
TARGET = ROADMAP / 'definition-compiler/target'
COMPILER = OUT / 'compile_profile'
PLAN = ROADMAP / 'immutable-root-literals/matched/plan.json'
REFERENCE = 'mech-literals-enabled'


def main():
    assert git('rev-parse', BRANCH).decode().strip() == BASE
    plan = read(PLAN)
    incumbent = plan['settings'][REFERENCE]
    assert sha(incumbent['compile'][0]) == incumbent['compiler_sha256']
    pins = read(HERE / 'source.json')
    assert pins['base'] == BASE and pins['patch_sha256'] == sha(HERE / 'compiler.patch')
    overlay = {}
    for rel, checks in pins['files'].items():
        path = CHECKOUT / rel
        original = git('show', f'{BASE}:{rel}')
        probe = HERE / 'source' / rel
        assert path.read_bytes() == original and sha(path) == checks['current']
        assert sha(probe) == checks['probe']
        overlay[path] = (path.stat(), original, probe.read_bytes())
    OUT.mkdir()
    build = OUT / 'disabled'
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    command = [sys.executable, '-B', CHECKOUT / PACKET / 'rebuild.py', build,
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    try:
        for path, (_, _, source) in overlay.items():
            path.write_bytes(source)
        with (OUT / 'build.log').open('w') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'},
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        compiler = TARGET / 'release/compile_profile'
        assert os.access(compiler, os.X_OK) and compiler.stat().st_size >= 10_000_000
        subprocess.run(['/bin/cp', '-c', compiler, COMPILER], check=True)
    finally:
        for path, (times, original, candidate) in overlay.items():
            assert path.read_bytes() in (original, candidate), path
            path.write_bytes(original)
            os.utime(path, ns=(times.st_atime_ns, times.st_mtime_ns))
    artifacts = {role: sha(build / name) for role, name in (
        ('bytecode', 'frontend/program.mecb'), ('encoder', 'native/program.o'),
        ('metadata', 'native/metadata.json'))}
    assert artifacts == incumbent['artifact_sha256'], (artifacts, incumbent['artifact_sha256'])
    write(OUT / 'result.json', {'base': BASE, 'compiler_sha256': sha(COMPILER),
        'artifacts_disabled': artifacts, 'source': pins, 'command': list(map(str, command)),
        'comparison_plan_sha256': sha(PLAN), 'cargo_log_sha256': sha(build / 'compiler-build.jsonl'),
        'rebuild_sha256': sha(build / 'rebuild.json')})
    print('Reproduced the maintained encoder and metadata with both options off.', flush=True)


if __name__ == '__main__':
    main()
