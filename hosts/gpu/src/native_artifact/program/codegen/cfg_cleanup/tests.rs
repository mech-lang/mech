use crate::native_artifact::program::codegen::cfg_cleanup::*;
use cranelift_codegen::ir::{AbiParam, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn early_identities_remove_invariant_loop_parameters_before_the_backend_data_pass() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut functions = Vec::new();
    for enabled in [false, true] {
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let id = module
            .declare_function(
                &format!("identity_loop_{enabled}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        let header;
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            header = b.create_block();
            let body = b.create_block();
            let exit = b.create_block();
            b.append_block_params_for_function_params(entry);
            for _ in 0..3 {
                b.append_block_param(header, types::I64);
            }
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let zero = b.ins().iconst(types::I64, 0);
            b.ins()
                .jump(header, &[zero.into(), args[1].into(), zero.into()]);
            b.switch_to_block(header);
            let params = b.block_params(header).to_vec();
            let more = b.ins().icmp(IntCC::UnsignedLessThan, params[0], args[2]);
            b.ins().brif(more, body, &[], exit, &[]);
            b.switch_to_block(body);
            let combined = b.ins().bxor(params[2], params[1]);
            let value = b.ins().iadd(combined, params[0]);
            b.ins().store(MemFlags::new(), value, args[0], 0);
            let next = b.ins().iadd_imm(params[0], 1);
            let credit = b.ins().iadd_imm(params[1], 0);
            let credit = b.ins().bxor(credit, zero);
            b.ins()
                .jump(header, &[next.into(), credit.into(), value.into()]);
            b.switch_to_block(exit);
            b.ins().return_(&[params[2]]);
            b.seal_all_blocks();
            b.finalize();
        }
        assert_eq!(context.func.dfg.block_params(header).len(), 3);
        let options = if enabled {
            crate::native_artifact::codegen::options::OptimizationOptions::default()
        } else {
            crate::native_artifact::codegen::options::OptimizationOptions::without(
                OptimizationPass::EarlyIdentities,
            )
        };
        let report = optimize(
            &CompilationControls::new(options),
            &mut context,
            module.isa(),
        )
        .unwrap();
        assert_eq!(report.pre_codegen_data_passes, 0);
        assert_eq!(
            context.func.dfg.block_params(header).len(),
            if enabled { 2 } else { 3 }
        );
        assert_eq!(
            report.neutral_operations_removed,
            if enabled { 2 } else { 0 }
        );
        module.define_function(id, &mut context).unwrap();
        functions.push(id);
    }
    module.finalize_definitions().unwrap();
    for id in functions {
        // SAFETY: these owned functions use exactly pointer/credit/count words.
        let run: unsafe extern "C" fn(*mut u64, u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for credit in [0, 1, 127, 1u64 << 63, u64::MAX] {
            for count in [0, 1, 2, 3, 10, 97, 127] {
                let mut expected = 0u64;
                for at in 0..count {
                    expected = (expected ^ credit).wrapping_add(at);
                }
                let mut stored = 17;
                assert_eq!(unsafe { run(&mut stored, credit, count) }, expected);
                assert_eq!(stored, if count == 0 { 17 } else { expected });
            }
        }
    }
}

#[test]
fn early_integer_identities_preserve_dynamic_native_values_at_all_widths() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut functions = Vec::new();
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        let mask = integer_mask(ty.bits()).unwrap();
        for operator in 0..14 {
            for enabled in [false, true] {
                let mut context = module.make_context();
                context.func.signature.params = vec![AbiParam::new(types::I64)];
                context.func.signature.returns = vec![AbiParam::new(types::I64)];
                let id = module
                    .declare_function(
                        &format!("identity_word_{}", functions.len()),
                        Linkage::Local,
                        &context.func.signature,
                    )
                    .unwrap();
                let mut fc = FunctionBuilderContext::new();
                {
                    let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
                    let entry = b.create_block();
                    b.append_block_params_for_function_params(entry);
                    b.switch_to_block(entry);
                    let input = b.block_params(entry)[0];
                    let value = if ty == types::I64 {
                        input
                    } else {
                        b.ins().ireduce(ty, input)
                    };
                    let zero = b.ins().iconst(ty, 0);
                    let one = b.ins().iconst(ty, 1);
                    let full = b.ins().iconst(ty, -1);
                    let output = match operator {
                        0 => b.ins().iadd(value, zero),
                        1 => b.ins().iadd(zero, value),
                        2 => b.ins().isub(value, zero),
                        3 => b.ins().imul(value, one),
                        4 => b.ins().imul(one, value),
                        5 => b.ins().band(value, full),
                        6 => b.ins().band(full, value),
                        7 => b.ins().bor(value, zero),
                        8 => b.ins().bxor(zero, value),
                        9 => b.ins().iadd_imm(value, 0),
                        10 => b.ins().imul_imm(value, 1),
                        11 => b.ins().band_imm(value, -1),
                        12 => b.ins().bor_imm(value, 0),
                        13 => b.ins().bxor_imm(value, 0),
                        _ => unreachable!(),
                    };
                    let output = if ty == types::I64 {
                        output
                    } else {
                        b.ins().uextend(types::I64, output)
                    };
                    b.ins().return_(&[output]);
                    b.seal_all_blocks();
                    b.finalize();
                }
                let options = if enabled {
                    crate::native_artifact::codegen::options::OptimizationOptions::default()
                } else {
                    crate::native_artifact::codegen::options::OptimizationOptions::without(
                        OptimizationPass::EarlyIdentities,
                    )
                };
                let report = optimize(
                    &CompilationControls::new(options),
                    &mut context,
                    module.isa(),
                )
                .unwrap();
                assert_eq!(report.neutral_operations_removed, usize::from(enabled));
                module.define_function(id, &mut context).unwrap();
                functions.push((id, mask));
            }
        }
    }
    module.finalize_definitions().unwrap();
    for (id, mask) in functions {
        // SAFETY: each owned function accepts and returns one exact word.
        let run: unsafe extern "C" fn(u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let mut state = 0x6a09e667f3bcc909u64;
        for value in [0, 1, 127, 128, 255, 256, u64::MAX, 1u64 << 63]
            .into_iter()
            .chain((0..128).map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state
            }))
        {
            assert_eq!(unsafe { run(value) }, value & mask);
        }
    }
}

