use crate::native_artifact::program::codegen::cfg_cleanup::*;
use cranelift_codegen::ir::{AbiParam, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn cfg_cleanup_selected_ranges_do_not_assume_a_side_when_both_edges_share_a_target() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64)];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function(
            "same_selected_destination",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        let shared = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let current = b.block_params(entry)[0];
        b.ins().brif(current, shared, &[], shared, &[]);
        b.switch_to_block(shared);
        b.ins().brif(current, yes, &[], no, &[]);
        for (block, value) in [(yes, 11), (no, 22)] {
            b.switch_to_block(block);
            let value = b.ins().iconst(types::I64, value);
            b.ins().return_(&[value]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    let controls = CompilationControls::new(
        crate::native_artifact::codegen::options::OptimizationOptions::default(),
    );
    let report = optimize(&controls, &mut context, module.isa()).unwrap();
    assert_eq!(report.selected_range_branches_removed, 0);
    context.verify(module.isa()).unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the retained executable has the exact one-word ABI; the owner
    // remains live for every synchronous original and repeated call.
    let entry: unsafe extern "C" fn(u64) -> u64 =
        unsafe { core::mem::transmute(module.get_finalized_function(id)) };
    for value in [1, 0, 2, u64::MAX, 0, 3] {
        assert_eq!(unsafe { entry(value) }, if value == 0 { 22 } else { 11 });
    }
}

#[test]
fn cfg_cleanup_selected_ranges_keep_joins_unselected_arms_widths_and_publication() {
    use crate::native_artifact::codegen::options::OptimizationOptions;
    use cranelift_codegen::ir::condcodes::IntCC;
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        for joined in [false, true] {
            for enabled in [false, true] {
                let mut module = cranelift_jit::JITModule::new(
                    cranelift_jit::JITBuilder::with_flags(
                        &[("opt_level", "speed")],
                        default_libcall_names(),
                    )
                    .unwrap(),
                );
                let mut context = module.make_context();
                context.func.signature.params = vec![AbiParam::new(types::I64); 3];
                context.func.signature.returns = vec![AbiParam::new(types::I64)];
                let id = module
                    .declare_function("selected_interval", Linkage::Local, &context.func.signature)
                    .unwrap();
                let mut fc = FunctionBuilderContext::new();
                let maximum = u64::MAX >> (64 - ty.bits());
                let upper = maximum.min(258);
                {
                    let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
                    let entry = b.create_block();
                    let checked = b.create_block();
                    let skipped = b.create_block();
                    let arithmetic = b.create_block();
                    let assertion = b.create_block();
                    let overflow = b.create_block();
                    let publish = b.create_block();
                    b.append_block_param(publish, types::I64);
                    b.append_block_params_for_function_params(entry);
                    b.switch_to_block(entry);
                    let args = b.block_params(entry).to_vec();
                    let word = if ty == types::I64 {
                        args[1]
                    } else {
                        b.ins().ireduce(ty, args[1])
                    };
                    b.ins().brif(args[2], checked, &[], skipped, &[]);
                    b.switch_to_block(checked);
                    let lower = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, word, 3);
                    let upper =
                        b.ins()
                            .icmp_imm(IntCC::UnsignedLessThanOrEqual, word, upper as i64);
                    let valid = b.ins().band(lower, upper);
                    b.ins().brif(valid, arithmetic, &[], assertion, &[]);
                    b.switch_to_block(skipped);
                    if joined {
                        b.ins().jump(arithmetic, &[]);
                    } else {
                        let value = b.ins().iconst(types::I64, 42);
                        b.ins().jump(publish, &[value.into()]);
                    }
                    b.switch_to_block(arithmetic);
                    let valid = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, word, 2);
                    let success = b.create_block();
                    b.ins().brif(valid, success, &[], overflow, &[]);
                    b.switch_to_block(success);
                    let result = b.ins().iadd_imm(word, -2);
                    let result = if ty == types::I64 {
                        result
                    } else {
                        b.ins().uextend(types::I64, result)
                    };
                    b.ins().jump(publish, &[result.into()]);
                    for (block, status) in [(assertion, 9), (overflow, 1)] {
                        b.switch_to_block(block);
                        let status = b.ins().iconst(types::I64, status);
                        b.ins().return_(&[status]);
                    }
                    b.switch_to_block(publish);
                    let result = b.block_params(publish)[0];
                    b.ins().store(MemFlags::new(), result, args[0], 0);
                    let zero = b.ins().iconst(types::I64, 0);
                    b.ins().return_(&[zero]);
                    b.seal_all_blocks();
                    b.finalize();
                }
                let options = if enabled {
                    OptimizationOptions::default()
                } else {
                    OptimizationOptions::without(OptimizationPass::CfgCleanup)
                };
                let report = optimize(
                    &CompilationControls::new(options),
                    &mut context,
                    module.isa(),
                )
                .unwrap();
                assert_eq!(
                    report.selected_range_branches_removed > 0,
                    enabled && !joined
                );
                context.verify(module.isa()).unwrap();
                module.define_function(id, &mut context).unwrap();
                module.finalize_definitions().unwrap();
                // SAFETY: the retained JIT owner has this exact bank/word/flag
                // ABI, and the initialized publication cell lives through each call.
                let entry: unsafe extern "C" fn(*mut u64, u64, u64) -> u64 =
                    unsafe { core::mem::transmute(module.get_finalized_function(id)) };
                let mut published = 12345;
                let mut inputs = (0..=512).collect::<Vec<u64>>();
                inputs.extend([u64::MAX, i64::MAX as u64, i64::MIN as u64, 4]);
                for flag in [1, 0, 2, u64::MAX] {
                    for &input in &inputs {
                        let word = input & maximum;
                        let prior = published;
                        let (status, expected) = if flag == 0 && !joined {
                            (0, 42)
                        } else if flag != 0 && !(3..=upper).contains(&word) {
                            (9, prior)
                        } else if word < 2 {
                            (1, prior)
                        } else {
                            (0, word - 2)
                        };
                        assert_eq!(
                            unsafe { entry(&mut published, input, flag) },
                            status,
                            "{ty} joined={joined} enabled={enabled} n={input} flag={flag}"
                        );
                        assert_eq!(published, expected);
                    }
                }
            }
        }
    }
}

