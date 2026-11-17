"""Build and exercise literal-backed SSA against the frozen source and peers."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys

from stage import BASE, CHECKOUT, FILES, HERE, PACKAGE, Q, ROADMAP, ROOT, WORDS, digest, git

OUTPUT = HERE / 'compiler'
BINARY = OUTPUT / 'compile_profile'
TARGET = ROADMAP / 'definition-compiler/target'
OLD_BUILD = ROADMAP / 'semantic-and-ssa-attribution/compiler/disabled/compiler-build.jsonl'
MATCHED = ROADMAP / 'ssa-local-and-first/matched-both/plan.json'
REFERENCE = 'mech-ssa-first-definition'
MODE = 'MECH_COMPACT_DIRECT_ENTRY_LITERALS'


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def tests():
    candidates = {}
    for line in OLD_BUILD.read_text().splitlines():
        if not line.startswith('{'):
            continue
        item = json.loads(line)
        if item.get('reason') == 'compiler-artifact':
            name = item['target']['name']
            if name in ('cranelift_codegen', 'cranelift_frontend'):
                (candidates[name],) = [file for file in item['filenames'] if file.endswith('.rlib')]
    assert len(candidates) == 2 and all(Path(path).is_file() for path in candidates.values())
    harness = OUTPUT / 'ssa-word-harness.rs'
    module = HERE / 'source' / WORDS
    harness.write_text(f'''#![allow(dead_code)]

mod native_artifact {{
    #[derive(Debug)]
    pub struct NativeCompileError;

    mod compiler {{
        use super::NativeCompileError;

        pub(crate) fn offset(slot: usize) -> Result<i32, NativeCompileError> {{
            slot.checked_mul(8).and_then(|n| i32::try_from(n).ok()).ok_or(NativeCompileError)
        }}
    }}

    mod codegen {{
        mod control {{
            pub mod plan {{
                use super::super::super::NativeCompileError;

                pub struct CodePlan<'a>(pub &'a [u64]);
                pub struct Slot(u32);
                impl Slot {{
                    pub fn get(&self) -> u32 {{
                        self.0
                    }}
                }}
                impl CodePlan<'_> {{
                    pub fn constant_count(&self) -> usize {{
                        self.0.len()
                    }}
                    pub fn constant(&self, index: usize) -> Result<(Slot, Option<u64>), NativeCompileError> {{
                        Ok((Slot(index as u32), Some(self.0[index])))
                    }}
                }}
            }}
        }}
        #[path = "{module}"]
        mod ssa_words;
    }}
}}
''')
    binary = OUTPUT / 'test-ssa-words'
    command = ['rustc', '+nightly-2026-03-03', '--edition=2024', '--test', '-C', 'opt-level=2',
               '--cfg', 'feature="canonical-native-program"', '-L', f'dependency={TARGET / "release/deps"}']
    for name, path in candidates.items():
        command.extend(('--extern', f'{name}={path}'))
    command.extend((str(harness), '-o', str(binary)))
    with (OUTPUT / 'tests.log').open('w') as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
        subprocess.run([binary], stdout=log, stderr=subprocess.STDOUT, check=True)
    log = OUTPUT / 'tests.log'
    assert '2 passed; 0 failed' in log.read_text()
    return {'command': command, 'passed': 2, 'log_sha256': sha(log), 'harness_sha256': sha(harness)}


def artifacts(folder):
    locations = {'bytecode': 'program.mecb', 'encoder': 'program.o', 'metadata': 'metadata.json'}
    return {role: sha(folder / filename) for role, filename in locations.items()}


def enabled(expected):
    plan = read(MATCHED)['actors'][REFERENCE]
    assert plan['artifact_sha256'] == expected
    root = OUTPUT / 'enabled'
    root.mkdir(exist_ok=False)
    target = root / 'frontend'
    command = [BINARY, Path(plan['compile'][1]), target, 'compact']
    flags = plan['flags'] | {MODE: '1'}
    assert all('LLVM' not in name for name in flags)
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    with (root / 'log.txt').open('w') as log:
        subprocess.run(command, env=env | flags, stdout=log, stderr=subprocess.STDOUT, check=True)
    log = root / 'log.txt'
    fields = log.read_text().splitlines()
    counts = [int(s.rsplit('=', 1)[1]) for s in fields if s.startswith('compact-stage: direct scalar literals=')]
    assert counts, 'missing attempted literal count'
    assert not any('direct literal attempt withdrew' in line for line in fields), 'candidate used fallback'
    assert artifacts(target)['bytecode'] == expected['bytecode']
    return {'command': list(map(str, command)), 'flags': flags, 'artifacts': artifacts(target),
            'literal_counts': counts, 'logs_sha256': sha(log)}


def main():
    assert git('rev-parse', 'refs/heads/codex/native-compact-source-handoff').decode().strip() == BASE
    pins = read(HERE / 'source.json')
    overlay = {}
    for rel_str, pin in pins['files'].items():
        path = Path(rel_str)
        baseline = git('show', f'{BASE}:{path}')
        candidate = (HERE / 'source' / path).read_bytes()
        assert (CHECKOUT / path).read_bytes() == baseline
        assert digest(baseline) == pin['baseline'] and digest(candidate) == pin['probe']
        overlay[path] = baseline, candidate
    OUTPUT.mkdir(exist_ok=False)
    suite = tests()
    command = [sys.executable, '-B', CHECKOUT / PACKAGE / 'rebuild.py', OUTPUT / 'disabled',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    try:
        for p, (_, candidate) in overlay.items():
            (CHECKOUT / p).write_bytes(candidate)
        with (OUTPUT / 'build.log').open('w') as log:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, stdout=log,
                           stderr=subprocess.STDOUT, check=True)
        subprocess.run(['/bin/cp', '-c', TARGET / 'release/compile_profile', BINARY], check=True)
    finally:
        for p, (baseline, candidate) in overlay.items():
            local = CHECKOUT / p
            assert local.read_bytes() in (baseline, candidate)
            local.write_bytes(baseline)
    expected = read(MATCHED)['actors'][REFERENCE]['artifact_sha256']
    rebuild = OUTPUT / 'disabled'
    actual = {'bytecode': sha(rebuild / 'frontend/program.mecb'),
              'encoder': sha(rebuild / 'native/program.o'), 'metadata': sha(rebuild / 'native/metadata.json')}
    assert actual == expected
    probe = enabled(expected)
    report = {'base': BASE, 'compiler_sha256': sha(BINARY), 'files': pins['files'],
              'build_command': list(map(str,command)), 'tests': suite, 'disabled': actual,
              'enabled': probe, 'exact': probe['artifacts'] == actual,
              'source_patch_sha256': sha(HERE / 'compiler.patch')}
    save(OUTPUT / 'result.json', report)
    print(json.dumps({'base': BASE, 'exact': report['exact'], 'literal_counts': probe['literal_counts'],
                      'tests': suite['passed'], 'enabled': probe['artifacts']}, indent=2), flush=True)


if __name__ == '__main__':
    main()
