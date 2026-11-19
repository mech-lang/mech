//! Share only an unchanged current index, never an earlier loop activation.
use super::{ImmutableViews, Read};
use crate::native_artifact::program::codegen::cfg_cleanup::incoming_block_arguments;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, MemFlags, Value, ValueDef, condcodes::IntCC, types,
};
use std::collections::BTreeSet;

#[derive(Clone)]
pub(super) struct Spec {
    pub index: Value,
    pub pointer_word: usize,
    pub length_word: usize,
    pub width: u8,
    pub flags: MemFlags,
}

pub(super) struct Group {
    spec: Spec,
    point: Inst,
    source: Inst,
    pointer: Value,
    length: Value,
}

fn available(f: &Function, dom: &DominatorTree, value: Value, point: Inst) -> bool {
    match f.dfg.value_def(f.dfg.resolve_aliases(value)) {
        ValueDef::Result(inst, _) => dom.dominates(inst, point, &f.layout),
        ValueDef::Param(block, _) => dom.dominates(block, point, &f.layout),
        _ => false,
    }
}

// Every incoming edge participates. A current dominating value is an atom:
// expanding its own recurrence would confuse separate outer-loop visits.
// A closed copy cycle alone supplies no seed, and an actual update refuses.
fn same_current(f: &Function, cfg: &ControlFlowGraph, value: Value, reference: Value) -> bool {
    let reference = f.dfg.resolve_aliases(reference);
    let mut pending = vec![value];
    let mut seen = BTreeSet::new();
    let mut seeded = false;
    let mut remaining = 1024;
    while let Some(value) = pending.pop() {
        if remaining == 0 {
            return false;
        }
        remaining -= 1;
        let value = f.dfg.resolve_aliases(value);
        if value == reference {
            seeded = true;
            continue;
        }
        if !seen.insert(value) {
            continue;
        }
        let ValueDef::Param(block, slot) = f.dfg.value_def(value) else {
            return false;
        };
        let mut incoming = false;
        for (_, value) in incoming_block_arguments(f, cfg, block, slot) {
            let Some(value) = value else {
                return false;
            };
            incoming = true;
            pending.push(value);
        }
        if !incoming {
            return false;
        }
    }
    seeded
}

fn common(dom: &DominatorTree, mut a: Block, mut b: Block) -> Option<Block> {
    let mut ancestors = BTreeSet::from([a]);
    while let Some(parent) = dom.idom(a) {
        ancestors.insert(parent);
        a = parent;
    }
    loop {
        if ancestors.contains(&b) {
            return Some(b);
        }
        b = dom.idom(b)?;
    }
}

pub(super) fn plan(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    reads: &[(Read, Option<Spec>)],
    immutable: ImmutableViews<'_>,
) -> (Vec<Group>, Vec<Option<usize>>) {
    let packet_value = |word, point| {
        immutable.0.iter().find_map(|(&value, &(_, at))| {
            (at == word && available(f, dom, value, point)).then_some(value)
        })
    };
    let mut groups: Vec<Group> = Vec::new();
    let mut owners = vec![None; reads.len()];
    for (ordinal, (read, spec)) in reads.iter().enumerate() {
        let Some(spec) = spec else { continue };
        for (index, group) in groups.iter_mut().enumerate() {
            if (spec.pointer_word, spec.length_word, spec.width, spec.flags)
                != (
                    group.spec.pointer_word,
                    group.spec.length_word,
                    group.spec.width,
                    group.spec.flags,
                )
            {
                continue;
            }
            let Some(block) = common(
                dom,
                f.layout.inst_block(group.point).unwrap(),
                f.layout.inst_block(read.branch).unwrap(),
            ) else {
                continue;
            };
            let point = f.layout.last_inst(block).unwrap();
            let current = if available(f, dom, group.spec.index, point)
                && same_current(f, cfg, spec.index, group.spec.index)
            {
                group.spec.index
            } else if available(f, dom, spec.index, point)
                && same_current(f, cfg, group.spec.index, spec.index)
            {
                spec.index
            } else {
                continue;
            };
            let (Some(pointer), Some(length)) = (
                packet_value(spec.pointer_word, point),
                packet_value(spec.length_word, point),
            ) else {
                continue;
            };
            group.spec.index = current;
            group.point = point;
            group.pointer = pointer;
            group.length = length;
            owners[ordinal] = Some(index);
            break;
        }
        if owners[ordinal].is_none() {
            let (Some(pointer), Some(length)) = (
                packet_value(spec.pointer_word, read.branch),
                packet_value(spec.length_word, read.branch),
            ) else {
                continue;
            };
            if !available(f, dom, spec.index, read.branch) {
                continue;
            }
            owners[ordinal] = Some(groups.len());
            groups.push(Group {
                spec: spec.clone(),
                point: read.branch,
                source: read.load,
                pointer,
                length,
            });
        }
    }
    (groups, owners)
}

impl Group {
    pub(super) fn emit(
        &self,
        cursor: &mut FuncCursor<'_>,
        zero: Value,
        rematerialize: bool,
    ) -> Value {
        cursor.goto_inst(self.point);
        cursor.use_srcloc(self.source);
        let index = self.spec.index;
        let bounded = cursor
            .ins()
            .icmp(IntCC::UnsignedLessThan, index, self.length);
        let bytes = cursor.ins().imul_imm(index, i64::from(self.spec.width));
        let address = cursor.ins().iadd(self.pointer, bytes);
        // The wrapped address is never selected outside the admitted extent.
        let selected = cursor.ins().select(bounded, address, zero);
        // Pin the common-dominator placement by default. Pure-load motion is
        // a separate policy: it may trade repeated loads/address arithmetic
        // for shorter live ranges and fewer spills at distant uses.
        let flags = self.spec.flags.with_readonly().with_notrap();
        let flags = if rematerialize {
            flags.with_can_move()
        } else {
            flags
        };
        let ty = match self.spec.width {
            1 => types::I8,
            4 => types::I32,
            _ => unreachable!("proved narrow immutable defaulted read"),
        };
        let narrow = cursor.ins().load(ty, flags, selected, 0);
        cursor.ins().uextend(types::I64, narrow)
    }
}
