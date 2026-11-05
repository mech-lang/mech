//! Prove checked unsigned additions from dominating unsigned range tests.
//! Only an impossible failure edge is removed. No operation, debit, memory
//! access, failure ordering or caller-provided value is speculated.
use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Block, Function, InstBuilder, InstructionData as D, Opcode as O, Value, ValueDef, types,
};
use std::collections::BTreeMap;

const MAX_FACTS: usize = 4096;

fn definition(function: &Function, value: Value) -> Option<&D> {
    let value = function.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    Some(&function.dfg.insts[inst])
}

fn constant(function: &Function, value: Value) -> Option<u64> {
    if function.dfg.value_type(value) != types::I64 {
        return None;
    }
    match definition(function, value)? {
        D::UnaryImm {
            opcode: O::Iconst,
            imm,
            ..
        } => Some(imm.bits() as u64),
        _ => None,
    }
}

fn comparison(
    function: &Function,
    value: Value,
    truth: bool,
    depth: usize,
) -> Option<(IntCC, Value, Value)> {
    if depth == 0 {
        return None;
    }
    match *definition(function, value)? {
        D::Unary {
            opcode: O::Uextend,
            arg,
        } => comparison(function, arg, truth, depth - 1),
        D::IntCompare { cond, args, .. } => {
            let [left, right] = args.map(|v| function.dfg.resolve_aliases(v));
            // Equality against zero/one can wrap a native comparison. The
            // recursive proof accepts only comparison results and extensions,
            // never arbitrary words merely declared Bool.
            for (inner, literal) in [(left, right), (right, left)] {
                if matches!(cond, IntCC::Equal | IntCC::NotEqual) {
                    if let Some(word @ (0 | 1)) = constant(function, literal) {
                        let wanted = truth == ((cond == IntCC::Equal) == (word == 1));
                        if let Some(result) = comparison(function, inner, wanted, depth - 1) {
                            return Some(result);
                        }
                    }
                }
            }
            (function.dfg.value_type(left) == types::I64
                && function.dfg.value_type(right) == types::I64)
                .then_some((if truth { cond } else { cond.complement() }, left, right))
        }
        _ => None,
    }
}

fn addition(function: &Function, sum: Value, base: Value) -> Option<u64> {
    let D::Binary {
        opcode: O::Iadd,
        args,
    } = *definition(function, sum)?
    else {
        return None;
    };
    let [a, b] = args.map(|v| function.dfg.resolve_aliases(v));
    if a == base {
        constant(function, b)
    } else if b == base {
        constant(function, a)
    } else {
        None
    }
}

pub(in crate::native_artifact) fn optimize(context: &mut Context) -> usize {
    context.compute_cfg();
    context.compute_domtree();
    let function = &context.func;
    let mut predecessors = vec![0usize; function.dfg.num_blocks()];
    for block in function.layout.blocks() {
        if let Some(inst) = function.layout.last_inst(block) {
            for call in function.dfg.insts[inst]
                .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables)
            {
                predecessors[call.block(&function.dfg.value_lists).as_u32() as usize] += 1;
            }
        }
    }
    let mut upper = BTreeMap::<Value, Vec<(Block, u64)>>::new();
    let mut facts = 0;
    for block in function.layout.blocks() {
        let Some(inst) = function.layout.last_inst(block) else {
            continue;
        };
        let D::Brif { arg, blocks, .. } = function.dfg.insts[inst] else {
            continue;
        };
        for (truth, call) in [(true, blocks[0]), (false, blocks[1])] {
            let target = call.block(&function.dfg.value_lists);
            // A sole incoming edge makes the chosen comparison true on every
            // entry to this block. Block dominance then carries that fact to
            // the use. Shared joins and alternate loop entries stay unknown.
            if facts == MAX_FACTS
                || Some(target) == function.layout.entry_block()
                || predecessors[target.as_u32() as usize] != 1
            {
                continue;
            }
            let Some((cond, a, b)) = comparison(function, arg, truth, 8) else {
                continue;
            };
            let (base, bound, strict) = match cond {
                IntCC::UnsignedLessThan => (a, b, true),
                IntCC::UnsignedLessThanOrEqual => (a, b, false),
                IntCC::UnsignedGreaterThan => (b, a, true),
                IntCC::UnsignedGreaterThanOrEqual => (b, a, false),
                _ => continue,
            };
            let limit = constant(function, bound).unwrap_or(u64::MAX);
            let Some(limit) = limit.checked_sub(u64::from(strict)) else {
                continue;
            };
            if limit < u64::MAX {
                upper.entry(base).or_default().push((target, limit));
                facts += 1;
            }
        }
    }
    let mut changes = Vec::new();
    for block in function.layout.blocks() {
        let Some(inst) = function.layout.last_inst(block) else {
            continue;
        };
        let D::Brif { arg, blocks, .. } = function.dfg.insts[inst] else {
            continue;
        };
        let Some((cond, a, b)) = comparison(function, arg, true, 8) else {
            continue;
        };
        let (sum, base, success) = match cond {
            IntCC::UnsignedGreaterThanOrEqual => (a, b, 0),
            IntCC::UnsignedLessThanOrEqual => (b, a, 0),
            IntCC::UnsignedLessThan => (a, b, 1),
            IntCC::UnsignedGreaterThan => (b, a, 1),
            _ => continue,
        };
        let Some(increment) = addition(function, sum, base) else {
            continue;
        };
        let limit = u64::MAX - increment;
        if upper.get(&base).is_some_and(|bounds| {
            bounds
                .iter()
                .any(|&(at, bound)| bound <= limit && context.domtree.block_dominates(at, block))
        }) {
            let call = blocks[success];
            changes.push((
                inst,
                call.block(&function.dfg.value_lists),
                call.args(&function.dfg.value_lists).collect::<Vec<_>>(),
            ));
        }
    }
    for (inst, target, args) in &changes {
        context.func.dfg.replace(*inst).jump(*target, args);
    }
    if !changes.is_empty() {
        context.compute_cfg();
        context.compute_domtree();
    }
    changes.len()
}

#[cfg(test)]
mod tests;
