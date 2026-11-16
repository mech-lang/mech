use super::test_support::*;
use super::*;
use crate::native_artifact::codegen::options::{
    CompilationControls, OptimizationOptions, OptimizationPass,
};
use crate::native_artifact::program::codegen::cfg_cleanup::thread_loop_edges;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, default_libcall_names};

#[test]
fn threaded_edges_transport_current_integer_and_float_bits_without_cloning() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for (i, variant) in ["normal", "duplicate"].into_iter().enumerate() {
        let pair = pair(&mut module, variant, i);
        assert!(pair.2.threaded_edges > 0, "{variant}");
        assert!(pair.2.repair_parameters > 0);
        for seed in [
            0,
            1,
            2,
            u32::MAX as u64,
            u64::MAX,
            1 << 63,
            0x7ff8000000000001,
            0x7ff0000000000001,
            0xfff0000000000000,
            0x8000000000000001,
        ] {
            for limit in [0, 1, 2, 3, 5] {
                for flag in [0, 1, 2, u64::MAX] {
                    check(&pair, seed, limit, flag, variant);
                }
            }
        }
    }
}

#[test]
fn threaded_edges_preserve_current_assertion_overflow_order_and_fresh_visits() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for (i, variant) in ["normal", "overflow-first"].into_iter().enumerate() {
        let pair = pair(&mut module, variant, i);
        assert!(pair.2.threaded_edges > 0);
        for seed in [0, u64::MAX - 1, u64::MAX] {
            for limit in [0, 1, 2, 3] {
                for flag in [1, 0, 1, u64::MAX, 0, 1] {
                    check(&pair, seed, limit, flag, variant);
                }
            }
        }
    }
}

#[test]
fn threaded_edges_refuse_loads_observable_header_effects_and_shared_live_merges() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for (i, variant) in ["effect", "load", "merge"].into_iter().enumerate() {
        let pair = pair(&mut module, variant, i);
        assert_eq!(pair.2.threaded_edges, 0, "{variant}");
        for seed in [0, 17, u32::MAX as u64, 1 << 63, u64::MAX] {
            for limit in [0, 1, 2, 3] {
                for flag in [0, 1, 2, u64::MAX] {
                    check(&pair, seed, limit, flag, variant);
                }
            }
        }
    }
}

#[test]
fn threaded_edges_keep_pure_cycles_without_compile_time_execution() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let header = b.create_block();
        let back = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(header, types::I64);
        b.switch_to_block(entry);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().jump(header, &[one.into()]);
        b.switch_to_block(header);
        let arg = b.block_params(header)[0];
        b.ins().brif(arg, back, &[], back, &[]);
        b.switch_to_block(back);
        b.ins().jump(header, &[one.into()]);
        b.seal_all_blocks();
        b.finalize();
    }
    c.verify(module.isa()).unwrap();
    c.compute_cfg();
    c.compute_domtree();
    let before = c.func.display().to_string();
    assert_eq!(optimize(&mut c.func, &c.cfg, &c.domtree).threaded_edges, 0);
    assert_eq!(c.func.display().to_string(), before);
}

#[test]
fn threaded_edges_respect_existing_pass_control_and_are_idempotent_on_terminal_loops() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = fixture(&module, "duplicate");
    let before = c.func.display().to_string();
    let off = CompilationControls::new(OptimizationOptions::without(OptimizationPass::CfgCleanup));
    assert_eq!(
        thread_loop_edges(&off, &mut c, module.isa())
            .unwrap()
            .threaded_edges,
        0
    );
    assert_eq!(c.func.display().to_string(), before);
    let on = CompilationControls::new(OptimizationOptions::default());
    assert!(
        thread_loop_edges(&on, &mut c, module.isa())
            .unwrap()
            .threaded_edges
            > 0
    );
    c.verify(module.isa()).unwrap();
    let after = c.func.display().to_string();
    assert_eq!(
        thread_loop_edges(&on, &mut c, module.isa())
            .unwrap()
            .threaded_edges,
        0
    );
    assert_eq!(c.func.display().to_string(), after);
}

#[test]
fn threaded_edges_do_not_speculate_unsupported_header_computations() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = fixture(&module, "opaque");
    c.compute_cfg();
    c.compute_domtree();
    let before = c.func.display().to_string();
    assert_eq!(optimize(&mut c.func, &c.cfg, &c.domtree).threaded_edges, 0);
    assert_eq!(c.func.display().to_string(), before);
    c.verify(module.isa()).unwrap();
}

#[test]
fn threaded_edges_snapshot_source_words_before_binding_the_next_loop_visit() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 4];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let header = b.create_block();
        let source = b.create_block();
        let bridge = b.create_block();
        let good = b.create_block();
        let bad = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(header, types::I64);
        b.append_block_param(header, types::I64);
        b.append_block_param(bridge, types::I64);
        b.switch_to_block(entry);
        let seed = b.block_params(entry)[0];
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().jump(header, &[zero.into(), seed.into()]);
        b.switch_to_block(header);
        let h = b.block_params(header).to_vec();
        let step = b.ins().icmp_imm(IntCC::Equal, h[0], 0);
        b.ins().brif(step, source, &[], bridge, &[h[1].into()]);
        b.switch_to_block(source);
        let old = b.ins().icmp_imm(IntCC::Equal, h[0], 0);
        let captured = b.ins().uextend(types::I64, old);
        b.ins()
            .brif(old, header, &[one.into(), captured.into()], bad, &[]);
        b.switch_to_block(bridge);
        let current = b.block_params(bridge)[0];
        b.ins().brif(current, good, &[], bad, &[]);
        b.switch_to_block(good);
        let yes = b.ins().iconst(types::I64, 123);
        b.ins().return_(&[yes]);
        b.switch_to_block(bad);
        let no = b.ins().iconst(types::I64, 456);
        b.ins().return_(&[no]);
        b.seal_all_blocks();
        b.finalize();
    }
    c.verify(module.isa()).unwrap();
    let mut old = module.make_context();
    old.func = c.func.clone();
    let before = define(&mut module, &mut old, "atom_before");
    let on = CompilationControls::new(OptimizationOptions::default());
    assert!(
        thread_loop_edges(&on, &mut c, module.isa())
            .unwrap()
            .threaded_edges
            > 0
    );
    c.verify(module.isa()).unwrap();
    let after = define(&mut module, &mut c, "atom_after");
    for seed in [0, 1, u64::MAX] {
        let mut memory = [0, 0];
        assert_eq!(unsafe { before(seed, 0, 0, memory.as_mut_ptr()) }, 123);
        assert_eq!(unsafe { after(seed, 0, 0, memory.as_mut_ptr()) }, 123);
    }
}
