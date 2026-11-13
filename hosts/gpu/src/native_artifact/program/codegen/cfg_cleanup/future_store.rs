//! Look ahead to a bounded write into a fixed, exclusively lent vector.
use super::{MAX_CONSTANT_DEPTH, MAX_CONSTANT_STEPS, constant_integer, word_ranges::Facts};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode as Op, Value, ValueDef,
    types,
};
use cranelift_codegen::loop_analysis::{Loop, LoopAnalysis};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct Write {
    pub store: Inst,
    pub array: usize,
    pub site: usize,
    pub width: u8,
}

#[derive(Clone, Copy)]
struct Address {
    base: Value,
    selector: Value,
    offset: u64,
    stride: u64,
    mask: u64,
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Hint {
    source_write: usize,
    byte_offset: u64,
    byte_stride: u64,
    selector_mask: u64,
}

fn definition(function: &Function, value: Value) -> Option<Data> {
    let value = function.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    (function.dfg.value_type(value) == types::I64).then_some(function.dfg.insts[inst])
}

fn constant_operand(function: &Function, value: Value) -> Option<(Op, Value, u64)> {
    use super::word_ranges::literal;
    match definition(function, value)? {
        Data::Binary {
            opcode,
            args: [left, right],
        } => literal(function, right)
            .map(|constant| (opcode, left, constant))
            .or_else(|| literal(function, left).map(|constant| (opcode, right, constant))),
        Data::BinaryImm64 { opcode, arg, imm } => Some((opcode, arg, imm.bits() as u64)),
        _ => None,
    }
}

fn address(function: &Function, write: Write, minimum_length: u64) -> Option<Address> {
    // Recognize the emitted flat address and its original one-based ordinal.
    // Leave displaced, non-flat and wrapping source addresses alone.
    let Data::Store { offset, .. } = function.dfg.insts[write.store] else {
        return None;
    };
    if i32::from(offset) != 0 {
        return None;
    }
    let [value, address] = *function.dfg.inst_args(write.store) else {
        return None;
    };
    if function.dfg.value_type(value).bytes() != u32::from(write.width) {
        return None;
    }
    let Data::Binary {
        opcode: Op::Iadd,
        args: [base, bytes],
    } = definition(function, address)?
    else {
        return None;
    };
    let (Op::ImulImm, ordinal, width) = constant_operand(function, bytes)? else {
        return None;
    };
    if width != u64::from(write.width) {
        return None;
    }
    let (Op::IaddImm, mut index, u64::MAX) = constant_operand(function, ordinal)? else {
        return None;
    };
    let (mut stride, mut bias) = (1u64, 0u64);
    let (selector, mask) = loop {
        let (opcode, argument, constant) = constant_operand(function, index)?;
        match opcode {
            Op::Band | Op::BandImm => break (function.dfg.resolve_aliases(argument), constant),
            Op::Iadd | Op::IaddImm => bias = bias.checked_add(stride.checked_mul(constant)?)?,
            Op::Imul | Op::ImulImm if constant != 0 => stride = stride.checked_mul(constant)?,
            _ => return None,
        }
        index = argument;
    };
    let stride = stride.checked_mul(width)?;
    let offset = bias.checked_sub(1)?.checked_mul(width)?;
    let varying_bytes = stride.checked_mul(mask)?;
    let end = offset.checked_add(varying_bytes)?.checked_add(width)?;
    // A fixed or very small address range gains little from a hint.
    if varying_bytes < 64 || end > minimum_length.checked_mul(width)? || end > i64::MAX as u64 {
        return None;
    }
    Some(Address {
        base: function.dfg.resolve_aliases(base),
        selector,
        offset,
        stride,
        mask,
    })
}

fn definition_block(function: &Function, value: Value) -> Option<Block> {
    match function.dfg.value_def(value) {
        ValueDef::Result(inst, _) => function.layout.inst_block(inst),
        ValueDef::Param(block, _) => Some(block),
        ValueDef::Union(..) => None,
    }
}

fn fresh_bound(function: &Function, facts: &mut Facts<'_>, value: Value) -> Option<u64> {
    match definition(function, value)? {
        Data::Binary {
            opcode: Op::Ushr,
            args: [input, count],
        } => {
            let mut steps = MAX_CONSTANT_STEPS;
            let shift = constant_integer(function, count, MAX_CONSTANT_DEPTH, &mut steps)?;
            Some(facts.value_range(input)?.1 >> (shift & 63))
        }
        Data::BinaryImm64 {
            opcode: Op::UshrImm,
            arg,
            imm,
        } => Some(facts.value_range(arg)?.1 >> (imm.bits() as u64 & 63)),
        Data::Binary {
            opcode: Op::Band, ..
        }
        | Data::BinaryImm64 {
            opcode: Op::BandImm,
            ..
        }
        | Data::Unary {
            opcode: Op::Uextend,
            ..
        } => Some(facts.value_range(value)?.1),
        _ => None,
    }
}

fn returning_values(
    function: &Function,
    cfg: &ControlFlowGraph,
    loops: &LoopAnalysis,
    region: Loop,
    selector: Value,
) -> Vec<(Inst, Value)> {
    let mut pending = vec![selector];
    let mut visited = BTreeSet::new();
    let mut returns = Vec::new();
    while let Some(value) = pending.pop() {
        if !visited.insert(value) {
            continue;
        }
        let ValueDef::Param(block, ordinal) = function.dfg.value_def(value) else {
            continue;
        };
        if !loops.is_in_loop(block, region) {
            continue;
        }
        for predecessor in cfg.pred_iter(block) {
            if !loops.is_in_loop(predecessor.block, region) {
                continue;
            }
            let transfer = function.dfg.insts[predecessor.inst];
            for target in transfer
                .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables)
            {
                if target.block(&function.dfg.value_lists) != block {
                    continue;
                }
                let Some(argument) = target
                    .args(&function.dfg.value_lists)
                    .nth(ordinal)
                    .and_then(|argument| argument.as_value())
                else {
                    continue;
                };
                let argument = function.dfg.resolve_aliases(argument);
                match function.dfg.value_def(argument) {
                    ValueDef::Param(..) => pending.push(argument),
                    ValueDef::Result(inst, 0)
                        if matches!(transfer, Data::Jump { .. })
                            && function
                                .layout
                                .inst_block(inst)
                                .is_some_and(|block| loops.is_in_loop(block, region)) =>
                    {
                        returns.push((predecessor.inst, argument));
                    }
                    _ => {}
                }
            }
        }
    }
    returns
}

