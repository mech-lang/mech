//! Schedule two disjoint private reads together without moving their stores.
use super::word_ranges::{constant_offset, literal};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Endianness, Function, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode as Op,
    Value, ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct View {
    pub base: Value,
    pub element_bytes: u8,
    pub minimum_bytes: u64,
}

#[derive(Clone, Copy)]
struct Address {
    base: Value,
    index: Value,
    mask: u64,
    stride: u64,
    offset: u64,
}

impl Address {
    fn bytes(self, width: u64) -> Option<Range<u64>> {
        let end = self
            .offset
            .checked_add(self.mask.checked_mul(self.stride)?)?;
        Some(self.offset..end.checked_add(width)?)
    }

    fn disjoint(self, width: u64, other: Range<u64>) -> bool {
        self.bytes(width)
            .is_some_and(|range| range.end <= other.start || other.end <= range.start)
    }

    fn same_cell(self, other: Self) -> bool {
        (self.base, self.index, self.mask, self.stride, self.offset)
            == (
                other.base,
                other.index,
                other.mask,
                other.stride,
                other.offset,
            )
    }
}

fn definition(f: &Function, value: Value) -> Option<Data> {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
        return None;
    };
    Some(f.dfg.insts[inst])
}

fn constant(f: &Function, value: Value) -> Option<(Op, Value, u64)> {
    match definition(f, value)? {
        Data::Binary {
            opcode,
            args: [a, b],
        } => literal(f, b)
            .map(|n| (opcode, a, n))
            .or_else(|| literal(f, a).map(|n| (opcode, b, n))),
        Data::BinaryImm64 { opcode, arg, imm } => Some((opcode, arg, imm.bits() as u64)),
        _ => None,
    }
}

fn address(f: &Function, value: Value, offset: i32, views: &[View]) -> Option<Address> {
    let Data::Binary {
        opcode: Op::Iadd,
        args: [a, b],
    } = definition(f, value)?
    else {
        return None;
    };
    for (base, bytes) in [(a, b), (b, a)] {
        let base = f.dfg.resolve_aliases(base);
        if !views.iter().any(|view| view.base == base) {
            continue;
        }
        let (Op::ImulImm, elements, mut stride) = constant(f, bytes)? else {
            return None;
        };
        let (index, bias) = constant_offset(f, elements);
        let offset = bias.wrapping_mul(stride).wrapping_add(offset as u64);
        let (index, mask) = match constant(f, index)? {
            (Op::Imul | Op::ImulImm, masked, scale) => {
                stride = stride.checked_mul(scale)?;
                let (Op::Band | Op::BandImm, index, mask) = constant(f, masked)? else {
                    return None;
                };
                (index, mask)
            }
            (Op::Band | Op::BandImm, index, mask) => (index, mask),
            _ => return None,
        };
        if stride != 0 && f.dfg.value_type(index) == types::I64 {
            return Some(Address {
                base,
                index: f.dfg.resolve_aliases(index),
                mask,
                stride,
                offset,
            });
        }
    }
    None
}

fn available(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    value: Value,
    at: Inst,
) -> Option<Value> {
    let mut pending = vec![(value, false)];
    let mut visiting = BTreeSet::new();
    let mut complete = BTreeSet::new();
    let mut common = None;
    while let Some((value, post)) = pending.pop() {
        let value = f.dfg.resolve_aliases(value);
        if post {
            visiting.remove(&value);
            complete.insert(value);
            continue;
        }
        if complete.contains(&value) {
            continue;
        }
        match f.dfg.value_def(value) {
            ValueDef::Result(inst, _) if dom.dominates(inst, at, &f.layout) => {}
            ValueDef::Param(block, _) if dom.dominates(block, at, &f.layout) => {}
            ValueDef::Param(block, ordinal) => {
                if !visiting.insert(value) {
                    return None;
                }
                pending.push((value, true));
                let size = pending.len();
                for pred in cfg.pred_iter(block) {
                    for target in f.dfg.insts[pred.inst]
                        .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                    {
                        if target.block(&f.dfg.value_lists) == block {
                            let arg = target.args(&f.dfg.value_lists).nth(ordinal)?.as_value()?;
                            pending.push((arg, false));
                        }
                    }
                }
                if size == pending.len() {
                    return None;
                }
                continue;
            }
            _ => return None,
        }
        if common.is_some_and(|prior| prior != value) {
            return None;
        }
        common = Some(value);
        complete.insert(value);
    }
    common
}

