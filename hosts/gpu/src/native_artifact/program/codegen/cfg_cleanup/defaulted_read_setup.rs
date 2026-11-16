//! Normalize selected scalar reads in a composed, admitted no-growth phase.
use super::composed_reads::{Capture, arm, definition, packet_word, read_comparison, seed_word};
use super::word_ranges::{constant_offset, literal};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode as Op,
    Value, condcodes::IntCC, types,
};

#[derive(Default, Debug)]
pub(in crate::native_artifact) struct Changes {
    pub rebased: bool,
    pub narrowed_loads: usize,
    pub narrow_items: usize,
    pub cold_defaults: usize,
    pub identical_routes: usize,
}

pub(in crate::native_artifact) fn optimize(
    function: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    capture: &Capture<'_>,
    cursor_word: usize,
    output_base_word: usize,
) -> Changes {
    let mut changes = Changes::default();
    let mut guards = Vec::new();
    let mut cold = Vec::new();
    let mut identical = Vec::new();
    let mut users = std::collections::BTreeMap::<Value, Vec<Inst>>::new();
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            for value in function.dfg.inst_values(inst) {
                users
                    .entry(function.dfg.resolve_aliases(value))
                    .or_default()
                    .push(inst);
            }
        }
    }
    for block in function.layout.blocks() {
        let Some(inst) = function.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif { arg, blocks, .. } = function.dfg.insts[inst] else {
            continue;
        };
        if let Some(target) = identical_jump_routes(function, cfg, blocks) {
            identical.push((inst, target));
            continue;
        }
        let Some(compare) = read_comparison(function, arg) else {
            continue;
        };
        let Data::IntCompare {
            cond: IntCC::UnsignedLessThan,
            args: [index, length],
            ..
        } = function.dfg.insts[compare]
        else {
            continue;
        };
        let Some((join, yes, _, loads)) =
            arm(function, cfg, blocks[0].block(&function.dfg.value_lists))
        else {
            continue;
        };
        let Some((other_join, zero, defaults, no_loads)) =
            arm(function, cfg, blocks[1].block(&function.dfg.value_lists))
        else {
            continue;
        };
        let [load] = loads[..] else {
            continue;
        };
        if join != other_join
            || function.dfg.value_type(index) != types::I64
            || cfg.pred_iter(join).count() != 2
            || !no_loads.is_empty()
            || literal(function, zero) != Some(0)
        {
            continue;
        }
        cold.extend(defaults);
        if blocks
            .iter()
            .all(|edge| edge.args(&function.dfg.value_lists).next().is_none())
            && let Some(base) = previous_word(function, &users, join, yes, load, index, length)
            && packet_word(function, cfg, capture.packet_loads, base) == Some(output_base_word)
        {
            guards.push((inst, length, load, base));
        }
    }
    let items = narrow_items(function);
    let header = capture.header;
    let mut slots = function
        .dfg
        .block_params(header)
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(slot, value)| {
            (seed_word(function, cfg, dom, capture, value) == Some(cursor_word)).then_some(slot)
        });
    let slot = match (slots.next(), slots.next()) {
        (Some(slot), None) => Some(slot),
        _ => None,
    };
    if let Some(slot) = slot {
        changes.rebased = rebase_cursor(function, cfg, dom, header, slot);
    }
    for (inst, target) in identical {
        function.dfg.replace(inst).jump(target, &[]);
        changes.identical_routes += 1;
    }
    for block in cold {
        function.layout.set_cold(block);
        changes.cold_defaults += 1;
    }
    let mut cursor = FuncCursor::new(function);
    for (branch, length, load, base) in guards {
        let Data::Brif { ref mut arg, .. } = cursor.func.dfg.insts[branch] else {
            unreachable!("recorded branch");
        };
        *arg = length;
        cursor.goto_inst(load);
        let bytes = cursor.ins().ishl_imm(length, 3);
        let before = cursor.ins().iadd_imm(base, -8);
        let address = cursor.ins().iadd(before, bytes);
        cursor
            .func
            .dfg
            .replace(load)
            .uload32(MemFlags::new(), address, 0);
        changes.narrowed_loads += 1;
    }
    for (inst, source, amount, tag) in items {
        cursor.goto_inst(inst);
        let low = cursor.ins().ireduce(types::I32, source);
        let shifted = cursor.ins().ishl_imm(low, i64::from(amount));
        let tag = cursor.ins().iconst(types::I32, i64::from(tag));
        let value = cursor.ins().bor(shifted, tag);
        cursor.func.dfg.replace(inst).uextend(types::I64, value);
        changes.narrow_items += 1;
    }
    changes
}

