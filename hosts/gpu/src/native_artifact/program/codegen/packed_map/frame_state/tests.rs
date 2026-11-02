use crate::native_artifact::codegen::options::OptimizationOptions;
use crate::native_artifact::program::codegen::packed_map::frame_state::*;
use cranelift_codegen::ir::{AbiParam, Function, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::default_libcall_names;

fn memory_instructions(function: &Function) -> (usize, usize) {
    let mut counts = (0, 0);
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            match function.dfg.insts[inst].opcode() {
                Opcode::Store => counts.0 += 1,
                Opcode::Load => counts.1 += 1,
                _ => {}
            }
        }
    }
    counts
}

#[test]
fn actual_call_emission_keeps_unknown_ops_and_terminal_spills_conservative() {
    let module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[], default_libcall_names()).unwrap(),
    );
    for op in [0, 1, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, u64::MAX] {
        // All three paths use the real call emitter, including its status
        // branch. This test inspects IR; the end-to-end fixtures execute it.
        for mode in 0..3 {
            let mut function = Function::new();
            function.signature.params = vec![AbiParam::new(types::I64)];
            function.signature.returns = vec![AbiParam::new(types::I64)];
            let mut context = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut function, &mut context);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.switch_to_block(entry);
            let pointer = b.block_params(entry)[0];
            let hooks = Hooks::load(true, &mut b, &module, pointer, pointer, error)
                .unwrap()
                .unwrap();
            let state = match mode {
                0 => FrameState::registers(
                    &mut b,
                    pointer,
                    Domain::ascii(),
                    OptimizationOptions::default(),
                ),
                1 => FrameState::registers(
                    &mut b,
                    pointer,
                    Domain::ascii(),
                    OptimizationOptions::without(OptimizationPass::FrameEffects),
                ),
                _ => FrameState::memory(pointer, Domain::ascii()),
            };
            let zero = b.ins().iconst(types::I64, 0);
            let before = memory_instructions(b.func);
            state.call(
                &mut b,
                &hooks,
                pointer,
                pointer,
                17,
                OwnerOperation::decode(op),
                [zero; 3],
                error,
            );
            let after = memory_instructions(b.func);
            let expected = match (mode, effects(OwnerOperation::decode(op))) {
                (0, Some(effect)) => (
                    effect.stores().count_ones() as usize,
                    effect.writes.count_ones() as usize,
                ),
                (0 | 1, _) => (14, 14),
                _ => (0, 0),
            };
            assert_eq!(
                (after.0 - before.0, after.1 - before.1),
                expected,
                "op={op}, mode={mode}"
            );
            assert_eq!(
                !state.effect_emissions().is_empty(),
                mode == 0 && effects(OwnerOperation::decode(op)).is_some()
            );
            state.spill(&mut b);
            let final_spill = memory_instructions(b.func);
            assert_eq!(
                (final_spill.0 - after.0, final_spill.1 - after.1),
                (if mode == 2 { 0 } else { 7 }, 0),
                "all generated exit/error state synchronized"
            );
            b.ins().return_(&[zero]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            b.ins().return_(&[status]);
            b.seal_all_blocks();
            b.finalize();
        }
    }
}

#[test]
fn private_effect_masks_cover_validation_growth_and_all_generated_writes() {
    assert_eq!((MUTABLE.count_ones(), GENERATED.count_ones()), (14, 7));
    assert_eq!(IMMUTABLE & GENERATED, 0);
    let refill = effects(OwnerOperation::Refill).unwrap();
    assert_eq!(
        (refill.stores().count_ones(), refill.writes.count_ones()),
        (2, 3)
    );
    assert_eq!(refill.stores(), fields!(fuel, entry_known));
    assert_eq!(refill.writes, fields!(fuel, entry_known, entry_pointer));
    let insert = effects(OwnerOperation::InsertMissingPacked).unwrap();
    assert_eq!(
        (insert.stores().count_ones(), insert.writes.count_ones()),
        (6, 9)
    );
    assert_eq!(insert.stores(), GENERATED & !fields!(previous_start));
    assert_eq!(
        insert.writes,
        fields!(
            slots,
            controls,
            capacity,
            len,
            updates,
            entry_pointer,
            range_call_allowed,
            read_call_allowed,
            write_call_allowed
        )
    );
    for op in [0, 7, 10, 11, 12] {
        let effect = effects(OwnerOperation::decode(op)).unwrap();
        assert_eq!(effect.reads & VALIDATION_READS, VALIDATION_READS);
        assert_eq!(effect.writes & IMMUTABLE, 0);
    }
    for op in [6, 8, 9] {
        assert_eq!(
            effects(OwnerOperation::decode(op)),
            Some(Effects {
                reads: 0,
                writes: 0
            })
        );
    }
    for op in [1, 2, 3, 4, 5, 13, 14, u64::MAX] {
        assert_eq!(effects(OwnerOperation::decode(op)), None);
    }
}
