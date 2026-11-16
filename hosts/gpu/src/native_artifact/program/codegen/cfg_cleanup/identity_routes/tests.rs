use crate::native_artifact::codegen::options::{
    CompilationControls, OptimizationOptions, OptimizationPass,
};
use crate::native_artifact::program::codegen::cfg_cleanup::forward_identity_routes;
use cranelift_codegen::{
    Context,
    ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
type Entry = unsafe extern "C" fn(u64, u64, u64, u64, *mut u64) -> u64;
unsafe extern "C" fn observe(memory: *mut u64, value: u64) -> u64 {
    unsafe { *memory.add(1) = (*memory.add(1)).wrapping_add(7) };
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
        let header = b.create_block();
        let choose = b.create_block();
        let positive = b.create_block();
        let zero_test = b.create_block();
        let zero_arm = b.create_block();
        let join = b.create_block();
        let yes = b.create_block();
        let no_test = b.create_block();
        let no = b.create_block();
        let finish = b.create_block();
        let bad = b.create_block();
        let overflow = b.create_block();
        let guard = b.create_block();
        let stored = b.create_block();
        b.append_block_params_for_function_params(entry);
        for ty in [types::I64, types::I64, types::F64] {
            b.append_block_param(header, ty);
        }
        for ty in [types::I64, types::I64, types::F64] {
            b.append_block_param(join, ty);
        }
        b.switch_to_block(entry);
        let a = b.block_params(entry).to_vec();
        let z = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let fp = b.ins().bitcast(types::F64, MemFlags::new(), a[3]);
        b.ins().jump(header, &[z.into(), a[1].into(), fp.into()]);
        b.switch_to_block(header);
        let h = b.block_params(header).to_vec();
        let more = b.ins().icmp(IntCC::UnsignedLessThan, h[0], a[2]);
        b.ins().brif(more, choose, &[], finish, &[]);
        b.switch_to_block(choose);
        let flag = if variant == "raw" {
            a[0]
        } else {
            let flag = b.ins().icmp_imm(IntCC::NotEqual, a[0], 0);
            b.ins().uextend(types::I64, flag)
        };
        let selected = b.ins().icmp_imm(IntCC::Equal, flag, 1);
        b.ins().brif(selected, positive, &[], zero_test, &[]);
        b.switch_to_block(zero_test);
        let selected = b.ins().icmp_imm(IntCC::Equal, flag, 0);
        b.ins().brif(selected, zero_arm, &[], bad, &[]);
        b.switch_to_block(zero_arm);
        let mut payload = h[1];
        if variant == "load" {
            payload = b.ins().load(types::I64, MemFlags::new(), a[4], 8);
        }
        if variant == "effect" {
            b.ins().store(MemFlags::new(), h[0], a[4], 8);
        }
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
        b.ins().jump(join, &[z.into(), payload.into(), h[2].into()]);
        b.switch_to_block(positive);
        b.ins().jump(join, &[one.into(), h[1].into(), h[2].into()]);
        b.switch_to_block(join);
        let j = b.block_params(join).to_vec();
        let selected = b.ins().icmp_imm(IntCC::Equal, j[0], 1);
        b.ins().brif(selected, yes, &[], no_test, &[]);
        b.switch_to_block(no_test);
        let selected = b.ins().icmp_imm(IntCC::Equal, j[0], 0);
        b.ins().brif(selected, no, &[], bad, &[]);
        b.switch_to_block(yes);
        let current_bits = b.ins().bitcast(types::I64, MemFlags::new(), j[2]);
        let sum;
        if variant == "overflow-first" {
            let (s, o) = b.ins().uadd_overflow(j[1], one);
            sum = s;
            b.ins().brif(o, overflow, &[], guard, &[]);
            b.switch_to_block(guard);
            let ok = b.ins().icmp_imm(IntCC::NotEqual, current_bits, 0);
            b.ins().brif(ok, stored, &[], bad, &[]);
        } else {
            let ok = b.ins().icmp_imm(IntCC::NotEqual, current_bits, 0);
            b.ins().brif(ok, guard, &[], bad, &[]);
            b.switch_to_block(guard);
            let (s, o) = b.ins().uadd_overflow(j[1], one);
            sum = s;
            b.ins().brif(o, overflow, &[], stored, &[]);
        }
        b.switch_to_block(stored);
        let bits = b.ins().bitcast(types::I64, MemFlags::new(), j[2]);
        let changed = b.ins().bxor(bits, a[0]);
        let changed = b.ins().bitcast(types::F64, MemFlags::new(), changed);
        let next = b.ins().iadd(h[0], one);
        b.ins().store(MemFlags::new(), next, a[4], 0);
        b.ins()
            .jump(header, &[next.into(), sum.into(), changed.into()]);
        b.switch_to_block(no);
        let next = b.ins().iadd(h[0], one);
        b.ins().store(MemFlags::new(), next, a[4], 0);
        b.ins()
            .jump(header, &[next.into(), j[1].into(), j[2].into()]);
        b.switch_to_block(finish);
        let bits = b.ins().bitcast(types::I64, MemFlags::new(), h[2]);
        let result = b.ins().bxor(h[1], bits);
        b.ins().return_(&[result]);
        b.switch_to_block(bad);
        let code = b.ins().iconst(types::I64, 100);
        b.ins().return_(&[code]);
        b.switch_to_block(overflow);
        let code = b.ins().iconst(types::I64, 200);
        b.ins().return_(&[code]);
        b.seal_all_blocks();
        b.finalize();
    }
    c.verify(module.isa()).unwrap();
    c
}
fn define(module: &mut JITModule, c: &mut Context, name: &str) -> Entry {
    let id = module
        .declare_function(name, Linkage::Local, &c.func.signature)
        .unwrap();
    module.define_function(id, c).unwrap();
    module.finalize_definitions().unwrap();
    unsafe { std::mem::transmute(module.get_finalized_function(id)) }
}
fn expected(flag: u64, mut payload: u64, n: u64, mut bits: u64, variant: &str) -> (u64, [u64; 2]) {
    let mut memory = [0, 0xabcd];
    for i in 0..n {
        if variant == "raw" && flag > 1 {
            return (100, memory);
        }
        if flag == 0 {
            match variant {
                "load" => payload = memory[1],
                "effect" => memory[1] = i,
                "call" => memory[1] = memory[1].wrapping_add(7),
                _ => {}
            }
        } else {
            if variant == "overflow-first" && payload == u64::MAX {
                return (200, memory);
            }
            if bits == 0 {
                return (100, memory);
            }
            let Some(next) = payload.checked_add(1) else {
                return (200, memory);
            };
            payload = next;
            bits ^= flag;
        }
        memory[0] = i + 1;
    }
    (payload ^ bits, memory)
}
#[test]
fn identity_routes_transport_current_words_float_payloads_and_failure_prefix() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for (i, variant) in ["normal", "raw", "overflow-first", "load", "effect", "call"]
        .into_iter()
        .enumerate()
    {
        let mut old = fixture(&module, variant);
        let before = define(&mut module, &mut old, &format!("old{i}"));
        let mut new = fixture(&module, variant);
        let on = CompilationControls::new(OptimizationOptions::default());
        let report = forward_identity_routes(&on, &mut new, module.isa()).unwrap();
        assert!(report.threaded_edges > 0, "{variant}");
        let after = define(&mut module, &mut new, &format!("new{i}"));
        for flag in [0, 1, 2, u64::MAX] {
            for payload in [0, 17, u64::MAX - 1, u64::MAX] {
                for n in [0, 1, 2, 4] {
                    for bits in [
                        0,
                        1,
                        0x8000000000000000,
                        0x7ff8000000000001,
                        0x7ff0000000000001,
                    ] {
                        let want = expected(flag, payload, n, bits, variant);
                        for f in [before, after] {
                            let mut memory = [0, 0xabcd];
                            let got = unsafe { f(flag, payload, n, bits, memory.as_mut_ptr()) };
                            assert_eq!(
                                (got, memory),
                                want,
                                "{variant} {flag} {payload} {n} {bits}"
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn identity_routes_honor_disabled_cleanup_and_keep_typed_ir_valid() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = fixture(&module, "normal");
    let old = c.func.display().to_string();
    let off = CompilationControls::new(OptimizationOptions::without(OptimizationPass::CfgCleanup));
    assert_eq!(
        forward_identity_routes(&off, &mut c, module.isa())
            .unwrap()
            .threaded_edges,
        0
    );
    assert_eq!(c.func.display().to_string(), old);
    let on = CompilationControls::new(OptimizationOptions::default());
    assert!(
        forward_identity_routes(&on, &mut c, module.isa())
            .unwrap()
            .threaded_edges
            > 0
    );
    c.verify(module.isa()).unwrap();
}

#[test]
fn identity_routes_preserve_previous_predicate_snapshots_and_decline_reentry() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 5];
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
    assert_eq!(
        forward_identity_routes(&on, &mut c, module.isa())
            .unwrap()
            .threaded_edges,
        0
    );
    c.verify(module.isa()).unwrap();
    let after = define(&mut module, &mut c, "atom_after");
    for seed in [0, 1, u64::MAX] {
        let mut memory = [0, 0];
        assert_eq!(unsafe { before(seed, 0, 0, 0, memory.as_mut_ptr()) }, 123);
        assert_eq!(unsafe { after(seed, 0, 0, 0, memory.as_mut_ptr()) }, 123);
    }
}
#[test]
fn identity_routes_keep_both_edges_to_one_join_and_parallel_payload_order() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 5];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let join = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        b.append_block_params_for_function_params(entry);
        for _ in 0..3 {
            b.append_block_param(join, types::I64);
        }
        b.switch_to_block(entry);
        let a = b.block_params(entry).to_vec();
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().brif(
            a[0],
            join,
            &[one.into(), a[1].into(), a[2].into()],
            join,
            &[zero.into(), a[2].into(), a[1].into()],
        );
        b.switch_to_block(join);
        let j = b.block_params(join).to_vec();
        let is_zero = b.ins().icmp_imm(IntCC::Equal, j[0], 0);
        b.ins().brif(is_zero, yes, &[], no, &[]);
        b.switch_to_block(yes);
        b.ins().store(MemFlags::new(), j[1], a[4], 0);
        b.ins().store(MemFlags::new(), j[2], a[4], 8);
        b.ins().return_(&[zero]);
        b.switch_to_block(no);
        b.ins().store(MemFlags::new(), j[1], a[4], 0);
        b.ins().store(MemFlags::new(), j[2], a[4], 8);
        b.ins().return_(&[one]);
        b.seal_all_blocks();
        b.finalize();
    }
    let mut old = module.make_context();
    old.func = c.func.clone();
    let before = define(&mut module, &mut old, "same_join_before");
    let on = CompilationControls::new(OptimizationOptions::default());
    let report = forward_identity_routes(&on, &mut c, module.isa()).unwrap();
    assert_eq!(report.threaded_edges, 2);
    assert!(report.repair_parameters >= 4);
    let after = define(&mut module, &mut c, "same_join_after");
    for flag in [0, 1, 2, u64::MAX] {
        for left in [0, 17, u64::MAX] {
            for right in [1, 37, u64::MAX] {
                let want = if flag == 0 {
                    (0, [right, left])
                } else {
                    (1, [left, right])
                };
                for f in [before, after] {
                    let mut memory = [0xa55a; 2];
                    let result = unsafe { f(flag, left, right, 0, memory.as_mut_ptr()) };
                    assert_eq!((result, memory), want);
                }
            }
        }
    }
}
