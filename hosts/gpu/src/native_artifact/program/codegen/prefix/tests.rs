//! Check the entry-only facts without executing or weakening any ABI guard.
use crate::native_artifact::program::codegen::prefix::*;
use cranelift_codegen::ir::{AbiParam, Function, InstructionData, Opcode};
use cranelift_frontend::FunctionBuilderContext;

fn comparison(function: &Function, value: Value, argument: Value, expected: i64) {
    let instruction = function.dfg.value_def(value).unwrap_inst();
    let InstructionData::IntCompareImm { cond, arg, imm, .. } = function.dfg.insts[instruction]
    else {
        panic!("entry predicate must remain an exact comparison");
    };
    assert_eq!(cond, IntCC::Equal);
    assert_eq!(arg, argument);
    assert_eq!(imm.bits(), expected);
}

#[test]
fn entry_facts_load_only_immutable_fields_and_distinguish_pending_from_final() {
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 2];
    function.signature.returns = vec![AbiParam::new(types::I64)];
    let mut context = FunctionBuilderContext::new();
    let (entry, body, frame, phase, facts);
    {
        let mut builder = FunctionBuilder::new(&mut function, &mut context);
        entry = builder.create_block();
        body = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        frame = builder.block_params(entry)[0];
        phase = builder.block_params(entry)[1];
        facts = EntryFacts::load(&mut builder, frame, phase);
        builder.ins().jump(body, &[]);
        builder.switch_to_block(body);
        builder.ins().return_(&[facts.available]);
        builder.seal_all_blocks();
        builder.finalize();
    }
    let loads = function
        .layout
        .blocks()
        .flat_map(|block| {
            function
                .layout
                .block_insts(block)
                .map(move |inst| (block, inst))
        })
        .filter(|(_, inst)| function.dfg.insts[*inst].opcode() == Opcode::Load)
        .collect::<Vec<_>>();
    assert_eq!(loads.len(), 2, "no mutable local or work-counter reads");
    assert!(loads.iter().all(|(block, _)| *block == entry));
    assert!(
        function
            .layout
            .block_insts(body)
            .all(|inst| { function.dfg.insts[inst].opcode() != Opcode::Load })
    );
    let eof = function.dfg.first_result(loads[0].1);
    for ((_, inst), expected) in loads.into_iter().zip([
        core::mem::offset_of!(
            crate::native_artifact::program::protocol::abi::Frame,
            input_eof
        ),
        core::mem::offset_of!(
            crate::native_artifact::program::protocol::abi::Frame,
            available_lines
        ),
    ]) {
        let InstructionData::Load { arg, offset, .. } = function.dfg.insts[inst] else {
            unreachable!();
        };
        assert_eq!(arg, frame);
        assert_eq!(i32::from(offset), expected as i32);
    }
    comparison(
        &function,
        facts.is_prefix,
        phase,
        crate::native_artifact::program::protocol::abi::PHASE_PREFIX as i64,
    );
    comparison(&function, facts.final_input, eof, 1);
    let pending = function.dfg.value_def(facts.pending_count).unwrap_inst();
    let InstructionData::Binary { opcode, args } = function.dfg.insts[pending] else {
        panic!("pending count must require BOTH prefix phase and EOF == 0");
    };
    assert_eq!(opcode, Opcode::Band);
    assert_eq!(args[0], facts.is_prefix);
    // EOF 2/u64::MAX must remain neither pending nor final. Negating the final
    // predicate here would change malformed-frame behavior at existing guards.
    comparison(&function, args[1], eof, 0);
}