#[test]
fn cfg_cleanup_selected_ranges_recheck_each_changed_loop_argument_and_recover() {
    use cranelift_codegen::ir::condcodes::IntCC;
    for enabled in [false, true] {
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let id = module
            .declare_function(
                "fresh_selected_loop",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            let header = b.create_block();
            let check = b.create_block();
            let arithmetic = b.create_block();
            let body = b.create_block();
            let reject = b.create_block();
            let overflow = b.create_block();
            let done = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(header, types::I64);
            b.append_block_param(header, types::I64);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            b.ins().jump(header, &[args[1].into(), args[2].into()]);
            b.switch_to_block(header);
            let current = b.block_params(header).to_vec();
            let more = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, current[1], 0);
            b.ins().brif(more, check, &[], done, &[]);
            b.switch_to_block(check);
            let lower = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, current[0], 3);
            let upper = b
                .ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, current[0], 258);
            let valid = b.ins().band(lower, upper);
            b.ins().brif(valid, arithmetic, &[], reject, &[]);
            b.switch_to_block(arithmetic);
            let valid = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, current[0], 2);
            b.ins().brif(valid, body, &[], overflow, &[]);
            b.switch_to_block(body);
            let next = b.ins().iadd_imm(current[0], -2);
            let remaining = b.ins().iadd_imm(current[1], -1);
            b.ins().jump(header, &[next.into(), remaining.into()]);
            for (block, status) in [(reject, 9), (overflow, 1)] {
                b.switch_to_block(block);
                let status = b.ins().iconst(types::I64, status);
                b.ins().return_(&[status]);
            }
            b.switch_to_block(done);
            b.ins().store(MemFlags::new(), current[0], args[0], 0);
            let zero = b.ins().iconst(types::I64, 0);
            b.ins().return_(&[zero]);
            b.seal_all_blocks();
            b.finalize();
        }
        let options = if enabled {
            crate::native_artifact::codegen::options::OptimizationOptions::default()
        } else {
            crate::native_artifact::codegen::options::OptimizationOptions::without(
                OptimizationPass::CfgCleanup,
            )
        };
        let report = optimize(
            &CompilationControls::new(options),
            &mut context,
            module.isa(),
        )
        .unwrap();
        assert_eq!(report.selected_range_branches_removed > 0, enabled);
        context.verify(module.isa()).unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: owned generated code exactly matches this publication/word/count
        // ABI; every call retains the initialized output and the executable.
        let entry: unsafe extern "C" fn(*mut u64, u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let mut output = 55;
        for (n, count, expected) in [
            (6, 2, Some(2)),
            (3, 2, None),
            (0, 0, Some(0)),
            (258, 3, Some(252)),
            (u64::MAX, 1, None),
            (3, 1, Some(1)),
            (1, 1, None),
            (7, 2, Some(3)),
        ] {
            let previous = output;
            assert_eq!(
                unsafe { entry(&mut output, n, count) },
                if expected.is_some() { 0 } else { 9 }
            );
            assert_eq!(output, expected.unwrap_or(previous));
        }
    }
}

