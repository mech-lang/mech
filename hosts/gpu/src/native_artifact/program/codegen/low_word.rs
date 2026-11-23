//! Lower wrapping arithmetic in the width that every consumer observes.
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{
    Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::BTreeMap;

const LOW_WORD: i64 = u32::MAX as i64;

fn is_low_mask(function: &Function, value: Value) -> bool {
    let value = function.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, _) = function.dfg.value_def(value) else {
        return false;
    };
    matches!(function.dfg.insts[inst], Data::UnaryImm { opcode: Opcode::Iconst, imm }
        if i64::from(imm) == LOW_WORD)
}

fn masked_input(function: &Function, inst: Inst) -> Option<Value> {
    let [result] = function.dfg.inst_results(inst) else {
        return None;
    };
    if function.dfg.value_type(*result) != types::I64 {
        return None;
    }
    match function.dfg.insts[inst] {
        Data::Binary {
            opcode: Opcode::Band,
            args,
        } => {
            if is_low_mask(function, args[1]) {
                Some(args[0])
            } else if is_low_mask(function, args[0]) {
                Some(args[1])
            } else {
                None
            }
        }
        Data::BinaryImm64 {
            opcode: Opcode::BandImm,
            arg,
            imm,
        } if i64::from(imm) == LOW_WORD => Some(arg),
        _ => None,
    }
}

pub(in crate::native_artifact) fn narrow(function: &mut Function) -> usize {
    let mut uses = BTreeMap::<Value, (usize, usize)>::new();
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            let masked = masked_input(function, inst).map(|v| function.dfg.resolve_aliases(v));
            for value in function.dfg.inst_values(inst) {
                let value = function.dfg.resolve_aliases(value);
                let (total, low) = uses.entry(value).or_default();
                *total += 1;
                *low += usize::from(masked == Some(value));
            }
        }
    }
    let arithmetic = uses
        .into_iter()
        .filter_map(|(value, (total, low))| {
            if total != low || function.dfg.value_type(value) != types::I64 {
                return None;
            }
            let ValueDef::Result(inst, _) = function.dfg.value_def(value) else {
                return None;
            };
            match function.dfg.insts[inst] {
                Data::Binary { opcode, args }
                    if matches!(opcode, Opcode::Iadd | Opcode::Isub | Opcode::Imul) =>
                {
                    Some((inst, opcode, args))
                }
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    let mut cursor = FuncCursor::new(function);
    for &(inst, opcode, [left, right]) in &arithmetic {
        cursor.goto_inst(inst);
        let left = cursor.ins().ireduce(types::I32, left);
        let right = cursor.ins().ireduce(types::I32, right);
        // The low 32 bits of these wrapping operations depend only on the
        // low 32 bits of both inputs. Every current use already masks to that
        // width, including uses on other successors.
        let result = match opcode {
            Opcode::Iadd => cursor.ins().iadd(left, right),
            Opcode::Isub => cursor.ins().isub(left, right),
            Opcode::Imul => cursor.ins().imul(left, right),
            _ => unreachable!(),
        };
        cursor.func.dfg.replace(inst).uextend(types::I64, result);
    }
    arithmetic.len()
}
