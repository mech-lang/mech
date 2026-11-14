use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::program::codegen::cfg_cleanup::fold_ordered_capture_spans;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    Ordered,
    Reversed,
    MissingOrder,
    MissingOverflow,
    DifferentLength,
    DifferentExtent,
    AlternateEntry,
}

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;

fn compile(route: Route) -> (cranelift_jit::JITModule, Entry) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let prove = b.create_block();
        let bounded = b.create_block();
        let capture = b.create_block();
        let selected = b.create_block();
        let refused = b.create_block();
        let wrapped = b.create_block();
        let outside = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [left, right, extent, length, other] = b.block_params(entry).try_into().unwrap();
        if route == Route::AlternateEntry {
            let bypass = b.ins().icmp_imm(IntCC::Equal, left, 0);
            b.ins().brif(bypass, capture, &[], prove, &[]);
        } else {
            b.ins().jump(prove, &[]);
        }
        b.switch_to_block(prove);
        let (early, late) = if route == Route::Reversed {
            (right, left)
        } else {
            (left, right)
        };
        let end = b.ins().iadd(late, extent);
        if route == Route::MissingOverflow {
            b.ins().jump(bounded, &[]);
        } else {
            let overflow = b.ins().icmp(IntCC::UnsignedLessThan, end, late);
            b.ins().brif(overflow, wrapped, &[], bounded, &[]);
        }
        b.switch_to_block(bounded);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, length);
        let condition = if route == Route::MissingOrder {
            fits
        } else {
            let ordered = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, early, late);
            let fits = b.ins().uextend(types::I64, fits);
            let ordered = b.ins().uextend(types::I64, ordered);
            b.ins().band(fits, ordered)
        };
        b.ins().brif(condition, capture, &[], outside, &[]);
        b.switch_to_block(capture);
        let mut enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, extent, 8);
        for (base, count, limit) in [
            (left, extent, length),
            (
                right,
                if route == Route::DifferentExtent {
                    other
                } else {
                    extent
                },
                if route == Route::DifferentLength {
                    other
                } else {
                    length
                },
            ),
        ] {
            let start_ok = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, base, limit);
            let remainder = b.ins().isub(limit, base);
            let end_ok = b
                .ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, count, remainder);
            enough = b.ins().band(enough, start_ok);
            enough = b.ins().band(enough, end_ok);
        }
        b.ins().brif(enough, selected, &[], refused, &[]);
        for (block, result) in [(selected, 1), (refused, 0), (wrapped, 2), (outside, 3)] {
            b.switch_to_block(block);
            let result = b.ins().iconst(types::I64, result);
            b.ins().return_(&[result]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    let controls = CompilationControls::new(OptimizationOptions::default());
    let changed = fold_ordered_capture_spans(&controls, &mut context, module.isa()).unwrap();
    assert_eq!(
        changed,
        usize::from(matches!(route, Route::Ordered | Route::Reversed)),
        "{route:?}"
    );
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "ordered_capture_span",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // The verified entry takes five unsigned words and returns one word.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}

fn reference(route: Route, args: [u64; 5]) -> u64 {
    let [left, right, extent, length, other] = args;
    if route != Route::AlternateEntry || left != 0 {
        let (early, late) = if route == Route::Reversed {
            (right, left)
        } else {
            (left, right)
        };
        if route != Route::MissingOverflow && late.checked_add(extent).is_none() {
            return 2;
        }
        if late.wrapping_add(extent) > length || (route != Route::MissingOrder && early > late) {
            return 3;
        }
    }
    let right_extent = if route == Route::DifferentExtent {
        other
    } else {
        extent
    };
    let right_length = if route == Route::DifferentLength {
        other
    } else {
        length
    };
    u64::from(
        extent >= 8
            && left.checked_add(extent).is_some_and(|end| end <= length)
            && right
                .checked_add(right_extent)
                .is_some_and(|end| end <= right_length),
    )
}

#[test]
fn ordered_capture_spans_require_both_current_bounds_and_nonwrap() {
    let values = [0, 1, 7, 8, 9, 16, u64::MAX - 7, u64::MAX];
    let mut calls = 0;
    for route in [
        Route::Ordered,
        Route::Reversed,
        Route::MissingOrder,
        Route::MissingOverflow,
        Route::DifferentLength,
        Route::DifferentExtent,
        Route::AlternateEntry,
    ] {
        let (_module, call) = compile(route);
        for left in values {
            for right in values {
                for extent in values {
                    for length in values {
                        for other in values {
                            let actual = unsafe { call(left, right, extent, length, other) };
                            let expected = reference(route, [left, right, extent, length, other]);
                            assert_eq!(
                                actual, expected,
                                "{route:?} {left} {right} {extent} {length} {other}"
                            );
                            calls += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(calls, 229_376);
}
