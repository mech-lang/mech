//! Insert a disjoint masked unsigned field without masking it separately.
use super::{
    I32, I64, ImmLogic, ImmShift, Inst, InstOutput, InstructionData, Lower, MInst, Opcode,
    OperandSize, ValueRegs,
};
use crate::ir::{DataFlowGraph, Type, Value};
use crate::isa::aarch64::inst::ALUOp;

fn constant(dfg: &DataFlowGraph, value: Value) -> Option<u64> {
    let value = dfg.resolve_aliases(value);
    let inst = dfg.value_def(value).inst()?;
    match dfg.insts[inst] {
        InstructionData::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } => Some(imm.bits() as u64),
        _ => None,
    }
}

fn masked(dfg: &DataFlowGraph, value: Value, ty: Type) -> Option<(Value, u64)> {
    let value = dfg.resolve_aliases(value);
    if dfg.value_type(value) != ty {
        return None;
    }
    let inst = dfg.value_def(value).inst()?;
    let (arg, mask) = match dfg.insts[inst] {
        InstructionData::BinaryImm64 {
            opcode: Opcode::BandImm,
            arg,
            imm,
        } => (arg, imm.bits() as u64),
        InstructionData::Binary {
            opcode: Opcode::Band,
            args: [a, b],
        } => {
            if let Some(mask) = constant(dfg, b) {
                (a, mask)
            } else {
                (b, constant(dfg, a)?)
            }
        }
        _ => return None,
    };
    Some((arg, mask & (u64::MAX >> (64 - ty.bits()))))
}

fn shifted(dfg: &DataFlowGraph, value: Value, ty: Type) -> Option<(Value, u8)> {
    let value = dfg.resolve_aliases(value);
    if dfg.value_type(value) != ty {
        return None;
    }
    let inst = dfg.value_def(value).inst()?;
    let (arg, count) = match dfg.insts[inst] {
        InstructionData::BinaryImm64 {
            opcode: Opcode::UshrImm,
            arg,
            imm,
        } => (arg, imm.bits() as u64),
        InstructionData::Binary {
            opcode: Opcode::Ushr,
            args: [arg, count],
        } => (arg, constant(dfg, count)?),
        _ => return None,
    };
    Some((arg, (count & u64::from(ty.bits() - 1)) as u8))
}

fn unsigned_bits(dfg: &DataFlowGraph, value: Value, ty: Type) -> Option<u32> {
    let value = dfg.resolve_aliases(value);
    if dfg.value_type(value) != ty {
        return None;
    }
    let inst = dfg.value_def(value).inst()?;
    if let InstructionData::Unary {
        opcode: Opcode::Uextend,
        arg,
    } = dfg.insts[inst]
    {
        // This is a semantic bound on the wide value, not an assumption
        // about dirty physical bits of the original narrow register.
        return Some(dfg.value_type(arg).bits());
    }
    let word_mask = u64::MAX >> (64 - ty.bits());
    if let Some(value) = constant(dfg, value) {
        return Some(64 - (value & word_mask).leading_zeros());
    }
    masked(dfg, value, ty).map(|(_, mask)| 64 - mask.leading_zeros())
}

