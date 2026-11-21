use super::identical_arguments::coalesce;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, Function, InstBuilder, Type, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[path = "fixture.rs"]
mod fixture;

fn expected(outer: u64, inner: u64, seed: u64, mode: u8, width: u32) -> u64 {
    let mask = u64::MAX >> (64 - width);
    let mut sum = 0u64;
    for o in 0..outer {
        for i in 0..inner {
            sum = sum.wrapping_add(i);
        }
        for j in 0..fixture::FIELDS as u64 {
            let extra = if mode == 1 {
                inner
            } else if mode == 2 && o & 1 != 0 && inner != 0 {
                100
            } else {
                0
            };
            sum = sum.wrapping_add(seed.wrapping_add(o).wrapping_add(j).wrapping_add(extra) & mask);
        }
    }
    sum
}

type LoopEntry = unsafe extern "C" fn(*mut u64, u64, u64, u64, u64) -> u64;

fn loop_entry(mode: u8, ty: Type, optimize: bool) -> (JITModule, LoopEntry, usize) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names()).unwrap(),
    );
    let mut context = module.make_context();
    context.func = fixture::build(mode, ty).0;
    context.func.signature.call_conv = module.make_signature().call_conv;
    context.flowgraph();
    context.remove_constant_phis(module.isa()).unwrap();
    let removed = if optimize {
        coalesce(&mut context.func, &context.cfg, &context.domtree)
    } else {
        0
    };
    context.func.dfg.resolve_all_aliases();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "same_current_value",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the fixture declares exactly this ABI and three writable words.
    // The returned module owns the code throughout all synchronous calls.
    let entry =
        unsafe { core::mem::transmute::<*const u8, LoopEntry>(module.get_finalized_function(id)) };
    (module, entry, removed)
}

#[test]
fn identical_arguments_preserve_live_outer_values_widths_effects_and_retry() {
    for ty in [
        types::I8,
        types::I16,
        types::I32,
        types::I64,
        types::F32,
        types::F64,
    ] {
        for mode in 0..=4 {
            let (_original, before, _) = loop_entry(mode, ty, false);
            let (_optimized, after, removed) = loop_entry(mode, ty, true);
            assert!(removed >= fixture::FIELDS, "{ty} mode {mode}: {removed}");
            for (outer, inner) in [(0, 0), (0, 4), (3, 0), (3, 4), (4, 7)] {
                for seed in [0, 1, 0xff, 1 << 63, 0x7ff8_1234_0000_0000, u64::MAX - 8] {
                    let mut original = [0xfeed, 0, u64::MAX];
                    let mut optimized = original;
                    for fail in [u64::MAX, 0, 3, u64::MAX] {
                        let published = original[0];
                        let a = unsafe { before(original.as_mut_ptr(), outer, inner, seed, fail) };
                        let b = unsafe { after(optimized.as_mut_ptr(), outer, inner, seed, fail) };
                        assert_eq!((a, original), (b, optimized), "{ty} mode {mode}");
                        if mode == 4 && outer != 0 && fail < inner {
                            assert_eq!(b, 0xbad);
                            assert_eq!(optimized[0], published);
                        } else {
                            assert_eq!(b, expected(outer, inner, seed, mode, ty.bits()));
                        }
                    }
                }
            }
        }
    }
}

fn diamond(distinct: bool) -> (Function, Block) {
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 3];
    function.signature.returns = vec![AbiParam::new(types::I64)];
    let mut frontend = FunctionBuilderContext::new();
    let join;
    {
        let mut b = FunctionBuilder::new(&mut function, &mut frontend);
        let entry = b.create_block();
        let header = b.create_block();
        let body = b.create_block();
        join = b.create_block();
        let done = b.create_block();
        b.append_block_params_for_function_params(entry);
        for _ in 0..3 {
            b.append_block_param(header, types::I64);
        }
        b.append_block_param(join, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let zero = b.ins().iconst(types::I64, 0);
        b.ins()
            .jump(header, &[zero.into(), zero.into(), args[1].into()]);
        b.switch_to_block(header);
        let current = b.block_params(header).to_vec();
        let more = b.ins().icmp(IntCC::UnsignedLessThan, current[0], args[0]);
        b.ins().brif(more, body, &[], done, &[]);
        b.switch_to_block(body);
        let parity = b.ins().bxor(current[0], args[2]);
        let odd = b.ins().band_imm(parity, 1);
        let other = if distinct {
            b.ins().iadd_imm(current[2], 2)
        } else {
            current[2]
        };
        // Both destinations are the same block; each edge still has its own
        // actual argument. The changing outer phi is not a global constant.
        b.ins()
            .brif(odd, join, &[current[2].into()], join, &[other.into()]);
        b.switch_to_block(join);
        let selected = b.block_params(join)[0];
        let next = b.ins().iadd_imm(current[0], 1);
        let sum = b.ins().iadd(current[1], selected);
        let value = b.ins().iadd_imm(current[2], 1);
        b.ins()
            .jump(header, &[next.into(), sum.into(), value.into()]);
        b.switch_to_block(done);
        b.ins().return_(&[current[1]]);
        b.seal_all_blocks();
        b.finalize();
    }
    (function, join)
}

#[test]
fn identical_arguments_keep_different_values_on_two_edges_to_one_destination() {
    type Entry = unsafe extern "C" fn(u64, u64, u64) -> u64;
    for distinct in [false, true] {
        let mut module = JITModule::new(
            JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names()).unwrap(),
        );
        let mut context = module.make_context();
        let (function, join) = diamond(distinct);
        context.func = function;
        context.func.signature.call_conv = module.make_signature().call_conv;
        context.flowgraph();
        context.remove_constant_phis(module.isa()).unwrap();
        assert_eq!(context.func.dfg.block_params(join).len(), 1);
        let removed = coalesce(&mut context.func, &context.cfg, &context.domtree);
        assert_eq!(
            context.func.dfg.block_params(join).len(),
            usize::from(distinct)
        );
        if !distinct {
            assert!(removed >= 1);
        }
        context.func.dfg.resolve_all_aliases();
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(
                "two_distinct_edges",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: exactly this three-word ABI is declared; module owns the code.
        let entry =
            unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        for n in [0, 1, 2, 5, 11] {
            for seed in [0u64, 1, u64::MAX, u64::MAX - 8] {
                for toggle in 0..4 {
                    let expected = (0..n).fold(0u64, |sum, i| {
                        let extra = if distinct && (i ^ toggle) & 1 == 0 {
                            2
                        } else {
                            0
                        };
                        sum.wrapping_add(seed.wrapping_add(i).wrapping_add(extra))
                    });
                    assert_eq!(unsafe { entry(n, seed, toggle) }, expected);
                }
            }
        }
    }
}
