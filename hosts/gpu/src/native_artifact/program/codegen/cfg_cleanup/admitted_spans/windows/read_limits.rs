//! A selected clamp floor also admits a fixed-width read from this cursor.
use super::*;

fn saturated_condition(proof: &Proof<'_>, arg: Value, tail: u64) -> bool {
    let f = &proof.ctx.func;
    let Some(inst) = comparison(f, arg) else {
        return false;
    };
    match f.dfg.insts[inst] {
        Data::IntCompare {
            cond, args: [a, b], ..
        } => {
            (cond == IntCC::UnsignedGreaterThanOrEqual
                && proof.length(a)
                && literal(f, b) == Some(tail))
                || (cond == IntCC::UnsignedLessThanOrEqual
                    && proof.length(b)
                    && literal(f, a) == Some(tail))
        }
        Data::IntCompareImm {
            cond: IntCC::UnsignedGreaterThanOrEqual,
            arg,
            imm,
            ..
        } => proof.length(arg) && imm.bits() as u64 == tail,
        _ => false,
    }
}

// The later floor proof implies length >= tail, so the subtraction is
// nonwrapping and the guarded zero default cannot be selected.
fn limit(proof: &Proof<'_>, value: Value, depth: usize) -> Option<u64> {
    if depth == 0 {
        return None;
    }
    let f = &proof.ctx.func;
    let value = f.dfg.resolve_aliases(value);
    let (base, amount) = constant_offset(f, value);
    if proof.length(base) {
        return Some(amount.wrapping_neg());
    }
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    let Data::Ternary {
        opcode: Opcode::Select,
        args: [condition, yes, no],
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    if let Some(value) = super::super::super::constant_integer(f, condition, 16, &mut 64) {
        return limit(proof, if value == 0 { no } else { yes }, depth - 1);
    }
    let tail = limit(proof, yes, depth - 1)?;
    (literal(f, no) == Some(0) && saturated_condition(proof, condition, tail)).then_some(tail)
}

fn minimum(proof: &Proof<'_>, arg: Value, truth: bool) -> Option<(Value, u64)> {
    let f = &proof.ctx.func;
    let inst = comparison(f, arg)?;
    let (value, bound, cond) = match f.dfg.insts[inst] {
        Data::IntCompare {
            cond, args: [a, b], ..
        } => {
            if let Some(bound) = literal(f, b) {
                (a, bound, cond)
            } else {
                (b, literal(f, a)?, cond.swap_args())
            }
        }
        Data::IntCompareImm { cond, arg, imm, .. } => (arg, imm.bits() as u64, cond),
        _ => return None,
    };
    let floor = match (truth, cond) {
        (true, IntCC::UnsignedGreaterThanOrEqual | IntCC::Equal)
        | (false, IntCC::UnsignedLessThan | IntCC::NotEqual) => bound,
        (true, IntCC::UnsignedGreaterThan) | (false, IntCC::UnsignedLessThanOrEqual) => {
            bound.checked_add(1)?
        }
        _ => return None,
    };
    Some((value, floor))
}

fn floor(proof: &mut Proof<'_>, mut block: Block, needed: u64) -> bool {
    let f = &proof.ctx.func;
    let mut visited = BTreeSet::new();
    while proof.fuel != 0 && visited.len() < 64 && visited.insert(block) {
        proof.fuel -= 1;
        let Some((prior, inst)) = sole_predecessor(&proof.ctx.cfg, block) else {
            return false;
        };
        if let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] {
            let edges = blocks.map(|call| call.block(&f.dfg.value_lists) == block);
            if edges[0] == edges[1] {
                return false;
            }
            if let Some((window, min)) = minimum(proof, arg, edges[0]) {
                if min >= needed && proof.bounded(window, prior) {
                    return true;
                }
            }
        } else if !matches!(f.dfg.insts[inst], Data::Jump { .. }) {
            return false;
        }
        block = prior;
    }
    false
}

pub(super) fn prove(
    proof: &mut Proof<'_>,
    a: Value,
    b: Value,
    cond: IntCC,
    block: Block,
) -> Option<i64> {
    let f = &proof.ctx.func;
    for (index, bound, cond) in [(a, b, cond), (b, a, cond.swap_args())] {
        let (strict, result) = match cond {
            IntCC::UnsignedLessThan => (true, 1),
            IntCC::UnsignedLessThanOrEqual => (false, 1),
            IntCC::UnsignedGreaterThan => (false, 0),
            IntCC::UnsignedGreaterThanOrEqual => (true, 0),
            _ => continue,
        };
        let (position, amount) = constant_offset(f, index);
        if !proof.same_cursor(position) {
            continue;
        }
        let Some(tail) = limit(proof, bound, 16) else {
            continue;
        };
        let Some(needed) = amount
            .checked_add(tail)
            .and_then(|n| n.checked_add(u64::from(strict)))
        else {
            continue;
        };
        // The existing span pass already handles cursor < length itself.
        if needed >= 2 && floor(proof, block, needed) {
            return Some(result);
        }
    }
    None
}
