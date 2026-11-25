//! Reuse native byte definitions whose entire register is already unsigned.
use super::{
    Endianness, I8, I16, I32, I64, Inst, InstOutput, InstructionData, Lower, MInst, Opcode,
    ValueRegs,
};
use crate::ir::{DataFlowGraph, Value};

// An I8 type alone says nothing about the physical upper register bits.
// CSET and the exact constants below define a full-register 0 or 1; AND,
// OR and XOR preserve that property when both inputs have it. This also
// covers the combined-comparison CSET lowering. Decline unknown producers,
// block parameters and over-budget trees without changing their emission.
fn clean_boolean(dfg: &DataFlowGraph, value: Value, depth: usize, nodes: &mut usize) -> bool {
    if depth == 0 || *nodes == 0 {
        return false;
    }
    *nodes -= 1;
    let value = dfg.resolve_aliases(value);
    if dfg.value_type(value) != I8 {
        return false;
    }
    let Some(producer) = dfg.value_def(value).inst() else {
        return false;
    };
    match dfg.insts[producer] {
        InstructionData::IntCompare {
            opcode: Opcode::Icmp,
            args: [lhs, rhs],
            ..
        } => {
            dfg.value_type(lhs) == dfg.value_type(rhs)
                && matches!(dfg.value_type(lhs), I8 | I16 | I32 | I64)
        }
        InstructionData::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } => matches!(imm.bits(), 0 | 1),
        InstructionData::Binary {
            opcode: Opcode::Band | Opcode::Bor | Opcode::Bxor,
            args: [lhs, rhs],
        } => clean_boolean(dfg, lhs, depth - 1, nodes) && clean_boolean(dfg, rhs, depth - 1, nodes),
        _ => false,
    }
}

pub(super) fn lower(
    lower: &mut Lower<MInst>,
    inst: Inst,
    options: &crate::isa::Aarch64CompilationOptions,
) -> Option<InstOutput> {
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
    let selected = match dfg.insts[producer] {
        // The existing scalar I8 load rule emits ULoad8 (LDRB), clearing
        // the rest of the register. Reuse the definition, even when shared;
        // do not sink or duplicate the load or change its flags or address.
        InstructionData::Load {
            opcode: Opcode::Load,
            flags,
            ..
        } if flags.endianness(Endianness::Little) == Endianness::Little => options.reuse_i8_loads,
        // Scalar comparisons lower through flags_and_cc_to_bool, whose CSet
        // writes exactly 0 or 1 to the entire register. Vector comparisons,
        // arbitrary I8 inputs, signed loads and reductions are excluded.
        InstructionData::IntCompare {
            opcode: Opcode::Icmp,
            args: [lhs, _],
            ..
        } if matches!(dfg.value_type(lhs), I8 | I16 | I32 | I64) => options.reuse_i8_compares,
        InstructionData::Binary {
            opcode: Opcode::Band | Opcode::Bor | Opcode::Bxor,
            ..
        } => options.reuse_i8_booleans && clean_boolean(dfg, arg, 16, &mut 64),
        _ => false,
    };
    if !selected {
        return None;
    }
    let reg = lower.put_value_in_regs(arg).only_reg()?;
    Some(smallvec::smallvec![ValueRegs::one(reg)])
}
