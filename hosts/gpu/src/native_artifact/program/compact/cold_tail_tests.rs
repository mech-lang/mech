#![cfg(all(target_arch = "aarch64", target_os = "macos"))]

use cranelift_codegen::Context;
use cranelift_codegen::control::ControlPlane;
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, SourceLoc, StackSlotData, StackSlotKind, TrapCode, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};

extern "C" fn observe(value: u64, output: *mut u64) -> u64 {
    // SAFETY: callers provide a live five-word output with the middle three writable.
    unsafe { output.add(2).write(value.rotate_left(23)) };
    value.wrapping_mul(97)
}

fn module(opt: &str, imported: bool) -> (JITModule, FuncId) {
    let mut builder = JITBuilder::with_flags(
        &[
            ("opt_level", opt),
            ("enable_verifier", "true"),
            ("regalloc_checker", "true"),
            ("preserve_frame_pointers", "true"),
            ("unwind_info", "true"),
        ],
        default_libcall_names(),
    )
    .unwrap();
    builder.symbol("cold_tail_observe", observe as *const u8);
    let mut module = JITModule::new(builder);
    let mut sig = module.make_signature();
    sig.params = vec![AbiParam::new(types::I64); 2];
    sig.returns = vec![AbiParam::new(types::I64)];
    let helper = module
        .declare_function(
            "cold_tail_observe",
            if imported {
                Linkage::Import
            } else {
                Linkage::Local
            },
            &sig,
        )
        .unwrap();
    if !imported {
        let mut context = module.make_context();
        context.func.signature = sig;
        let mut fb = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [value, output] = b.block_params(entry)[..] else {
            unreachable!()
        };
        let rotated = b.ins().rotl_imm(value, 23);
        b.ins().store(MemFlags::trusted(), rotated, output, 16);
        let result = b.ins().imul_imm(value, 97);
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
        module.define_function(helper, &mut context).unwrap();
    }
    (module, helper)
}

fn function(module: &mut JITModule, helper: FuncId, cold: bool, locations: bool) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    context.func.share_cold_epilogues = true;
    let helper = module.declare_func_in_func(helper, &mut context.func);
    let slot = context.func.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        96,
        3,
    ));
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
    let entry = b.create_block();
    let success = b.create_block();
    let dispatch = [b.create_block(), b.create_block(), b.create_block()];
    let errors = [
        b.create_block(),
        b.create_block(),
        b.create_block(),
        b.create_block(),
    ];
    let join = b.create_block();
    let trap = b.create_block();
    let done = b.create_block();
    if cold {
        for block in dispatch.into_iter().chain(errors).chain([join, trap, done]) {
            b.set_cold_block(block);
        }
    }
    b.append_block_params_for_function_params(entry);
    b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    if locations {
        b.set_srcloc(SourceLoc::new(17));
    }
    let [mode, input, output] = b.block_params(entry)[..] else {
        unreachable!()
    };
    b.ins().stack_store(input, slot, 0);
    b.ins().brif(mode, dispatch[0], &[], success, &[]);
    b.switch_to_block(success);
    b.ins().return_(&[input]);
    for (index, block) in dispatch.into_iter().enumerate() {
        b.switch_to_block(block);
        let test = b.ins().icmp_imm(
            cranelift_codegen::ir::condcodes::IntCC::Equal,
            mode,
            index as i64 + 1,
        );
        b.ins().brif(
            test,
            errors[index],
            &[],
            dispatch.get(index + 1).copied().unwrap_or(errors[3]),
            &[],
        );
    }
    for (index, block) in errors.into_iter().enumerate() {
        b.switch_to_block(block);
        let origin = b.ins().iconst(types::I64, 100 + index as i64);
        b.ins().store(MemFlags::trusted(), origin, output, 0);
        let value = b.ins().iadd_imm(input, 0x345);
        let value = b.ins().imul_imm(value, 1237);
        let value = b.ins().bxor_imm(value, 0x345678);
        let value = b.ins().rotl_imm(value, 17);
        b.ins().store(MemFlags::trusted(), value, output, 8);
        b.ins().jump(join, &[value.into()]);
    }
    b.switch_to_block(join);
    let value = b.block_params(join)[0];
    let call = b.ins().call(helper, &[value, output]);
    let result = b.inst_results(call)[0];
    let original = b.ins().stack_load(types::I64, slot, 0);
    let result = b.ins().bxor(result, original);
    let bad = b
        .ins()
        .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, mode, 99);
    b.ins().brif(bad, trap, &[], done, &[]);
    b.switch_to_block(trap);
    b.ins().trap(TrapCode::user(7).unwrap());
    b.switch_to_block(done);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    context
}

