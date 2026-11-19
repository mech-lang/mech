use super::{
    Candidate, CaptureBindings, RegionKind, controller_candidates, share, share_controllers,
    share_families, share_phase_regions,
};
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use std::collections::BTreeMap;

fn jit_builder() -> JITBuilder {
    JITBuilder::with_flags(
        &[("preserve_frame_pointers", "true"), ("unwind_info", "true")],
        default_libcall_names(),
    )
    .unwrap()
}

fn define(
    module: &mut JITModule,
    enabled: bool,
    extra_inputs: usize,
    return_error_origin: bool,
    origin_offset: i64,
) -> FuncId {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [choose, path, value] = *b.block_params(entry) else {
        unreachable!()
    };
    let extras = (1..=extra_inputs)
        .map(|index| b.ins().iadd_imm(value, index as i64))
        .collect::<Vec<_>>();
    let zero = b.ins().iconst(types::I64, 0);
    let seed = b.ins().iconst(types::I64, 11);
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    b.ins().stack_store(seed, slot, 0);
    let first = b.create_block();
    let second = b.create_block();
    for guard in [first, second] {
        for _ in 0..=extra_inputs {
            b.append_block_param(guard, types::I64);
        }
    }
    let initial = vec![zero.into(); 1 + extra_inputs];
    b.ins().brif(choose, second, &initial, first, &initial);
    let mut parts = Vec::new();
    for (guard, header) in [(first, 30), (second, 90)] {
        let start = b.create_block();
        let ready = b.create_block();
        let error = b.create_block();
        let choice = b.create_block();
        let exit = b.create_block();
        parts.push(Candidate {
            resume: Some(guard),
            start,
            exit,
            header,
            origin_base: None,
            span: 12,
        });
        for _ in 0..=extra_inputs {
            b.append_block_param(exit, types::I64);
        }
        b.switch_to_block(guard);
        let incoming = b.block_params(guard).to_vec();
        let (carry, rest) = incoming.split_first().unwrap();
        let carry = *carry;
        b.ins().brif(carry, ready, &[], start, &[]);
        b.switch_to_block(ready);
        let old = b.ins().stack_load(types::I64, slot, 0);
        let sum = rest.iter().fold(b.ins().iadd(old, carry), |sum, &value| {
            b.ins().iadd(sum, value)
        });
        let returned = std::iter::once(sum)
            .chain(rest.iter().copied())
            .map(Into::into)
            .collect::<Vec<_>>();
        b.ins().jump(exit, &returned);
        b.switch_to_block(start);
        let initial = b.ins().stack_load(types::I64, slot, 0);
        let advanced = extras
            .iter()
            .fold(b.ins().iadd(initial, value), |sum, &next| {
                b.ins().iadd(sum, next)
            });
        b.ins().stack_store(advanced, slot, 0);
        let reject = b.ins().icmp_imm(IntCC::Equal, path, 2);
        b.ins().brif(reject, error, &[], choice, &[]);
        b.switch_to_block(error);
        b.set_cold_block(error);
        let code = b.ins().iconst(types::I64, 17);
        let origin = b.ins().iconst(types::I64, header as i64 + origin_offset);
        b.ins().return_(&[code, origin]);
        b.switch_to_block(choice);
        let transition = std::iter::once(advanced)
            .chain(extras.iter().copied())
            .map(Into::into)
            .collect::<Vec<_>>();
        b.ins().brif(path, guard, &transition, exit, &transition);
        b.switch_to_block(exit);
        let output = b.block_params(exit).to_vec();
        let result = output[1..]
            .iter()
            .fold(output[0], |sum, &value| b.ins().iadd(sum, value));
        b.ins().return_(&[zero, result]);
    }
    b.seal_all_blocks();
    b.finalize();

    let id = module
        .declare_function(
            if enabled {
                "outlined_test"
            } else {
                "original_test"
            },
            Linkage::Export,
            &ctx.func.signature,
        )
        .unwrap();
    if enabled {
        let report = if extra_inputs == 0 {
            share(module, &mut ctx.func, BTreeMap::from([(id, parts)]), false)
        } else {
            assert!(
                share_families(
                    module,
                    &mut ctx.func,
                    vec![parts.clone()],
                    RegionKind::Refusal,
                    false,
                    CaptureBindings::default()
                )
                .unwrap()
                .is_none()
            );
            share_phase_regions(
                module,
                &mut ctx.func,
                vec![parts],
                CaptureBindings {
                    return_error_origin,
                    ..CaptureBindings::default()
                },
            )
        }
        .unwrap()
        .expect("both traversals selected");
        assert_eq!(report[0]["headers"], serde_json::json!([30, 90]));
        if extra_inputs != 0 {
            assert_eq!(
                report[0]["packet_words"],
                serde_json::json!([[47, 1], [47, 1]])
            );
            assert_eq!(report[0]["resume_packet_words"], serde_json::json!([1, 1]));
            assert_eq!(
                report[0]["returns_error_origin"].as_bool(),
                (return_error_origin
                    && u32::try_from(90 + origin_offset).is_ok()
                    && origin_offset >= 0)
                    .then_some(true)
            );
        }
    }
    ctx.verify(module.isa()).unwrap();
    module.define_function(id, &mut ctx).unwrap();
    id
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
struct ResultPair(u64, u64);

#[test]
fn shared_traversal_preserves_exit_resume_and_failure_origin() {
    let builder = jit_builder();
    let mut module = JITModule::new(builder);
    let original = define(&mut module, false, 0, false, 4);
    let outlined = define(&mut module, true, 0, false, 4);
    module.finalize_definitions().unwrap();
    // Both functions were defined above with the host C calling convention
    // and a pair of 64-bit integer return values.
    let bind = |id| unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    let old = bind(original);
    let new = bind(outlined);
    for (choice, origin) in [(0, 34), (1, 94)] {
        for (path, expected) in [
            (0, ResultPair(0, 16)),
            (1, ResultPair(0, 32)),
            (2, ResultPair(17, origin)),
        ] {
            assert_eq!(old(choice, path, 5), expected);
            assert_eq!(new(choice, path, 5), expected);
        }
    }
}

#[test]
fn packed_refusal_preserves_partial_resume_and_failure_origin() {
    let additional = 5 * 44 + 44 * 45 / 2;
    let advance = 11 + 5 + 2 * additional;
    for (return_error_origin, offset) in [
        (false, 4),
        (true, 0),
        (true, 4),
        (true, i64::from(u32::MAX) - 90),
        (true, -1),
        (true, i64::from(u32::MAX) + 1),
    ] {
        let mut module = JITModule::new(jit_builder());
        let original = define(&mut module, false, 44, false, offset);
        let outlined = define(&mut module, true, 44, return_error_origin, offset);
        module.finalize_definitions().unwrap();
        let bind = |id| unsafe {
            std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
                module.get_finalized_function(id),
            )
        };
        let old = bind(original);
        let new = bind(outlined);
        for (choice, header) in [(0, 30), (1, 90)] {
            for (path, expected) in [
                (0, ResultPair(0, advance)),
                (1, ResultPair(0, 2 * advance)),
                (2, ResultPair(17, (header + offset) as u64)),
            ] {
                assert_eq!(old(choice, path, 5), expected);
                assert_eq!(new(choice, path, 5), expected);
            }
        }
    }
}

