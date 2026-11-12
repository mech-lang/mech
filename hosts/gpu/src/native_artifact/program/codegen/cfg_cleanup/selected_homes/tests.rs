use super::*;
use crate::native_artifact::codegen::options::{
    CompilationControls, OptimizationOptions, OptimizationPass,
};
use crate::native_artifact::program::codegen::cfg_cleanup::rematerialize_boolean_homes;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, MemFlags, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, *mut u64) -> u64;

unsafe extern "C" fn observe(memory: *mut u64, value: u64) -> u64 {
    unsafe { *memory.add(2) = (*memory.add(2)).wrapping_add(7) };
    value
}

fn fixture(module: &JITModule, variant: &str) -> Context {
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 5];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let select = b.create_block();
        let positive = b.create_block();
        let header = b.create_block();
        let body = b.create_block();
        let advance = b.create_block();
        let leave = b.create_block();
        let negative = b.create_block();
        let join = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        let failed = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(header, types::I64);
        b.append_block_param(header, types::I64);
        b.append_block_param(join, types::I64);
        b.switch_to_block(entry);
        let a = b.block_params(entry).to_vec();
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let raw = if variant == "fresh-load" {
            b.ins().load(types::I64, MemFlags::new(), a[4], 0)
        } else {
            a[0]
        };
        let predicate = if variant == "raw-mask" {
            raw
        } else {
            b.ins().icmp_imm(IntCC::NotEqual, raw, 0)
        };
        let home = if variant == "raw-mask" {
            predicate
        } else {
            b.ins().uextend(types::I64, predicate)
        };
        if variant == "bypass" {
            b.ins().brif(a[3], join, &[a[1].into()], select, &[]);
        } else {
            b.ins().jump(select, &[]);
        }
        b.switch_to_block(select);
        let bit = b.ins().band_imm(predicate, 1);
        if variant == "duplicate-edges" {
            b.ins()
                .brif(bit, join, &[a[1].into()], join, &[a[2].into()]);
        } else {
            b.ins().brif(bit, positive, &[], negative, &[]);
        }
        b.switch_to_block(positive);
        // Retain the exact original source-home store and ordered effects.
        b.ins().store(MemFlags::new(), home, a[4], 8);
        b.ins().jump(header, &[zero.into(), a[1].into()]);
        b.switch_to_block(header);
        let h = b.block_params(header).to_vec();
        let more = b.ins().icmp(IntCC::UnsignedLessThan, h[0], a[2]);
        b.ins().brif(more, body, &[], leave, &[]);
        b.switch_to_block(body);
        let mut payload = h[1];
        if variant == "call" {
            let mut sig = module.make_signature();
            sig.params = vec![AbiParam::new(types::I64); 2];
            sig.returns = vec![AbiParam::new(types::I64)];
            let sig = b.import_signature(sig);
            let callee = b
                .ins()
                .iconst(types::I64, observe as *const () as usize as i64);
            let call = b.ins().call_indirect(sig, callee, &[a[4], payload]);
            payload = b.inst_results(call)[0];
        }
        // This store precedes overflow, including on a failing visit.
        b.ins().store(MemFlags::new(), h[0], a[4], 16);
        let (next_payload, overflow) = b.ins().uadd_overflow(payload, one);
        b.ins().brif(overflow, failed, &[], advance, &[]);
        b.switch_to_block(advance);
        let next = b.ins().iadd(h[0], one);
        b.ins().jump(header, &[next.into(), next_payload.into()]);
        b.switch_to_block(leave);
        b.ins().jump(join, &[h[1].into()]);
        b.switch_to_block(negative);
        b.ins().store(MemFlags::new(), home, a[4], 8);
        b.ins().jump(join, &[a[1].into()]);
        b.switch_to_block(join);
        let payload = b.block_params(join)[0];
        if variant == "fresh-load" {
            b.ins().store(MemFlags::new(), a[3], a[4], 0);
            let fresh = b.ins().load(types::I64, MemFlags::new(), a[4], 0);
            let result = b.ins().bxor(fresh, payload);
            b.ins().return_(&[result]);
        } else {
            b.ins().brif(predicate, yes, &[], no, &[]);
        }
        b.switch_to_block(yes);
        let result = b.ins().bxor(payload, home);
        b.ins().return_(&[result]);
        b.switch_to_block(no);
        b.ins().return_(&[payload]);
        b.switch_to_block(failed);
        let origin = b.ins().iconst(types::I64, 9001);
        b.ins().store(MemFlags::new(), origin, a[4], 24);
        b.ins().return_(&[home]);
        b.seal_all_blocks();
        b.finalize();
    }
    c.verify(module.isa()).unwrap();
    c
}