#[test]
fn cold_tails_preserve_native_calls_stack_values_outputs_and_metadata() {
    let mut total_saved = 0;
    for opt in ["none", "speed"] {
        let (mut module, helper) = module(opt, false);
        let mut control = function(&mut module, helper, true, false);
        control
            .compile(module.isa(), &mut ControlPlane::default())
            .unwrap();
        let mut candidate = function(&mut module, helper, true, false);
        candidate.func.aarch64_share_cold_tails = true;
        let id = module
            .declare_function(
                "cold_tail_checked",
                Linkage::Local,
                &candidate.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut candidate).unwrap();
        let before = &control.compiled_code().unwrap().buffer;
        let after = &candidate.compiled_code().unwrap().buffer;
        assert!(after.data().len() <= before.data().len());
        total_saved += before.data().len() - after.data().len();
        assert_eq!(before.unwind_info, after.unwind_info);
        assert_eq!(before.frame_layout(), after.frame_layout());
        assert_eq!(before.relocs().len(), after.relocs().len());
        for (old, new) in before.relocs().iter().zip(after.relocs()) {
            assert_eq!(
                (old.kind, &old.target, old.addend),
                (new.kind, &new.target, new.addend)
            );
            assert_eq!(
                &before.data()[old.offset as usize..old.offset as usize + 4],
                &after.data()[new.offset as usize..new.offset as usize + 4]
            );
        }
        assert_eq!(before.traps().len(), after.traps().len());
        for (old, new) in before.traps().iter().zip(after.traps()) {
            assert_eq!(old.code, new.code);
            assert_eq!(
                &before.data()[old.offset as usize..old.offset as usize + 4],
                &after.data()[new.offset as usize..new.offset as usize + 4]
            );
        }
        let old_calls = before.call_sites().collect::<Vec<_>>();
        let new_calls = after.call_sites().collect::<Vec<_>>();
        assert!(!old_calls.is_empty());
        assert_eq!(old_calls.len(), new_calls.len());
        for (old, new) in old_calls.iter().zip(&new_calls) {
            assert_eq!(old.frame_offset, new.frame_offset);
            assert_eq!(
                &before.data()[old.ret_addr as usize - 4..old.ret_addr as usize],
                &after.data()[new.ret_addr as usize - 4..new.ret_addr as usize]
            );
        }
        module.finalize_definitions().unwrap();
        type Call = unsafe extern "C" fn(u64, u64, *mut u64) -> u64;
        // SAFETY: this live module owns a function with exactly this C signature.
        let call: Call = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        for seed in 0..1024u64 {
            let input = seed
                .wrapping_mul(0x9876543210fedcbb)
                .rotate_left(seed as u32 & 63);
            for mode in 0..5 {
                let mut out = [0xa5a5_a5a5_a5a5_a5a5; 5];
                let prior = out;
                // SAFETY: the callee receives the middle three initialized writable words.
                let result = unsafe { call(mode, input, out.as_mut_ptr().add(1)) };
                assert_eq!(out[0], prior[0]);
                assert_eq!(out[4], prior[4]);
                if mode == 0 {
                    assert_eq!(out, prior);
                    assert_eq!(result, input);
                } else {
                    let value =
                        (input.wrapping_add(0x345).wrapping_mul(1237) ^ 0x345678).rotate_left(17);
                    assert_eq!(out[1], 99 + mode);
                    assert_eq!(out[2], value);
                    assert_eq!(out[3], value.rotate_left(23));
                    assert_eq!(result, value.wrapping_mul(97) ^ input);
                }
            }
        }
    }
    assert!(
        total_saved > 0,
        "the exercised function must actually share a tail"
    );
}