fn store_address(f: &Function, inst: Inst, views: &[View]) -> Option<(Address, u64)> {
    let Data::Store {
        opcode: Op::Store,
        args: [value, base],
        offset,
        flags,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    if flags != MemFlags::new() {
        return None;
    }
    Some((
        address(f, base, offset.into(), views)?,
        u64::from(f.dfg.value_type(value).bytes()),
    ))
}

/// Only inspect the finite acyclic portion between the two reads. Every
/// intervening store must be proved disjoint or a compiler-owned exit store.
fn clear_between(
    f: &Function,
    dom: &DominatorTree,
    start: Inst,
    end: Inst,
    read: Address,
    bytes: Range<u64>,
    writes: &BTreeMap<Inst, Value>,
    views: &[View],
    output: Value,
) -> Option<usize> {
    let end_block = f.layout.inst_block(end)?;
    let mut pending = vec![(
        f.layout.inst_block(start)?,
        f.layout.next_inst(start)?,
        false,
    )];
    let mut visiting = BTreeSet::new();
    let mut complete = BTreeSet::new();
    let mut stores = 0;
    while let Some((block, first, post)) = pending.pop() {
        if post {
            visiting.remove(&block);
            complete.insert(block);
            continue;
        }
        if complete.contains(&block) {
            continue;
        }
        if !dom.is_reachable(block) || !visiting.insert(block) {
            return None;
        }
        pending.push((block, first, true));
        let mut inst = first;
        loop {
            if inst == end {
                break;
            }
            let data = f.dfg.insts[inst];
            match data {
                Data::Jump { .. } | Data::Brif { .. } => {
                    for call in data.branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                    {
                        let target = call.block(&f.dfg.value_lists);
                        if target != end_block && dom.block_dominates(target, block) {
                            return None;
                        }
                        pending.push((target, f.layout.first_inst(target)?, false));
                    }
                    break;
                }
                Data::MultiAry {
                    opcode: Op::Return, ..
                } if f.layout.is_cold(block) => break,
                Data::Store {
                    opcode: Op::Store,
                    args: [_, pointer],
                    ..
                } => {
                    let pointer = f.dfg.resolve_aliases(pointer);
                    if pointer != output {
                        let base = *writes.get(&inst)?;
                        if base == read.base {
                            let (write, width) = store_address(f, inst, views)?;
                            if !write.disjoint(width, bytes.clone()) {
                                return None;
                            }
                            stores += 1;
                        }
                    }
                }
                _ if data.opcode().is_call()
                    || data.opcode().can_store()
                    || data.opcode().is_terminator()
                    || data.opcode().can_trap()
                    || data.opcode().other_side_effects() =>
                {
                    return None;
                }
                _ => {}
            }
            inst = f.layout.next_inst(inst)?;
        }
    }
    (complete.contains(&end_block) && stores > 0).then_some(stores)
}

/// The caller supplies current flat, fixed, exclusively held views and the
/// typed stores from those views. Distinct entries have distinct owners and
/// backing. Its output packet is separate from their payloads.
pub(in crate::native_artifact) fn schedule(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    views: &[View],
    writes: &BTreeMap<Inst, Value>,
) -> bool {
    if views.len() < 2 || writes.is_empty() {
        return false;
    }
    let Some(&[_, _, output]) = f
        .layout
        .entry_block()
        .map(|entry| f.dfg.block_params(entry))
    else {
        return false;
    };
    let mut anchors = Vec::new();
    let mut loads = Vec::new();
    for block in dom.cfg_postorder().iter().rev().copied() {
        for inst in f.layout.block_insts(block) {
            let Data::Load {
                opcode,
                arg,
                offset,
                flags,
            } = f.dfg.insts[inst]
            else {
                continue;
            };
            if !matches!(opcode, Op::Uload16 | Op::Uload32)
                || f.dfg.value_type(f.dfg.first_result(inst)) != types::I64
            {
                continue;
            }
            let Some(addr) = address(f, arg, offset.into(), views) else {
                continue;
            };
            let Some(view) = views.iter().find(|view| view.base == addr.base) else {
                continue;
            };
            if view.element_bytes != 2 {
                continue;
            }
            if opcode == Op::Uload32
                && flags == MemFlags::new().with_endianness(Endianness::Little)
                && addr.stride >= 4
                && addr.bytes(4).is_some_and(|r| r.end <= view.minimum_bytes)
            {
                anchors.push((inst, addr));
            } else if opcode == Op::Uload16 && flags == MemFlags::new() && addr.stride == 2 {
                if let Some(bytes) = addr.bytes(2).filter(|r| r.end <= view.minimum_bytes) {
                    loads.push((inst, addr, bytes));
                }
            }
        }
    }
    for (load, read, bytes) in loads {
        let stores = writes.keys().copied().filter(|&inst| {
            f.layout.inst_block(inst).is_some()
                && dom.dominates(load, inst, &f.layout)
                && matches!(f.dfg.insts[inst], Data::Store { offset, .. } if i32::from(offset) == 0)
                && store_address(f, inst, views).is_some_and(|(addr, width)| width == 2 && addr.same_cell(read))
        }).collect::<Vec<_>>();
        let [store] = stores.as_slice() else {
            continue;
        };
        for &(first, first_address) in &anchors {
            if first_address.base != read.base
                || !first_address.disjoint(4, bytes.clone())
                || !dom.dominates(first, load, &f.layout)
                || read.offset > i64::MAX as u64
                || read.mask > i64::MAX as u64
            {
                continue;
            }
            let Some(index) = available(f, cfg, dom, read.index, first) else {
                continue;
            };
            let Some(_) = clear_between(
                f,
                dom,
                first,
                load,
                read,
                bytes.clone(),
                writes,
                views,
                output,
            ) else {
                continue;
            };
            let Some(next) = f.layout.next_inst(first) else {
                continue;
            };
            let mut cursor = FuncCursor::new(f);
            cursor.goto_inst(next);
            cursor.set_srcloc(cursor.func.srcloc(load));
            let index = cursor.ins().band_imm(index, read.mask as i64);
            let offset = cursor.ins().ishl_imm(index, 1);
            let offset = cursor.ins().iadd_imm(offset, read.offset as i64);
            let pointer = cursor.ins().iadd(read.base, offset);
            let flags = MemFlags::new()
                .with_notrap()
                .with_endianness(Endianness::Little);
            cursor
                .func
                .dfg
                .replace(load)
                .uload16(types::I64, flags, pointer, 0);
            cursor.func.layout.remove_inst(load);
            cursor.func.layout.insert_inst(load, next);
            let [_, base] = cursor.func.dfg.inst_args_mut(*store) else {
                unreachable!("verified fixed view store");
            };
            *base = pointer;
            return true;
        }
    }
    false
}

#[cfg(all(test, feature = "jit", target_arch = "aarch64"))]
#[path = "independent_private_read/tests.rs"]
mod tests;
