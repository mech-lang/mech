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
    // All scalar I32 add/subtract/multiply forms, including shifted,
    // immediate and fused multiply forms, write a W register. Do not infer
    // this for parameters, reductions, loads, or arbitrary I32 values.
    if !matches!(opcode, Opcode::Iadd | Opcode::Isub | Opcode::Imul) {
        return None;
    }
    let selection = std::env::var("MECH_ARM64_REUSE_I32_ARITHMETIC").ok()?;
    let selected = match selection.as_str() {
        "add" => opcode == Opcode::Iadd,
        "sub" => opcode == Opcode::Isub,
        "mul" => opcode == Opcode::Imul,
        "all" | "1" => true,
        _ => false,
    };
    if !selected {
        return None;
    }
    let reg = lower.put_value_in_regs(arg).only_reg()?;
    Some(smallvec::smallvec![ValueRegs::one(reg)])
}
