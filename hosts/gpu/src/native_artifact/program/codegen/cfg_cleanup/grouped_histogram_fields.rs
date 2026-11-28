//! Fold packed tally fields proved by current unsigned word ranges.
use super::{definition, with_constant};
use crate::native_artifact::program::codegen::cfg_cleanup::word_ranges::Facts;
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{
    Function, InstBuilder, InstructionData as Data, Opcode as Op, Value, condcodes::IntCC, types,
};

enum Field {
    Word(Value),
    Borrow {
        left: Value,
        right: Value,
        bits: u64,
    },
}

fn borrow(f: &Function, facts: &mut Facts<'_>, value: Value) -> Option<Field> {
    let (masked, shift) = with_constant(f, value, Op::Ushr, Op::UshrImm, false)?;
    if !(1..64).contains(&shift) {
        return None;
    }
    let (difference, mask) = with_constant(f, masked, Op::Band, Op::BandImm, true)?;
    if mask & mask.wrapping_add(1) != 0 || mask >> shift == 0 {
        return None;
    }
    let Data::Binary {
        opcode: Op::Isub,
        args: [left, right],
    } = definition(f, difference)?
    else {
        return None;
    };
    // Both inputs fit below the extract boundary. A nonnegative difference
    // has no bits there. Wrapping a negative difference makes every selected
    // bit above that boundary one.
    let boundary = 1u64 << shift;
    if facts.value_range(left)?.1 >= boundary || facts.value_range(right)?.1 >= boundary {
        return None;
    }
    Some(Field::Borrow {
        left,
        right,
        bits: mask >> shift,
    })
}

pub(in crate::native_artifact) fn fold(context: &mut Context, mode: &str) -> (usize, usize) {
    context.compute_cfg();
    let f = &context.func;
    let mut facts = Facts::bitwise_edges(f, &context.cfg);
    let changes = f
        .layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .filter_map(|inst| {
            let &value = f.dfg.inst_results(inst).first()?;
            if f.dfg.value_type(value) != types::I64 {
                return None;
            }
            if matches!(mode, "borrow" | "all")
                && let Some(field) = borrow(f, &mut facts, value)
            {
                return Some((inst, field));
            }
            if !matches!(mode, "mask" | "all") {
                return None;
            }
            let (word, mask) = with_constant(f, value, Op::Band, Op::BandImm, true)?;
            if mask & mask.wrapping_add(1) == 0 && facts.value_range(word)?.1 <= mask {
                return Some((inst, Field::Word(word)));
            }
            None
        })
        .collect::<Vec<_>>();
    let mut masks = 0;
    let mut borrows = 0;
    for (inst, field) in changes {
        let mut cursor = FuncCursor::new(&mut context.func).at_inst(inst);
        match field {
            Field::Word(word) => {
                cursor.func.dfg.replace(inst).iadd_imm(word, 0);
                masks += 1;
            }
            Field::Borrow { left, right, bits } => {
                let negative = cursor.ins().icmp(IntCC::UnsignedLessThan, left, right);
                let ones = cursor.ins().iconst(types::I64, bits as i64);
                let zero = cursor.ins().iconst(types::I64, 0);
                cursor.func.dfg.replace(inst).select(negative, ones, zero);
                borrows += 1;
            }
        }
    }
    (masks, borrows)
}
