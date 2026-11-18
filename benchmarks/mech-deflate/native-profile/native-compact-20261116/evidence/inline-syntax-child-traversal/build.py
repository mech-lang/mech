"""Rebuild this candidate and check its bytecode, object and metadata."""

import os
from pathlib import Path
import subprocess
import sys

from stage import BASE, BRANCH, CHECKOUT, HERE, INCUMBENT, PACKET, PATCH, PLAN, SOURCE, TARGET
from stage import git, read, sha, write

OUT = HERE / 'compiler'
BINARY = OUT / 'compile_profile'


def main():
    assert git('rev-parse', BRANCH).decode().strip() == BASE
    binding = read(HERE / 'source.json')
    tests = read(HERE / 'tests/result.json')
    assert binding['base'] == BASE and binding['patch_sha256'] == sha(PATCH)
    assert tests['source_sha256'] == binding['candidate_sha256'] and tests['passed'] == 3
    local = CHECKOUT / SOURCE
    prior = git('show', f'{BASE}:{SOURCE}')
    measured = HERE / 'source' / SOURCE
    assert local.read_bytes() == prior and sha(local) == binding['control_sha256']
    assert sha(measured) == binding['candidate_sha256']
    previous = local.stat()
    OUT.mkdir()
    build = OUT / 'rebuilt'
    command = [sys.executable, '-B', CHECKOUT / PACKET / 'rebuild.py', build,
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    try:
        local.write_bytes(measured.read_bytes())
        with (OUT / 'build.log').open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True,
                           env=env | {'CARGO_NET_OFFLINE': 'true'})
        binary = TARGET / 'release' / 'compile_profile'
        assert os.access(binary, os.X_OK) and binary.stat().st_size > 10_000_000
        subprocess.run(['/bin/cp', '-c', binary, BINARY], check=True)
    finally:
        assert local.read_bytes() in (prior, measured.read_bytes())
        local.write_bytes(prior)
        os.utime(local, ns=(previous.st_atime_ns, previous.st_mtime_ns))
    files = {'bytecode': 'frontend/program.mecb', 'encoder': 'native/program.o', 'metadata': 'native/metadata.json'}
    artifacts = {role: sha(build / name) for role, name in files.items()}
    current = read(PLAN)['settings'][INCUMBENT]
    assert sha(current['compile'][0]) == current['compiler_sha256']
    assert artifacts == current['artifact_sha256'], (artifacts, current['artifact_sha256'])
    result = {'base': BASE, 'source': binding, 'tests': tests, 'compiler_sha256': sha(BINARY),
              'command': list(map(str, command)), 'artifacts': artifacts,
              'comparison_plan_sha256': sha(PLAN),
              'rebuild_sha256': sha(build / 'rebuild.json'),
              'compiler_build_log_sha256': sha(build / 'compiler-build.jsonl')}
    write(OUT / 'result.json', result)
    print(BINARY, result['compiler_sha256'], 'reproduces the pinned bytecode, object and metadata.', flush=True)


if __name__ == '__main__':
    main()
