//! Reuse one immutable comparison when its false route has no effects before
//! repeating the comparison and rejecting.
use super::window_bounds::comparison;
use super::word_ranges::constant_offset;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, Opcode as Op, Value,
    ValueDef, types,
};
use std::collections::BTreeSet;

fn unsigned_operands(f: &Function, predicate: Value) -> Option<[(Value, u64); 2]> {
    let [index, length] = match f.dfg.insts[comparison(f, predicate)?] {
        Data::IntCompare {
            cond: IntCC::UnsignedLessThan,
            args,
            ..
        } => args,
        Data::IntCompare {
            cond: IntCC::UnsignedGreaterThan,
            args: [length, index],
            ..
        } => [index, length],
        _ => return None,
    };
    if f.dfg.value_type(index) != types::I64 {
        return None;
    }
    Some([index, length].map(|value| constant_offset(f, value)))
}

fn repeated_test(f: &Function, first: Inst, mut call: BlockCall) -> Option<Inst> {
    let mut visited = BTreeSet::from([f.layout.inst_block(first)?]);
    loop {
        let block = call.block(&f.dfg.value_lists);
        if !visited.insert(block) {
            return None;
        }
        let end = f.layout.last_inst(block)?;
        if !f
            .layout
            .block_insts(block)
            .take_while(|&inst| inst != end)
            .all(|inst| {
                matches!(
                    f.dfg.insts[inst].opcode(),
                    Op::Iconst
                        | Op::Icmp
                        | Op::IcmpImm
                        | Op::Iadd
                        | Op::IaddImm
                        | Op::Isub
                        | Op::Band
                        | Op::Uextend
                        | Op::Ireduce
                )
            })
        {
            return None;
        }
        match f.dfg.insts[end] {
            Data::Jump { destination, .. } => call = destination,
            Data::Brif { .. } => return Some(end),
            _ => return None,
        }
    }
}

fn available(
    f: &Function,
    dom: &DominatorTree,
    first: Inst,
    visited: &BTreeSet<Block>,
    value: Value,
) -> bool {
    match f.dfg.value_def(f.dfg.resolve_aliases(value)) {
        ValueDef::Param(block, _) => {
            visited.contains(&block) || dom.dominates(block, first, &f.layout)
        }
        ValueDef::Result(inst, _) => {
            f.layout
                .inst_block(inst)
                .is_some_and(|block| visited.contains(&block))
                || dom.dominates(inst, first, &f.layout)
        }
        ValueDef::Union(..) => false,
    }
}

fn available_rejection(
    f: &Function,
    dom: &DominatorTree,
    first: Inst,
    mut call: BlockCall,
) -> bool {
    // The original rejection must stay terminal. Its arguments and body may use
    // values from the new incoming edge and definitions already on that route.
    let mut visited = BTreeSet::new();
    loop {
        let block = call.block(&f.dfg.value_lists);
        if !f.layout.is_cold(block)
            || !call.args(&f.dfg.value_lists).all(|arg| {
                arg.as_value()
                    .is_some_and(|value| available(f, dom, first, &visited, value))
            })
            || !visited.insert(block)
        {
            return false;
        }
        if !f.layout.block_insts(block).all(|inst| {
            f.dfg
                .inst_args(inst)
                .iter()
                .copied()
                .all(|value| available(f, dom, first, &visited, value))
        }) {
            return false;
        }
        let Some(end) = f.layout.last_inst(block) else {
            return false;
        };
        match f.dfg.insts[end] {
            Data::Jump { destination, .. } => call = destination,
            Data::MultiAry {
                opcode: Op::Return, ..
            } => return true,
            _ => return false,
        }
    }
}

pub(super) fn fold(f: &mut Function, dom: &DominatorTree) -> usize {
    let mut edits = Vec::new();
    let mut used = BTreeSet::new();
    for first in f
        .layout
        .blocks()
        .filter_map(|block| f.layout.last_inst(block))
    {
        let Data::Brif {
            arg,
            blocks: [_, no],
            ..
        } = f.dfg.insts[first]
        else {
            continue;
        };
        let Some(operands) = unsigned_operands(f, arg) else {
            continue;
        };
        let Some(second) = repeated_test(f, first, no) else {
            continue;
        };
        let Data::Brif {
            arg,
            blocks: [yes, no],
            ..
        } = f.dfg.insts[second]
        else {
            unreachable!("repeated test terminates at a branch");
        };
        // The first comparison's SSA operands are still current on every visit
        // to the dominated second test. Its original false route now rejects
        // directly, so every remaining visit takes the original true edge.
        if used.contains(&first)
            || used.contains(&second)
            || !dom.dominates(first, second, &f.layout)
            || unsigned_operands(f, arg) != Some(operands)
            || !available_rejection(f, dom, first, no)
        {
            continue;
        }
        used.extend([first, second]);
        edits.push((first, second, yes, no));
    }
    for &(first, second, yes, no) in &edits {
        let Data::Brif { blocks, .. } = &mut f.dfg.insts[first] else {
            unreachable!("recorded comparison");
        };
        blocks[1] = no;
        let target = yes.block(&f.dfg.value_lists);
        let args = yes.args(&f.dfg.value_lists).collect::<Vec<_>>();
        f.dfg.replace(second).jump(target, &args);
    }
    edits.len()
}

#[cfg(test)]
mod tests;
