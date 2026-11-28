//! Fold unsigned narrow fields into ordinary scalar logical instructions.
use super::{
    I8, I16, I32, I64, Inst, InstOutput, InstructionData, Lower, MInst, Opcode, OperandSize,
    ShiftOp, ShiftOpAndAmt, ShiftOpShiftImm, ValueRegs,
};
use crate::ir::{DataFlowGraph, Type, Value};
use crate::isa::aarch64::inst::ALUOp;

fn extension(dfg: &DataFlowGraph, value: Value, output: Type) -> Option<Value> {
    let value = dfg.resolve_aliases(value);
    if dfg.value_type(value) != output {
        return None;
    }
    let inst = dfg.value_def(value).inst()?;
    let InstructionData::Unary {
        opcode: Opcode::Uextend,
        arg,
    } = dfg.insts[inst]
    else {
        return None;
    };
    let ty = dfg.value_type(arg);
    (matches!(ty, I8 | I16 | I32) && ty.bits() < output.bits()).then_some(arg)
}

fn shifted(dfg: &DataFlowGraph, value: Value, ty: Type) -> Option<(Value, u8)> {
    let value = dfg.resolve_aliases(value);
    if dfg.value_type(value) != ty {
        return None;
    }
    let inst = dfg.value_def(value).inst()?;
    let (arg, count) = match dfg.insts[inst] {
        InstructionData::BinaryImm64 {
            opcode: Opcode::IshlImm,
            arg,
            imm,
        } => (arg, imm.bits() as u64),
        InstructionData::Binary {
            opcode: Opcode::Ishl,
            args: [arg, count],
        } => {
            let count = dfg.resolve_aliases(count);
            let inst = dfg.value_def(count).inst()?;
            let InstructionData::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            } = dfg.insts[inst]
            else {
                return None;
            };
            (arg, imm.bits() as u64)
        }
        _ => return None,
    };
    let count = (count & u64::from(ty.bits() - 1)) as u8;
    (count != 0).then_some((arg, count))
}

enum Plan {
    Pack {
        low: Value,
        high: Value,
        half: u8,
    },
    Shift {
        plain: Value,
        narrow: Value,
        count: u8,
    },
}

pub(super) fn lower(
    lower: &mut Lower<MInst>,
    inst: Inst,
    options: &crate::isa::Aarch64CompilationOptions,
) -> Option<InstOutput> {
    let dfg = lower.dfg();
    let InstructionData::Binary { opcode, args } = dfg.insts[inst] else {
        return None;
    };
    let op = match opcode {
        Opcode::Bor => ALUOp::Orr,
        Opcode::Band => ALUOp::And,
        Opcode::Bxor => ALUOp::Eor,
        _ => return None,
    };
    let ty = lower.output_ty(inst, 0);
    if !matches!(ty, I32 | I64) {
        return None;
    }
    let pack = options.pack_narrow_halves;
    let fold = options.fold_shifted_extensions;
    if !pack && !fold {
        return None;
    }
    // Read-only, fixed-size preflight. No producer is consumed or moved;
    // independently shared extensions and original wide values keep their uses.
    let mut plan = None;
    for (plain, rhs) in [(args[0], args[1]), (args[1], args[0])] {
        let Some((wide, count)) = shifted(dfg, rhs, ty) else {
            continue;
        };
        let Some(narrow) = extension(dfg, wide, ty) else {
            continue;
        };
        let bits = dfg.value_type(narrow).bits() as u8;
        if pack
            && matches!(opcode, Opcode::Bor | Opcode::Bxor)
            && u32::from(bits) * 2 == ty.bits()
            && count == bits
            && let Some(low) = extension(dfg, plain, ty)
            && dfg.value_type(low) == dfg.value_type(narrow)
        {
            // The two zero-extended fields are disjoint and cover the entire
            // output word. BFI overwrites all dirty bits above the low field.
            plan = Some(Plan::Pack {
                low,
                high: narrow,
                half: bits,
            });
            break;
        }
        if fold && u32::from(bits) + u32::from(count) >= ty.bits() {
            // All physical bits outside the narrow value are shifted out of
            // the output word. The other logical operand remains unchanged.
            plan = Some(Plan::Shift {
                plain,
                narrow,
                count,
            });
            break;
        }
    }
    let plan = plan?;
    let size = if ty == I32 {
        OperandSize::Size32
    } else {
        OperandSize::Size64
    };
    let rd = lower.alloc_tmp(ty).only_reg().unwrap();
    match plan {
        Plan::Pack { low, high, half } => {
            let low = lower.put_value_in_regs(low).only_reg().unwrap();
            let rn = lower.put_value_in_regs(high).only_reg().unwrap();
            let old = lower.alloc_tmp(I64).only_reg().unwrap();
            // A coalescible copy supports both shared inputs and the physical
            // zero register. Bits outside the low field are overwritten.
            lower.emit(MInst::Mov {
                size: OperandSize::Size64,
                rd: old,
                rm: low,
            });
            lower.emit(MInst::Bfi {
                size,
                rd,
                old: old.to_reg(),
                rn,
                lsb: half,
                width: half,
            });
        }
        Plan::Shift {
            plain,
            narrow,
            count,
        } => {
            let rn = lower.put_value_in_regs(plain).only_reg().unwrap();
            let rm = lower.put_value_in_regs(narrow).only_reg().unwrap();
            let shiftop = ShiftOpAndAmt::new(
                ShiftOp::LSL,
                ShiftOpShiftImm::maybe_from_shift(u64::from(count)).unwrap(),
            );
            lower.emit(MInst::AluRRRShift {
                alu_op: op,
                size,
                rd,
                rn,
                rm,
                shiftop,
            });
        }
    }
    Some(smallvec::smallvec![ValueRegs::one(rd.to_reg())])
}