#[test]
fn cfg_cleanup_selected_ranges_keep_fresh_loads_distinct_after_an_alias_write() {
    use cranelift_codegen::ir::condcodes::IntCC;
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64)];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let id = module
        .declare_function("fresh_alias_word", Linkage::Local, &context.func.signature)
        .unwrap();
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        let selected = b.create_block();
        let reject = b.create_block();
        let overflow = b.create_block();
        let publish = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let bank = b.block_params(entry)[0];
        let old = b.ins().load(types::I64, MemFlags::new(), bank, 0);
        let lower = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, old, 3);
        let upper = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, old, 258);
        let valid = b.ins().band(lower, upper);
        b.ins().brif(valid, selected, &[], reject, &[]);
        b.switch_to_block(selected);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().store(MemFlags::new(), zero, bank, 0);
        let fresh = b.ins().load(types::I64, MemFlags::new(), bank, 0);
        let valid = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, fresh, 2);
        b.ins().brif(valid, publish, &[], overflow, &[]);
        b.switch_to_block(publish);
        let result = b.ins().iadd_imm(fresh, -2);
        b.ins().store(MemFlags::new(), result, bank, 8);
        b.ins().return_(&[zero]);
        for (block, status) in [(reject, 9), (overflow, 1)] {
            b.switch_to_block(block);
            let status = b.ins().iconst(types::I64, status);
            b.ins().return_(&[status]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    let options = crate::native_artifact::codegen::options::OptimizationOptions::default();
    let report = optimize(
        &CompilationControls::new(options),
        &mut context,
        module.isa(),
    )
    .unwrap();
    assert_eq!(report.selected_range_branches_removed, 0);
    context.verify(module.isa()).unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the owned JIT entry accepts this initialized two-word bank and
    // cannot outlive its module; publication is observed after each call.
    let entry: unsafe extern "C" fn(*mut u64) -> u64 =
        unsafe { core::mem::transmute(module.get_finalized_function(id)) };
    for initial in [3, 258, 4, 259, 0, 7] {
        let mut bank = [initial, 99];
        assert_eq!(
            unsafe { entry(bank.as_mut_ptr()) },
            if (3..=258).contains(&initial) { 1 } else { 9 }
        );
        assert_eq!(
            bank,
            [
                if (3..=258).contains(&initial) {
                    0
                } else {
                    initial
                },
                99
            ]
        );
    }
}

#[test]
fn cfg_cleanup_unsigned_intervals_execute_boundaries_at_all_word_widths_and_keep_effects() {
    use crate::native_artifact::codegen::options::OptimizationOptions;
    use cranelift_codegen::ir::condcodes::IntCC;
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        let mask = u64::MAX >> (64 - ty.bits());
        for (lower, upper) in [
            (3, 258.min(mask)),
            (0, 0),
            (0, mask),
            (mask, mask),
            (mask / 2 + 1, mask),
            (0, mask / 2),
        ] {
            for form in 0..4 {
                for enabled in [false, true] {
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
                            "unsigned_interval",
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
                        let args = b.block_params(entry).to_vec();
                        let word = if ty == types::I64 {
                            args[1]
                        } else {
                            b.ins().ireduce(ty, args[1])
                        };
                        let lo = b.ins().iconst(ty, lower as i64);
                        let hi = b.ins().iconst(ty, upper as i64);
                        let left = match form {
                            1 => b.ins().icmp(IntCC::UnsignedLessThanOrEqual, lo, word),
                            2 if lower > 0 => b.ins().icmp_imm(
                                IntCC::UnsignedGreaterThan,
                                word,
                                (lower - 1) as i64,
                            ),
                            2 => b.ins().icmp_imm(
                                IntCC::UnsignedGreaterThanOrEqual,
                                word,
                                lower as i64,
                            ),
                            _ => b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, word, lo),
                        };
                        // Even a false first predicate must retain the reached
                        // source effect between the two pure comparisons.
                        b.ins().store(MemFlags::new(), args[1], args[0], 0);
                        let right = match form {
                            1 => b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, hi, word),
                            2 if upper < mask => {
                                b.ins()
                                    .icmp_imm(IntCC::UnsignedLessThan, word, (upper + 1) as i64)
                            }
                            2 => {
                                b.ins()
                                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, word, upper as i64)
                            }
                            _ => b.ins().icmp(IntCC::UnsignedLessThanOrEqual, word, hi),
                        };
                        let widen = |b: &mut FunctionBuilder<'_>, value| {
                            let value = if form == 3 {
                                b.ins().uextend(types::I16, value)
                            } else {
                                value
                            };
                            if form == 0 {
                                value
                            } else {
                                b.ins().uextend(types::I64, value)
                            }
                        };
                        let left = widen(&mut b, left);
                        let right = widen(&mut b, right);
                        let valid = b.ins().band(left, right);
                        let valid = if form == 0 {
                            b.ins().uextend(types::I64, valid)
                        } else {
                            valid
                        };
                        b.ins().return_(&[valid]);
                        b.seal_all_blocks();
                        b.finalize();
                    }
                    let options = if enabled {
                        OptimizationOptions::default()
                    } else {
                        OptimizationOptions::without(OptimizationPass::CfgCleanup)
                    };
                    let report = optimize(
                        &CompilationControls::new(options),
                        &mut context,
                        module.isa(),
                    )
                    .unwrap();
                    assert_eq!(
                        report.unsigned_intervals_combined,
                        usize::from(enabled),
                        "{ty} {lower} {upper} {form}"
                    );
                    context.verify(module.isa()).unwrap();
                    module.define_function(id, &mut context).unwrap();
                    module.finalize_definitions().unwrap();
                    // SAFETY: this owned JIT entry has exactly the declared
                    // bank/word ABI; the bank and code live through each call.
                    let entry: unsafe extern "C" fn(*mut u64, u64) -> u64 =
                        unsafe { core::mem::transmute(module.get_finalized_function(id)) };
                    let mut inputs = (0..=255).collect::<Vec<u64>>();
                    inputs.extend([
                        lower.wrapping_sub(1),
                        lower,
                        lower.wrapping_add(1),
                        upper.wrapping_sub(1),
                        upper,
                        upper.wrapping_add(1),
                        mask,
                        mask.wrapping_add(1),
                        i64::MAX as u64,
                        i64::MIN as u64,
                        u64::MAX,
                    ]);
                    for input in inputs {
                        let mut observed = !input;
                        let value = input & mask;
                        assert_eq!(
                            unsafe { entry(&mut observed, input) },
                            u64::from(lower <= value && value <= upper),
                            "{ty} {lower} {upper} {form} {input}"
                        );
                        assert_eq!(observed, input);
                    }
                }
            }
        }
    }
}