fn define_controller(
    module: &mut JITModule,
    enabled: bool,
    varying_bounds: bool,
    register_inputs: bool,
    register_outputs: bool,
) -> FuncId {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [choice, count, reject] = *b.block_params(entry) else {
        unreachable!()
    };
    let zero = b.ins().iconst(types::I64, 0);
    let invariant = b.ins().iadd_imm(count, 21);
    let entries = [b.create_block(), b.create_block()];
    for block in entries {
        b.append_block_param(block, types::I64);
        b.append_block_param(block, types::I64);
        b.append_block_param(block, types::I64);
    }
    let args = [count.into(), zero.into(), invariant.into()];
    b.ins().brif(choice, entries[1], &args, entries[0], &args);
    let mut source = vec![None; 92];
    for (start, header) in entries.into_iter().zip([30, 90]) {
        let error = b.create_block();
        let body = b.create_block();
        let exit = b.create_block();
        source[header] = Some(start);
        source[header + 1] = Some(exit);
        b.append_block_param(exit, types::I64);
        b.append_block_param(exit, types::I64);
        b.switch_to_block(start);
        let [remaining, sum, unchanged] = *b.block_params(start) else {
            unreachable!()
        };
        b.ins().brif(reject, error, &[], body, &[]);
        b.switch_to_block(error);
        b.set_cold_block(error);
        let status = b.ins().iconst(types::I64, 17);
        let origin = b.ins().iconst(types::I64, header as i64 + 4);
        b.ins().return_(&[status, origin]);
        b.switch_to_block(body);
        let next = b.ins().iadd(sum, remaining);
        let more = if varying_bounds {
            let limit = b.ins().iconst(types::I64, if header == 30 { 5 } else { 9 });
            b.ins().icmp(IntCC::UnsignedGreaterThan, remaining, limit)
        } else {
            b.ins().icmp_imm(IntCC::UnsignedGreaterThan, remaining, 1)
        };
        let decreased = b.ins().iadd_imm(remaining, -1);
        b.ins().brif(
            more,
            start,
            &[decreased.into(), next.into(), unchanged.into()],
            exit,
            &[next.into(), unchanged.into()],
        );
        b.switch_to_block(exit);
        let [explicit, hold] = *b.block_params(exit) else {
            unreachable!()
        };
        let doubled = b.ins().iadd(explicit, next);
        let with_hold = b.ins().iadd(doubled, hold);
        let with_unchanged = b.ins().iadd(sum, unchanged);
        b.ins().return_(&[with_unchanged, with_hold]);
    }
    b.seal_all_blocks();
    b.finalize();
    let id = module
        .declare_function(
            if enabled {
                "shared_controller"
            } else {
                "inline_controller"
            },
            Linkage::Export,
            &ctx.func.signature,
        )
        .unwrap();
    if enabled {
        let mut candidates = controller_candidates("30:31,90:91", &source).unwrap();
        for candidate in &mut candidates {
            candidate.origin_base = Some(candidate.header as u32 - 7);
            candidate.header += 100;
        }
        let report = share_controllers(
            module,
            &mut ctx.func,
            vec![candidates],
            CaptureBindings {
                literals: varying_bounds,
                return_error_origin: register_inputs || register_outputs,
                register_inputs,
                register_outputs,
                ..CaptureBindings::default()
            },
        )
        .unwrap()
        .expect("requested controllers selected");
        assert_eq!(report[0]["headers"], serde_json::json!([130, 190]));
        let input_words = if varying_bounds { 4 } else { 3 };
        assert_eq!(
            report[0]["packet_words"],
            serde_json::json!([[input_words, 3], [input_words, 3]])
        );
        if varying_bounds {
            assert_eq!(report[0]["literal_inputs"], serde_json::json!([[5], [9]]));
        }
        if register_inputs {
            assert_eq!(report[0]["register_input_words"], input_words);
        }
        if register_outputs {
            assert_eq!(report[0]["register_output_words"], 3);
        }
        if register_inputs || register_outputs {
            assert_eq!(report[0]["returns_error_origin"], true);
        }
    }
    ctx.verify(module.isa()).unwrap();
    module.define_function(id, &mut ctx).unwrap();
    id
}

