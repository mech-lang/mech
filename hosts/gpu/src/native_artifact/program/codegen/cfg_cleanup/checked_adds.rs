//! Replace only checked arithmetic predicates fixed by unsigned SSA bounds.
use super::{integer_mask, word_ranges::Facts};
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    flowgraph::ControlFlowGraph,
    ir::{Function, InstBuilder, InstructionData, Opcode, Value, condcodes::IntCC, types},
};

pub(in crate::native_artifact) fn fold(function: &mut Function, cfg: &ControlFlowGraph) -> usize {
    let mut facts = Facts::across_edges(function, cfg, None);
    let changes = function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .filter_map(|inst| {
            let InstructionData::Binary {
                opcode: Opcode::UaddOverflow,
                args: [left, right],
            } = function.dfg.insts[inst]
            else {
                return None;
            };
            let mask = integer_mask(function.dfg.value_type(left).bits())?;
            let carry = constant_carry(&mut facts, mask, left, right)?;
            Some((inst, left, right, carry))
        })
        .collect::<Vec<_>>();
    let mut cursor = FuncCursor::new(function);
    for &(inst, left, right, carry) in &changes {
        let &[sum, flag] = cursor.func.dfg.inst_results(inst) else {
            unreachable!("uadd_overflow defines a sum and a carry flag");
        };
        cursor.goto_inst(inst);
        cursor.use_srcloc(inst);
        cursor.func.dfg.detach_inst_results(inst);
        cursor.ins().with_result(sum).iadd(left, right);
        cursor
            .ins()
            .with_result(flag)
            .iconst(types::I8, i64::from(carry));
        cursor.remove_inst();
    }
    changes.len()
}

/// Recognize the unsigned carry comparisons left after a checked sum has
/// been emitted as ordinary SSA arithmetic. Both operands must have bounds
/// on every incoming edge, and the comparison must use an exact addend.
pub(in crate::native_artifact) fn fold_lowered_carries(
    function: &mut Function,
    cfg: &ControlFlowGraph,
) -> usize {
    let mut facts = Facts::across_edges(function, cfg, None);
    let changes = function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .filter_map(|inst| {
            let InstructionData::IntCompare {
                cond,
                args: [first, second],
                ..
            } = function.dfg.insts[inst]
            else {
                return None;
            };
            let (sum, addend, invert) = match cond {
                IntCC::UnsignedLessThan => (first, second, false),
                IntCC::UnsignedGreaterThan => (second, first, false),
                IntCC::UnsignedGreaterThanOrEqual => (first, second, true),
                IntCC::UnsignedLessThanOrEqual => (second, first, true),
                _ => return None,
            };
            let sum = function.dfg.resolve_aliases(sum);
            let addend = function.dfg.resolve_aliases(addend);
            let definition = function.dfg.value_def(sum).inst()?;
            let InstructionData::Binary {
                opcode: Opcode::Iadd,
                args: [left, right],
            } = function.dfg.insts[definition]
            else {
                return None;
            };
            if addend != function.dfg.resolve_aliases(left)
                && addend != function.dfg.resolve_aliases(right)
            {
                return None;
            }
            let mask = integer_mask(function.dfg.value_type(sum).bits())?;
            let carry = constant_carry(&mut facts, mask, left, right)?;
            Some((inst, carry ^ invert))
        })
        .collect::<Vec<_>>();
    for &(inst, predicate) in &changes {
        function
            .dfg
            .replace(inst)
            .iconst(types::I8, i64::from(predicate));
    }
    changes.len()
}

fn constant_carry(facts: &mut Facts<'_>, mask: u64, left: Value, right: Value) -> Option<bool> {
    let a = facts.value_range(left)?;
    let b = facts.value_range(right)?;
    if a.1 <= mask - b.1 {
        Some(false)
    } else if a.0 > mask - b.0 {
        Some(true)
    } else {
        None
    }
}

