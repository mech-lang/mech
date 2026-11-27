//! Reuse the zero upper word produced by scalar W-register arithmetic.
use super::{I32, I64, Inst, InstOutput, InstructionData, Lower, MInst, Opcode, ValueRegs};

pub(super) fn lower(lower: &mut Lower<MInst>, inst: Inst) -> Option<InstOutput> {
    let dfg = lower.dfg();
    let InstructionData::Unary {
        opcode: Opcode::Uextend,
        arg,
    } = dfg.insts[inst]
    else {
        return None;
    };
    if lower.output_ty(inst, 0) != I64 || dfg.value_type(arg) != I32 {
        return None;
    }
    let arg = dfg.resolve_aliases(arg);
    let producer = dfg.value_def(arg).inst()?;
    let opcode = dfg.insts[producer].opcode();
    let selected = if matches!(opcode, Opcode::Iadd | Opcode::Isub | Opcode::Imul) {
        // These scalar forms, including shifted/immediate/fused arithmetic,
        // write a W register. Preserve the existing arithmetic selection.
        let selection = std::env::var("MECH_ARM64_REUSE_I32_ARITHMETIC").ok()?;
        match selection.as_str() {
            "add" => opcode == Opcode::Iadd,
            "sub" => opcode == Opcode::Isub,
            "mul" => opcode == Opcode::Imul,
            "all" | "1" => true,
            _ => false,
        }
    } else if matches!(opcode, Opcode::Ushr | Opcode::UshrImm)
        && std::env::var_os("MECH_ARM64_REUSE_I32_SHIFTS").is_some_and(|mode| mode == "1")
    {
        let count = match dfg.insts[producer] {
            InstructionData::BinaryImm64 { imm, .. } => imm.bits() as u64,
            InstructionData::Binary {
                args: [_, count], ..
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
                imm.bits() as u64
            }
            _ => return None,
        };
        // ushr_fits_in_32 -> do_shift_imm -> alu_rr_imm_shift uses Size32,
        // so a nonzero fixed logical shift clears the entire upper word.
        // Identity shifts may forward a dirty input without a W write.
        // Parameters, reductions, variable shifts and other producers stay
        // on the ordinary extension path. CLIF masks I32 counts modulo 32.
        count & 31 != 0
    } else {
        false
    };
    if !selected {
        return None;
    }
    let reg = lower.put_value_in_regs(arg).only_reg()?;
    Some(smallvec::smallvec![ValueRegs::one(reg)])
}
