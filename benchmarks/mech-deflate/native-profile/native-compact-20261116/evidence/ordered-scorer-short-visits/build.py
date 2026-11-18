"""Build independent short-visit reductions against the published source."""

from difflib import unified_diff
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
REPO = Q.parent
BASE = '3cde720cdf8b24d6bc4f1818988479d8eb5ae960'
BRANCH = 'refs/heads/codex/native-compact-source-handoff'
CHECKOUT = Q / 'native-review-pr/checkout'
TARGET = ROADMAP / 'definition-compiler/target'
PACKAGE = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
PATH = Path('hosts/gpu/src/native_artifact/program/codegen/ordered_reduction/emit.rs')
OUTPUT = HERE / 'compiler'
COMPILER = OUTPUT / 'compile_profile'
LATCH = ROADMAP / 'scorer-vector-latch/latch-kept-root-layout'
PLAN = ROADMAP / 'ssa-local-and-first/matched-both/plan.json'


def read(path):
    return json.loads(path.read_bytes())


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def original(path):
    return subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=REPO)


def change(source):
    # Reuse the exact one-file bottom-tested loop from the preceding study.
    import runpy
    front = runpy.run_path(str(ROADMAP / 'scorer-vector-latch/production.py'))
    source = front['change_emit'](source)
    source = replace(source, '''        let long = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 16);
''', '''        let vector_minimum = if cfg!(target_arch = "aarch64") {
            match std::env::var("MECH_ORDERED_VECTOR_MINIMUM").as_deref() {
                Ok("4") => 4,
                Ok("8") => 8,
                Ok("12") => 12,
                _ => 16,
            }
        } else {
            16
        };
        let long = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThanOrEqual,
            remaining,
            vector_minimum,
        );
''')
    source = replace(source, '''        let vector = b.create_block();
''', '''        let pair = biased.filter(|_| {
            cfg!(target_arch = "aarch64")
                && std::env::var_os("MECH_ORDERED_PAIRED_CONTINUATION").is_some()
        });
        let scalar_entry = if pair.is_some() {
            let entry = b.create_block();
            for ty in [types::I64, types::I32, types::I64, types::I64, types::I64] {
                b.append_block_param(entry, ty);
            }
            entry
        } else {
            scalar_body
        };
        let vector = b.create_block();
''')
    for old, new in [
        ('b.ins().jump(scalar_body, &args);', 'b.ins().jump(scalar_entry, &args);'),
        ('                scalar_body,\n                &[start.into(),', '                scalar_entry,\n                &[start.into(),'),
        ('                scalar_body,\n                &[end.into(),', '                scalar_entry,\n                &[end.into(),'),
    ]:
        if old.startswith('b.ins().jump'):
            assert source.count(old) == 2
            source = source.replace(old, new)
        else:
            source = replace(source, old, new)
    section = '''        // Short entries have completed their first visit. Vector entries
'''
    return replace(source, section, '''        if let Some((front, other)) = pair {
            let back = reverse.expect("paired continuation has a reverse cursor");
            let group = b.create_block();
            let tail = b.create_block();
            for ty in [types::I64, types::I32, types::I64, types::I64, types::I64] {
                b.append_block_param(group, ty);
            }
            b.switch_to_block(scalar_entry);
            let args = b.block_params(scalar_entry).to_vec();
            let has_pair = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, args[4], 2);
            let args: Vec<_> = args.into_iter().map(Into::into).collect();
            b.ins().brif(has_pair, group, &args, scalar_body, &args);
            b.switch_to_block(group);
            let args = b.block_params(group).to_vec();
            let index = args[0];
            let next = b.ins().iadd_imm(index, 1);
            let stride = i64::from(proof.arrays[back.array].width);
            let back_first = b.ins().iadd_imm(args[3], -stride);
            let back_second = b.ins().iadd_imm(args[3], -2 * stride);
            let front_stride = i64::from(proof.arrays[front.array].width);
            let front_base = words.read(b, shadow + proof.view(front.array));
            let front_offset = b.ins().imul_imm(index, front_stride);
            let front_first = b.ins().iadd(front_base, front_offset);
            let front_second = b.ins().iadd_imm(front_first, front_stride);
            let cost = |b: &mut FunctionBuilder<'_>, index, front_address, back_address| {
                inputs.scalar(
                    b,
                    proof,
                    &self.cost,
                    index,
                    &[
                        (front, ScalarRead::LowWord(front_address)),
                        (back, ScalarRead::LowWord(back_address)),
                    ],
                )
            };
            // The complete-child witness admits both visits. A strict comparison
            // preserves the first member of a tie, including the incoming best.
            let a = cost(b, index, front_first, back_first);
            let d = cost(b, next, front_second, back_second);
            let later = b.ins().icmp(IntCC::UnsignedLessThan, d, a);
            let pair_min = b.ins().select(later, d, a);
            let winner = b.ins().select(later, next, index);
            let better = b.ins().icmp(IntCC::UnsignedLessThan, pair_min, args[1]);
            let minimum = b.ins().select(better, pair_min, args[1]);
            let shifted = emit_item_low32(b, words, proof, shadow, other, index, ordinary);
            let winner = b.ins().ireduce(types::I32, winner);
            let selected = b.ins().bor(winner, shifted);
            let selected = b.ins().uextend(types::I64, selected);
            let item = b.ins().select(better, selected, args[2]);
            let next = b.ins().iadd_imm(index, 2);
            let left = b.ins().iadd_imm(args[4], -2);
            let more = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, left, 2);
            let next_args = [next.into(), minimum.into(), item.into(), back_second.into(), left.into()];
            b.ins().brif(more, group, &next_args, tail, &[]);
            b.switch_to_block(tail);
            let odd = b.ins().icmp_imm(IntCC::NotEqual, left, 0);
            b.ins().brif(odd, scalar_body, &next_args, scalar_finish, &[minimum.into(), item.into()]);
        }
''' + section).replace('This path admits at least 16 elements. Its rounded prefix',
                      'This path admits at least four elements. Its rounded prefix')


