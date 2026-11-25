//! Replace only the carry flags fixed by unsigned SSA bounds.
use super::{integer_mask, word_ranges::Facts};
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    flowgraph::ControlFlowGraph,
    ir::{Function, InstBuilder, InstructionData, Opcode, types},
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
            let a = facts.value_range(left)?;
            let b = facts.value_range(right)?;
            let carry = if a.1 <= mask - b.1 {
                false
            } else if a.0 > mask - b.0 {
                true
            } else {
                return None;
            };
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
