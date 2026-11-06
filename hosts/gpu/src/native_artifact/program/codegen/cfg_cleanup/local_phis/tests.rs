use super::*;
use cranelift_codegen::ir::types;
use cranelift_module::{Linkage, Module, default_libcall_names};
#[path = "fixture.rs"]
mod fixture;
use fixture::{FIELDS, build};

type Entry = unsafe extern "C" fn(*mut u64, u64, u64, u64, u64) -> u64;
fn compiled(
    mode: u8,
    ty: cranelift_codegen::ir::Type,
    optimize: bool,
) -> (cranelift_jit::JITModule, Entry, usize, usize) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    let (function, child) = build(mode, ty);
    context.func = function;
    context.func.signature.call_conv = module.make_signature().call_conv;
    context.verify(module.isa()).unwrap();
    context.compute_cfg();
    context.compute_domtree();
    context.remove_constant_phis(module.isa()).unwrap();
    let before = context.func.dfg.block_params(child).len();
    assert_eq!(before, FIELDS + 2);
    let removed = if optimize {
        simplify(&mut context.func, &context.cfg, &context.domtree)
    } else {
        0
    };
    context.verify(module.isa()).unwrap();
    let after = context.func.dfg.block_params(child).len();
    let id = module
        .declare_function(
            "nested_live_parent",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: exactly this five-word ABI was declared. The first pointer covers
    // three live words, and the owned module outlives every synchronous call.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry, removed, after)
}
fn expected(outer: u64, inner: u64, seed: u64, mode: u8, width: u32) -> u64 {
    let mask = u64::MAX >> (64 - width);
    let mut sum = 0u64;
    for o in 0..outer {
        for i in 0..inner {
            sum = sum.wrapping_add(i);
        }
        for j in 0..FIELDS as u64 {
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

#[test]
fn local_phis_replace_unchanged_inner_fields_with_current_outer_values() {
    let (_module, entry, removed, after) = compiled(0, types::I64, true);
    assert!(removed >= FIELDS * 2);
    assert_eq!(after, 2);
    for (outer, inner, seed) in [
        (0, 0, 2),
        (0, 4, 2),
        (3, 0, 2),
        (3, 4, 2),
        (2, 7, 19),
        (3, 4, u64::MAX - 8),
    ] {
        let mut output = [0xfeed, 0, u64::MAX];
        assert_eq!(
            unsafe { entry(output.as_mut_ptr(), outer, inner, seed, u64::MAX) },
            expected(outer, inner, seed, 0, 64)
        );
        assert_eq!(output, [0xfeed, 0, u64::MAX]);
    }
}

#[test]
fn local_phis_keep_genuine_inner_updates_and_multiple_outside_values() {
    for mode in [1, 2, 3] {
        let (_module, entry, _removed, after) = compiled(mode, types::I64, true);
        assert_eq!(after, FIELDS + 2, "mode {mode}");
        for (outer, inner, seed) in [
            (0, 0, 0),
            (3, 0, 7),
            (3, 4, 2),
            (4, 7, 91),
            (3, 4, u64::MAX - 8),
        ] {
            let mut output = [0xfeed, 0, u64::MAX];
            assert_eq!(
                unsafe { entry(output.as_mut_ptr(), outer, inner, seed, u64::MAX) },
                expected(outer, inner, seed, mode, 64)
            );
        }
    }
}

#[test]
fn local_phis_preserve_scalar_widths_and_float_bit_patterns() {
    for ty in [
        types::I8,
        types::I16,
        types::I32,
        types::I64,
        types::F32,
        types::F64,
    ] {
        let (_module, entry, removed, after) = compiled(0, ty, true);
        assert!(removed >= FIELDS * 2);
        assert_eq!(after, 2);
        for seed in [
            0,
            1,
            2,
            0xff,
            1u64 << 63,
            0x7ff8_1234_0000_0000,
            u64::MAX - 8,
        ] {
            let mut output = [0xfeed, 0, u64::MAX];
            assert_eq!(
                unsafe { entry(output.as_mut_ptr(), 3, 4, seed, u64::MAX) },
                expected(3, 4, seed, 0, ty.bits())
            );
        }
    }
}

#[test]
fn local_phis_preserve_effects_prior_publication_early_failure_and_retry() {
    let (_before, before, _, _) = compiled(4, types::I64, false);
    let (_after, after, removed, parameters) = compiled(4, types::I64, true);
    assert!(removed >= FIELDS * 2);
    assert_eq!(parameters, 2);
    let mut original = [0xfeed, 0, u64::MAX];
    let mut optimized = original;
    for (seed, fail) in [(7, u64::MAX), (19, 0), (31, 3), (43, u64::MAX)] {
        let published = optimized[0];
        let a = unsafe { before(original.as_mut_ptr(), 3, 4, seed, fail) };
        let b = unsafe { after(optimized.as_mut_ptr(), 3, 4, seed, fail) };
        assert_eq!((a, original), (b, optimized));
        if fail != u64::MAX {
            assert_eq!(b, 0xbad);
            assert_eq!(optimized[0], published);
        } else {
            assert_eq!(b, expected(3, 4, seed, 0, 64));
            assert_eq!(optimized[0], b);
        }
    }
}
