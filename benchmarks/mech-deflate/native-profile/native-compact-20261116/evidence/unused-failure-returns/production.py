"""Stage the verified omit-unused behavior without an experimental flag."""
import difflib
import json
import os
from shutil import copy2
import subprocess
import sys

from build import CHECKOUT, FLAG, HEAD, HERE, PACKET, PATH, TARGET, git, replace, sha, write

OLD = HERE.parent / 'statement-compiler/candidate'
OUT = HERE / 'production'


def stage():
    before = git('show', f'{HEAD}:{PATH}')
    assert before == (CHECKOUT / PATH).read_bytes()
    prototype = HERE / 'overlay' / PATH
    binding = json.loads((HERE / 'source-bindings.json').read_bytes())
    assert sha(prototype) == binding['source_sha256']
    candidate = prototype.read_text()
    first = f'''        let skip_unused_failure_returns = outline_allowed
            && std::env::var_os("{FLAG}").is_some();
        let mut unused_failure_returns = 0;
'''
    candidate = replace(candidate, first, '')
    candidate = replace(candidate, 'let sealed_error = if skip_unused_failure_returns {',
                        'let sealed_error = if outline_allowed {')
    candidate = replace(candidate, 'if !skip_unused_failure_returns || !b.is_unreachable() {',
                        'if sealed_error.is_none() || !b.is_unreachable() {')
    candidate = replace(candidate, '            } else {\n                unused_failure_returns += 1;\n', '')
    candidate = replace(candidate, '''        if skip_unused_failure_returns {
            eprintln!("compact unused failure returns skipped: {unused_failure_returns}");
        }
''', '')
    assert 'unused_failure_returns' not in candidate
    path = OUT / 'source' / PATH
    path.parent.mkdir(parents=True)
    path.write_text(candidate)
    subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition', '2024',
                    '--config', 'skip_children=true', path], check=True)
    patch = ''.join(difflib.unified_diff(before.decode().splitlines(keepends=True),
                                        path.read_text().splitlines(keepends=True), f'a/{PATH}', f'b/{PATH}'))
    assert len(patch.splitlines()) <= 68
    (OUT / 'omit-unused-failure-returns.patch').write_text(patch)
    write(OUT / 'source-bindings.json', {'base': HEAD, 'patch_sha256': sha(OUT / 'omit-unused-failure-returns.patch'),
                                         'production_sources': {str(PATH): sha(path)},
                                         'prototype_sha256': sha(prototype)})


def build():
    path = OUT / 'source' / PATH
    current = CHECKOUT / PATH
    old = git('show', f'{HEAD}:{PATH}')
    assert current.read_bytes() == old
    assert sha(path) == json.loads((OUT / 'source-bindings.json').read_text())['production_sources'][str(PATH)]
    assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').strip().decode() == HEAD
    pending = current.with_suffix('.no-unused-errors')
    copy2(path, pending)
    os.replace(pending, current)
    success = False
    try:
        command = [sys.executable, '-B', PACKET / 'rebuild.py', OUT / 'rebuild',
                   '--variant', 'fresh-owner-counts-cold-tails-zero-branches',
                   '--compiler-opt-level', '3', '--direct-source', '--preselect-root-inlining',
                   '--skip-refused-general-ssa', '--target-dir', TARGET]
        env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
        with (OUT / 'rebuild.log').open('wb') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'},
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        compiler = OUT / 'compile_profile'
        copy2(TARGET / 'release/compile_profile', compiler)
        assert sha(compiler) != sha(OLD / 'compile_profile')
        for name in ('program.o', 'metadata.json'):
            assert sha(OUT / 'rebuild/native' / name) == sha(OLD / 'rebuild/native' / name)
        assert sha(OUT / 'rebuild/frontend/program.mecb') == sha(OLD / 'rebuild/frontend/program.mecb')
        (OUT / 'compiler.sha256').write_text(sha(compiler) + '\n')
        success = True
        print(compiler, sha(compiler), flush=True)
    finally:
        if not success:
            assert current.read_bytes() == path.read_bytes()
            pending.write_bytes(old)
            os.replace(pending, current)


if __name__ == '__main__':
    {'stage': stage, 'build': build}[sys.argv[1]]()
