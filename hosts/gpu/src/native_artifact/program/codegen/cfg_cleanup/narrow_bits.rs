//! Discard proved zero high fields when only a packed word's low bits are used.
use cranelift_codegen::ir::{Function, InstBuilder, InstructionData, Opcode, Value, ValueDef};

/// This changes only scalar integer reductions. A full-width packed value,
/// including any store or error/resource use, retains its original definition.
pub(in crate::native_artifact) fn fold(function: &mut Function) -> usize {
    let mut changes = Vec::new();
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            let InstructionData::Unary {
                opcode: Opcode::Ireduce,
                arg,
            } = function.dfg.insts[inst]
            else {
                continue;
            };
            let ty = function.dfg.value_type(function.dfg.first_result(inst));
            if !ty.is_int() || ty.is_vector() || ty.bits() > 64 {
                continue;
            }
            let bits = ty.bits();
            let mut steps = 64;
            if zero_low_bits(function, arg, bits, 8, &mut steps) {
                changes.push((inst, ty, None));
                continue;
            }
            let ValueDef::Result(source, 0) =
                function.dfg.value_def(function.dfg.resolve_aliases(arg))
            else {
                continue;
            };
            let InstructionData::Binary { opcode, args } = function.dfg.insts[source] else {
                continue;
            };
            if !matches!(opcode, Opcode::Bor | Opcode::Bxor) {
                continue;
            }
            for (zero, keep) in [(args[0], args[1]), (args[1], args[0])] {
                let mut steps = 64;
                if zero_low_bits(function, zero, bits, 8, &mut steps) {
                    changes.push((inst, ty, Some(keep)));
                    break;
                }
            }
        }
    }
    for &(inst, ty, replacement) in &changes {
        if let Some(value) = replacement {
            function.dfg.replace(inst).ireduce(ty, value);
        } else {
            function.dfg.replace(inst).iconst(ty, 0);
        }
    }
    changes.len()
}

fn zero_low_bits(
    function: &Function,
    value: Value,
    bits: u32,
    depth: usize,
    steps: &mut usize,
) -> bool {
    if depth == 0 || *steps == 0 {
        return false;
    }
    *steps -= 1;
    let value = function.dfg.resolve_aliases(value);
    let ty = function.dfg.value_type(value);
    if !ty.is_int() || ty.is_vector() || ty.bits() > 64 || bits == 0 || bits > ty.bits() {
        return false;
    }
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return false;
    };
    let mut zero = |value, bits| zero_low_bits(function, value, bits, depth - 1, steps);
    match function.dfg.insts[inst] {
        InstructionData::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } => (imm.bits() as u64) & (u64::MAX >> (64 - bits)) == 0,
        InstructionData::Unary {
            opcode: Opcode::Uextend | Opcode::Sextend | Opcode::Ireduce,
            arg,
        } => zero(arg, bits.min(function.dfg.value_type(arg).bits())),
        InstructionData::Binary { opcode, args } => match opcode {
            Opcode::Band => zero(args[0], bits) || zero(args[1], bits),
            Opcode::Bor | Opcode::Bxor => zero(args[0], bits) && zero(args[1], bits),
            Opcode::Ishl => {
                let mut constant_steps = 32;
                let Some(amount) =
                    super::constant_integer(function, args[1], 8, &mut constant_steps)
                else {
                    return false;
                };
                // CLIF masks a shift by the source width, not the destination
                // reduction width: i64 x << 64 is x, not a zero low word.
                let amount = (amount & u64::from(ty.bits() - 1)) as u32;
                amount >= bits || zero(args[0], bits - amount)
            }
            _ => false,
        },
        InstructionData::BinaryImm64 {
            opcode: Opcode::IshlImm,
            arg,
            imm,
        } => {
            let amount = (imm.bits() as u64 & u64::from(ty.bits() - 1)) as u32;
            amount >= bits || zero(arg, bits - amount)
        }
        _ => false,
    }
}

#[cfg(all(test, feature = "jit"))]
#[path = "narrow_bits_tests.rs"]
mod tests;