#[test]
fn cfg_cleanup_unsigned_intervals_refuse_other_words_signed_dynamic_and_empty_ranges() {
    use cranelift_codegen::ir::condcodes::IntCC;
    for mode in 0..7 {
        let mut function = Function::new();
        function.signature.params = vec![AbiParam::new(types::I64); 2];
        function.signature.returns = vec![AbiParam::new(types::I64)];
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut function, &mut fc);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let left = match mode {
                0 => b
                    .ins()
                    .icmp_imm(IntCC::SignedGreaterThanOrEqual, args[0], 3),
                3 => b
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, args[0], 9),
                4 => b.ins().icmp_imm(IntCC::UnsignedGreaterThan, args[0], -1),
                5 => b
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, args[0], 0),
                _ => b
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, args[0], 3),
            };
            let right = match mode {
                0 => b.ins().icmp_imm(IntCC::SignedLessThanOrEqual, args[0], 258),
                1 => b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, args[1], 258),
                2 => b.ins().icmp_imm(IntCC::UnsignedGreaterThan, args[0], 2),
                3 => b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, args[0], 3),
                4 => b
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, args[0], -1),
                5 => b.ins().icmp_imm(IntCC::UnsignedLessThan, args[0], 0),
                _ => b
                    .ins()
                    .icmp(IntCC::UnsignedLessThanOrEqual, args[0], args[1]),
            };
            let valid = b.ins().band(left, right);
            let valid = b.ins().uextend(types::I64, valid);
            b.ins().return_(&[valid]);
            b.seal_all_blocks();
            b.finalize();
        }
        assert_eq!(unsigned_intervals::combine(&mut function), 0, "mode={mode}");
    }
}

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

