//! Bounds for pure SSA selects. A minimum/maximum requires the current
//! unsigned comparison to select those exact same operands; source types and
//! unrelated conditions supply no such relation.
use super::{masked_integer_comparison, word_ranges};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Function, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef,
};

pub(super) fn unsigned_minimum(
    f: &Function,
    condition: Value,
    yes: Value,
    no: Value,
) -> Option<bool> {
    let [yes, no] = [yes, no].map(|v| f.dfg.resolve_aliases(v));
    let ty = f.dfg.value_type(yes);
    if ty != f.dfg.value_type(no)
        || !ty.is_int()
        || ty.is_vector()
        || !(1..=64).contains(&ty.bits())
    {
        return None;
    }
    let mut value = condition;
    for _ in 0..8 {
        value = f.dfg.resolve_aliases(value);
        let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
            return None;
        };
        let data = f.dfg.insts[inst];
        if let Some(comparison) =
            masked_integer_comparison(f, &data, |v| Some(f.dfg.resolve_aliases(v)))
        {
            value = comparison;
            continue;
        }
        let (cond, left, right, immediate) = match data {
            Data::Unary {
                opcode: Opcode::Uextend,
                arg,
            } => {
                value = arg;
                continue;
            }
            Data::IntCompare {
                cond,
                args: [left, right],
                ..
            } if f.dfg.value_type(left) == ty && f.dfg.value_type(right) == ty => (
                cond,
                f.dfg.resolve_aliases(left),
                Some(f.dfg.resolve_aliases(right)),
                None,
            ),
            Data::IntCompareImm { cond, arg, imm, .. } if f.dfg.value_type(arg) == ty => (
                cond,
                f.dfg.resolve_aliases(arg),
                None,
                Some(imm.bits() as u64 & (u64::MAX >> (64 - ty.bits()))),
            ),
            _ => return None,
        };
        let is_right = |v| {
            right == Some(v) || immediate.is_some_and(|n| word_ranges::literal(f, v) == Some(n))
        };
        let cond = if left == yes && is_right(no) {
            cond
        } else if left == no && is_right(yes) {
            cond.swap_args()
        } else {
            return None;
        };
        return match cond {
            IntCC::UnsignedLessThan | IntCC::UnsignedLessThanOrEqual => Some(true),
            IntCC::UnsignedGreaterThan | IntCC::UnsignedGreaterThanOrEqual => Some(false),
            _ => None,
        };
    }
    None
}

pub(super) fn fold(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> usize {
    if f.layout
        .blocks()
        .flat_map(|b| f.layout.block_insts(b))
        .any(|inst| f.dfg.insts[inst].exception_table().is_some())
    {
        return 0;
    }
    let mut facts = word_ranges::Facts::across_edges(f, cfg, None).with_select_ranges();
    let changes = f
        .layout
        .blocks()
        .filter(|&b| dom.is_reachable(b))
        .filter_map(|block| {
            let inst = f.layout.last_inst(block)?;
            let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
                return None;
            };
            let truth = facts.condition(arg)?;
            Some((inst, blocks[usize::from(!truth)]))
        })
        .collect::<Vec<_>>();
    let count = changes.len();
    for (inst, target) in changes {
        let block = target.block(&f.dfg.value_lists);
        let arguments = target.args(&f.dfg.value_lists).collect::<Vec<_>>();
        f.dfg.replace(inst).jump(block, &arguments);
    }
    count
}

#[cfg(test)]
#[path = "selected_values_tests.rs"]
mod tests;