#[test]
fn early_identities_keep_memory_effects_divisions_and_float_zero_semantics() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64)];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function(
            "identity_loaded_value",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let pointer = b.block_params(entry)[0];
        let loaded = b.ins().load(types::I64, MemFlags::new(), pointer, 0);
        let same = b.ins().iadd_imm(loaded, 0);
        let marker = b.ins().iconst(types::I64, 99);
        b.ins().store(MemFlags::new(), marker, pointer, 0);
        b.ins().return_(&[same]);
        b.seal_all_blocks();
        b.finalize();
    }
    let report = optimize(
        &CompilationControls::new(Default::default()),
        &mut context,
        module.isa(),
    )
    .unwrap();
    assert_eq!(report.neutral_operations_removed, 1);
    let opcodes = context
        .func
        .layout
        .blocks()
        .flat_map(|b| context.func.layout.block_insts(b))
        .map(|i| context.func.dfg.insts[i].opcode())
        .collect::<Vec<_>>();
    assert!(opcodes.contains(&Opcode::Load) && opcodes.contains(&Opcode::Store));
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the cell lives through this synchronous exact-pointer ABI call.
    let run: unsafe extern "C" fn(*mut u64) -> u64 =
        unsafe { core::mem::transmute(module.get_finalized_function(id)) };
    for value in [0, 1, u64::MAX] {
        let mut cell = value;
        assert_eq!(unsafe { run(&mut cell) }, value);
        assert_eq!(cell, 99);
    }

    let mut context = module.make_context();
    let mut fc = FunctionBuilderContext::new();
    context.func.signature.params = vec![AbiParam::new(types::F64), AbiParam::new(types::I64)];
    context.func.signature.returns = vec![AbiParam::new(types::F64)];
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let float = b.block_params(entry)[0];
        let word = b.block_params(entry)[1];
        let zero = b.ins().f64const(0.0);
        let value = b.ins().fadd(float, zero);
        let one = b.ins().iconst(types::I64, 1);
        let _division = b.ins().udiv(word, one);
        b.ins().return_(&[value]);
        b.seal_all_blocks();
        b.finalize();
    }
    assert_eq!(
        fold_identities(&mut context.func),
        0,
        "float addition and trapping-op vocabulary are not integer identity candidates"
    );
}
#[test]
fn cfg_cleanup_executes_only_selected_effects_and_keeps_dynamic_branches_and_block_arguments() {
    for (enabled, literal, wrapped) in [false, true]
        .into_iter()
        .flat_map(|enabled| [0i64, -128].map(|literal| (enabled, literal)))
        .flat_map(|(enabled, literal)| [false, true].map(|wrapped| (enabled, literal, wrapped)))
    {
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 2];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let id = module
            .declare_function(
                "constant_branch_effects",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            let live = b.create_block();
            let dead = b.create_block();
            let yes = b.create_block();
            let no = b.create_block();
            let join = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(join, types::I64);
            b.switch_to_block(entry);
            let pointer = b.block_params(entry)[0];
            let input = b.block_params(entry)[1];
            let mut constant = b.ins().iconst(types::I8, literal);
            if wrapped {
                // The real primitive's nonzero-divisor guard, combined with
                // a wrapped Bool comparison, must fold before the data pass.
                let divisor = b.ins().iconst(types::I64, 6);
                let valid = b.ins().icmp_imm(IntCC::NotEqual, divisor, 0);
                let value = b.ins().icmp_imm(IntCC::NotEqual, constant, 0);
                constant = b.ins().band(valid, value);
                constant = b.ins().uextend(types::I64, constant);
            }
            b.ins().brif(constant, live, &[], dead, &[]);
            b.switch_to_block(dead);
            let bad = b.ins().iconst(types::I64, 999);
            b.ins().store(MemFlags::new(), bad, pointer, 0);
            b.ins().jump(join, &[bad.into()]);
            b.switch_to_block(live);
            b.ins().brif(input, yes, &[], no, &[]);
            b.switch_to_block(yes);
            let a = b.ins().iconst(types::I64, 11);
            b.ins().store(MemFlags::new(), a, pointer, 0);
            b.ins().jump(join, &[a.into()]);
            b.switch_to_block(no);
            let c = b.ins().iconst(types::I64, 22);
            b.ins().store(MemFlags::new(), c, pointer, 0);
            b.ins().jump(join, &[c.into()]);
            b.switch_to_block(join);
            let output = b.block_params(join)[0];
            b.ins().return_(&[output]);
            b.seal_all_blocks();
            b.finalize();
        }
        let options = if enabled {
            crate::native_artifact::codegen::options::OptimizationOptions::default()
        } else {
            crate::native_artifact::codegen::options::OptimizationOptions::without(
                crate::native_artifact::codegen::options::OptimizationPass::CfgCleanup,
            )
        };
        let controls = CompilationControls::new(options);
        let report = optimize(&controls, &mut context, module.isa()).unwrap();
        assert_eq!(report.enabled, enabled);
        if enabled {
            assert!(report.fixed_point && report.constant_branches_removed > 0);
            assert!(report.after_blocks < report.before_blocks);
        }
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: this matching owned JIT function accepts exactly these
        // two words; the mutable output lives through each synchronous call.
        let entry: unsafe extern "C" fn(*mut u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for (condition, expected) in [(0, 22), (1, 11), (2, 11), (u64::MAX, 11)] {
            let expected = if literal == 0 { 999 } else { expected };
            let mut output = 0;
            assert_eq!(unsafe { entry(&mut output, condition) }, expected);
            assert_eq!(output, expected);
        }
    }
}