fn check_controller(
    varying_bounds: bool,
    bounds: [u64; 2],
    register_inputs: bool,
    register_outputs: bool,
) {
    let builder = jit_builder();
    let mut module = JITModule::new(builder);
    let original = define_controller(&mut module, false, varying_bounds, false, false);
    let outlined = define_controller(
        &mut module,
        true,
        varying_bounds,
        register_inputs,
        register_outputs,
    );
    module.finalize_definitions().unwrap();
    let bind = |id| unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    let old = bind(original);
    let new = bind(outlined);
    for (choice, origin, bound) in [(0, 34, bounds[0]), (1, 94, bounds[1])] {
        for count in 0..17 {
            let terminal = count.min(bound);
            let total = (terminal..=count).sum::<u64>();
            let hold = count + 21;
            let expected = ResultPair(total - terminal + hold, total * 2 + hold);
            assert_eq!(old(choice, count, 0), expected);
            assert_eq!(new(choice, count, 0), expected);
            assert_eq!(old(choice, count, 1), ResultPair(17, origin));
            assert_eq!(new(choice, count, 1), ResultPair(17, origin));
        }
    }
}

#[test]
fn shared_controller_preserves_loop_value_and_direct_uses_after_exit() {
    check_controller(false, [1, 1], false, false);
}

