//! Coalesce adjacent unsigned loads in a proved private fixed view.
use super::window_bounds::sole_predecessor;
use super::word_ranges::{Facts, constant_offset, literal};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Endianness, Function, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode, Value,
    ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Cell {
    base: Value,
    index: Value,
    displacement: u64,
}

fn definition(f: &Function, v: Value) -> Option<Data> {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(v)) else {
        return None;
    };
    Some(f.dfg.insts[inst])
}

fn cell(f: &Function, address: Value, offset: i32, floors: &BTreeMap<Value, u64>) -> Option<Cell> {
    let Data::Binary {
        opcode: Opcode::Iadd,
        args: [a, b],
    } = definition(f, address)?
    else {
        return None;
    };
    for (base, bytes) in [(a, b), (b, a)] {
        let base = f.dfg.resolve_aliases(base);
        if !floors.contains_key(&base) {
            continue;
        }
        let Data::BinaryImm64 {
            opcode: Opcode::ImulImm,
            arg,
            imm,
        } = definition(f, bytes)?
        else {
            continue;
        };
        if imm.bits() != 2 {
            continue;
        }
        let (index, bias) = constant_offset(f, arg);
        let displacement = bias.wrapping_mul(2).wrapping_add(offset as u64);
        if displacement % 2 == 0 {
            return Some(Cell {
                base,
                index,
                displacement,
            });
        }
    }
    None
}

fn bound(f: &Function, facts: &mut Facts<'_>, index: Value) -> Option<u64> {
    // The normalized index can be a separately materialized stride times a
    // masked value. Check the whole multiplication before trusting its range.
    match definition(f, index) {
        Some(Data::Binary {
            opcode: Opcode::Imul,
            args: [a, b],
        }) => {
            let (value, stride) = literal(f, b)
                .map(|s| (a, s))
                .or_else(|| literal(f, a).map(|s| (b, s)))?;
            facts.value_range(value)?.1.checked_mul(stride)
        }
        Some(Data::BinaryImm64 {
            opcode: Opcode::ImulImm,
            arg,
            imm,
        }) => facts.value_range(arg)?.1.checked_mul(imm.bits() as u64),
        _ => Some(facts.value_range(index)?.1),
    }
}

fn load(f: &Function, inst: Inst, floors: &BTreeMap<Value, u64>) -> Option<(Cell, Value, i32)> {
    let Data::Load {
        opcode: Opcode::Uload16,
        arg,
        offset,
        flags,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    if flags != MemFlags::new() || f.dfg.value_type(f.dfg.first_result(inst)) != types::I64 {
        return None;
    }
    let offset = i32::from(offset);
    Some((cell(f, arg, offset, floors)?, arg, offset))
}

fn store(f: &Function, inst: Inst, floors: &BTreeMap<Value, u64>) -> Option<(Cell, Value)> {
    let Data::Store {
        opcode: Opcode::Store,
        args: [value, addr],
        offset,
        flags,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    if flags != MemFlags::new() || f.dfg.value_type(value) != types::I16 {
        return None;
    }
    Some((cell(f, addr, offset.into(), floors)?, value))
}

fn following(f: &Function, cfg: &ControlFlowGraph, mut from: Inst) -> Option<Inst> {
    for _ in 0..128 {
        if let Some(next) = f.layout.next_inst(from) {
            from = next;
            let opcode = f.dfg.insts[next].opcode();
            if opcode.can_load()
                || opcode.can_store()
                || opcode.can_trap()
                || opcode.other_side_effects()
            {
                return Some(next);
            }
            if !opcode.is_terminator() {
                continue;
            }
        }
        let Data::Jump { destination, .. } = f.dfg.insts[from] else {
            return None;
        };
        let to = destination.block(&f.dfg.value_lists);
        if Some(to) == f.layout.entry_block()
            || !f.dfg.block_params(to).is_empty()
            || sole_predecessor(cfg, to) != Some((f.layout.inst_block(from)?, from))
        {
            return None;
        }
        let first = f.layout.first_inst(to)?;
        let opcode = f.dfg.insts[first].opcode();
        if opcode.can_load()
            || opcode.can_store()
            || opcode.can_trap()
            || opcode.other_side_effects()
        {
            return Some(first);
        }
        from = first;
    }
    None
}

pub(in crate::native_artifact) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    floors: &BTreeMap<Value, u64>,
) -> usize {
    let mut facts = Facts::new(f);
    let mut used = BTreeSet::new();
    let mut edits = Vec::new();
    for b in f.layout.blocks().filter(|&block| dom.is_reachable(block)) {
        for first in f.layout.block_insts(b) {
            if used.contains(&first) {
                continue;
            }
            let Some((lo, address, offset)) = load(f, first, floors) else {
                continue;
            };
            let Some(second) = following(f, cfg, first) else {
                continue;
            };
            let Some((hi, _, _)) = load(f, second, floors) else {
                continue;
            };
            if lo.base != hi.base
                || lo.index != hi.index
                || lo.displacement.checked_add(2) != Some(hi.displacement)
            {
                continue;
            }
            // Target the repeated read/replace pair. Two differently indexed
            // stores (or an intervening effect) would not share this address
            // computation and can extend both extracted values' lifetimes.
            let Some(write_lo) = following(f, cfg, second) else {
                continue;
            };
            let Some(write_hi) = following(f, cfg, write_lo) else {
                continue;
            };
            let Some((write_hi_cell, write_hi_value)) = store(f, write_hi, floors) else {
                continue;
            };
            if store(f, write_lo, floors).map(|(cell, _)| cell) != Some(lo)
                || write_hi_cell != hi
                || !matches!(definition(f,write_hi_value), Some(Data::Unary { opcode:Opcode::Ireduce,arg })
                    if f.dfg.resolve_aliases(arg)==f.dfg.first_result(first))
            {
                continue;
            }
            let extent = bound(f, &mut facts, lo.index)
                .and_then(|max| max.checked_mul(2))
                .and_then(|max| max.checked_add(hi.displacement)?.checked_add(2));
            if !extent.is_some_and(|end| end <= floors[&lo.base]) {
                continue;
            }
            used.insert(second);
            edits.push((first, second, address, offset));
        }
    }
    let mut cursor = FuncCursor::new(f);
    for &(first, second, address, offset) in &edits {
        let location = cursor.func.srcloc(first);
        cursor.goto_inst(first);
        cursor.set_srcloc(location);
        let flags = MemFlags::new().with_endianness(Endianness::Little);
        let pair = cursor.ins().uload32(flags, address, offset);
        cursor.func.dfg.replace(first).band_imm(pair, 0xffff);
        cursor.func.dfg.replace(second).ushr_imm(pair, 16);
    }
    edits.len()
}

#[cfg(all(test, feature = "jit", target_arch = "aarch64"))]
mod tests;