#[test]
fn cfg_cleanup_proves_masked_and_extended_widths_but_keeps_unknown_guards() {
    use cranelift_codegen::ir::condcodes::IntCC;
    for (width, mode, mask, proved) in [
        (8, 0, 255u64, true),
        (16, 0, 65535, true),
        (32, 0, 4294967295, true),
        (32, 0, 8589934591, false),
        (32, 0, u64::MAX, false),
        (8, 2, 255, true),
        (16, 2, 65535, true),
        (32, 2, 4294967295, true),
        (32, 2, 8589934591, false),
        (8, 1, 255, true),
        (16, 1, 65535, true),
        (32, 1, 4294967295, true),
    ] {
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
                "width_range_effects",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        let maximum = u64::MAX >> (64 - width);
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            let yes = b.create_block();
            let no = b.create_block();
            let join = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(join, types::I64);
            b.switch_to_block(entry);
            let pointer = b.block_params(entry)[0];
            let input = b.block_params(entry)[1];
            let value = if mode == 2 {
                b.ins().band_imm(input, mask as i64)
            } else if mode == 0 {
                let mask = b.ins().iconst(types::I64, mask as i64);
                b.ins().band(input, mask)
            } else {
                let ty = match width {
                    8 => types::I8,
                    16 => types::I16,
                    _ => types::I32,
                };
                let small = b.ins().ireduce(ty, input);
                b.ins().uextend(types::I64, small)
            };
            let valid = b
                .ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, value, maximum as i64);
            b.ins().brif(valid, yes, &[], no, &[]);
            b.switch_to_block(yes);
            b.ins().store(MemFlags::new(), value, pointer, 0);
            b.ins().jump(join, &[value.into()]);
            b.switch_to_block(no);
            let bad = b.ins().iconst(types::I64, 0xbad);
            b.ins().store(MemFlags::new(), bad, pointer, 0);
            b.ins().jump(join, &[bad.into()]);
            b.switch_to_block(join);
            let result = b.block_params(join)[0];
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
        }
        let controls = CompilationControls::new(
            crate::native_artifact::codegen::options::OptimizationOptions::default(),
        );
        let report = optimize(&controls, &mut context, module.isa()).unwrap();
        assert_eq!(
            report.constant_branches_removed > 0,
            proved,
            "mode={mode} width={width} mask={mask:#x}: removed={}",
            report.constant_branches_removed
        );
        context.verify(module.isa()).unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the owned JIT function has exactly this pointer/word ABI;
        // each output is valid through the synchronous call and module lives.
        let entry: unsafe extern "C" fn(*mut u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for input in [0, 1, maximum, maximum + 1, u64::MAX] {
            let value = input & mask;
            let expected = if value <= maximum { value } else { 0xbad };
            let mut output = 0;
            assert_eq!(unsafe { entry(&mut output, input) }, expected);
            assert_eq!(output, expected);
        }
    }
}

