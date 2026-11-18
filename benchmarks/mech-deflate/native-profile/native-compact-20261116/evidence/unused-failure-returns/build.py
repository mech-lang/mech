"""Test suppressing unreferenced failure returns in the checked root builder."""
from hashlib import file_digest
import difflib
import json
import os
from pathlib import Path
from shutil import copy2
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
CHECKOUT = Q / 'native-review-pr/checkout'
PACKET = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
BASELINE = HERE.parent / 'statement-compiler'
TARGET = HERE.parent / 'definition-compiler/target'
HEAD = 'e8b2bb4f2d8c44390e9a67dfe054c6ff40341249'
PATH = Path('hosts/gpu/src/native_artifact/program/compact/emit.rs')
FLAG = 'MECH_COMPACT_SKIP_UNUSED_FAILURE_RETURNS'


def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def write(path, item):
    path.write_text(json.dumps(item, indent=2) + '\n')


def git(*args):
    return subprocess.check_output(['git', *args], cwd=Q.parent)


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def stage():
    assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').decode().strip() == HEAD
    original = git('show', f'{HEAD}:{PATH}')
    assert (CHECKOUT / PATH).read_bytes() == original
    assert b'\n' == original[-1:]
    body = original.decode()
    body = replace(body, '    entry: Block,\n    last: &mut Option<Block>,',
                   '    entry: Block,\n    sealed_error: Option<Block>,\n    last: &mut Option<Block>,')
    body = replace(body, '        if block != entry && !source.contains(&block) {',
                   '        if block != entry && Some(block) != sealed_error && !source.contains(&block) {')
    body = replace(body, '        seal_auxiliary(&mut b, &source_blocks, entry, &mut last_layout);\n        // One shared',
                   '        seal_auxiliary(&mut b, &source_blocks, entry, None, &mut last_layout);\n        // One shared')
    body = replace(body, '        eprintln!("compact-stage: source CFG");',
                   f'''        let skip_unused_failure_returns =
            outline_allowed && std::env::var_os("{FLAG}").is_some();
        let mut unused_failure_returns = 0;
        eprintln!("compact-stage: source CFG");''')
    existing = '''            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            let at = b.ins().iconst(types::I64, site as i64);
            b.ins().return_(&[status, at]);
'''
    modified = '''            b.switch_to_block(error);
            let sealed_error = if skip_unused_failure_returns {
                // All branches to this local failure block are known after its
                // source site has emitted ordinary and specialized edges.
                b.seal_block(error);
                Some(error)
            } else {
                None
            };
            if !skip_unused_failure_returns || !b.is_unreachable() {
                let status = b.block_params(error)[0];
                let at = b.ins().iconst(types::I64, site as i64);
                b.ins().return_(&[status, at]);
            } else {
                unused_failure_returns += 1;
            }
'''
    body = replace(body, existing, modified)
    body = replace(body, '''            seal_auxiliary(&mut b, &source_blocks, entry, &mut last_layout);
        }
        assert!(remaining.iter().all(|&n| n == 0));''', '''            seal_auxiliary(&mut b, &source_blocks, entry, sealed_error, &mut last_layout);
        }
        if skip_unused_failure_returns {
            eprintln!("compact unused failure returns skipped: {unused_failure_returns}");
        }
        assert!(remaining.iter().all(|&n| n == 0));''')
    overlay = HERE / 'overlay' / PATH
    overlay.parent.mkdir(parents=True)
    overlay.write_text(body)
    args = ['rustfmt', '+nightly-2026-03-03', '--edition', '2024', '--config', 'skip_children=true', overlay]
    subprocess.run(args, check=True)
    patch = ''.join(difflib.unified_diff(original.decode().splitlines(keepends=True),
                                        overlay.read_text().splitlines(keepends=True), f'a/{PATH}', f'b/{PATH}'))
    assert 'skip_unused_failure_returns' in patch and len(patch.splitlines()) < 110
    (HERE / 'failure-returns.patch').write_text(patch)
    write(HERE / 'source-bindings.json', {'base': HEAD, 'source_path': str(PATH), 'source_sha256': sha(overlay),
                                         'patch_sha256': sha(HERE / 'failure-returns.patch')})
    return overlay, original


def build(overlay, original):
    proof = HERE / 'candidate'
    proof.mkdir()
    current = CHECKOUT / PATH
    pending = current.with_suffix('.skip-unused-failure-returns')
    copy2(overlay, pending)
    os.replace(pending, current)
    command = [sys.executable, '-B', PACKET / 'rebuild.py', proof / 'rebuild',
               '--variant', 'fresh-owner-counts-cold-tails-zero-branches', '--compiler-opt-level', '3',
               '--direct-source', '--preselect-root-inlining', '--skip-refused-general-ssa',
               '--target-dir', TARGET]
    try:
        env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
        with (proof / 'rebuild.log').open('wb') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log, stderr=subprocess.STDOUT,
                           check=True)
        compiler = proof / 'compile_profile'
        copy2(TARGET / 'release/compile_profile', compiler)
        assert sha(compiler) != sha(BASELINE / 'candidate/compile_profile')
        write(proof / 'build.json', {'source_pin': HEAD, 'compiler_sha256': sha(compiler), 'command': list(map(str, command)),
                                     'ablation_default': 'Materialize every source-site error return.'})
    finally:
        assert current.read_bytes() == overlay.read_bytes()
        pending.write_bytes(original)
        os.replace(pending, current)
    print(compiler, sha(compiler), flush=True)


if __name__ == '__main__':
    build(*stage())
