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