fn minus_one(f: &Function, value: Value) -> Option<Value> {
    let (_, data) = definition(f, value)?;
    match data {
        Data::Binary {
            opcode: Op::Isub,
            args: [base, one],
        } if literal(f, one) == Some(1) => Some(f.dfg.resolve_aliases(base)),
        Data::BinaryImm64 {
            opcode: Op::IaddImm,
            arg,
            imm,
        } if imm.bits() == -1 => Some(f.dfg.resolve_aliases(arg)),
        _ => None,
    }
}

fn rebase_cursor(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    header: Block,
    slot: usize,
) -> bool {
    let Some(&param) = f.dfg.block_params(header).get(slot) else {
        return false;
    };
    if Some(header) == f.layout.entry_block() || f.dfg.value_type(param) != types::I64 {
        return false;
    }
    let mut uses = Vec::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            if !f
                .dfg
                .inst_values(inst)
                .any(|v| f.dfg.resolve_aliases(v) == param)
            {
                continue;
            }
            let [value] = f.dfg.inst_results(inst) else {
                return false;
            };
            if minus_one(f, *value) != Some(param) {
                return false;
            }
            uses.push((inst, *value));
        }
    }
    if uses.len() != 1 {
        return false;
    }
    let mut edges = Vec::new();
    for predecessor in cfg.pred_iter(header) {
        let inst = predecessor.inst;
        let Data::Jump { destination, .. } = f.dfg.insts[inst] else {
            return false;
        };
        if destination.block(&f.dfg.value_lists) != header {
            return false;
        }
        let args = destination.args(&f.dfg.value_lists).collect::<Vec<_>>();
        let Some(value) = args.get(slot).and_then(|arg| arg.as_value()) else {
            return false;
        };
        edges.push((inst, args, value));
    }
    if edges.is_empty()
        || !edges
            .iter()
            .any(|(inst, _, _)| dom.dominates(header, *inst, &f.layout))
    {
        return false;
    }
    let mut cursor = FuncCursor::new(f);
    for (inst, mut args, value) in edges {
        cursor.goto_inst(inst);
        args[slot] = cursor.ins().iadd_imm(value, -1).into();
        cursor.func.dfg.replace(inst).jump(header, &args);
    }
    let [(inst, value)] = uses[..] else {
        unreachable!("one proved consumer");
    };
    cursor.func.layout.remove_inst(inst);
    cursor.func.dfg.clear_results(inst);
    cursor.func.dfg.change_to_alias(value, param);
    true
}

fn previous_word(
    f: &Function,
    users: &std::collections::BTreeMap<Value, Vec<Inst>>,
    join: Block,
    yes: Value,
    load: Inst,
    index: Value,
    length: Value,
) -> Option<Value> {
    let length = f.dfg.resolve_aliases(length);
    let index = f.dfg.resolve_aliases(index);
    if f.dfg.value_type(length) != types::I64 || minus_one(f, index)? != length {
        return None;
    }
    let [result] = f.dfg.block_params(join) else {
        return None;
    };
    let loaded = f.dfg.first_result(load);
    if loaded != f.dfg.resolve_aliases(yes) || f.dfg.value_type(loaded) != types::I64 {
        return None;
    }
    let [consumer] = users.get(&loaded)?.as_slice() else {
        return None;
    };
    let Data::Jump { destination, .. } = f.dfg.insts[*consumer] else {
        return None;
    };
    if destination.block(&f.dfg.value_lists) != join {
        return None;
    }
    if users.get(result)?.iter().any(|&inst| {
        !matches!(f.dfg.insts[inst], Data::Binary { opcode: Op::Band, args }
            if low_word_mask(f, args, *result))
            && !matches!(f.dfg.insts[inst], Data::Unary { opcode: Op::Ireduce, .. }
                if f.dfg.value_type(f.dfg.first_result(inst)).bits() <= 32)
    }) {
        return None;
    }
    let Data::Load {
        opcode: Op::Load,
        arg: address,
        offset,
        flags,
    } = f.dfg.insts[load]
    else {
        return None;
    };
    if i32::from(offset) != 0 || flags != MemFlags::new() {
        return None;
    }
    let (
        _,
        Data::Binary {
            opcode: Op::Iadd,
            args: [a, b],
        },
    ) = definition(f, address)?
    else {
        return None;
    };
    let scaled = |value| {
        matches!(definition(f, value), Some((_, Data::BinaryImm64 { opcode: Op::ImulImm, arg, imm }))
            if imm.bits() == 8 && constant_offset(f, arg) == (index, 0))
    };
    let base = if scaled(b) {
        a
    } else if scaled(a) {
        b
    } else {
        return None;
    };
    // The previous full cell is already admitted by the original length guard.
    // Lowering the access to its initialized low bytes retains that guard.
    Some(base)
}