def main():
    head = subprocess.check_output(['git', 'rev-parse', BRANCH], cwd=REPO).decode().strip()
    assert head == BASE
    source = original(PATH)
    target = CHECKOUT / PATH
    assert target.read_bytes() == source
    OUTPUT.mkdir(exist_ok=True)
    probe = OUTPUT / 'source' / PATH
    probe.parent.mkdir(parents=True, exist_ok=True)
    probe.write_text(change(source.decode()))
    subprocess.run(['rustfmt', '--edition=2024', '--config', 'skip_children=true', probe], check=True)
    candidate = probe.read_bytes()
    patch = OUTPUT / 'candidate.patch'
    patch.write_text(''.join(unified_diff(source.decode().splitlines(keepends=True),
        candidate.decode().splitlines(keepends=True), f'a/{PATH}', f'b/{PATH}')))
    command = [sys.executable, '-B', CHECKOUT / PACKAGE / 'rebuild.py', OUTPUT / 'disabled',
               '--variant', 'frame-copies', '--compiler-opt-level', '3', '--direct-source',
               '--preselect-root-inlining', '--skip-refused-general-ssa', '--target-dir', TARGET]
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    try:
        target.write_bytes(candidate)
        with (OUTPUT / 'build.log').open('wb') as stream:
            subprocess.run(command, env=env | {'CARGO_NET_OFFLINE': 'true'}, check=True,
                           stdout=stream, stderr=subprocess.STDOUT)
        subprocess.run(['/bin/cp', '-c', TARGET / 'release/compile_profile', COMPILER], check=True)
    finally:
        assert target.read_bytes() == candidate
        target.write_bytes(source)
    expected = read(PLAN)['actors']['mech-ssa-first-definition']['artifact_sha256']
    paths = {'bytecode': 'frontend/program.mecb', 'encoder': 'native/program.o', 'metadata': 'native/metadata.json'}
    observed = {key: sha(OUTPUT / 'disabled' / path) for key, path in paths.items()}
    assert observed == expected, (observed, expected)
    result = {'source_pin': BASE, 'path': str(PATH), 'sha256': sha(probe), 'patch_sha256': sha(patch),
              'folded_base_sha256': sha(target), 'compiler_sha256': sha(COMPILER),
              'disabled_artifacts': observed, 'command': list(map(str, command))}
    write(OUTPUT / 'result.json', result)
    print('Disabled probe reproduces the maintained bytecode, encoder, and metadata.', flush=True)


if __name__ == '__main__':
    main()
