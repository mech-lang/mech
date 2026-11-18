"""Build the exact two-file parser overlay and qualify the existing native output."""
from hashlib import file_digest
import json
import os
from pathlib import Path
from shutil import copy2, copyfile
import subprocess
import sys

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
CHECKOUT = Q / 'native-review-pr/checkout'
PACKET = CHECKOUT / 'benchmarks/mech-deflate/native-profile/native-compact-20261116'
OLD = HERE.parent / 'definition-compiler'
TARGET = OLD / 'target'
SAMPLE = Path('/private/tmp/mech-nested-function-replay.M7wroPHT')
HEAD = 'ff7c1dbc9c5059ed84d730b008847c02179fa2cf'


def sha(path):
    with path.open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def git(*args):
    return subprocess.check_output(['git', *args], cwd=Q.parent)


def main():
    assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').decode().strip() == HEAD
    comparison = json.loads((SAMPLE / 'measured/comparison.json').read_text())
    assert comparison['paired_ratio_to_control']['candidate'] < 1
    assert '535 passed; 0 failed' in (SAMPLE / 'validation/suite.log').read_text()
    assert '3 passed; 0 failed' in (SAMPLE / 'validation/focused.log').read_text()
    proof = HERE / 'candidate'
    proof.mkdir()
    binding = json.loads((SAMPLE / 'source-bindings.json').read_text())
    assert binding['pin'] == HEAD
    assert len(binding['production_sources']) == 2
    patch = SAMPLE / 'function-replay.patch'
    assert binding['patch'] == sha(patch)
    original = {}
    overlay = {}
    for name, expected in binding['production_sources'].items():
        assert name.startswith('src/syntax/src/document/parser/canonical/document/')
        current = CHECKOUT / name
        source = SAMPLE / 'candidate/workspace' / name
        assert sha(source) == expected
        original[current] = current.read_bytes()
        assert original[current] == git('show', f'{HEAD}:{name}')
        overlay[current] = source
    for current, source in overlay.items():
        temp = current.with_suffix('.function-replay')
        copyfile(source, temp)
        os.replace(temp, current)
    copy2(patch, proof / 'function-replay.patch')
    copy2(SAMPLE / 'source-bindings.json', proof / 'source-bindings.json')
    command = [sys.executable, '-B', PACKET / 'rebuild.py', proof / 'rebuild',
               '--variant', 'fresh-owner-counts-cold-tails-zero-branches', '--compiler-opt-level', '3',
               '--direct-source', '--preselect-root-inlining', '--skip-refused-general-ssa',
               '--target-dir', TARGET]
    try:
        with (proof / 'rebuild.log').open('wb') as log:
            subprocess.run(command, env=os.environ | {'CARGO_NET_OFFLINE': 'true'},
                           stdout=log, stderr=subprocess.STDOUT, check=True)
        compiler = proof / 'compile_profile'
        copy2(TARGET / 'release/compile_profile', compiler)
        before = sha(OLD / 'candidate/compile_profile')
        assert before == (OLD / 'candidate/compiler.sha256').read_text().strip()
        assert sha(compiler) != before, 'cached control compiler reused'
        (proof / 'compiler.sha256').write_text(sha(compiler) + '\n')
    except BaseException:
        for current, data in original.items():
            if current.read_bytes() == overlay[current].read_bytes():
                temporary = current.with_suffix('.function-replay-restore')
                temporary.write_bytes(data)
                os.replace(temporary, current)
        raise
    print(compiler, sha(compiler), flush=True)


if __name__ == '__main__':
    main()
