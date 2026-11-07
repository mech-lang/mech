use crate::native_artifact::program::codegen::cfg_cleanup::*;
use cranelift_codegen::ir::{AbiParam, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

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
fn cfg_cleanup_executes_only_selected_effects_and_keeps_dynamic_branches_and_block_arguments() {
    for (enabled, literal) in [false, true]
        .into_iter()
        .flat_map(|enabled| [0i64, -128].map(|literal| (enabled, literal)))
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
            let constant = b.ins().iconst(types::I8, literal);
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
