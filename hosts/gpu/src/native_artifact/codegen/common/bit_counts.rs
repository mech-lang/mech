//! Recognize the standard modulo-64 SWAR population count in sealed SSA.
//! Only pure integer dataflow changes. Original source visits, error edges,
//! memory operations and resource charges remain in the enclosing function.
use cranelift_codegen::ir::{
    Function, Inst, InstBuilder, InstructionData, Opcode as O, Value, ValueDef, types,
};

fn instruction(function: &Function, value: Value) -> Option<Inst> {
    let value = function.dfg.resolve_aliases(value);
    if function.dfg.value_type(value) != types::I64 {
        return None;
    }
    match function.dfg.value_def(value) {
        ValueDef::Result(inst, 0) if function.dfg.inst_results(inst).len() == 1 => Some(inst),
        _ => None,
    }
}

fn constant(function: &Function, value: Value, depth: usize) -> Option<u64> {
    if depth == 0 {
        return None;
    }
    let value = function.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    match function.dfg.insts[inst] {
        InstructionData::UnaryImm {
            opcode: O::Iconst,
            imm,
        } => Some(i64::from(imm) as u64),
        InstructionData::BinaryImm64 {
            opcode: O::BandImm,
            arg,
            imm,
        } => Some(constant(function, arg, depth - 1)? & (i64::from(imm) as u64)),
        InstructionData::Binary {
            opcode: O::Band,
            args,
        } => {
            Some(constant(function, args[0], depth - 1)? & constant(function, args[1], depth - 1)?)
        }
        _ => None,
    }
}

fn binary(function: &Function, value: Value, opcode: O) -> Option<[Value; 2]> {
    let inst = instruction(function, value)?;
    match function.dfg.insts[inst] {
        InstructionData::Binary {
            opcode: actual,
            args,
        } if actual == opcode => Some(args.map(|arg| function.dfg.resolve_aliases(arg))),
        _ => None,
    }
}

fn with_constant(
    function: &Function,
    value: Value,
    opcode: O,
    immediate: O,
    wanted: u64,
    commutative: bool,
) -> Option<Value> {
    let inst = instruction(function, value)?;
    match function.dfg.insts[inst] {
        InstructionData::Binary {
            opcode: actual,
            args,
        } if actual == opcode => {
            if constant(function, args[1], 4) == Some(wanted) {
                Some(function.dfg.resolve_aliases(args[0]))
            } else if commutative && constant(function, args[0], 4) == Some(wanted) {
                Some(function.dfg.resolve_aliases(args[1]))
            } else {
                None
            }
        }
        InstructionData::BinaryImm64 {
            opcode: actual,
            arg,
            imm,
        } if actual == immediate && i64::from(imm) as u64 == wanted => {
            Some(function.dfg.resolve_aliases(arg))
        }
        _ => None,
    }
}

fn shift(function: &Function, value: Value, count: u64) -> Option<Value> {
    with_constant(function, value, O::Ushr, O::UshrImm, count, false)
}
fn mask(function: &Function, value: Value, bits: u64) -> Option<Value> {
    with_constant(function, value, O::Band, O::BandImm, bits, true)
}

fn source(function: &Function, root: Value) -> Option<Value> {
    // (((b + (b >> 4)) & 0x0f...) * 0x0101...) >> 56
    let multiply = shift(function, root, 56)?;
    let c = with_constant(
        function,
        multiply,
        O::Imul,
        O::ImulImm,
        0x0101_0101_0101_0101,
        true,
    )?;
    let sum = mask(function, c, 0x0f0f_0f0f_0f0f_0f0f)?;
    let sum_args = binary(function, sum, O::Iadd)?;
    let b = if shift(function, sum_args[1], 4) == Some(sum_args[0]) {
        sum_args[0]
    } else if shift(function, sum_args[0], 4) == Some(sum_args[1]) {
        sum_args[1]
    } else {
        return None;
    };
    // b = (a & 0x3333...) + ((a >> 2) & 0x3333...)
    let pair = binary(function, b, O::Iadd)?;
    let left = mask(function, pair[0], 0x3333_3333_3333_3333)?;
    let right = mask(function, pair[1], 0x3333_3333_3333_3333)?;
    let a = if shift(function, right, 2) == Some(left) {
        left
    } else if shift(function, left, 2) == Some(right) {
        right
    } else {
        return None;
    };
    // a = x - ((x >> 1) & 0x5555...)
    let [x, subtract] = binary(function, a, O::Isub)?;
    let shifted = mask(function, subtract, 0x5555_5555_5555_5555)?;
    if shift(function, shifted, 1) != Some(x) || function.dfg.value_type(x) != types::I64 {
        return None;
    }
    Some(x)
}

pub(in crate::native_artifact) fn optimize(function: &mut Function) -> usize {
    let mut changed = 0;
    let mut block = function.layout.blocks().next();
    while let Some(current) = block {
        let mut inst = function.layout.first_inst(current);
        while let Some(at) = inst {
            let next = function.layout.next_inst(at);
            if matches!(function.dfg.insts[at].opcode(), O::Ushr | O::UshrImm)
                && let [root] = function.dfg.inst_results(at)
                && let Some(input) = source(function, *root)
            {
                // Keep this result ID and source location. Other users of an
                // intermediate still keep their original expression alive;
                // ordinary backend DCE removes only genuinely dead work.
                function.dfg.replace(at).popcnt(input);
                changed += 1;
            }
            inst = next;
        }
        block = function.layout.next_block(current);
    }
    changed
}
