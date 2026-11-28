use super::*;
use cranelift_codegen::ir::{AbiParam, Function};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
type Kernel = unsafe extern "C" fn(*const f64, *const f64, *mut u64, u64, u64, u64);
fn function(pairs: bool, quartets: bool) -> Function {
    let mut f = Function::new();
    f.signature.params = vec![AbiParam::new(types::I64); 6];
    let mut context = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut f, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let done = b.create_block();
        emit_body(
            &mut b, args[0], args[1], args[2], args[3], args[4], args[5], None, pairs, quartets,
            true, done,
        );
        b.switch_to_block(done);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    f
}
#[test]
fn numeric_product_views_dynamic_shared_kernel_ordered_rows_columns_empty_null_and_canaries() {
    for (pairs, quartets) in [(false, false), (true, false), (true, true)] {
        let mut module = cranelift_jit::JITModule::new(
            cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")],
                default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.func = function(pairs, quartets);
        context.func.signature.call_conv = module.make_signature().call_conv;
        context.set_disasm(true);
        let id = module
            .declare_function(
                "transposed_product",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        let assembly = context.compiled_code().unwrap().vcode.as_ref().unwrap();
        assert!(
            !assembly.contains("fmadd") && !assembly.contains("fmla"),
            "{assembly}"
        );
        module.finalize_definitions().unwrap();
        // SAFETY: exact six-word signature; the retained module owns the code.
        let kernel =
            unsafe { core::mem::transmute::<*const u8, Kernel>(module.get_finalized_function(id)) };
        for rows in [0, 1, 2, 3, 4, 5, 7] {
            for inner in [0, 1, 2, 3, 5, 9] {
                for columns in [0, 1, 2, 3, 5, 8] {
                    let pattern = [1e16, 1.0, -1e16, -0.0, 0.0, f64::from_bits(1), 2.5, -3.25];
                    let left = (0..inner * rows)
                        .map(|i| pattern[i % pattern.len()])
                        .collect::<Vec<_>>();
                    let right: Vec<f64> = (0..inner * columns)
                        .map(|i| if i % 3 == 0 { -1.0 } else { 1.0 })
                        .collect::<Vec<_>>();
                    let left_before = left.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
                    let right_before = right.iter().map(|v| v.to_bits()).collect::<Vec<_>>();
                    let sentinel = 0xbad0_bad0_bad0_bad0u64;
                    let mut out = vec![sentinel; rows * columns + 2];
                    let lp = if left.is_empty() {
                        core::ptr::null()
                    } else {
                        left.as_ptr()
                    };
                    let rp = if right.is_empty() {
                        core::ptr::null()
                    } else {
                        right.as_ptr()
                    };
                    // SAFETY: physical left is KxM, right KxN; every used bank
                    // and output rectangle is disjoint and exactly admitted.
                    unsafe {
                        kernel(
                            lp,
                            rp,
                            out.as_mut_ptr().add(1),
                            (rows * columns) as u64,
                            columns as u64,
                            inner as u64,
                        )
                    };
                    assert_eq!(out[0], sentinel);
                    assert_eq!(out[rows * columns + 1], sentinel);
                    for r in 0..rows {
                        for c in 0..columns {
                            let mut sum = 0.0f64;
                            for k in 0..inner {
                                let term = std::hint::black_box(
                                    left[k * rows + r] * right[k * columns + c],
                                );
                                sum = std::hint::black_box(sum + term);
                            }
                            assert_eq!(
                                out[1 + r * columns + c],
                                sum.to_bits(),
                                "rows{rows} inner{inner} columns{columns} tile{pairs}/{quartets} ({r},{c})"
                            );
                        }
                    }
                    assert_eq!(
                        left.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                        left_before
                    );
                    assert_eq!(
                        right.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                        right_before
                    );
                }
            }
        }
        // SAFETY: count0 must return without dividing by zero or using pointers.
        unsafe {
            kernel(
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null_mut(),
                0,
                0,
                u64::MAX,
            )
        };
    }
}