#[test]
fn cfg_cleanup_program_local_homes_retire_dead_reads_and_observe_current_boundary_writes() {
    use crate::native_artifact::codegen::common::memory_regions;
    use crate::native_artifact::codegen::options::{
        CompilationControls, OptimizationOptions, OptimizationPass,
    };
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    unsafe extern "C" fn change_current(bank: *mut u64) {
        // SAFETY: the synchronous test call retains the initialized two-word
        // owner; the callback may mutate its first word, as ordinary hooks do.
        unsafe { *bank = (*bank).wrapping_add(7) };
    }
    for (options, expected_removed) in [
        (OptimizationOptions::default(), 1),
        (
            OptimizationOptions::without(OptimizationPass::NontrappingHomes),
            0,
        ),
        (
            OptimizationOptions::without(OptimizationPass::DisjointState),
            1,
        ),
        (
            OptimizationOptions::without(OptimizationPass::CfgCleanup),
            0,
        ),
    ] {
        let mut module = JITModule::new(
            JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names()).unwrap(),
        );
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let id = module
            .declare_function(
                "current_local_homes",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        let mut builder_context = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut builder_context);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let (locals, incoming, callback) = (args[0], args[1], args[2]);
            let flags = memory_regions::program_locals(options);
            let _unused = b.ins().load(types::I64, flags, locals, 8);
            let previous = b.ins().load(types::I64, flags, locals, 0);
            b.ins().store(MemFlags::new(), incoming, locals, 0);
            let after_store = b.ins().load(types::I64, flags, locals, 0);
            let mut signature = module.make_signature();
            signature.params.push(AbiParam::new(types::I64));
            let signature = b.func.import_signature(signature);
            b.ins().call_indirect(signature, callback, &[locals]);
            let after_callback = b.ins().load(types::I64, flags, locals, 0);
            let result = b.ins().iadd(previous, after_store);
            let result = b.ins().iadd(result, after_callback);
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
        }
        let controls = CompilationControls::new(options);
        let report = crate::native_artifact::program::codegen::cfg_cleanup::optimize(
            &controls,
            &mut context,
            module.isa(),
        )
        .unwrap();
        assert_eq!(report.dead_loads_removed, expected_removed);
        context.verify(module.isa()).unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: this JIT function has exactly the declared bank/word/callback
        // ABI. Module, initialized bank and callback live through each call.
        let entry: unsafe extern "C" fn(*mut u64, u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let mut bank = [5u64, 99];
        for incoming in [11u64, 2, u64::MAX, 17] {
            let previous = bank[0];
            let changed = incoming.wrapping_add(7);
            assert_eq!(
                unsafe {
                    entry(
                        bank.as_mut_ptr(),
                        incoming,
                        change_current as *const () as usize as u64,
                    )
                },
                previous.wrapping_add(incoming).wrapping_add(changed)
            );
            assert_eq!(bank, [changed, 99]);
        }
    }
}