#[test]
fn shared_controller_keeps_each_callers_bound_and_error_origin() {
    check_controller(true, [5, 9], false, false);
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
#[test]
fn shared_controller_passes_normalized_inputs_in_registers() {
    check_controller(true, [5, 9], true, false);
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
#[test]
fn shared_controller_returns_normalized_outputs_in_registers() {
    for register_inputs in [false, true] {
        check_controller(true, [5, 9], register_inputs, true);
    }
}

#[test]
fn shared_continuation_preserves_nested_results_and_error_origins() {
    for direct_inputs in [0, 16] {
        check_nested_continuation(direct_inputs, false, 4);
    }
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
#[test]
fn nested_continuation_returns_full_error_origin_in_result_register() {
    for direct_inputs in [0, 16] {
        for offset in [4, -1, i64::from(u32::MAX) + 2, i64::MAX - 29] {
            check_nested_continuation(direct_inputs, true, offset);
        }
    }
}

fn check_nested_continuation(direct_inputs: usize, register_outputs: bool, offset: i64) {
    let mut module = JITModule::new(jit_builder());
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    let starts = [b.create_block(), b.create_block()];
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [choose, reject, address] = *b.block_params(entry) else {
        unreachable!()
    };
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().brif(choose, starts[1], &[], starts[0], &[]);
    let mut inner = Vec::new();
    let mut outer = Vec::new();
    for (start, base) in starts.into_iter().zip([30, 90]) {
        let body = b.create_block();
        let error = b.create_block();
        let success = b.create_block();
        let after = b.create_block();
        let finish = b.create_block();
        b.append_block_param(after, types::I64);
        b.append_block_param(finish, types::I64);
        inner.push(Candidate {
            resume: None,
            start: body,
            exit: after,
            header: base as usize,
            origin_base: Some(base),
            span: 0,
        });
        outer.push(Candidate {
            start,
            exit: finish,
            // The same header is allowed in a different stage.
            ..*inner.last().unwrap()
        });
        b.switch_to_block(start);
        let original = b.ins().load(types::I64, MemFlags::new(), address, 0);
        let marked = b.ins().iadd_imm(original, 1);
        b.ins().store(MemFlags::new(), marked, address, 0);
        b.ins().jump(body, &[]);
        b.switch_to_block(body);
        b.ins().brif(reject, error, &[], success, &[]);
        b.switch_to_block(error);
        b.set_cold_block(error);
        let status = b.ins().iconst(types::I64, 17);
        let origin = b
            .ins()
            .iconst(types::I64, i64::from(base).wrapping_add(offset));
        b.ins().return_(&[status, origin]);
        b.switch_to_block(success);
        let value = b.ins().iadd_imm(marked, 11);
        b.ins().jump(after, &[value.into()]);
        b.switch_to_block(after);
        let value = b.block_params(after)[0];
        let result = b.ins().iadd_imm(value, 2);
        b.ins().jump(finish, &[result.into()]);
        b.switch_to_block(finish);
        let result = b.block_params(finish)[0];
        b.ins().return_(&[zero, result]);
    }
    b.seal_all_blocks();
    b.finalize();
    let leaves = share_controllers(
        &mut module,
        &mut ctx.func,
        vec![inner],
        CaptureBindings {
            return_error_origin: true,
            ..CaptureBindings::default()
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(leaves[0]["headers"], serde_json::json!([30, 90]));
    let callers = share_families(
        &mut module,
        &mut ctx.func,
        vec![outer],
        RegionKind::Continuation { direct_inputs },
        false,
        CaptureBindings {
            register_outputs,
            ..CaptureBindings::default()
        },
    )
    .unwrap()
    .expect("shared caller with a nested checked call");
    assert_eq!(callers[0]["headers"], serde_json::json!([30, 90]));
    assert_eq!(
        callers[0]["input_words"],
        (if direct_inputs == 0 { 2 } else { 5 }) - usize::from(register_outputs)
    );
    if register_outputs {
        assert_eq!(callers[0]["register_output_words"], 1);
        assert!(callers[0]["returns_error_origin"].is_null());
    }
    let id = module
        .declare_function(
            "nested_checked_continuations",
            Linkage::Export,
            &ctx.func.signature,
        )
        .unwrap();
    ctx.verify(module.isa()).unwrap();
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    let call: extern "C" fn(u64, u64, &mut u64) -> ResultPair =
        unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    for (choice, base) in [(0, 30i64), (1, 90i64)] {
        let origin = base.wrapping_add(offset) as u64;
        let mut value = 100;
        assert_eq!(call(choice, 1, &mut value), ResultPair(17, origin));
        assert_eq!(value, 101);
        assert_eq!(call(choice, 0, &mut value), ResultPair(0, 115));
        assert_eq!(value, 102);
    }
}

#[test]
fn direct_refusal_shares_a_bound_used_by_multiple_constants() {
    let mut module = JITModule::new(jit_builder());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let starts = [b.create_block(), b.create_block()];
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [choose, path, input] = *b.block_params(entry) else {
        unreachable!()
    };
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().brif(choose, starts[1], &[], starts[0], &[]);
    let mut candidates = Vec::new();
    for (start, header, bound) in [(starts[0], 30, 5), (starts[1], 90, 9)] {
        let failure = b.create_block();
        let ready = b.create_block();
        let resume = b.create_block();
        let finish = b.create_block();
        candidates.push(Candidate {
            resume: Some(resume),
            start,
            exit: finish,
            header,
            origin_base: None,
            span: 10,
        });
        for block in [resume, finish] {
            b.append_block_param(block, types::I64);
        }
        b.switch_to_block(start);
        let limit = b.ins().iconst(types::I64, bound);
        let valid = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, input, limit);
        b.ins().brif(valid, ready, &[], failure, &[]);
        b.switch_to_block(failure);
        b.set_cold_block(failure);
        let status = b.ins().iconst(types::I64, 73);
        let origin = b.ins().iconst(types::I64, header as i64 + 4);
        b.ins().return_(&[status, origin]);
        b.switch_to_block(ready);
        let step = b.ins().iconst(types::I64, bound);
        let advanced = b.ins().iadd(input, step);
        b.ins()
            .brif(path, resume, &[advanced.into()], finish, &[advanced.into()]);
        b.switch_to_block(resume);
        let partial = b.block_params(resume)[0];
        let continued = b.ins().iadd_imm(partial, 11);
        b.ins().return_(&[zero, continued]);
        b.switch_to_block(finish);
        let result = b.block_params(finish)[0];
        b.ins().return_(&[zero, result]);
    }
    b.seal_all_blocks();
    b.finalize();
    let report = share_families(
        &mut module,
        &mut context.func,
        candidates
            .into_iter()
            .map(|candidate| vec![candidate])
            .collect(),
        RegionKind::Refusal,
        false,
        CaptureBindings {
            shared_literal_columns: true,
            ..CaptureBindings::default()
        },
    )
    .unwrap()
    .expect("one body across the original families");
    assert_eq!(report[0]["headers"], serde_json::json!([30, 90]));
    assert_eq!(report[0]["literal_inputs"], serde_json::json!([[5], [9]]));
    assert_eq!(report[0]["input_words"], 4);
    let id = module
        .declare_function(
            "direct_limited_refusal",
            Linkage::Export,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let call: extern "C" fn(u64, u64, u64) -> ResultPair =
        unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    for (choice, bound, origin) in [(0, 5, 34), (1, 9, 94)] {
        for path in [0, 1] {
            assert_eq!(call(choice, path, bound - 1), ResultPair(73, origin));
            assert_eq!(
                call(choice, path, bound),
                ResultPair(0, 2 * bound + path * 11)
            );
            assert_eq!(
                call(choice, path, 42),
                ResultPair(0, 42 + bound + path * 11)
            );
        }
    }
}

#[test]
fn single_phase_keeps_live_outs_partial_writes_and_a_bypassed_failure() {
    let mut module = JITModule::new(jit_builder());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let id = module
        .declare_function("single_phase", Linkage::Export, &context.func.signature)
        .unwrap();
    let mut builder_context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let entry = b.create_block();
    let start = b.create_block();
    let exit = b.create_block();
    let bypass = b.create_block();
    let failure = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(failure, types::I64);
    b.append_block_param(failure, types::I64);
    b.switch_to_block(entry);
    let [enter, reject, input] = *b.block_params(entry) else {
        unreachable!()
    };
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    b.ins().stack_store(input, slot, 0);
    b.ins().brif(enter, start, &[], bypass, &[]);
    b.switch_to_block(bypass);
    let status = b.ins().iconst(types::I64, 7);
    let origin = b.ins().iconst(types::I64, 11);
    b.ins().jump(failure, &[status.into(), origin.into()]);
    b.switch_to_block(start);
    let old = b.ins().stack_load(types::I64, slot, 0);
    let advanced = b.ins().iadd_imm(old, 1);
    b.ins().stack_store(advanced, slot, 0);
    let live_out = b.ins().iadd_imm(input, 23);
    let status = b.ins().iconst(types::I64, 19);
    let origin = b.ins().iconst(types::I64, 73);
    b.ins()
        .brif(reject, failure, &[status.into(), origin.into()], exit, &[]);
    b.switch_to_block(exit);
    let written = b.ins().stack_load(types::I64, slot, 0);
    let value = b.ins().iadd(written, live_out);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[zero, value]);
    b.switch_to_block(failure);
    let [status, origin] = *b.block_params(failure) else {
        unreachable!()
    };
    let written = b.ins().stack_load(types::I64, slot, 0);
    let observed = b.ins().iadd(origin, written);
    b.ins().return_(&[status, observed]);
    b.seal_all_blocks();
    b.finalize();

    let report = share_phase_regions(
        &mut module,
        &mut context.func,
        vec![vec![Candidate {
            start,
            exit,
            resume: Some(failure),
            header: 60,
            origin_base: Some(0),
            span: 0,
        }]],
        CaptureBindings {
            align_outputs: true,
            pad_outputs: true,
            ..CaptureBindings::default()
        },
    )
    .unwrap()
    .expect("one checked phase is enough to bound compilation");
    assert_eq!(report[0]["headers"], serde_json::json!([60]));
    context.verify(module.isa()).unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let run = unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    for input in 0..128 {
        assert_eq!(run(1, 0, input), ResultPair(0, input * 2 + 24));
        assert_eq!(run(1, 1, input), ResultPair(19, input + 74));
        assert_eq!(run(0, 0, input), ResultPair(7, input + 11));
        assert_eq!(run(0, 1, input), ResultPair(7, input + 11));
    }
}

#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
#[test]
fn captured_heap_setup_uses_each_phase_frame_and_refuses_a_non_entry_load() {
    use super::super::release_stubs::ReleaseStubs;

    extern "C" fn release(heap: u64, owner: u64) -> u64 {
        heap.rotate_left(11).wrapping_add(owner.wrapping_mul(7))
    }

    let mut jit = jit_builder();
    jit.symbol("mech_compact_release_owner", release as *const u8);
    let mut module = JITModule::new(jit);
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 2];
    signature.returns = vec![AbiParam::new(types::I64)];
    let provider = module
        .declare_function("mech_compact_release_owner", Linkage::Import, &signature)
        .unwrap();
    let mut functions = Vec::new();
    let mut frames = Vec::new();
    for (phase, padding, non_entry) in [
        (60, 64, false),
        (80, 256, false),
        (90, 128, false),
        (100, 64, true),
    ] {
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
        let provider = module.declare_func_in_func(provider, &mut context.func);
        let mut builder_context = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [capture, input, salt] = *b.block_params(entry) else {
            unreachable!()
        };
        let slot =
            b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, padding, 3));
        b.ins().stack_store(input, slot, 0);
        if non_entry {
            let body = b.create_block();
            b.ins().jump(body, &[]);
            b.switch_to_block(body);
        }
        let heap = if padding == 256 || padding == 128 {
            let offset = if padding == 128 { 8 } else { 0 };
            let pair = b
                .ins()
                .load(types::I128, MemFlags::new().with_notrap(), capture, offset);
            let (low, high) = b.ins().isplit(pair);
            if offset == 8 { low } else { high }
        } else {
            b.ins().load(types::I64, MemFlags::trusted(), capture, 8)
        };
        let owner = b.ins().iadd(input, salt);
        let first = b.ins().call(provider, &[heap, owner]);
        let first = b.inst_results(first)[0];
        let owner = b.ins().bxor(input, salt);
        let second = b.ins().call(provider, &[heap, owner]);
        let second = b.inst_results(second)[0];
        let sum = b.ins().iadd(first, second);
        let old = b.ins().stack_load(types::I64, slot, 0);
        let carried = b.ins().iadd(heap, old);
        b.ins().return_(&[sum, carried]);
        b.seal_all_blocks();
        b.finalize();
        let stubs = ReleaseStubs::prepare_captured(&mut module, &mut context.func, phase).unwrap();
        assert_eq!(stubs.is_some(), !non_entry);
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(
                &format!("captured_heap_{phase}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        let frame = context
            .compiled_code()
            .unwrap()
            .buffer
            .frame_layout()
            .unwrap();
        frames.push(frame.frame_to_fp_offset);
        if let Some(stubs) = stubs {
            stubs.define(&mut module, frame).unwrap();
        }
        functions.push(id);
    }
    assert_ne!(frames[0], frames[1]);
    module.finalize_definitions().unwrap();
    for id in functions {
        let run = unsafe {
            std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
                module.get_finalized_function(id),
            )
        };
        for heap in [0, 1, 0x1234_5678_90ab_cdef, u64::MAX] {
            let capture = [0xdead_beef, heap, 0xface_feed];
            for input in 0_u64..32 {
                let salt = input.wrapping_mul(97);
                let expected = release(heap, input.wrapping_add(salt))
                    .wrapping_add(release(heap, input ^ salt));
                assert_eq!(
                    run(capture.as_ptr() as u64, input, salt),
                    ResultPair(expected, heap.wrapping_add(input))
                );
            }
            assert_eq!(capture, [0xdead_beef, heap, 0xface_feed]);
        }
    }
}

#[test]
fn phase_regions_keep_the_value_used_after_success_and_the_shared_failure() {
    let mut module = JITModule::new(jit_builder());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let id = module
        .declare_function("phase_uses", Linkage::Export, &context.func.signature)
        .unwrap();
    let mut builder_context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let entry = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    let failure = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(failure, types::I64);
    b.append_block_param(failure, types::I64);

    b.switch_to_block(entry);
    let [choice, reject, input] = *b.block_params(entry) else {
        unreachable!()
    };
    b.ins().brif(choice, right, &[], left, &[]);
    b.switch_to_block(failure);
    let [status, origin] = *b.block_params(failure) else {
        unreachable!()
    };
    b.ins().return_(&[status, origin]);
    let mut sites = Vec::new();
    for (start, header) in [(left, 60), (right, 80)] {
        let exit = b.create_block();
        b.switch_to_block(start);
        let value = b.ins().iadd_imm(input, 23);
        let status = b.ins().iconst(types::I64, 19);
        let origin = b.ins().iconst(types::I64, header as i64);
        b.ins()
            .brif(reject, failure, &[status.into(), origin.into()], exit, &[]);
        b.switch_to_block(exit);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[zero, value]);
        sites.push(Candidate {
            start,
            exit,
            resume: Some(failure),
            header,
            origin_base: Some(0),
            span: 0,
        });
    }
    b.seal_all_blocks();
    b.finalize();
    let report = share_phase_regions(
        &mut module,
        &mut context.func,
        vec![sites],
        CaptureBindings {
            literals: true,
            ..CaptureBindings::default()
        },
    )
    .unwrap()
    .expect("both phase regions share a body");
    assert_eq!(report[0]["headers"], serde_json::json!([60, 80]));
    context.verify(module.isa()).unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let run = unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    for (choice, origin) in [(0, 60), (1, 80)] {
        assert_eq!(run(choice, 0, 100), ResultPair(0, 123));
        assert_eq!(run(choice, 1, 100), ResultPair(19, origin));
    }
}

#[test]
fn shared_body_takes_distinct_operand_roles_even_when_a_caller_repeats_a_value() {
    let mut module = JITModule::new(jit_builder());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let id = module
        .declare_function("operand_roles", Linkage::Export, &context.func.signature)
        .unwrap();
    let mut builder_context = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder_context);
    let entry = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [choice, a, other] = *b.block_params(entry) else {
        unreachable!()
    };
    b.ins().brif(choice, right, &[], left, &[]);
    let mut sites = Vec::new();
    for (start, header, second) in [(left, 60, a), (right, 80, other)] {
        let exit = b.create_block();
        b.switch_to_block(start);
        let result = b.ins().isub(a, second);
        b.ins().jump(exit, &[]);
        b.switch_to_block(exit);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[zero, result]);
        sites.push(Candidate {
            start,
            exit,
            resume: None,
            header,
            origin_base: Some(0),
            span: 0,
        });
    }
    b.seal_all_blocks();
    b.finalize();
    let report = share_controllers(
        &mut module,
        &mut context.func,
        vec![sites],
        CaptureBindings {
            input_roles: true,
            ..CaptureBindings::default()
        },
    )
    .unwrap()
    .expect("both operand roles use a shared body");
    assert_eq!(report[0]["aligned_input_words"], serde_json::json!([2, 2]));
    context.verify(module.isa()).unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let run = unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    assert_eq!(run(0, 9, 4), ResultPair(0, 0));
    assert_eq!(run(1, 9, 4), ResultPair(0, 5));
}

