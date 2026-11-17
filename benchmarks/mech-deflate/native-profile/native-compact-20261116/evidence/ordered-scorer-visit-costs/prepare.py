"""Prepare independent ordered-scorer probes against the pinned review tree."""

from difflib import unified_diff
from hashlib import sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
ROOT = Q.parent
CHECKOUT = Q / 'native-review-pr/checkout'
BASE = 'd03f097b06c8386822a8063de9826715a9c1e897'
NATIVE = Path('hosts/gpu/src/native_artifact/program')
RECEIPT = HERE / 'source.json'


def change(value, old, new, count=1):
    assert value.count(old) == count, old
    return value.replace(old, new)


def git_source(path):
    return subprocess.check_output(['git', 'show', f'{BASE}:{path}'], cwd=ROOT).decode()


def composed_reads(s):
    s = change(s, '    Block, Endianness,', '    AbiParam, Block, Endianness,')
    s = change(s, '    pub immutable_code_bytes: Option<u64>,\n}',
               '''    pub immutable_code_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_arguments: Option<AddressArguments>,
}

/// Private read-only addresses produced after the composed entry's checks.
#[derive(Clone, Copy, Eq, PartialEq, serde::Serialize)]
pub(in crate::native_artifact) struct AddressArguments {
    pub cost_offset: i64,
    #[serde(skip)]
    pub table: DataId,
}''')
    s = change(s, '            immutable_code_bytes,\n', '            immutable_code_bytes,\n            address_arguments: None,\n')
    s = change(s, '        dense: &DenseCodes,\n', '        dense: &DenseCodes,\n        address_arguments: bool,\n')
    s = change(s, '''        let global = module.declare_data_in_func(id, f);
        let flags = MemFlags::new();
        let mut c = FuncCursor::new(f).at_inst(start);
        let codes = c.ins().global_value(types::I64, global);
        let costs = c.ins().load(types::I64, flags, input, costs_at);
        let costs = if self.cost_byte_offset == 0 {
            costs
        } else {
            c.ins().iadd_imm(costs, self.cost_byte_offset)
        };
        c.goto_inst(self.slot.branch);''', '''        let mut admission = self.admission;
        let flags = MemFlags::new();
        let (costs, codes) = if address_arguments {
            // Both addresses are private synchronous operands. The caller
            // passes them only after it has admitted the current views.
            f.signature.params.extend([AbiParam::new(types::I64); 2]);
            let costs = f.dfg.append_block_param(entry, types::I64);
            let codes = f.dfg.append_block_param(entry, types::I64);
            admission.address_arguments = Some(AddressArguments {
                cost_offset: self.cost_byte_offset,
                table: id,
            });
            (costs, codes)
        } else {
            let global = module.declare_data_in_func(id, f);
            let mut c = FuncCursor::new(f).at_inst(start);
            let codes = c.ins().global_value(types::I64, global);
            let costs = c.ins().load(types::I64, flags, input, costs_at);
            let costs = if self.cost_byte_offset == 0 {
                costs
            } else {
                c.ins().iadd_imm(costs, self.cost_byte_offset)
            };
            (costs, codes)
        };
        let mut c = FuncCursor::new(f).at_inst(self.slot.branch);''')
    s = change(s, '        Ok(Some(self.admission))', '        Ok(Some(admission))')
    s = change(s, '''    capture: &Capture<'_>,
) -> Result<Option<Admission>, NativeCompileError>''',
               '''    capture: &Capture<'_>,
    address_arguments: bool,
) -> Result<Option<Admission>, NativeCompileError>''')
    return change(s, 'rewrite.emit_dense(f, module, tables, dense)', 'rewrite.emit_dense(f, module, tables, dense, address_arguments)')


def cfg_cleanup(s):
    return change(change(s, "    capture: &composed_reads::Capture<'_>,\n",
                         "    capture: &composed_reads::Capture<'_>,\n    address_arguments: bool,\n"),
                  "        capture,\n    )?;", "        capture,\n        address_arguments,\n    )?;")