#[test]
fn cfg_constant_integer_comparisons_match_native_execution_at_all_widths() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut cases = Vec::new();
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        let bits = ty.bits();
        let mask = integer_mask(bits).unwrap();
        let high = 1u64 << (bits - 1);
        for (left, right) in [
            (0, 0),
            (0, 1),
            (1, 0),
            (mask, 0),
            (high, mask),
            (mask, high),
            (high, 0),
            (mask, mask),
        ] {
            for condition in [
                IntCC::Equal,
                IntCC::NotEqual,
                IntCC::SignedLessThan,
                IntCC::SignedGreaterThanOrEqual,
                IntCC::SignedGreaterThan,
                IntCC::SignedLessThanOrEqual,
                IntCC::UnsignedLessThan,
                IntCC::UnsignedGreaterThanOrEqual,
                IntCC::UnsignedGreaterThan,
                IntCC::UnsignedLessThanOrEqual,
            ] {
                for immediate in [false, true] {
                    let mut context = module.make_context();
                    context.func.signature.returns = vec![AbiParam::new(types::I64)];
                    let id = module
                        .declare_function(
                            &format!("constant_compare_{}", cases.len()),
                            Linkage::Local,
                            &context.func.signature,
                        )
                        .unwrap();
                    let mut fc = FunctionBuilderContext::new();
                    let expression;
                    {
                        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
                        let entry = b.create_block();
                        b.switch_to_block(entry);
                        let a = b.ins().iconst(ty, signed_word(left, bits));
                        let test = if immediate {
                            b.ins().icmp_imm(condition, a, signed_word(right, bits))
                        } else {
                            let c = b.ins().iconst(ty, signed_word(right, bits));
                            b.ins().icmp(condition, a, c)
                        };
                        let extended = b.ins().uextend(types::I64, test);
                        expression = b.ins().ireduce(types::I8, extended);
                        let output = b.ins().uextend(types::I64, expression);
                        b.ins().return_(&[output]);
                        b.seal_all_blocks();
                        b.finalize();
                    }
                    let predicted = constant_condition(&context.func, expression).unwrap();
                    // Do not run our fold: the independent backend emits and
                    // executes the original comparison and type conversions.
                    module.define_function(id, &mut context).unwrap();
                    cases.push((id, predicted));
                }
            }
        }
    }
    module.finalize_definitions().unwrap();
    assert_eq!(cases.len(), 640);
    for (id, predicted) in cases {
        // SAFETY: each owned function has this exact zero-input word ABI.
        let entry: unsafe extern "C" fn() -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        assert_eq!(unsafe { entry() }, u64::from(predicted));
    }
}