pub(in crate::native_artifact) fn insert(
    function: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    writes: &[Write],
    fixed_lengths: &[(usize, u64)],
) -> Option<Hint> {
    let mut loops = LoopAnalysis::new();
    loops.compute(function, cfg, dom);
    let mut facts = Facts::new(function);
    let mut selected = None;
    for &write in writes {
        let Some(&(_, minimum)) = fixed_lengths
            .iter()
            .find(|&&(array, _)| array == write.array)
        else {
            continue;
        };
        let Some(address) = address(function, write, minimum) else {
            continue;
        };
        let Some(region) = function
            .layout
            .inst_block(write.store)
            .and_then(|block| loops.innermost_loop(block))
        else {
            continue;
        };
        let header = loops.loop_header(region);
        let Some(base_block) = definition_block(function, address.base) else {
            continue;
        };
        if base_block == header || !dom.block_dominates(base_block, header) {
            continue;
        }
        for (before, next) in returning_values(function, cfg, &loops, region, address.selector) {
            // Keep the hint after the current write and on the unconditional
            // edge carrying the new selector. The address dies on that edge.
            if !dom.dominates(write.store, before, &function.layout)
                || !fresh_bound(function, &mut facts, next)
                    .is_some_and(|maximum| maximum <= address.mask)
            {
                continue;
            }
            // Limit the pass to one hint per phase. Prefer the denser address
            // range, then stable source order for equally dense accesses.
            let rank = (address.stride, write.site, before);
            if selected.as_ref().is_none_or(|(old, _, _)| rank < *old) {
                selected = Some((rank, next, address));
            }
        }
    }
    let ((_, source_write, before), next, address) = selected?;
    let mut cursor = FuncCursor::new(function);
    cursor.goto_inst(before);
    let base = cursor.ins().iadd_imm(address.base, address.offset as i64);
    let bytes = cursor.ins().imul_imm(next, address.stride as i64);
    let pointer = cursor.ins().iadd(base, bytes);
    cursor.ins().prefetch_write(pointer);
    Some(Hint {
        source_write,
        byte_offset: address.offset,
        byte_stride: address.stride,
        selector_mask: address.mask,
    })
}

#[cfg(all(test, feature = "jit"))]
mod tests;