#[test]
fn cfg_cleanup_boolean_phis_execute_fresh_checked_visits_and_preserve_publication() {
    use crate::native_artifact::codegen::options::OptimizationOptions;
    use cranelift_codegen::ir::condcodes::IntCC;
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        for unknown in [false, true] {
            let mut module = cranelift_jit::JITModule::new(
                cranelift_jit::JITBuilder::with_flags(
                    &[("opt_level", "speed")],
                    default_libcall_names(),
                )
                .unwrap(),
            );
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 3];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let id = module
                .declare_function(
                    "boolean_phi_visits",
                    Linkage::Local,
                    &context.func.signature,
                )
                .unwrap();
            let mut fc = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
                let entry = b.create_block();
                let header = b.create_block();
                let inspect = b.create_block();
                let zero = b.create_block();
                let nonzero = b.create_block();
                let one = b.create_block();
                let advance = b.create_block();
                let fail = b.create_block();
                let done = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.append_block_param(header, types::I64);
                b.append_block_param(header, ty);
                b.switch_to_block(entry);
                let args = b.block_params(entry).to_vec();
                let seed = if unknown {
                    if ty == types::I64 {
                        args[1]
                    } else {
                        b.ins().ireduce(ty, args[1])
                    }
                } else {
                    let seed = b.ins().icmp_imm(IntCC::NotEqual, args[1], 0);
                    if ty == types::I8 {
                        seed
                    } else {
                        b.ins().uextend(ty, seed)
                    }
                };
                let n = b.ins().iconst(types::I64, 0);
                b.ins().jump(header, &[n.into(), seed.into()]);
                b.switch_to_block(header);
                let params = b.block_params(header).to_vec();
                let more = b.ins().icmp(IntCC::UnsignedLessThan, params[0], args[2]);
                b.ins().brif(more, inspect, &[], done, &[]);
                b.switch_to_block(inspect);
                let is_zero = b.ins().icmp_imm(IntCC::Equal, params[1], 0);
                b.ins().brif(is_zero, zero, &[], nonzero, &[]);
                b.switch_to_block(zero);
                b.ins().jump(advance, &[]);
                b.switch_to_block(nonzero);
                let is_one = b.ins().icmp_imm(IntCC::Equal, params[1], 1);
                b.ins().brif(is_one, one, &[], fail, &[]);
                b.switch_to_block(one);
                b.ins().jump(advance, &[]);
                b.switch_to_block(advance);
                let one = b.ins().iconst(ty, 1);
                let tag = b.ins().bxor(params[1], one);
                let n = b.ins().iadd_imm(params[0], 1);
                b.ins().jump(header, &[n.into(), tag.into()]);
                b.switch_to_block(fail);
                let status = b.ins().iconst(types::I64, 17);
                b.ins().return_(&[status]);
                b.switch_to_block(done);
                let tag = if ty == types::I64 {
                    params[1]
                } else {
                    b.ins().uextend(types::I64, params[1])
                };
                b.ins().store(MemFlags::new(), tag, args[0], 0);
                let status = b.ins().iconst(types::I64, 0);
                b.ins().return_(&[status]);
                b.seal_all_blocks();
                b.finalize();
            }
            let controls = CompilationControls::new(OptimizationOptions::default());
            let report = optimize(&controls, &mut context, module.isa()).unwrap();
            assert_eq!(report.boolean_phi_values > 0, !unknown, "{ty} {unknown}");
            assert_eq!(
                report.selected_range_branches_removed > 0,
                !unknown,
                "{ty} {unknown}"
            );
            context.verify(module.isa()).unwrap();
            module.define_function(id, &mut context).unwrap();
            module.finalize_definitions().unwrap();
            // SAFETY: the retained JIT and bank use the exact declared ABI;
            // both owners live throughout every synchronous invocation.
            let entry: unsafe extern "C" fn(*mut u64, u64, u64) -> u64 =
                unsafe { core::mem::transmute(module.get_finalized_function(id)) };
            let mask = u64::MAX >> (64 - ty.bits());
            let mut published = 91;
            for (seed, count) in [(1, 3), (2, 1), (0, 4), (u64::MAX, 2), (0, 0), (1, 6)] {
                let before = published;
                let seed = if unknown {
                    seed & mask
                } else {
                    u64::from(seed != 0)
                };
                let bad = count > 0 && seed > 1;
                assert_eq!(
                    unsafe {
                        entry(
                            &mut published,
                            if unknown { seed } else { seed * 17 },
                            count,
                        )
                    },
                    if bad { 17 } else { 0 }
                );
                assert_eq!(published, if bad { before } else { seed ^ (count & 1) });
            }
        }
    }
}