fn low_word_mask(f: &Function, [a, b]: [Value; 2], value: Value) -> bool {
    [(a, b), (b, a)].into_iter().any(|(input, mask)| {
        f.dfg.resolve_aliases(input) == value && literal(f, mask) == Some(u32::MAX as u64)
    })
}

fn narrow_items(f: &Function) -> Vec<(Inst, Value, u8, u32)> {
    f.layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .filter_map(|inst| {
            let Data::Binary {
                opcode: Op::Band,
                args: [a, b],
            } = f.dfg.insts[inst]
            else {
                return None;
            };
            let packed = if literal(f, b) == Some(u32::MAX as u64) {
                a
            } else if literal(f, a) == Some(u32::MAX as u64) {
                b
            } else {
                return None;
            };
            let (
                _,
                Data::Binary {
                    opcode: Op::Bor,
                    args: [shifted, tag],
                },
            ) = definition(f, packed)?
            else {
                return None;
            };
            let tag = u32::try_from(literal(f, tag)?).ok()?;
            let (
                _,
                Data::Binary {
                    opcode: Op::Ishl,
                    args: [source, amount],
                },
            ) = definition(f, shifted)?
            else {
                return None;
            };
            let amount = match literal(f, amount) {
                Some(n) => n,
                None => {
                    let mut facts = super::word_ranges::Facts::new(f);
                    let range = facts.value_range(amount)?;
                    if range.0 != range.1 {
                        return None;
                    }
                    range.0
                }
            } % 64;
            if amount >= 32
                || f.dfg.value_type(f.dfg.first_result(inst)) != types::I64
                || f.dfg.value_type(source) != types::I64
            {
                return None;
            }
            let (
                _,
                Data::Binary {
                    opcode: Op::Band,
                    args: [a, b],
                },
            ) = definition(f, source)?
            else {
                return None;
            };
            let unmasked = if literal(f, b) == Some(u32::MAX as u64) {
                a
            } else if literal(f, a) == Some(u32::MAX as u64) {
                b
            } else {
                return None;
            };
            Some((inst, unmasked, amount as u8, tag))
        })
        .collect()
}

fn identical_jump_routes(
    f: &Function,
    cfg: &ControlFlowGraph,
    edges: [BlockCall; 2],
) -> Option<Block> {
    let destination = |edge: BlockCall| {
        let mut block = edge.block(&f.dfg.value_lists);
        if edge.args(&f.dfg.value_lists).next().is_some() || cfg.pred_iter(block).count() != 1 {
            return None;
        }
        for _ in 0..16 {
            if !f.dfg.block_params(block).is_empty() {
                return None;
            }
            if cfg.pred_iter(block).count() != 1 {
                return Some(block);
            }
            let inst = f.layout.first_inst(block)?;
            let Data::Jump { destination, .. } = f.dfg.insts[inst] else {
                return None;
            };
            if destination.args(&f.dfg.value_lists).next().is_some() {
                return None;
            }
            block = destination.block(&f.dfg.value_lists);
        }
        None
    };
    let left = destination(edges[0])?;
    (destination(edges[1])? == left).then_some(left)
}

#[cfg(test)]
#[path = "defaulted_read_setup_tests.rs"]
mod tests;
