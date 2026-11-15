use super::{
    Candidate, RegionKind, controller_candidates, share, share_controllers, share_families,
    share_phase_regions,
};
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use std::collections::BTreeMap;

fn define(module: &mut JITModule, enabled: bool, extra_inputs: usize) -> FuncId {
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
        let origin = b.ins().iconst(types::I64, (header + 4) as i64);
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
                    false
                )
                .unwrap()
                .is_none()
            );
            share_phase_regions(module, &mut ctx.func, vec![parts], false)
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
    let builder = JITBuilder::new(default_libcall_names()).unwrap();
    let mut module = JITModule::new(builder);
    let original = define(&mut module, false, 0);
    let outlined = define(&mut module, true, 0);
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
    let builder = JITBuilder::new(default_libcall_names()).unwrap();
    let mut module = JITModule::new(builder);
    let original = define(&mut module, false, 44);
    let outlined = define(&mut module, true, 44);
    module.finalize_definitions().unwrap();
    let bind = |id| unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    let old = bind(original);
    let new = bind(outlined);
    let additional = 5 * 44 + 44 * 45 / 2;
    let advance = 11 + 5 + 2 * additional;
    for (choice, origin) in [(0, 34), (1, 94)] {
        for (path, expected) in [
            (0, ResultPair(0, advance)),
            (1, ResultPair(0, 2 * advance)),
            (2, ResultPair(17, origin)),
        ] {
            assert_eq!(old(choice, path, 5), expected);
            assert_eq!(new(choice, path, 5), expected);
        }
    }
}

fn define_controller(module: &mut JITModule, enabled: bool, varying_bounds: bool) -> FuncId {
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
        let report = share_controllers(module, &mut ctx.func, vec![candidates], varying_bounds)
            .unwrap()
            .expect("both controllers selected");
        assert_eq!(report[0]["headers"], serde_json::json!([130, 190]));
        let input_words = if varying_bounds { 4 } else { 3 };
        assert_eq!(
            report[0]["packet_words"],
            serde_json::json!([[input_words, 3], [input_words, 3]])
        );
        if varying_bounds {
            assert_eq!(report[0]["literal_inputs"], serde_json::json!([[5], [9]]));
        }
    }
    ctx.verify(module.isa()).unwrap();
    module.define_function(id, &mut ctx).unwrap();
    id
}

fn check_controller(varying_bounds: bool, bounds: [u64; 2]) {
    let builder = JITBuilder::new(default_libcall_names()).unwrap();
    let mut module = JITModule::new(builder);
    let original = define_controller(&mut module, false, varying_bounds);
    let outlined = define_controller(&mut module, true, varying_bounds);
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
    check_controller(false, [1, 1]);
}

#[test]
fn shared_controller_keeps_each_callers_bound_and_error_origin() {
    check_controller(true, [5, 9]);
}

#[test]
fn phase_regions_keep_the_value_used_after_success_and_the_shared_failure() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
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
    let report = share_phase_regions(&mut module, &mut context.func, vec![sites], true)
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