#[test]
fn unsupported_debug_metadata_and_embedded_literals_are_left_unchanged() {
    for (cold, locations, disasm, imported) in [
        (false, false, false, false),
        (true, true, false, false),
        (true, false, true, false),
        (true, false, false, true),
    ] {
        let (mut module, helper) = module("none", imported);
        let mut control = function(&mut module, helper, cold, locations);
        let mut candidate = function(&mut module, helper, cold, locations);
        candidate.func.aarch64_share_cold_tails = true;
        control.set_disasm(disasm);
        candidate.set_disasm(disasm);
        control
            .compile(module.isa(), &mut ControlPlane::default())
            .unwrap();
        candidate
            .compile(module.isa(), &mut ControlPlane::default())
            .unwrap();
        assert_eq!(
            control.compiled_code().unwrap(),
            candidate.compiled_code().unwrap()
        );
    }
}

#[test]
fn preserved_register_preference_keeps_live_values_across_calls_and_outputs() {
    for opt in ["none", "speed"] {
        for prefer in [false, true] {
            let (mut module, helper) = module(opt, true);
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 3];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            context.func.aarch64_prefer_preserved_regs = prefer;
            let id = module
                .declare_function("preserved_calls", Linkage::Local, &context.func.signature)
                .unwrap();
            let helper = module.declare_func_in_func(helper, &mut context.func);
            let mut frontend = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let [mode, input, output] = b.block_params(entry)[..] else {
                unreachable!()
            };
            b.ins().store(MemFlags::new(), mode, output, 0);
            let mut values = Vec::new();
            for ordinal in 0..18 {
                let value = b.ins().iadd_imm(input, ordinal * 193);
                let call = b.ins().call(helper, &[value, output]);
                values.push(b.inst_results(call)[0]);
            }
            let mut checksum = mode;
            for (ordinal, value) in values.into_iter().enumerate() {
                let value = b.ins().rotl_imm(value, ordinal as i64);
                checksum = b.ins().bxor(checksum, value);
            }
            b.ins().store(MemFlags::new(), checksum, output, 8);
            let result = b.ins().bxor(checksum, input);
            b.ins().return_(&[result]);
            b.seal_all_blocks();
            b.finalize();
            context.verify(module.isa()).unwrap();
            module.define_function(id, &mut context).unwrap();
            module.finalize_definitions().unwrap();
            type Call = unsafe extern "C" fn(u64, u64, *mut u64) -> u64;
            // SAFETY: the retained module owns this exact three-word entry;
            // calls lend only the middle three initialized writable words.
            let call: Call = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
            for seed in 0..128u64 {
                let input = seed.wrapping_mul(0x9876_5432_10fe_dcbb).rotate_left(17);
                let mode = seed.rotate_left(29);
                let checksum = (0..18u64).fold(mode, |sum, ordinal| {
                    sum ^ input
                        .wrapping_add(ordinal * 193)
                        .wrapping_mul(97)
                        .rotate_left(ordinal as u32)
                });
                let mut output = [0xa5a5_a5a5_a5a5_a5a5; 5];
                let sentinel = output[0];
                // SAFETY: output[1..4] is exclusive, initialized and live.
                let result = unsafe { call(mode, input, output.as_mut_ptr().add(1)) };
                assert_eq!(
                    output,
                    [
                        sentinel,
                        mode,
                        checksum,
                        input.wrapping_add(17 * 193).rotate_left(23),
                        sentinel
                    ]
                );
                assert_eq!(
                    result,
                    checksum ^ input,
                    "{opt}, prefer={prefer}, seed={seed}"
                );
            }
            context.clear();
            assert!(!context.func.aarch64_prefer_preserved_regs);
        }
    }
}
