//! Expose short countdown-loop lifetimes before register allocation.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Comparison {
    size: OperandSize,
    left: Reg,
    right: Reg,
}

fn comparison(inst: &Inst, canonical: &impl Fn(Reg) -> Reg) -> Option<Comparison> {
    let Inst::AluRRR {
        alu_op: ALUOp::SubS,
        size,
        rd,
        rn,
        rm,
    } = *inst
    else {
        return None;
    };
    (rd.to_reg() == zero_reg()).then(|| Comparison {
        size,
        left: canonical(rn),
        right: canonical(rm),
    })
}

/// Explicit operands of the pure instructions allowed between selections.
/// The selected forms have no implicit writes other than the comparison's flags.
fn operands(inst: &Inst) -> Option<(Reg, Reg, Reg)> {
    match *inst {
        Inst::AluRRR {
            alu_op: ALUOp::Add | ALUOp::SubS,
            rd,
            rn,
            rm,
            ..
        }
        | Inst::AluRRRShift {
            alu_op: ALUOp::Orr,
            rd,
            rn,
            rm,
            ..
        }
        | Inst::CSel { rd, rn, rm, .. } => Some((rd.to_reg(), rn, rm)),
        Inst::Nop0 => Some((zero_reg(), zero_reg(), zero_reg())),
        _ => None,
    }
}

fn reuse_comparison(body: &mut [Inst], canonical: &impl Fn(Reg) -> Reg) -> bool {
    let mut current = None;
    let mut selected = false;
    let mut changed = false;
    for index in 0..body.len() {
        let inst = &body[index];
        if let Some(next) = comparison(inst, canonical) {
            if current == Some(next)
                && selected
                && matches!(body.get(index + 1), Some(Inst::CSel { .. }))
            {
                body[index] = Inst::Nop0;
                changed = true;
            }
            current = Some(next);
            selected = false;
        } else if let (Some(prior), Some((destination, _, _))) = (current, operands(inst))
            && !matches!(
                inst,
                Inst::AluRRR {
                    alu_op: ALUOp::SubS,
                    ..
                }
            )
            && destination != zero_reg()
            && canonical(destination) != prior.left
            && canonical(destination) != prior.right
        {
            selected |= matches!(inst, Inst::CSel { .. });
        } else {
            current = None;
            selected = false;
        }
    }
    changed
}

fn delay_increment(body: &mut [Inst], canonical: &impl Fn(Reg) -> Reg) {
    for index in (0..body.len()).rev() {
        let Inst::AluRRImm12 {
            alu_op: ALUOp::Add,
            rd,
            rn,
            imm12,
            ..
        } = body[index]
        else {
            continue;
        };
        let (destination, source) = (canonical(rd.to_reg()), canonical(rn));
        if imm12.value() != 1
            || !destination.is_virtual()
            || !source.is_virtual()
            || destination == source
        {
            continue;
        }
        let mut last_use = index;
        for next in index + 1..body.len() {
            let Some((output, left, right)) = operands(&body[next]) else {
                break;
            };
            let (output, left, right) = (canonical(output), canonical(left), canonical(right));
            if output == source
                || output == destination
                || left == destination
                || right == destination
            {
                break;
            }
            if left == source || right == source {
                last_use = next;
            }
        }
        if last_use != index {
            body[index..=last_use].rotate_left(1);
            return;
        }
    }
}

pub(super) fn rewrite(block: &mut [Inst], canonical: impl Fn(Reg) -> Reg) {
    let [
        body @ ..,
        update,
        compare,
        Inst::CondBr {
            kind: CondBrKind::Cond(Cond::Eq | Cond::Ne),
            ..
        },
    ] = block
    else {
        return;
    };
    let Inst::AluRRImm12 {
        alu_op: ALUOp::Sub,
        size,
        rd,
        rn,
        imm12,
    } = *update
    else {
        return;
    };
    let Inst::AluRRImm12 {
        alu_op: ALUOp::SubS,
        size: compare_size,
        rd: flags,
        rn: compare_input,
        imm12: compare_imm,
    } = *compare
    else {
        return;
    };
    if size != compare_size
        || imm12.value() != compare_imm.value()
        || flags.to_reg() != zero_reg()
        || !canonical(rd.to_reg()).is_virtual()
        || !canonical(rn).is_virtual()
        || canonical(rn) != canonical(compare_input)
        || canonical(rd.to_reg()) == canonical(rn)
    {
        return;
    }
    if !reuse_comparison(body, &canonical) {
        return;
    }
    delay_increment(body, &canonical);
    *update = Inst::AluRRImm12 {
        alu_op: ALUOp::SubS,
        size,
        rd,
        rn,
        imm12,
    };
    *compare = Inst::Nop0;
}