def phase_function(s):
    s = change(s, '            if let Some(admission) = cfg_cleanup::compose_readonly_lookups(',
               '''            let address_arguments = !observe
                && module.isa().name() == "aarch64"
                && std::env::var_os("MECH_COMPOSE_ADDRESS_ARGUMENTS").is_some();
            if let Some(admission) = cfg_cleanup::compose_readonly_lookups(''')
    s = change(s, '                dense_tables,\n                &capture,\n', '                dense_tables,\n                &capture,\n                address_arguments,\n')
    s = change(s, '                            &fast_capture,\n', '                            &fast_capture,\n                            address_arguments,\n')
    begin = s.index('                let name = format!("{symbol}_composed");')
    return s[:begin] + change(s[begin:], '.declare_function(&name, Linkage::Local, &signature)',
                             '.declare_function(&name, Linkage::Local, &candidate.func.signature)')


def phase_storage(s):
    return change(s, '    let &[_, input, _] = function.dfg.block_params(entry) else {',
                  '''    // A composed alternative can take extra private operands. The
    // immutable packet pointer keeps the same argument position.
    let &[_, input, _, ..] = function.dfg.block_params(entry) else {''')


def phases(s):
    s = change(s, '            let mut routes = vec![(fast, no_growth.function), (general, self.function)];',
               '''            let mut routes = vec![
                (fast, no_growth.function, None),
                (general, self.function, None),
            ];''')
    return change(s, '''                routes.push((prepare, composed.function));
            }
            for (block, function) in routes {
                b.switch_to_block(block);
                let function = module.declare_func_in_func(function, b.func);
                let call = b.ins().call(function, &[storage.heap, input, out]);''',
                  '''                let addresses = admission.address_arguments.map(|plan| (admission.costs, plan));
                routes.push((prepare, composed.function, addresses));
            }
            for (block, function, addresses) in routes {
                b.switch_to_block(block);
                let function = module.declare_func_in_func(function, b.func);
                let call = if let Some((costs, plan)) = addresses {
                    let base = words.read(b, self.arrays[costs].0);
                    let costs = b.ins().iadd_imm(base, plan.cost_offset);
                    let table = module.declare_data_in_func(plan.table, b.func);
                    let table = b.ins().global_value(types::I64, table);
                    b.ins().call(function, &[storage.heap, input, out, costs, table])
                } else {
                    b.ins().call(function, &[storage.heap, input, out])
                };''')


