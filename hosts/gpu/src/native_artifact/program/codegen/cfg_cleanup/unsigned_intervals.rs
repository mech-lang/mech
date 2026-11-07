//! Combine pure unsigned comparisons of the same immutable SSA word. The
//! wrapping subtraction belongs to this private predicate, not source arithmetic.
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Function, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef,
};

fn mask(function: &Function, value: Value) -> Option<u64> {
    let ty = function.dfg.value_type(value);
    (ty.is_int() && !ty.is_vector() && (1..=64).contains(&ty.bits()))
        .then(|| u64::MAX >> (64 - ty.bits()))
}

fn constant(function: &Function, value: Value) -> Option<u64> {
    let value = function.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    let Data::UnaryImm {
        opcode: Opcode::Iconst,
        imm,
    } = function.dfg.insts[inst]
    else {
        return None;
    };
    Some(imm.bits() as u64 & mask(function, value)?)
}

enum Bound {
    Lower(u64),
    Upper(u64),
}

fn bound(function: &Function, mut value: Value) -> Option<(Value, Bound)> {
    // Source Boolean values may be widened into word homes. Do not look
    // through arbitrary arithmetic, truncation, loads or phi definitions.
    for _ in 0..4 {
        value = function.dfg.resolve_aliases(value);
        let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
            return None;
        };
        let (cond, word, limit) = match function.dfg.insts[inst] {
            Data::Unary {
                opcode: Opcode::Uextend,
                arg,
            } => {
                value = arg;
                continue;
            }
            Data::IntCompare { cond, args, .. } => {
                if let Some(limit) = constant(function, args[1]) {
                    (cond, args[0], limit)
                } else {
                    (cond.swap_args(), args[1], constant(function, args[0])?)
                }
            }
            Data::IntCompareImm { cond, arg, imm, .. } => {
                (cond, arg, imm.bits() as u64 & mask(function, arg)?)
            }
            _ => return None,
        };
        let word = function.dfg.resolve_aliases(word);
        let maximum = mask(function, word)?;
        let bound = match cond {
            IntCC::UnsignedGreaterThanOrEqual => Bound::Lower(limit),
            IntCC::UnsignedGreaterThan if limit < maximum => Bound::Lower(limit + 1),
            IntCC::UnsignedLessThanOrEqual => Bound::Upper(limit),
            IntCC::UnsignedLessThan if limit > 0 => Bound::Upper(limit - 1),
            _ => return None,
        };
        return Some((word, bound));
    }
    None
}

pub(super) fn combine(function: &mut Function) -> usize {
    let changes = function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .filter_map(|inst| {
            let Data::Binary {
                opcode: Opcode::Band,
                args,
            } = function.dfg.insts[inst]
            else {
                return None;
            };
            let (a, left) = bound(function, args[0])?;
            let (b, right) = bound(function, args[1])?;
            if a != b {
                return None;
            }
            let (lower, upper) = match (left, right) {
                (Bound::Lower(lower), Bound::Upper(upper))
                | (Bound::Upper(upper), Bound::Lower(lower)) => (lower, upper),
                _ => return None,
            };
            (lower <= upper).then_some((inst, a, lower, upper))
        })
        .collect::<Vec<_>>();
    for &(inst, word, lower, upper) in &changes {
        let output = function.dfg.inst_results(inst)[0];
        let ty = function.dfg.value_type(output);
        let location = function.srcloc(inst);
        let mut cursor = FuncCursor::new(function).at_inst(inst);
        cursor.set_srcloc(location);
        let offset = cursor.ins().iadd_imm(word, 0u64.wrapping_sub(lower) as i64);
        if ty.bits() == 8 {
            cursor.func.dfg.replace(inst).icmp_imm(
                IntCC::UnsignedLessThanOrEqual,
                offset,
                (upper - lower) as i64,
            );
        } else {
            let valid = cursor.ins().icmp_imm(
                IntCC::UnsignedLessThanOrEqual,
                offset,
                (upper - lower) as i64,
            );
            cursor.func.dfg.replace(inst).uextend(ty, valid);
        }
    }
    changes.len()
}
