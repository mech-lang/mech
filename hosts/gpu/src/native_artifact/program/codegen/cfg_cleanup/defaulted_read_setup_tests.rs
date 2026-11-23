use super::*;
use cranelift_codegen::Context;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use std::collections::BTreeMap;

fn native() -> cranelift_codegen::isa::OwnedTargetIsa {
    cranelift_native::builder()
        .unwrap()
        .finish(cranelift_codegen::settings::Flags::new(
            cranelift_codegen::settings::builder(),
        ))
        .unwrap()
}

#[test]
fn narrow_load_requires_the_current_private_base_and_a_low_word_only_result() {
    for (output_word, whole_value, effect, bias, expected) in [
        (0, false, false, -1, 1),
        (1, false, false, -1, 0),
        (0, true, false, -1, 0),
        (0, false, true, -1, 0),
        (0, false, false, -2, 0),
    ] {
        let isa = native();
        let mut context = Context::new();
        context.func.signature.params = vec![AbiParam::new(types::I64); 2];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut state);
        let entry = b.create_block();
        let read = b.create_block();
        let default = b.create_block();
        let join = b.create_block();
        b.append_block_param(join, types::I64);
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let packet = b.block_params(entry)[0];
        let length = b.block_params(entry)[1];
        let base = b.ins().load(types::I64, MemFlags::new(), packet, 0);
        let distance = b.ins().iconst(types::I64, -bias);
        let comparison_index = b.ins().isub(length, distance);
        let readable = b
            .ins()
            .icmp(IntCC::UnsignedLessThan, comparison_index, length);
        let zero = b.ins().iconst(types::I64, 0);
        let guard = b.ins().brif(readable, read, &[], default, &[]);
        b.switch_to_block(read);
        let one_based = b.ins().iadd_imm(comparison_index, 1);
        let ordinal = b.ins().iadd_imm(one_based, -1);
        let offset = b.ins().imul_imm(ordinal, 8);
        let addr = b.ins().iadd(base, offset);
        let loaded = b.ins().load(types::I64, MemFlags::new(), addr, 0);
        b.ins().jump(join, &[loaded.into()]);
        b.switch_to_block(default);
        if effect {
            b.ins().store(MemFlags::new(), zero, packet, 0);
        }
        b.ins().jump(join, &[zero.into()]);
        b.switch_to_block(join);
        let value = b.block_params(join)[0];
        let result = if whole_value {
            value
        } else {
            let mask = b.ins().iconst(types::I64, u32::MAX as i64);
            b.ins().band(value, mask)
        };
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();

        let base_load = context.func.dfg.value_def(base).inst().unwrap();
        let loads = BTreeMap::from([(base, (base_load, 0))]);
        context.compute_cfg();
        context.compute_domtree();
        let changes = optimize(
            &mut context.func,
            &context.cfg,
            &context.domtree,
            &Capture {
                packet_loads: &loads,
                views: &[],
                header: entry,
                countdown_word: 99,
            },
            98,
            output_word,
        );
        context.verify(&*isa).unwrap();
        assert_eq!(
            changes.narrowed_loads, expected,
            "case {output_word} {whole_value} {effect} {bias}: {changes:?}"
        );
        let Data::Brif { arg, .. } = context.func.dfg.insts[guard] else {
            panic!("checked read still selects between two paths");
        };
        assert_eq!(arg, if expected == 0 { readable } else { length });
    }
}

#[test]
fn cursor_rebases_all_incoming_edges_and_declines_a_live_unbiased_use() {
    for unbiased_use in [false, true] {
        let isa = native();
        let mut context = Context::new();
        context
            .func
            .signature
            .params
            .push(AbiParam::new(types::I64));
        context
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I64));
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut state);
        let entry = b.create_block();
        let header = b.create_block();
        let repeat = b.create_block();
        let finish = b.create_block();
        b.append_block_param(header, types::I64);
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let initial = b.block_params(entry)[0];
        let entry_edge = b.ins().jump(header, &[initial.into()]);
        b.switch_to_block(header);
        let input = b.block_params(header)[0];
        let index = b.ins().iadd_imm(input, -1);
        b.ins().brif(index, repeat, &[], finish, &[]);
        b.switch_to_block(repeat);
        let backedge = b.ins().jump(header, &[index.into()]);
        b.switch_to_block(finish);
        b.ins().return_(&[if unbiased_use { input } else { index }]);
        b.seal_all_blocks();
        b.finalize();

        context.compute_cfg();
        context.compute_domtree();
        let changed = rebase_cursor(&mut context.func, &context.cfg, &context.domtree, header, 0);
        context.verify(&*isa).unwrap();
        assert_eq!(changed, !unbiased_use);
        if changed {
            assert_eq!(context.func.dfg.resolve_aliases(index), input);
            for (edge, expected) in [(entry_edge, initial), (backedge, input)] {
                let Data::Jump { destination, .. } = context.func.dfg.insts[edge] else {
                    panic!("rebasing preserves the jump");
                };
                let value = destination
                    .args(&context.func.dfg.value_lists)
                    .next()
                    .unwrap();
                assert_eq!(
                    minus_one(&context.func, value.as_value().unwrap()),
                    Some(expected)
                );
            }
        }
    }
}