#[test]
fn cfg_constant_integer_rejects_dynamic_memory_traps_and_exhausted_expressions() {
    let mut context = cranelift_codegen::Context::new();
    context.func.signature.params = vec![AbiParam::new(types::I64)];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let (dynamic, loaded, divided, deep, wide, wrapped, selected, signed);
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let parameter = b.block_params(entry)[0];
        dynamic = b.ins().icmp_imm(IntCC::NotEqual, parameter, 0);
        let value = b.ins().load(
            types::I64,
            MemFlags::new()
                .with_notrap()
                .with_readonly()
                .with_can_move(),
            parameter,
            0,
        );
        loaded = b.ins().icmp_imm(IntCC::NotEqual, value, 0);
        let one = b.ins().iconst(types::I64, 1);
        let zero = b.ins().iconst(types::I64, 0);
        let invalid = b.ins().udiv(one, zero);
        divided = b.ins().icmp_imm(IntCC::NotEqual, invalid, 0);
        let mut value = one;
        for _ in 0..MAX_CONSTANT_DEPTH + 2 {
            value = b.ins().iadd_imm(value, 1);
        }
        deep = value;
        let mut value = one;
        for _ in 0..7 {
            value = b.ins().band(value, value);
        }
        wide = value;
        let byte = b.ins().iconst(types::I8, -1);
        let byte_one = b.ins().iconst(types::I8, 1);
        wrapped = b.ins().iadd(byte, byte_one);
        selected = b.ins().select(wrapped, one, zero);
        signed = b.ins().sextend(types::I64, byte);
        b.ins().return_(&[zero]);
        b.seal_all_blocks();
        b.finalize();
    }
    for value in [dynamic, loaded, divided, deep, wide] {
        assert_eq!(constant_condition(&context.func, value), None);
    }
    assert_eq!(constant_condition(&context.func, wrapped), Some(false));
    assert_eq!(constant_condition(&context.func, selected), Some(false));
    let mut steps = MAX_CONSTANT_STEPS;
    assert_eq!(
        constant_integer(&context.func, signed, MAX_CONSTANT_DEPTH, &mut steps),
        Some(u64::MAX)
    );
}