def reduction(s):
    s = change(s, '''        let singleton =
            carry_item && std::env::var_os("MECH_SINGLETON_ORDERED_REDUCTION").is_some();''',
               '''        let singleton =
            carry_item && std::env::var_os("MECH_SINGLETON_ORDERED_REDUCTION").is_some();
        let branch_winner = if singleton && cfg!(target_arch = "aarch64") {
            std::env::var_os("MECH_ORDERED_BRANCH_WINNER")
        } else {
            None
        };''')
    s = change(s, '''            let selected = if let Some((_, _, index, other)) = cursor {
                let other = emit_item_low32(b, words, proof, shadow, other, start, ordinary);
                let index = b.ins().ireduce(types::I32, index);
                let item = b.ins().bor(index, other);
                b.ins().uextend(types::I64, item)
            } else {
                emit_item(
                    b,
                    words,
                    proof,
                    shadow,
                    &self.selected,
                    start,
                    ordinary,
                    true,
                )
            };
            let minimum = b.ins().select(less, cost, best);
            let item = b.ins().select(less, selected, old_item);''',
               '''            let selected = |b: &mut FunctionBuilder<'_>| {
                if let Some((_, _, index, other)) = cursor {
                    let other = emit_item_low32(b, words, proof, shadow, other, start, ordinary);
                    let index = b.ins().ireduce(types::I32, index);
                    let item = b.ins().bor(index, other);
                    b.ins().uextend(types::I64, item)
                } else {
                    emit_item(b, words, proof, shadow, &self.selected, start, ordinary, true)
                }
            };
            let (minimum, item) = match branch_winner.as_deref().and_then(std::ffi::OsStr::to_str) {
                Some("first" | "all") => use_strict_winner(b, less, cost, best, old_item, selected),
                _ => {
                    let selected = selected(b);
                    let minimum = b.ins().select(less, cost, best);
                    let item = b.ins().select(less, selected, old_item);
                    (minimum, item)
                }
            };''')
    start = s.index('        let minimum = if forward.is_some() {')
    stop = s.index('        let next = b.ins().iadd_imm(params[0], 1);', start)
    s = s[:start] + '''        let selected = |b: &mut FunctionBuilder<'_>| {
            if let Some((_, other)) = biased {
                let other = emit_item_low32(b, words, proof, shadow, other, params[0], ordinary);
                let index = b.ins().ireduce(types::I32, params[0]);
                let low = b.ins().bor(index, other);
                b.ins().uextend(types::I64, low)
            } else if carry_item {
                emit_item(b, words, proof, shadow, &self.selected, params[0], ordinary, true)
            } else {
                b.ins().ireduce(types::I32, params[0])
            }
        };
        let (minimum, choice) = if forward.is_some()
            && branch_winner.as_deref().is_some_and(|mode| mode == "all")
        {
            use_strict_winner(b, less, cost, params[1], params[2], selected)
        } else {
            let minimum = if forward.is_some() {
                b.ins().select(less, cost, params[1])
            } else {
                b.ins().umin(cost, params[1])
            };
            let selected = selected(b);
            let choice = b.ins().select(less, selected, params[2]);
            (minimum, choice)
        };
''' + s[stop:]
    return change(s, '/// Broadcast the unsigned minimum using the existing portable vector vocabulary.',
                  '''/// A non-winner cannot change either carried value. Construct the pure
/// selected item on the strict improvement edge, which also keeps ties stable.
fn use_strict_winner(
    b: &mut FunctionBuilder<'_>,
    improved: Value,
    cost: Value,
    old_cost: Value,
    old_item: Value,
    selected: impl FnOnce(&mut FunctionBuilder<'_>) -> Value,
) -> (Value, Value) {
    let winner = b.create_block();
    let done = b.create_block();
    let minimum = b.append_block_param(done, types::I32);
    let item = b.append_block_param(done, types::I64);
    b.ins().brif(improved, winner, &[], done, &[old_cost.into(), old_item.into()]);
    b.switch_to_block(winner);
    let selected = selected(b);
    b.ins().jump(done, &[cost.into(), selected.into()]);
    b.switch_to_block(done);
    (minimum, item)
}

/// Broadcast the unsigned minimum using the existing portable vector vocabulary.''')


TRANSFORMS = {
    'codegen/cfg_cleanup/composed_reads.rs': composed_reads,
    'codegen/cfg_cleanup/mod.rs': cfg_cleanup,
    'compact/phase_function.rs': phase_function,
    'compact/phase_storage.rs': phase_storage,
    'compact/phases.rs': phases,
    'codegen/ordered_reduction/emit.rs': reduction,
}


def main():
    expected = BASE.encode() + b'\n'
    assert subprocess.check_output(
        ['git', 'rev-parse', 'refs/heads/codex/native-compact-source-handoff'], cwd=ROOT) == expected
    pending = []
    for relative, update in TRANSFORMS.items():
        path = NATIVE / relative
        old = git_source(path)
        assert (CHECKOUT / path).read_text() == old, path
        new = update(old)
        assert new != old
        pending.append((path, old, new))
    patches = []
    files = {}
    for path, old, new in pending:
        (CHECKOUT / path).write_text(new)
        patches.extend(unified_diff(old.splitlines(keepends=True), new.splitlines(keepends=True),
                                    f'a/{path}', f'b/{path}'))
        files[str(path)] = sha256(new.encode()).hexdigest()
    (HERE / 'scorer.patch').write_text(''.join(patches))
    RECEIPT.write_text(json.dumps({'base': BASE, 'source_sha256': files}, indent=2) + '\n')
    print(*files, sep='\n')


if __name__ == '__main__':
    main()
