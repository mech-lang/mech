use crate::native_artifact::program::codegen::cfg_cleanup::*;
use cranelift_codegen::ir::{AbiParam, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
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
