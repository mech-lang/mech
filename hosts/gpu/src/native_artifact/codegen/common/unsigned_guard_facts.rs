//! Prove checked unsigned additions from dominating tests and intrinsic ranges.
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

#[derive(Clone, Copy)]
enum Operand {
    Value(Value),
    Constant(u64),
}
impl Operand {
    fn value(self) -> Option<Value> {
        if let Self::Value(v) = self {
            Some(v)
        } else {
            None
        }
    }
    fn constant(self, f: &Function) -> Option<u64> {
        match self {
            Self::Value(v) => constant(f, v),
            Self::Constant(v) => Some(v),
        }
    }
    fn upper(self, f: &Function) -> u64 {
        match self {
            Self::Value(v) => intrinsic_upper_bound(f, v, 8),
            Self::Constant(v) => v,
        }
    }
    fn word(self, f: &Function) -> bool {
        match self {
            Self::Value(v) => f.dfg.value_type(v) == types::I64,
            Self::Constant(_) => true,
        }
    }
}
fn comparison(
    function: &Function,
    value: Value,
    truth: bool,
    depth: usize,
) -> Option<(IntCC, Operand, Operand)> {
    if depth == 0 {
        return None;
    }
    let (cond, left, right) = match *definition(function, value)? {
        D::Unary {
            opcode: O::Uextend,
            arg,
        } => return comparison(function, arg, truth, depth - 1),
        D::IntCompare { cond, args, .. } => {
            let [a, b] = args.map(|v| Operand::Value(function.dfg.resolve_aliases(v)));
            (cond, a, b)
        }
        D::IntCompareImm { cond, arg, imm, .. } => (
            cond,
            Operand::Value(function.dfg.resolve_aliases(arg)),
            Operand::Constant(imm.bits() as u64),
        ),
        _ => return None,
    };
    // Only actual native comparison expressions can be unwrapped as Bool.
    for (inner, literal) in [(left, right), (right, left)] {
        if matches!(cond, IntCC::Equal | IntCC::NotEqual) {
            if let (Some(inner), Some(word @ (0 | 1))) = (inner.value(), literal.constant(function))
            {
                let wanted = truth == ((cond == IntCC::Equal) == (word == 1));
                if let Some(result) = comparison(function, inner, wanted, depth - 1) {
                    return Some(result);
                }
            }
        }
    }
    (left.word(function) && right.word(function)).then_some((
        if truth { cond } else { cond.complement() },
        left,
        right,
    ))
}

fn addition(function: &Function, sum: Value, base: Value) -> Option<u64> {
    if let D::BinaryImm64 {
        opcode: O::IaddImm,
        arg,
        imm,
    } = *definition(function, sum)?
    {
        return (function.dfg.resolve_aliases(arg) == base).then_some(imm.bits() as u64);
    }
    let D::Binary {
        opcode: O::Iadd,
        args,
    } = *definition(function, sum)?
    else {
        return None;
    };
    let [a, b] = args.map(|v| function.dfg.resolve_aliases(v));
    if a == base {
        Some(intrinsic_upper_bound(function, b, 8))
    } else if b == base {
        Some(intrinsic_upper_bound(function, a, 8))
    } else {
        None
    }
}

// Intrinsic unsigned ranges need no CFG/alias assumptions. Unknown values,
// cyclic block parameters, signed arithmetic and exhausted expression depth
// retain the full range. No load is moved or allowed to trap differently.
fn intrinsic_upper_bound(function: &Function, value: Value, depth: usize) -> u64 {
    let value = function.dfg.resolve_aliases(value);
    let ty = function.dfg.value_type(value);
    if !ty.is_int() || ty.bits() > 64 {
        return u64::MAX;
    }
    if ty.bits() < 64 {
        return (1u64 << ty.bits()) - 1;
    }
    if let Some(value) = constant(function, value) {
        return value;
    }
    if depth == 0 {
        return u64::MAX;
    }
    let Some(data) = definition(function, value) else {
        return u64::MAX;
    };
    if let D::BinaryImm64 { opcode, arg, imm } = *data {
        let left = intrinsic_upper_bound(function, arg, depth - 1);
        let right = imm.bits() as u64;
        let cover = |v: u64| {
            if v == 0 {
                0
            } else {
                u64::MAX >> v.leading_zeros()
            }
        };
        return match opcode {
            O::BandImm => left.min(right),
            O::BorImm | O::BxorImm => cover(left) | right,
            O::UshrImm if right < 64 => left >> right,
            O::IshlImm if right < 64 => cover(left) << right,
            O::UdivImm if right != 0 => left / right,
            O::UremImm if right != 0 => left.min(right - 1),
            O::IaddImm => left.checked_add(right).unwrap_or(u64::MAX),
            _ => u64::MAX,
        };
    }
    let args = data.arguments(&function.dfg.value_lists);
    let bound = |at: usize| intrinsic_upper_bound(function, args[at], depth - 1);
    match data.opcode() {
        O::Uload8 => u8::MAX as u64,
        O::Uload16 => u16::MAX as u64,
        O::Uload32 => u32::MAX as u64,
        O::Uextend => bound(0),
        O::Popcnt | O::Clz | O::Ctz => function.dfg.value_type(args[0]).bits() as u64,
        O::Band => bound(0).min(bound(1)),
        O::Bor | O::Bxor => {
            let cover = |v: u64| {
                if v == 0 {
                    0
                } else {
                    u64::MAX >> v.leading_zeros()
                }
            };
            cover(bound(0)) | cover(bound(1))
        }
        O::Ishl => match constant(function, args[1]) {
            Some(shift) if shift < 64 => {
                let left = bound(0);
                let cover = if left == 0 {
                    0
                } else {
                    u64::MAX >> left.leading_zeros()
                };
                cover << shift
            }
            _ => u64::MAX,
        },
        O::Ushr => match constant(function, args[1]) {
            Some(shift) if shift < 64 => bound(0) >> shift,
            _ => u64::MAX,
        },
        O::Urem => match constant(function, args[1]) {
            Some(divisor) if divisor != 0 => bound(0).min(divisor - 1),
            _ => u64::MAX,
        },
        O::Udiv => match constant(function, args[1]) {
            Some(divisor) if divisor != 0 => bound(0) / divisor,
            _ => u64::MAX,
        },
        O::Iadd => bound(0).checked_add(bound(1)).unwrap_or(u64::MAX),
        O::Select => bound(1).max(bound(2)),
        _ => u64::MAX,
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
                IntCC::Equal if a.value().is_some() => (a, b, false),
                IntCC::Equal => (b, a, false),
                IntCC::NotEqual if b.constant(function) == Some(u64::MAX) => (a, b, true),
                IntCC::NotEqual if a.constant(function) == Some(u64::MAX) => (b, a, true),
                _ => continue,
            };
            let Some(base) = base.value() else {
                continue;
            };
            let limit = bound.upper(function);
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
        let (Some(a), Some(b)) = (a.value(), b.value()) else {
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
        if intrinsic_upper_bound(function, base, 8) <= limit
            || upper.get(&base).is_some_and(|bounds| {
                bounds.iter().any(|&(at, bound)| {
                    bound <= limit && context.domtree.block_dominates(at, block)
                })
            })
        {
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
