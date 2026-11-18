"""Prepare one cost-length specialization for the checked composed phase."""

from difflib import unified_diff
from hashlib import file_digest
import json
from pathlib import Path
import runpy
import subprocess

HERE = Path(__file__).resolve().parent
ROADMAP = HERE.parent
Q = ROADMAP.parent
REPO = Q.parent
BASE = '8c7cec1199fa1348e364c2052d9ec82b0934754e'
BRANCH = 'refs/heads/codex/native-compact-source-handoff'
CHECKOUT = Q / 'native-review-pr/checkout'
PACKET = Path('benchmarks/mech-deflate/native-profile/native-compact-20261116')
PROGRAM = Path('hosts/gpu/src/native_artifact/program')
SETUP = PROGRAM / 'codegen/cfg_cleanup/defaulted_read_setup.rs'
PHASE = PROGRAM / 'compact/phase_function.rs'
REDUCTION = PROGRAM / 'codegen/ordered_reduction/emit.rs'
PROBE = 'MECH_COMPOSE_SPLIT_COST_LENGTH'
TESTS = PROGRAM / 'codegen/cfg_cleanup/defaulted_read_setup_tests.rs'


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(data if isinstance(data, str) else json.dumps(data, indent=2) + '\n')


def git(*args):
    return subprocess.check_output(['git', *args], cwd=REPO)


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def split_helper(text):
    text = replace(text, 'use super::composed_reads::{Capture, arm, definition, packet_word, read_comparison, seed_word};',
        'use super::composed_reads::{Admission, Capture, arm, definition, packet_word, read_comparison, seed_word};')
    point = '\nfn minus_one(f: &Function, value: Value) -> Option<Value> {'
    helper = '''
/// Use the caller's admitted cost extent for prefix reads. A larger index
/// keeps the original full-length check, including the zero default.
pub(in crate::native_artifact) fn split_admitted_cost_checks(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    capture: &Capture<'_>,
    admission: Admission,
) -> usize {
    let Some(view) = capture.views.iter().find(|v| v.array == admission.costs) else {
        return 0;
    };
    let Some(offset) = view.length_word.checked_mul(8).and_then(|n| i32::try_from(n).ok()) else {
        return 0;
    };
    let Ok(floor) = i64::try_from(admission.minimum_costs) else {
        return 0;
    };
    let Some(entry) = f.layout.entry_block() else {
        return 0;
    };
    let &[_, input, _] = f.dfg.block_params(entry) else {
        return 0;
    };
    let checks = f.layout.blocks().filter_map(|block| {
        let branch = f.layout.last_inst(block)?;
        let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
            return None;
        };
        let [yes, no] = blocks;
        if blocks.iter().any(|edge| edge.args(&f.dfg.value_lists).next().is_some()) {
            return None;
        }
        let compare = read_comparison(f, arg)?;
        let Data::IntCompare {
            cond: IntCC::UnsignedLessThan,
            args: [index, length],
            ..
        } = f.dfg.insts[compare] else {
            return None;
        };
        if f.dfg.value_type(index) != types::I64
            || packet_word(f, cfg, capture.packet_loads, length) != Some(view.length_word)
            || !matches!(constant_offset(f, index).1, bias if bias > 0 && bias < floor as u64)
        {
            return None;
        }
        Some((branch, index, yes.block(&f.dfg.value_lists), no.block(&f.dfg.value_lists)))
    }).collect::<Vec<_>>();
    let count = checks.len();
    for (branch, index, yes, no) in checks {
        let fallback = f.dfg.make_block();
        f.layout.append_block(fallback);
        f.layout.set_cold(fallback);
        let mut cursor = FuncCursor::new(f).at_inst(branch);
        let prefix = cursor.ins().icmp_imm(IntCC::UnsignedLessThan, index, floor);
        cursor.func.dfg.replace(branch).brif(prefix, yes, &[], fallback, &[]);

        // The caller's immutable input packet supplies the same length that
        // the original comparison propagated through the loop.
        cursor.goto_bottom(fallback);
        let length = cursor.ins().load(types::I64, MemFlags::new(), input, offset);
        let fits = cursor.ins().icmp(IntCC::UnsignedLessThan, index, length);
        cursor.ins().brif(fits, yes, &[], no, &[]);
    }
    count
}
'''
    return replace(text, point, '\n' + helper + point)


def hook(text):
    point = '''                let id = module
                    .declare_function(&name, Linkage::Local, &signature)
                    .map_err(backend)?;
                if !observe && module.isa().name() == "aarch64" {
                    phase_layout::apply(&mut candidate, &name).map_err(backend)?;
'''
    call = '''                if admission.immutable_code_bytes.is_some()
                    && module.isa().name() == "aarch64"
                    && std::env::var_os("MECH_COMPOSE_SPLIT_COST_LENGTH").is_some()
                    && let Some(loads) =
                        phase_storage::input_word_loads(&candidate.func, input_words)
                {
                    candidate.flowgraph();
                    let capture = Capture {
                        packet_loads: &loads,
                        header: composed_header,
                        ..capture
                    };
                    let split = cfg_cleanup::defaulted_read_setup::split_admitted_cost_checks(
                        &mut candidate.func,
                        &candidate.cfg,
                        &capture,
                        admission,
                    );
                    if split != 0 {
                        candidate.flowgraph();
                        candidate.verify(module.isa()).map_err(backend)?;
                        eprintln!("{name} split admitted cost checks: {split}");
                    }
                }
'''
    return replace(text, point, call + point)


def main():
    assert git('rev-parse', BRANCH).decode().strip() == BASE
    latch = runpy.run_path(str(ROADMAP / 'scorer-vector-latch/production.py'))['change_emit']
    probes = {SETUP: split_helper, PHASE: hook, REDUCTION: latch}
    records, diffs = {}, []
    for path, apply in probes.items():
        original = git('show', f'{BASE}:{path}').decode()
        assert (CHECKOUT / path).read_text() == original, path
        source = HERE / 'source' / path
        write(source, apply(original))
        subprocess.run(['rustfmt', '+nightly-2026-03-03', '--edition', '2024',
                        '--config', 'skip_children=true', source], check=True)
        records[str(path)] = {'current': sha(CHECKOUT / path), 'probe': sha(source)}
        diffs.extend(unified_diff(original.splitlines(keepends=True),
                                  source.read_text().splitlines(keepends=True),
                                  f'a/{path}', f'b/{path}'))
    write(HERE / 'compiler.patch', ''.join(diffs))
    write(HERE / 'source.json', {'base': BASE, 'files': records, 'patch_sha256': sha(HERE / 'compiler.patch')})
    print('Staged', ', '.join(path.name for path in probes))


if __name__ == '__main__':
    main()