/// A checked unsigned multiplication cannot overflow when every incoming
/// definition bounds both operands to a product that fits in the low word.
pub(in crate::native_artifact) fn fold_zero_high_products(
    function: &mut Function,
    cfg: &ControlFlowGraph,
) -> usize {
    let mut facts = Facts::across_edges(function, cfg, None);
    let changes = function
        .layout
        .blocks()
        .flat_map(|block| function.layout.block_insts(block))
        .filter_map(|inst| {
            let InstructionData::Binary {
                opcode: Opcode::Umulhi,
                args: [left, right],
            } = function.dfg.insts[inst]
            else {
                return None;
            };
            if function.dfg.value_type(left) != types::I64 {
                return None;
            }
            facts
                .value_range(left)?
                .1
                .checked_mul(facts.value_range(right)?.1)?;
            Some(inst)
        })
        .collect::<Vec<_>>();
    for &inst in &changes {
        function.dfg.replace(inst).iconst(types::I64, 0);
    }
    changes.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::{
        ir::{AbiParam, ValueDef},
        settings,
        verifier::verify_function,
    };
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    #[test]
    fn lowered_carry_checks_use_the_exact_addends_and_full_ranges() {
        let mut function = Function::new();
        function.signature.params.push(AbiParam::new(types::I64));
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut function, &mut state);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let input = b.block_params(entry)[0];
        let small = b.ins().band_imm(input, 65535);
        let one = b.ins().iconst(types::I64, 1);
        let sum = b.ins().iadd(small, one);
        let wide_sum = b.ins().iadd(input, one);
        let tests = [
            (b.ins().icmp(IntCC::UnsignedLessThan, sum, small), Some(0)),
            (b.ins().icmp(IntCC::UnsignedGreaterThan, one, sum), Some(0)),
            (
                b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, sum, one),
                Some(1),
            ),
            (
                b.ins().icmp(IntCC::UnsignedLessThanOrEqual, small, sum),
                Some(1),
            ),
            (b.ins().icmp(IntCC::UnsignedLessThan, sum, input), None),
            (b.ins().icmp(IntCC::UnsignedLessThan, wide_sum, input), None),
        ];
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        let mut cfg = ControlFlowGraph::new();
        cfg.compute(&function);
        assert_eq!(fold_lowered_carries(&mut function, &cfg), 4);
        verify_function(&function, &settings::Flags::new(settings::builder())).unwrap();
        for (predicate, expected) in tests {
            let inst = function.dfg.value_def(predicate).inst().unwrap();
            match expected {
                Some(expected) => assert!(matches!(
                    function.dfg.insts[inst],
                    InstructionData::UnaryImm { opcode: Opcode::Iconst, imm } if imm.bits() == expected
                )),
                None => assert_eq!(function.dfg.insts[inst].opcode(), Opcode::Icmp),
            }
        }
    }

    #[test]
    fn product_high_word_requires_bounds_on_both_incoming_edges() {
        let mut function = Function::new();
        function.signature.params = vec![
            AbiParam::new(types::I32),
            AbiParam::new(types::I16),
            AbiParam::new(types::I8),
            AbiParam::new(types::I64),
        ];
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut function, &mut state);
        let source = b.create_block();
        let wide = b.create_block();
        let join = b.create_block();
        b.append_block_params_for_function_params(source);
        let bounded = b.append_block_param(join, types::I64);
        let unbounded = b.append_block_param(join, types::I64);
        b.switch_to_block(source);
        let [frequency, code_width, extra, unknown]: [Value; 4] =
            b.block_params(source).try_into().unwrap();
        let left = b.ins().uextend(types::I64, frequency);
        let width = b.ins().uextend(types::I64, code_width);
        let extra_word = b.ins().uextend(types::I64, extra);
        let term = b.ins().iadd(width, extra_word);
        b.ins()
            .brif(extra, wide, &[], join, &[term.into(), term.into()]);
        b.switch_to_block(wide);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(join, &[zero.into(), unknown.into()]);
        b.switch_to_block(join);
        let fits = b.ins().umulhi(left, bounded);
        let needs_check = b.ins().umulhi(left, unbounded);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        let mut cfg = ControlFlowGraph::new();
        cfg.compute(&function);
        assert_eq!(fold_zero_high_products(&mut function, &cfg), 1);
        verify_function(&function, &settings::Flags::new(settings::builder())).unwrap();
        let hi = function.dfg.value_def(fits).inst().unwrap();
        assert!(matches!(
            function.dfg.insts[hi],
            InstructionData::UnaryImm { opcode: Opcode::Iconst, imm } if imm.bits() == 0
        ));
        let hi = function.dfg.value_def(needs_check).inst().unwrap();
        assert_eq!(function.dfg.insts[hi].opcode(), Opcode::Umulhi);
    }

    #[test]
    fn retains_sum_and_carry_for_unproved_inputs() {
        let mut function = Function::new();
        function.signature.params.push(AbiParam::new(types::I8));
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut function, &mut state);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let input = b.block_params(entry)[0];
        let one = b.ins().iconst(types::I8, 1);
        let high = b.ins().iconst(types::I8, 128);
        let low = b.ins().band_imm(input, 127);
        let no_carry = b.ins().uadd_overflow(low, one);
        let always_carries = b.ins().uadd_overflow(high, high);
        let dynamic = b.ins().uadd_overflow(input, one);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        let mut cfg = ControlFlowGraph::new();
        cfg.compute(&function);
        assert_eq!(fold(&mut function, &cfg), 2);
        verify_function(&function, &settings::Flags::new(settings::builder())).unwrap();

        for ((sum, flag), operands, expected) in
            [(no_carry, [low, one], 0), (always_carries, [high, high], 1)]
        {
            let ValueDef::Result(sum, 0) = function.dfg.value_def(sum) else {
                panic!("expected replacement sum");
            };
            let ValueDef::Result(flag, 0) = function.dfg.value_def(flag) else {
                panic!("expected replacement carry");
            };
            assert!(matches!(
                function.dfg.insts[sum],
                InstructionData::Binary { opcode: Opcode::Iadd, args } if args == operands
            ));
            assert!(matches!(
                function.dfg.insts[flag],
                InstructionData::UnaryImm { opcode: Opcode::Iconst, imm } if imm.bits() == expected
            ));
        }
        let ValueDef::Result(add, 1) = function.dfg.value_def(dynamic.1) else {
            panic!("the original carry must still be computed");
        };
        assert_eq!(function.dfg.inst_results(add), &[dynamic.0, dynamic.1]);
        assert_eq!(function.dfg.insts[add].opcode(), Opcode::UaddOverflow);
    }
}