fn define(module: &mut JITModule, context: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, context).unwrap();
    module.finalize_definitions().unwrap();
    unsafe { core::mem::transmute(module.get_finalized_function(id)) }
}

#[test]
fn selected_boolean_homes_preserve_loop_effects_alias_widths_and_failure_prefix() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for (i, variant) in [
        "normal",
        "raw-mask",
        "call",
        "bypass",
        "duplicate-edges",
        "fresh-load",
    ]
    .into_iter()
    .enumerate()
    {
        let mut before = fixture(&module, variant);
        let mut after = fixture(&module, variant);
        let on = CompilationControls::new(OptimizationOptions::default());
        let report = rematerialize_boolean_homes(&on, &mut after, module.isa()).unwrap();
        if matches!(variant, "normal" | "call") {
            assert!(report.merge_parameters > 0, "{variant}");
        }
        if matches!(variant, "raw-mask" | "duplicate-edges" | "bypass") {
            assert_eq!(report.merge_parameters, 0, "{variant}");
        }
        let old = define(&mut module, &mut before, &format!("old{i}"));
        let new = define(&mut module, &mut after, &format!("new{i}"));
        for flag in [0, 1, 2, 3, 0x8000_0000_0000_0000, u64::MAX] {
            for payload in [0, 17, u64::MAX - 1, u64::MAX] {
                for count in [0, 1, 2, 4] {
                    for bypass in [0, 1, 9] {
                        let mut left = [flag, 0xabcd, 0x2222, 0x3333];
                        let mut right = left;
                        let expected =
                            unsafe { old(flag, payload, count, bypass, left.as_mut_ptr()) };
                        let actual =
                            unsafe { new(flag, payload, count, bypass, right.as_mut_ptr()) };
                        assert_eq!(
                            (actual, right),
                            (expected, left),
                            "{variant} {flag} {payload} {count} {bypass}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn selected_boolean_homes_honor_emission_ablation() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for pass in [
        OptimizationPass::CfgCleanup,
        OptimizationPass::ScalarEdgeHomes,
    ] {
        let mut context = fixture(&module, "normal");
        let before = context.func.display().to_string();
        let controls = CompilationControls::new(OptimizationOptions::without(pass));
        let report = rematerialize_boolean_homes(&controls, &mut context, module.isa()).unwrap();
        assert_eq!((report.constant_operands, report.merge_parameters), (0, 0));
        assert_eq!(context.func.display().to_string(), before);
    }
}

#[test]
fn selected_boolean_homes_use_typed_literals_and_reconstruct_the_proved_merge() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut context = fixture(&module, "normal");
    context.compute_cfg();
    context.compute_domtree();
    let original = context
        .func
        .layout
        .blocks()
        .map(|block| (block, context.func.dfg.block_params(block).len()))
        .collect::<BTreeMap<_, _>>();
    let report = optimize(&mut context.func, &context.cfg, &context.domtree);
    assert!(report.constant_operands > 0);
    assert!(report.merge_parameters > 0);
    for block in context.func.layout.blocks() {
        for &value in &context.func.dfg.block_params(block)[original[&block]..] {
            assert_eq!(context.func.dfg.value_type(value), types::I8);
        }
        for inst in context.func.layout.block_insts(block) {
            if let Data::Store { offset, args, .. } = context.func.dfg.insts[inst]
                && i32::from(offset) == 8
            {
                assert_eq!(context.func.dfg.value_type(args[0]), types::I64);
            }
        }
    }
    context.verify(module.isa()).unwrap();
}

#[test]
fn selected_boolean_homes_preserve_a_previous_loop_predicate_snapshot() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 5];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let header = b.create_block();
        let capture = b.create_block();
        let exit = b.create_block();
        let good = b.create_block();
        let bad = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(header, types::I64);
        b.append_block_param(header, types::I64);
        b.append_block_param(exit, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().jump(header, &[zero.into(), args[0].into()]);
        b.switch_to_block(header);
        let h = b.block_params(header).to_vec();
        let first = b.ins().icmp_imm(IntCC::Equal, h[0], 0);
        let bit = b.ins().band_imm(first, 1);
        b.ins().brif(bit, capture, &[], exit, &[h[1].into()]);
        b.switch_to_block(capture);
        let old_predicate = b.ins().icmp_imm(IntCC::Equal, h[0], 0);
        let snapshot = b.ins().uextend(types::I64, old_predicate);
        b.ins().store(MemFlags::new(), snapshot, args[4], 8);
        b.ins().jump(header, &[one.into(), snapshot.into()]);
        b.switch_to_block(exit);
        let saved = b.block_params(exit)[0];
        // The current predicate is false while the previous snapshot is true.
        b.ins().store(MemFlags::new(), saved, args[4], 0);
        b.ins().brif(saved, good, &[], bad, &[]);
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
    let before = define(&mut module, &mut old, "snapshot_before");
    let controls = CompilationControls::new(OptimizationOptions::default());
    rematerialize_boolean_homes(&controls, &mut c, module.isa()).unwrap();
    let after = define(&mut module, &mut c, "snapshot_after");
    for seed in [0, 1, 2, u64::MAX] {
        for call in [before, after] {
            let mut memory = [0xaabb; 4];
            assert_eq!(unsafe { call(seed, 0, 0, 0, memory.as_mut_ptr()) }, 123);
            assert_eq!(&memory[..2], &[1, 1]);
        }
    }
}

#[test]
fn selected_boolean_assumptions_bound_shared_dags_and_propagate_initial_facts_once() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64)];
    context.func.signature.returns = vec![AbiParam::new(types::I8)];
    let mut fc = FunctionBuilderContext::new();
    let (leaf, root);
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let input = b.block_params(entry)[0];
        leaf = b.ins().icmp_imm(IntCC::NotEqual, input, 0);
        let mut shared = leaf;
        for _ in 0..35 {
            shared = b.ins().band(shared, shared);
        }
        root = shared;
        b.ins().return_(&[root]);
        b.seal_all_blocks();
        b.finalize();
    }
    context.verify(module.isa()).unwrap();
    let mut known = BTreeMap::from([(root, Range(1, 1))]);
    let mut remaining = 128;
    assert!(condition::assume_bounded(
        &context.func,
        root,
        true,
        &mut known,
        &mut remaining
    ));
    assert!(remaining >= 40, "shared DAG should take linear proof work");
    assert_eq!(known.get(&leaf), Some(&Range(1, 1)));
    let mut insufficient = 4;
    let mut declined = BTreeMap::new();
    assert!(!condition::assume_bounded(
        &context.func,
        root,
        true,
        &mut declined,
        &mut insufficient
    ));
    assert_eq!(insufficient, 0);
    let mut ordinary = BTreeMap::from([(root, Range(1, 1))]);
    assert!(condition::assume(&context.func, root, true, &mut ordinary));
    assert_eq!(ordinary.get(&leaf), Some(&Range(1, 1)));
}
