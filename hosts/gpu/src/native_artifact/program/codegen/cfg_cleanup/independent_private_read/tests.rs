use super::*;
use cranelift_codegen::ir::{AbiParam, condcodes::IntCC};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Case {
    Disjoint,
    Overlap,
    InsufficientFloor,
    DifferentSelector,
    UnprovedStore,
    UnknownView,
    AlternateEntry,
}

type Entry = unsafe extern "C" fn(*mut u16, *const u64, *mut u64) -> u64;

fn cell(b: &mut FunctionBuilder<'_>, base: Value, index: Value, stride: u64, bias: i64) -> Value {
    let index = b.ins().band_imm(index, 3);
    let index = if stride == 2 {
        b.ins().imul_imm(index, 2)
    } else {
        index
    };
    let index = b.ins().iadd_imm(index, bias + 1);
    let ordinal = b.ins().iadd_imm(index, -1);
    let bytes = b.ins().imul_imm(ordinal, 2);
    b.ins().iadd(base, bytes)
}

fn emit(case: Case, optimize: bool) -> (JITModule, Entry) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut frontend = FunctionBuilderContext::new();
    let (table, cache, first, later, stores);
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
        let entry = b.create_block();
        let selected = b.create_block();
        let updated = b.create_block();
        let later_block = b.create_block();
        let checked = b.create_block();
        let reject = b.create_block();
        b.set_cold_block(reject);
        b.append_block_params_for_function_params(entry);
        b.append_block_param(later_block, types::I64);
        b.switch_to_block(entry);
        let &[tree, inputs, output] = b.block_params(entry) else {
            unreachable!()
        };
        let flags = MemFlags::new();
        let counts = b.ins().load(types::I64, flags, inputs, 0);
        let selector = b.ins().load(types::I64, flags, inputs, 8);
        let update = b.ins().load(types::I64, flags, inputs, 16);
        let control = b.ins().load(types::I64, flags, inputs, 24);
        table = tree;
        cache = counts;
        let zero = b.ins().iconst(types::I64, 0);
        if case == Case::AlternateEntry {
            b.ins()
                .brif(control, selected, &[], later_block, &[selector.into()]);
        } else {
            b.ins().jump(selected, &[]);
        }
        b.switch_to_block(selected);
        let short = cell(&mut b, tree, selector, 2, 8);
        let packed = flags.with_endianness(Endianness::Little);
        let word = b.ins().uload32(packed, short, 0);
        first = b.func.dfg.value_def(word).unwrap_inst();
        let replacement = b.ins().ireduce(types::I16, update);
        let short_store = b.ins().store(flags, replacement, short, 0);
        let low = b.ins().ireduce(types::I16, word);
        let short_previous = b.ins().store(flags, low, short, 2);
        let valid = b.ins().icmp_imm(IntCC::UnsignedLessThan, control, 2);
        b.ins().brif(valid, checked, &[], reject, &[]);
        b.switch_to_block(reject);
        let failed = b.ins().iconst(types::I64, 1236);
        b.ins().store(flags, failed, output, 0);
        b.ins().return_(&[failed]);
        b.switch_to_block(checked);
        b.ins()
            .brif(control, updated, &[], later_block, &[selector.into()]);
        b.switch_to_block(updated);
        let next = if case == Case::DifferentSelector {
            b.ins().iadd_imm(selector, 1)
        } else {
            selector
        };
        let value = b.ins().ireduce(types::I32, update);
        let cache_store = b.ins().store(flags, value, cache, 0);
        b.ins().jump(later_block, &[next.into()]);
        b.switch_to_block(later_block);
        let current = b.block_params(later_block)[0];
        let bias = if case == Case::Overlap { 8 } else { 0 };
        let long = cell(&mut b, tree, current, 1, bias);
        let old = b.ins().uload16(types::I64, flags, long, 0);
        later = b.func.dfg.value_def(old).unwrap_inst();
        let replacement = b.ins().ireduce(types::I16, update);
        let long_store = b.ins().store(flags, replacement, long, 0);
        b.ins().store(flags, old, output, 8);
        b.ins().return_(&[zero]);
        stores = [
            (short_store, table),
            (short_previous, table),
            (cache_store, cache),
            (long_store, table),
        ];
        b.seal_all_blocks();
        b.finalize();
    }
    ctx.compute_cfg();
    ctx.compute_domtree();
    if optimize {
        let floor = if case == Case::InsufficientFloor {
            30
        } else {
            32
        };
        let mut views = vec![View {
            base: table,
            element_bytes: 2,
            minimum_bytes: floor,
        }];
        if case != Case::UnknownView {
            views.push(View {
                base: cache,
                element_bytes: 4,
                minimum_bytes: 0,
            });
        }
        let writes = stores
            .into_iter()
            .enumerate()
            .filter_map(|(i, value)| (case != Case::UnprovedStore || i != 2).then_some(value))
            .collect();
        let changed = schedule(&mut ctx.func, &ctx.cfg, &ctx.domtree, &views, &writes);
        assert_eq!(changed, case == Case::Disjoint, "{case:?}");
        if changed {
            assert_eq!(
                ctx.func.layout.inst_block(later),
                ctx.func.layout.inst_block(first)
            );
            assert!(!schedule(
                &mut ctx.func,
                &ctx.cfg,
                &ctx.domtree,
                &views,
                &writes
            ));
        }
    }
    ctx.verify(module.isa()).unwrap();
    let id = module
        .declare_function("scheduled", Linkage::Local, &ctx.func.signature)
        .unwrap();
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    // The typed fixture passes live, initialized and disjoint allocations.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}

#[test]
fn independent_private_read_keeps_writes_rejection_and_retry() {
    let mut calls = 0;
    for case in [
        Case::Disjoint,
        Case::Overlap,
        Case::InsufficientFloor,
        Case::DifferentSelector,
        Case::UnprovedStore,
        Case::UnknownView,
        Case::AlternateEntry,
    ] {
        let (_before, original) = emit(case, false);
        let (_after, optimized) = emit(case, true);
        for index in [0, 1, 2, 3, u64::MAX] {
            for seed in [0u16, 1, 127, 65535] {
                let source = core::array::from_fn::<_, 16, _>(|i| seed.wrapping_add(i as u16));
                let mut a = source;
                let mut b = source;
                let mut cache_a = [77u32];
                let mut cache_b = cache_a;
                let mut out_a = [51u64, 99];
                let mut out_b = out_a;
                for (mode, value) in [(0, 31), (2, 17), (1, 19)] {
                    let old = out_b;
                    let read = b[(index & 3) as usize];
                    // Both typed workers get the same initialized contents and
                    // the same success/rejection/retry sequence.
                    let input_a = [cache_a.as_mut_ptr() as u64, index, value, mode];
                    let input_b = [cache_b.as_mut_ptr() as u64, index, value, mode];
                    let left =
                        unsafe { original(a.as_mut_ptr(), input_a.as_ptr(), out_a.as_mut_ptr()) };
                    let right =
                        unsafe { optimized(b.as_mut_ptr(), input_b.as_ptr(), out_b.as_mut_ptr()) };
                    assert_eq!(
                        (left, a, cache_a, out_a),
                        (right, b, cache_b, out_b),
                        "{case:?}"
                    );
                    if mode == 2 && case != Case::AlternateEntry {
                        assert_eq!((right, out_b), (1236, [1236, old[1]]));
                    } else if case == Case::Disjoint {
                        assert_eq!((right, out_b[1]), (0, u64::from(read)));
                    }
                    calls += 2;
                }
            }
        }
    }
    assert_eq!(calls, 840);
}