#[test]
fn cfg_cleanup_boolean_phis_decline_aliases_fresh_loads_and_mutating_cycles() {
    use cranelift_codegen::ir::condcodes::IntCC;
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 3];
    let mut fc = FunctionBuilderContext::new();
    let values;
    {
        let mut b = FunctionBuilder::new(&mut function, &mut fc);
        let entry = b.create_block();
        let header = b.create_block();
        let body = b.create_block();
        let done = b.create_block();
        b.append_block_params_for_function_params(entry);
        for _ in 0..5 {
            b.append_block_param(header, types::I64);
        }
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let loaded = b.ins().load(types::I64, MemFlags::new(), args[0], 0);
        let one = b.ins().iconst(types::I64, 1);
        let masked = b.ins().band_imm(args[1], 1);
        b.ins().jump(
            header,
            &[
                args[1].into(),
                loaded.into(),
                one.into(),
                masked.into(),
                masked.into(),
            ],
        );
        b.switch_to_block(header);
        values = b.block_params(header).to_vec();
        b.ins().brif(args[2], body, &[], done, &[]);
        b.switch_to_block(body);
        let changed = b.ins().iadd_imm(values[2], 1);
        let masked = b.ins().band_imm(values[3], 1);
        let compared = b.ins().icmp(IntCC::Equal, values[0], values[1]);
        let compared = b.ins().uextend(types::I64, compared);
        b.ins().store(MemFlags::new(), values[0], args[0], 0);
        let fresh = b.ins().load(types::I64, MemFlags::new(), args[0], 0);
        b.ins().jump(
            header,
            &[
                values[0].into(),
                fresh.into(),
                changed.into(),
                masked.into(),
                compared.into(),
            ],
        );
        b.switch_to_block(done);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    let mut cfg = cranelift_codegen::flowgraph::ControlFlowGraph::new();
    cfg.compute(&function);
    let known = boolean_words::derive(&function, &cfg);
    for (i, value) in values.into_iter().enumerate() {
        assert_eq!(
            known.contains_key(&function.dfg.resolve_aliases(value)),
            i >= 3,
            "{i}"
        );
    }
}

#[test]
fn cfg_cleanup_boolean_phis_reject_unknown_seeds_through_mutual_cycles() {
    for unknown in [false, true] {
        let mut function = Function::new();
        function.signature.params = vec![AbiParam::new(types::I64); 2];
        let mut fc = FunctionBuilderContext::new();
        let values;
        {
            let mut b = FunctionBuilder::new(&mut function, &mut fc);
            let entry = b.create_block();
            let header = b.create_block();
            let body = b.create_block();
            let done = b.create_block();
            b.append_block_params_for_function_params(entry);
            for _ in 0..16 {
                b.append_block_param(header, types::I64);
            }
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let zero = b.ins().iconst(types::I64, 0);
            let seed = if unknown {
                args[0]
            } else {
                b.ins().band_imm(args[0], 1)
            };
            let mut seeds = vec![zero.into(); 16];
            seeds[0] = seed.into();
            b.ins().jump(header, &seeds);
            b.switch_to_block(header);
            values = b.block_params(header).to_vec();
            b.ins().brif(args[1], body, &[], done, &[]);
            b.switch_to_block(body);
            let mut rotated = values.iter().copied().map(Into::into).collect::<Vec<_>>();
            rotated.rotate_left(1);
            b.ins().jump(header, &rotated);
            b.switch_to_block(done);
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
        }
        let mut cfg = cranelift_codegen::flowgraph::ControlFlowGraph::new();
        cfg.compute(&function);
        let known = boolean_words::derive(&function, &cfg);
        for value in values {
            assert_eq!(
                known.contains_key(&function.dfg.resolve_aliases(value)),
                !unknown
            );
        }
    }
}
