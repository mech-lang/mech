use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::program::codegen::cfg_cleanup::combine_window_bounds;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    Shared,
    DifferentRejection,
    Effect,
    AlternateEntry,
}
type Entry = unsafe extern "C" fn(u64, u64, u64, *mut u64, u64) -> u64;

fn compile(width: u64, route: Route) -> (cranelift_jit::JITModule, Entry) {
    compile_with_mask(width, route, None)
}

fn compile_with_mask(
    width: u64,
    route: Route,
    mask: Option<i64>,
) -> (cranelift_jit::JITModule, Entry) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        let check = b.create_block();
        let first = b.create_block();
        let next = b.create_block();
        let second = b.create_block();
        let done = b.create_block();
        let fail = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(fail, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let seven = b.ins().iconst(types::I64, 7);
        let nine = b.ins().iconst(types::I64, 9);
        if route == Route::AlternateEntry {
            b.ins().brif(args[4], first, &[], check, &[]);
        } else {
            b.ins().jump(check, &[]);
        }
        for (at, inside, after, index) in
            [(check, first, next, args[1]), (next, second, done, args[2])]
        {
            b.switch_to_block(at);
            let enough = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, args[0], width as i64);
            let enough = match mask {
                Some(mask) => b.ins().band_imm(enough, mask),
                None => enough,
            };
            b.ins().brif(enough, inside, &[], fail, &[seven.into()]);
            b.switch_to_block(inside);
            if route == Route::Effect {
                b.ins().store(MemFlags::new(), nine, args[3], 0);
            }
            let limit = b.ins().iadd_imm(args[0], width.wrapping_neg() as i64);
            let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, index, limit);
            let fits = match mask {
                Some(mask) => b.ins().band_imm(fits, mask),
                None => fits,
            };
            let rejected = if route == Route::DifferentRejection {
                nine
            } else {
                seven
            };
            b.ins().brif(fits, after, &[], fail, &[rejected.into()]);
        }
        b.switch_to_block(done);
        let value = b.ins().iadd(args[1], args[2]);
        b.ins().store(MemFlags::new(), value, args[3], 0);
        let ok = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[ok]);
        b.switch_to_block(fail);
        let status = b.block_params(fail)[0];
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
    }
    let controls = CompilationControls::new(OptimizationOptions::default());
    let removed = combine_window_bounds(&controls, &mut context, module.isa()).unwrap();
    assert_eq!(
        removed,
        if route == Route::Shared && width != 0 && matches!(mask, None | Some(1)) {
            2
        } else {
            0
        },
        "{width} {route:?} {mask:?}"
    );
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("current_windows", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: the retained JIT and initialized bank have exactly this ABI and
    // remain live for each synchronous call.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}

#[test]
fn shared_windows_execute_short_exact_wrapping_and_retry_without_early_publication() {
    for width in [1, 2, 8, 256, u64::MAX] {
        let (_module, entry) = compile(width, Route::Shared);
        let mut published = 123;
        for length in [0, 1, 7, 8, 9, 256, u64::MAX - 1, u64::MAX] {
            for a in [0, 1, 8, length, u64::MAX] {
                for z in [0, 1, length, u64::MAX] {
                    let prior = published;
                    let ok = length >= width && a <= length - width && z <= length - width;
                    assert_eq!(
                        unsafe { entry(length, a, z, &mut published, 0) },
                        if ok { 0 } else { 7 }
                    );
                    assert_eq!(published, if ok { a.wrapping_add(z) } else { prior });
                }
            }
        }
        assert_eq!(unsafe { entry(width, 0, 0, &mut published, 0) }, 0);
        assert_eq!(published, 0);
    }
}

#[test]
fn shared_windows_decline_different_rejections_effects_alternate_entries_and_zero_width() {
    for route in [
        Route::DifferentRejection,
        Route::Effect,
        Route::AlternateEntry,
    ] {
        let (_module, entry) = compile(8, route);
        let mut bank = 123;
        let status = unsafe { entry(8, 1, 0, &mut bank, 0) };
        assert_eq!(
            status,
            if route == Route::DifferentRejection {
                9
            } else {
                7
            }
        );
        assert_eq!(bank, if route == Route::Effect { 9 } else { 123 });
        assert_eq!(unsafe { entry(8, 0, 0, &mut bank, 0) }, 0);
        assert_eq!(bank, 0);
    }
    let (_module, entry) = compile(0, Route::Shared);
    let mut bank = 123;
    assert_eq!(unsafe { entry(0, 0, 0, &mut bank, 0) }, 0);
    assert_eq!(bank, 0);
}

#[test]
fn shared_windows_accept_only_the_canonical_comparison_mask() {
    let (_masked, entry) = compile_with_mask(8, Route::Shared, Some(1));
    let mut published = 123;
    assert_eq!(unsafe { entry(7, 0, 0, &mut published, 0) }, 7);
    assert_eq!(published, 123);
    assert_eq!(unsafe { entry(9, 1, 1, &mut published, 0) }, 0);
    assert_eq!(published, 2);
    assert_eq!(unsafe { entry(9, 1, 2, &mut published, 0) }, 7);
    assert_eq!(published, 2);
    let (_lossy, rejected) = compile_with_mask(8, Route::Shared, Some(2));
    assert_eq!(unsafe { rejected(9, 1, 1, &mut published, 0) }, 7);
    assert_eq!(published, 2);
}