/// Reuse the condition flags from a wrapping subtraction already needed on
/// a one-predecessor edge. The inputs are exact SSA registers defined before
/// the branch. NZCV is consumed within its producing VCode block on ARM64.
pub(super) fn reuse_successor_subtract(
    tail: &[Inst],
    first: &Inst,
    canonical: &impl Fn(Reg) -> Reg,
) -> Option<([Inst; 2], Inst)> {
    let [
        Inst::AluRRR {
            alu_op: ALUOp::SubS,
            size: compare_size,
            rd: discard,
            rn: compare_left,
            rm: compare_right,
        },
        Inst::CondBr {
            taken,
            not_taken,
            kind: CondBrKind::Cond(cc),
        },
    ] = *tail
    else {
        return None;
    };
    let Inst::AluRRR {
        alu_op: ALUOp::Sub,
        size,
        rd,
        rn,
        rm,
    } = *first
    else {
        return None;
    };
    let (left, right, dst) = (canonical(rn), canonical(rm), canonical(rd.to_reg()));
    if size != compare_size
        || discard.to_reg() != zero_reg()
        || !left.is_virtual()
        || !right.is_virtual()
        || !dst.is_virtual()
        || dst == left
        || dst == right
    {
        return None;
    }
    let cc = match (canonical(compare_left), canonical(compare_right)) {
        operands if operands == (left, right) => cc,
        operands if operands == (right, left) => match cc {
            Cond::Eq | Cond::Ne => cc,
            Cond::Lo => Cond::Hi,
            Cond::Hs => Cond::Ls,
            Cond::Hi => Cond::Lo,
            Cond::Ls => Cond::Hs,
            _ => return None,
        },
        _ => return None,
    };
    let difference = Inst::AluRRR {
        alu_op: ALUOp::SubS,
        size,
        rd,
        rn,
        rm,
    };
    let branch = Inst::CondBr {
        taken,
        not_taken,
        kind: CondBrKind::Cond(cc),
    };
    Some(([difference, branch], Inst::Nop0))
}

/// Fuse the branch over an admitted single visit with the remainder update.
pub(super) fn reuse_successor_decrement(
    tail: &[Inst],
    head: &[Inst],
    canonical: &impl Fn(Reg) -> Reg,
) -> Option<([Inst; 2], usize, Inst)> {
    let [
        Inst::AluRRImm12 {
            alu_op: ALUOp::SubS,
            size: cmp_size,
            rd: discard,
            rn: cmp_src,
            imm12: cmp_imm,
        },
        branch @ Inst::CondBr {
            kind: CondBrKind::Cond(Cond::Eq | Cond::Ne),
            ..
        },
    ] = tail
    else {
        return None;
    };
    let (index, update) = match head {
        [
            update @ Inst::AluRRImm12 {
                alu_op: ALUOp::Sub, ..
            },
            ..,
        ] => (0, update),
        [
            Inst::AluRRImm12 {
                alu_op: ALUOp::Add,
                rd,
                rn,
                ..
            },
            update,
        ] if canonical(rd.to_reg()).is_virtual() && canonical(*rn).is_virtual() => (1, update),
        _ => return None,
    };
    let Inst::AluRRImm12 {
        alu_op: ALUOp::Sub,
        size,
        rd,
        rn,
        imm12,
    } = *update
    else {
        return None;
    };
    let (src, dst) = (canonical(rn), canonical(rd.to_reg()));
    if *cmp_size != size
        || discard.to_reg() != zero_reg()
        || cmp_imm.value() != 1
        || imm12.value() != 1
        || canonical(*cmp_src) != src
        || !src.is_virtual()
        || !dst.is_virtual()
        || src == dst
    {
        return None;
    }
    let combined = Inst::AluRRImm12 {
        alu_op: ALUOp::SubS,
        size,
        rd,
        rn,
        imm12,
    };
    Some(([combined, branch.clone()], index, Inst::Nop0))
}
