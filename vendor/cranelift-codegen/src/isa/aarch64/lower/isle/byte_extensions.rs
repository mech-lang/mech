//! Reuse native byte definitions whose entire register is already unsigned.
use super::{
    Endianness, I8, I16, I32, I64, Inst, InstOutput, InstructionData, Lower, MInst, Opcode,
    ValueRegs,
};

pub(super) fn lower(lower: &mut Lower<MInst>, inst: Inst) -> Option<InstOutput> {
    let dfg = lower.dfg();
    let InstructionData::Unary {
        opcode: Opcode::Uextend,
        arg,
    } = dfg.insts[inst]
    else {
        return None;
    };
    if lower.output_ty(inst, 0) != I64 || dfg.value_type(arg) != I8 {
        return None;
    }
    let arg = dfg.resolve_aliases(arg);
    let producer = dfg.value_def(arg).inst()?;
    let class = match dfg.insts[producer] {
        // The existing scalar I8 load rule emits ULoad8 (LDRB), clearing
        // the rest of the register. Reuse the definition, even when shared;
        // do not sink or duplicate the load or change its flags or address.
        InstructionData::Load {
            opcode: Opcode::Load,
            flags,
            ..
        } if flags.endianness(Endianness::Little) == Endianness::Little => "load",
        // Scalar comparisons lower through flags_and_cc_to_bool, whose CSet
        // writes exactly 0 or 1 to the entire register. Vector comparisons,
        // arbitrary I8 inputs, signed loads and reductions are excluded.
        InstructionData::IntCompare {
            opcode: Opcode::Icmp,
            args: [lhs, _],
            ..
        } if matches!(dfg.value_type(lhs), I8 | I16 | I32 | I64) => "compare",
        _ => return None,
    };
    let selection = std::env::var("MECH_ARM64_REUSE_I8_DEFINITIONS").ok()?;
    if selection != class && !matches!(selection.as_str(), "all" | "1") {
        return None;
    }
    let reg = lower.put_value_in_regs(arg).only_reg()?;
    Some(smallvec::smallvec![ValueRegs::one(reg)])
}