pub(super) fn lower(
    lower: &mut Lower<MInst>,
    inst: Inst,
    mode: crate::isa::Aarch64MaskedFields,
) -> Option<InstOutput> {
    use crate::isa::Aarch64MaskedFields::{Off, Wide, WideCopy};
    let dfg = lower.dfg();
    let InstructionData::Binary {
        opcode: Opcode::Bor | Opcode::Bxor,
        args,
    } = dfg.insts[inst]
    else {
        return None;
    };
    let ty = lower.output_ty(inst, 0);
    if !matches!(ty, I32 | I64) {
        return None;
    }
    if mode == Off {
        return None;
    }
    let tie = mode == Wide;
    let wide = tie || mode == WideCopy;
    // Fixed-size, read-only preflight. The original producers retain any
    // independent uses; loads, stores, guards and arithmetic are never moved.
    let mut plan = None;
    for (base, field) in [(args[0], args[1]), (args[1], args[0])] {
        let Some((base, base_mask)) = masked(dfg, base, ty) else {
            continue;
        };
        let Some((field, field_mask)) = masked(dfg, field, ty) else {
            continue;
        };
        if field_mask == 0 || base_mask & field_mask != 0 {
            continue;
        }
        let lsb = field_mask.trailing_zeros();
        let width = (field_mask >> lsb).trailing_ones();
        if (field_mask >> lsb) != u64::MAX >> (64 - width) {
            continue;
        }
        let Some((field, shift)) = shifted(dfg, field, ty) else {
            continue;
        };
        // Shift counts in CLIF wrap modulo the operand width, but adding
        // the destination field offset must not wrap. Out-of-range combined
        // shifts keep their original lowering, including its zero fill.
        let shift = u32::from(shift) + lsb;
        if shift >= ty.bits() {
            continue;
        }
        let Some(mask) = ImmLogic::maybe_from_u64(base_mask, ty) else {
            continue;
        };
        let widen = wide
            && lsb > 0
            && base_mask == (1u64 << lsb) - 1
            && unsigned_bits(dfg, field, ty).is_some_and(|bits| bits <= shift + width);
        plan = Some((
            base,
            field,
            mask,
            shift as u8,
            lsb as u8,
            width as u8,
            widen,
        ));
        break;
    }
    let (base, field, mask, shift, lsb, width, widen) = plan?;
    let base = lower.put_value_in_regs(base).only_reg()?;
    let field = lower.put_value_in_regs(field).only_reg()?;
    let size = OperandSize::from_ty(ty);
    if widen {
        let shifted = lower.alloc_tmp(ty).only_reg()?;
        lower.emit(MInst::AluRRImmShift {
            alu_op: ALUOp::Lsr,
            size,
            rd: shifted,
            rn: field,
            immshift: ImmShift::maybe_from_u64(u64::from(shift)).unwrap(),
        });
        let old = if base.is_real() || !tie {
            // A fixed physical source, including XZR, cannot be tied to a
            // newly allocated destination. Give it an ordinary virtual copy.
            let old = lower.alloc_tmp(ty).only_reg()?;
            lower.emit(MInst::Mov {
                size: OperandSize::Size64,
                rd: old,
                rm: base,
            });
            old.to_reg()
        } else {
            // reg_reuse_def ties the destination to this use. Regalloc
            // inserts a copy when the input remains live; a dead input can
            // be overwritten directly without an emitted self-copy.
            base
        };
        let rd = lower.alloc_tmp(ty).only_reg()?;
        // Every bit above the low base field is overwritten. The semantic
        // source bound proves that insertion beyond the original field only
        // writes zeros. Tied allocation preserves shared/physical inputs.
        lower.emit(MInst::Bfi {
            size,
            rd,
            old,
            rn: shifted.to_reg(),
            lsb,
            width: ty.bits() as u8 - lsb,
        });
        return Some(smallvec::smallvec![ValueRegs::one(rd.to_reg())]);
    }
    let old = lower.alloc_tmp(ty).only_reg()?;
    lower.emit(MInst::AluRRImmLogic {
        alu_op: ALUOp::And,
        size,
        rd: old,
        rn: base,
        imml: mask,
    });
    let field = if shift == 0 {
        field
    } else {
        let shifted = lower.alloc_tmp(ty).only_reg()?;
        lower.emit(MInst::AluRRImmShift {
            alu_op: ALUOp::Lsr,
            size,
            rd: shifted,
            rn: field,
            immshift: ImmShift::maybe_from_u64(u64::from(shift)).unwrap(),
        });
        shifted.to_reg()
    };
    let rd = lower.alloc_tmp(ty).only_reg()?;
    // BFI reads both inputs before writing the tied destination. The AND
    // clears every inserted bit; disjointness makes both OR and XOR exact.
    // Size32 also discards dirty physical bits above an I32 input.
    lower.emit(MInst::Bfi {
        size,
        rd,
        old: old.to_reg(),
        rn: field,
        lsb,
        width,
    });
    Some(smallvec::smallvec![ValueRegs::one(rd.to_reg())])
}