#[test]
fn shared_loop_writes_the_union_of_current_outputs() {
    verify_loop_output_packets(CaptureBindings {
        align_outputs: true,
        ..CaptureBindings::default()
    });
}

#[test]
fn shared_loop_can_pad_a_single_output() {
    verify_loop_output_packets(CaptureBindings {
        pad_outputs: true,
        ..CaptureBindings::default()
    });
}

fn verify_loop_output_packets(bindings: CaptureBindings) {
    let mut module = JITModule::new(jit_builder());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let id = module
        .declare_function("output_roles", Linkage::Export, &context.func.signature)
        .unwrap();
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
    let entry = b.create_block();
    let starts = [b.create_block(), b.create_block()];
    b.append_block_params_for_function_params(entry);
    for start in starts {
        for _ in 0..3 {
            b.append_block_param(start, types::I64);
        }
    }
    b.switch_to_block(entry);
    let [choice, trips, seed] = *b.block_params(entry) else {
        unreachable!()
    };
    let zero = b.ins().iconst(types::I64, 0);
    let initial = [trips.into(), seed.into(), zero.into()];
    b.ins()
        .brif(choice, starts[1], &initial, starts[0], &initial);
    let mut candidates = Vec::new();
    for (index, start) in starts.into_iter().enumerate() {
        let step = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, types::I64);
        b.switch_to_block(start);
        let [trips, a, b_value] = *b.block_params(start) else {
            unreachable!()
        };
        let sum = b.ins().iadd(a, b_value);
        b.ins().brif(trips, step, &[], done, &[sum.into()]);
        b.switch_to_block(step);
        let remaining = b.ins().iadd_imm(trips, -1);
        let first = b.ins().iadd_imm(a, 3);
        let second = b.ins().iadd_imm(b_value, 7);
        b.ins()
            .jump(start, &[remaining.into(), first.into(), second.into()]);
        b.switch_to_block(done);
        let sum = b.block_params(done)[0];
        let result = b.ins().isub(sum, b_value);
        let result = if index == 0 {
            result
        } else {
            b.ins().iadd(result, a)
        };
        b.ins().return_(&[zero, result]);
        candidates.push(Candidate {
            start,
            exit: done,
            resume: None,
            header: 90 + index,
            origin_base: None,
            span: 0,
        });
    }
    b.seal_all_blocks();
    b.finalize();
    assert!(
        share_controllers(
            &mut module,
            &mut context.func.clone(),
            vec![candidates.clone()],
            CaptureBindings::default()
        )
        .unwrap()
        .is_none()
    );
    let report = share_controllers(&mut module, &mut context.func, vec![candidates], bindings)
        .unwrap()
        .expect("the checked bodies publish the same union of three words");
    assert_eq!(report[0]["aligned_outputs"], true);
    assert_eq!(
        report[0]["packet_words"],
        serde_json::json!([[3, 3], [3, 3]])
    );
    context.verify(module.isa()).unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let run = unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    for trips in [0, 1, 5] {
        let first = 19 + 3 * trips;
        assert_eq!(run(0, trips, 19), ResultPair(0, first));
        assert_eq!(run(1, trips, 19), ResultPair(0, 2 * first));
    }
}
