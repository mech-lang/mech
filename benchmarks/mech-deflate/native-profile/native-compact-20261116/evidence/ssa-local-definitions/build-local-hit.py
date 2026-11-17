"""Build and verify the already-resolved SSA definition fast path."""

from difflib import unified_diff
from hashlib import file_digest
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from stage import BASE as STAGED_BASE, HERE, PATH, Q, ROOT, PROFILE, local_hit

BASE = '8fb7fa9683a3034866104db196ca5a72d7d312da'
BRANCH = 'refs/heads/codex/native-compact-source-handoff'
CHECKOUT = Q / 'native-review-pr/checkout'
TARGET = HERE.parent / 'definition-compiler/target'
BENCH = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
PUBLISHED = HERE.parent / 'ascii-literal-miss/production/matched/plan.json'
OUT = HERE / 'local-hit/build'
SOURCE = OUT / 'source'


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def save(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def git(*args):
    return subprocess.check_output(['git', *map(str, args)], cwd=ROOT)


def rust_dependency(target):
    receipt = HERE.parent / 'ascii-literal-miss/production/rebuild/compiler-build.jsonl'
    matches = []
    for line in receipt.read_text().splitlines():
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if record.get('target', {}).get('name') == target:
            matches.extend(Path(name) for name in record['filenames'] if name.endswith('.rlib'))
    (product,) = matches
    assert product.is_file() and product.is_relative_to(TARGET)
    return product


def tests():
    test_binary = OUT / 'test-ssa'
    root_source = OUT / 'test-ssa.rs'
    module = CHECKOUT / PATH.with_name('variable.rs')
    root_source.write_text(f"""#![allow(dead_code)]
extern crate alloc;
#[path = "{module}"]
mod variable;
use variable::Variable;
#[path = "{SOURCE / PATH}"]
mod ssa;
""")
    dep = rust_dependency('cranelift_codegen')
    command = ['rustc', '+nightly-2026-03-03', '--edition=2024', '--test', '-C', 'opt-level=2',
               '-L', f'dependency={dep.parent}', '--extern', f'cranelift_codegen={dep}', str(root_source),
               '-o', str(test_binary)]
    with (OUT / 'tests.log').open('wb') as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
        subprocess.run([test_binary], stdout=log, stderr=subprocess.STDOUT, check=True)
    observed = re.search(r'test result: ok\. (\d+) passed; 0 failed', (OUT / 'tests.log').read_text())
    assert observed and int(observed[1]) == 15
    return {'tests_passed': int(observed[1]), 'command': command,
            'test_harness_sha256': sha(root_source), 'tests_log_sha256': sha(OUT / 'tests.log'),
            'dependency_sha256': sha(dep)}


def main():
    assert git('rev-parse', BRANCH).decode().strip() == BASE
    original = git('show', f'{BASE}:{PATH}')
    assert git('show', f'{STAGED_BASE}:{PATH}') == original
    assert (CHECKOUT / PATH).read_bytes() == original
    source = SOURCE / PATH
    source.parent.mkdir(parents=True)
    source.write_text(local_hit(original.decode()))
    subprocess.run(['rustfmt', '--edition=2024', '--config', 'skip_children=true', source], check=True)
    patch = ''.join(unified_diff(original.decode().splitlines(keepends=True),
        source.read_text().splitlines(keepends=True), f'a/{PATH}', f'b/{PATH}'))
    patch_file = OUT / 'compiler.patch'
    patch_file.write_text(patch)
    verified = tests()
    command = [sys.executable, '-B', BENCH / 'rebuild.py', OUT / 'compiled', '--variant', 'frame-copies',
               '--compiler-opt-level', '3', '--direct-source', '--preselect-root-inlining',
               '--skip-refused-general-ssa', '--target-dir', TARGET]
    clean = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    local = CHECKOUT / PATH
    updated = source.read_bytes()
    try:
        local.write_bytes(updated)
        with (OUT / 'build.log').open('wb') as log:
            subprocess.run(command, env=clean | {'CARGO_NET_OFFLINE': 'true'},
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        executable = OUT / 'compile_profile'
        subprocess.run(['/bin/cp', '-c', TARGET / 'release/compile_profile', executable], check=True)
    finally:
        assert local.read_bytes() in (original, updated)
        local.write_bytes(original)
    compiled = OUT / 'compiled'
    artifact_paths = {'encoder': 'native/program.o', 'metadata': 'native/metadata.json',
                      'bytecode': 'frontend/program.mecb'}
    artifacts = {key: sha(compiled / filename) for key, filename in artifact_paths.items()}
    control = read(PUBLISHED)['actors']['mech-ascii-miss']
    assert artifacts == control['artifact_sha256']
    record = {'base': BASE, 'path': str(PATH), 'source_sha256': sha(source),
              'patch_sha256': sha(patch_file), 'sample_sha256': sha(PROFILE),
              'build_plan_sha256': sha(PUBLISHED), 'compiler_sha256': sha(executable),
              'artifacts_sha256': artifacts, 'command': list(map(str, command)), 'tests': verified}
    save(OUT / 'result.json', record)
    print('Verified exact bytecode, object, and metadata after shortcut', flush=True)


if __name__ == '__main__':
    main()
