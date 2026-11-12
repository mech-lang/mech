//! Use one current address for adjacent typed accesses on a straight-line path.
use super::window_bounds::sole_predecessor;
use super::word_ranges::constant_offset;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstructionData as Data, Opcode, Value, ValueDef, types,
};

fn definition(f: &Function, value: Value) -> Option<Data> {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
        return None;
    };
    Some(f.dfg.insts[inst])
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    pointer: Value,
    index: Value,
    width: u32,
}

#[derive(Clone, Copy)]
struct Access {
    key: Key,
    address: Value,
    offset: i32,
    displacement: u64,
}

fn access(f: &Function, inst: Inst) -> Option<Access> {
    let (address, offset, element) = match f.dfg.insts[inst] {
        Data::Load {
            opcode: Opcode::Load,
            arg,
            offset,
            ..
        } => (arg, offset, f.dfg.first_result(inst)),
        Data::Store {
            opcode: Opcode::Store,
            args: [element, arg],
            offset,
            ..
        } => (arg, offset, element),
        _ => return None,
    };
    let ty = f.dfg.value_type(element);
    if !matches!(ty, types::I16 | types::I32 | types::I64)
        || f.dfg.value_type(address) != types::I64
    {
        return None;
    }
    let width = ty.bytes();
    let Data::Binary {
        opcode: Opcode::Iadd,
        args: [left, right],
    } = definition(f, address)?
    else {
        return None;
    };
    for (pointer, bytes) in [(left, right), (right, left)] {
        let Some(Data::BinaryImm64 {
            opcode: Opcode::ImulImm,
            arg,
            imm,
        }) = definition(f, bytes)
        else {
            continue;
        };
        if imm.bits() != i64::from(width) {
            continue;
        }
        let (index, elements) = constant_offset(f, arg);
        let offset = i32::from(offset);
        return Some(Access {
            key: Key {
                pointer: f.dfg.resolve_aliases(pointer),
                index,
                width,
            },
            address: f.dfg.resolve_aliases(address),
            offset,
            displacement: elements
                .wrapping_mul(u64::from(width))
                .wrapping_add(offset as u64),
        });
    }
    None
}

struct Group {
    anchor: Access,
    adjacent: bool,
    edits: Vec<(Inst, Value, i32)>,
}

impl Group {
    fn include(&mut self, inst: Inst, current: Access) -> bool {
        if current.key != self.anchor.key {
            return false;
        }
        let delta = current.displacement.wrapping_sub(self.anchor.displacement) as i64;
        if delta != 0 && delta.unsigned_abs() != u64::from(current.key.width) {
            return false;
        }
        let Ok(offset) = i32::try_from(i64::from(self.anchor.offset) + delta) else {
            return false;
        };
        self.adjacent |= delta != 0;
        if current.address != self.anchor.address || current.offset != offset {
            self.edits.push((inst, self.anchor.address, offset));
        }
        true
    }

    fn finish(self, edits: &mut Vec<(Inst, Value, i32)>) {
        if self.adjacent {
            edits.extend(self.edits);
        }
    }
}

fn continuation(f: &Function, cfg: &ControlFlowGraph, from: Block) -> Option<Block> {
    let inst = f.layout.last_inst(from)?;
    let Data::Jump { destination, .. } = f.dfg.insts[inst] else {
        return None;
    };
    let to = destination.block(&f.dfg.value_lists);
    (sole_predecessor(cfg, to) == Some((from, inst))).then_some(to)
}

pub(super) fn fold(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> usize {
    let mut edits = Vec::new();
    for root in f.layout.blocks().filter(|&block| dom.is_reachable(block)) {
        if let Some((prev, _)) = sole_predecessor(cfg, root)
            && dom.is_reachable(prev)
            && continuation(f, cfg, prev) == Some(root)
        {
            continue;
        }
        let mut group: Option<Group> = None;
        let mut block = Some(root);
        while let Some(current) = block {
            for inst in f.layout.block_insts(current) {
                if let Some(access) = access(f, inst) {
                    if group
                        .as_mut()
                        .is_some_and(|group| group.include(inst, access))
                    {
                        continue;
                    }
                    if let Some(old) = group.replace(Group {
                        anchor: access,
                        adjacent: false,
                        edits: Vec::new(),
                    }) {
                        old.finish(&mut edits);
                    }
                } else {
                    let opcode = f.dfg.insts[inst].opcode();
                    if opcode != Opcode::Jump
                        && (opcode.can_load() || opcode.can_store() || opcode.other_side_effects())
                        && let Some(old) = group.take()
                    {
                        old.finish(&mut edits);
                    }
                }
            }
            block = continuation(f, cfg, current);
        }
        if let Some(group) = group {
            group.finish(&mut edits);
        }
    }
    for &(inst, address, byte_offset) in &edits {
        let (arg, offset) = match &mut f.dfg.insts[inst] {
            Data::Load { arg, offset, .. } => (arg, offset),
            Data::Store {
                args: [_, arg],
                offset,
                ..
            } => (arg, offset),
            _ => unreachable!("recorded typed memory instruction"),
        };
        // The effective machine address is identical modulo the I64 address
        // width. Memory flags, types and source ordering stay in place.
        *arg = address;
        *offset = Offset32::new(byte_offset);
    }
    edits.len()
}

#[cfg(test)]
mod tests;
