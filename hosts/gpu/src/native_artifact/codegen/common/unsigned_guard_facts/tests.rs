use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn test_function(mode: usize, increment: u64) -> Context {
    let mut context = Context::new();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let range = b.create_block();
        let body = b.create_block();
        let success = b.create_block();
        let overflow = b.create_block();
        let outside = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let x = b.block_params(entry)[0];
        let end = b.block_params(entry)[1];
        let bypass = b.block_params(entry)[2];
        if mode == 3 {
            b.ins().brif(bypass, body, &[], range, &[]);
        } else {
            b.ins().jump(range, &[]);
        }
        b.switch_to_block(range);
        let cc = if mode == 2 {
            IntCC::SignedLessThan
        } else {
            IntCC::UnsignedLessThan
        };
        let less = b.ins().icmp(cc, x, end);
        let test = if mode == 1 {
            let full = b.ins().uextend(types::I64, less);
            let one = b.ins().iconst(types::I64, 1);
            b.ins().icmp(IntCC::Equal, full, one)
        } else {
            less
        };
        b.ins().brif(test, body, &[], outside, &[]);
        b.switch_to_block(body);
        let c = b.ins().iconst(types::I64, increment as i64);
        let sum = if mode == 4 {
            b.ins().iadd(c, x)
        } else {
            b.ins().iadd(x, c)
        };
        if mode == 4 {
            let bad = b.ins().icmp(IntCC::UnsignedLessThan, sum, x);
            b.ins().brif(bad, overflow, &[], success, &[]);
        } else {
            let good = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, sum, x);
            b.ins().brif(good, success, &[], overflow, &[]);
        }
        b.switch_to_block(success);
        b.ins().return_(&[sum]);
        b.switch_to_block(overflow);
        let failed = b.ins().iconst(types::I64, -2);
        b.ins().return_(&[failed]);
        b.switch_to_block(outside);
        let skipped = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[skipped]);
        b.seal_all_blocks();
        b.finalize();
    }
    context
}

#[test]
fn dominated_unsigned_add_preserves_extremes_signed_tests_and_shared_entries() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut entries = Vec::new();
    for mode in 0..5 {
        for increment in [1, 2, 1 << 63, u64::MAX] {
            let mut context = test_function(mode, increment);
            assert_eq!(
                optimize(&mut context),
                usize::from(mode != 2 && mode != 3 && increment == 1)
            );
            context.verify(module.isa()).unwrap();
            let id = module
                .declare_function(
                    &format!("guards_{mode}_{increment}"),
                    Linkage::Local,
                    &context.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            entries.push((mode, increment, id));
        }
    }
    module.finalize_definitions().unwrap();
    let values = [
        0,
        1,
        2,
        63,
        64,
        (1 << 63) - 1,
        1 << 63,
        u64::MAX - 2,
        u64::MAX - 1,
        u64::MAX,
    ];
    for (mode, increment, id) in entries {
        // SAFETY: module owns this live JIT entry with the exact declared ABI.
        let run: unsafe extern "C" fn(u64, u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for x in values {
            for end in values {
                for bypass in [0, 1, u64::MAX] {
                    let admitted = if mode == 3 && bypass != 0 {
                        true
                    } else if mode == 2 {
                        (x as i64) < (end as i64)
                    } else {
                        x < end
                    };
                    let expected = if admitted {
                        x.checked_add(increment).unwrap_or(u64::MAX - 1)
                    } else {
                        u64::MAX
                    };
                    assert_eq!(
                        unsafe { run(x, end, bypass) },
                        expected,
                        "mode={mode} x={x} end={end} add={increment} bypass={bypass}"
                    );
                }
            }
        }
    }
}

fn mask(b: &mut FunctionBuilder<'_>, value: Value, bits: u64, immediate: bool) -> Value {
    if immediate {
        b.ins().band_imm(value, bits as i64)
    } else {
        let c = b.ins().iconst(types::I64, bits as i64);
        b.ins().band(value, c)
    }
}
fn intrinsic_function(mode: usize, increment: u64, immediate: bool) -> Context {
    let mut context = Context::new();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let success = b.create_block();
        let overflow = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let x = b.block_params(entry)[0];
        let pointer = b.block_params(entry)[1];
        let base = match mode {
            0 => mask(&mut b, x, 255, immediate),
            1 => {
                if immediate {
                    b.ins().ushr_imm(x, 6)
                } else {
                    let c = b.ins().iconst(types::I64, 6);
                    b.ins().ushr(x, c)
                }
            }
            2 => {
                if immediate {
                    b.ins().udiv_imm(x, 64)
                } else {
                    let c = b.ins().iconst(types::I64, 64);
                    b.ins().udiv(x, c)
                }
            }
            3 => {
                let n = b.ins().ireduce(types::I8, x);
                b.ins().uextend(types::I64, n)
            }
            4 => b.ins().popcnt(x),
            5 => b.ins().clz(x),
            6 => b.ins().ctz(x),
            7 => b.ins().uload8(
                types::I64,
                cranelift_codegen::ir::MemFlags::new(),
                pointer,
                0,
            ),
            8 => b.ins().uload16(
                types::I64,
                cranelift_codegen::ir::MemFlags::new(),
                pointer,
                0,
            ),
            9 => b
                .ins()
                .uload32(cranelift_codegen::ir::MemFlags::new(), pointer, 0),
            10 => {
                let small = mask(&mut b, x, 63, immediate);
                let c = b.ins().iconst(types::I64, 255);
                let flag = b.ins().icmp_imm(IntCC::Equal, x, 0);
                b.ins().select(flag, small, c)
            }
            11 | 12 => {
                let small = mask(&mut b, x, 255, immediate);
                if immediate {
                    if mode == 11 {
                        b.ins().bor_imm(small, 255)
                    } else {
                        b.ins().bxor_imm(small, 255)
                    }
                } else {
                    let c = b.ins().iconst(types::I64, 255);
                    if mode == 11 {
                        b.ins().bor(small, c)
                    } else {
                        b.ins().bxor(small, c)
                    }
                }
            }
            13 => {
                let small = mask(&mut b, x, 255, immediate);
                if immediate {
                    b.ins().iadd_imm(small, 100)
                } else {
                    let c = b.ins().iconst(types::I64, 100);
                    b.ins().iadd(small, c)
                }
            }
            14 => x,
            15 => {
                let n = b.ins().ireduce(types::I8, x);
                b.ins().sextend(types::I64, n)
            }
            16 => {
                if immediate {
                    b.ins().sdiv_imm(x, 64)
                } else {
                    let c = b.ins().iconst(types::I64, 64);
                    b.ins().sdiv(x, c)
                }
            }
            17 => {
                if immediate {
                    b.ins().urem_imm(x, 64)
                } else {
                    let c = b.ins().iconst(types::I64, 64);
                    b.ins().urem(x, c)
                }
            }
            18 | 19 => {
                let value = if mode == 18 {
                    mask(&mut b, x, 255, immediate)
                } else {
                    x
                };
                if immediate {
                    b.ins().ishl_imm(value, 56)
                } else {
                    let c = b.ins().iconst(types::I64, 56);
                    b.ins().ishl(value, c)
                }
            }
            _ => unreachable!(),
        };
        let sum = if immediate {
            b.ins().iadd_imm(base, increment as i64)
        } else {
            let c = b.ins().iconst(types::I64, increment as i64);
            b.ins().iadd(base, c)
        };
        let good = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, sum, base);
        b.ins().brif(good, success, &[], overflow, &[]);
        b.switch_to_block(success);
        b.ins().return_(&[sum]);
        b.switch_to_block(overflow);
        let failed = b.ins().iconst(types::I64, -2);
        b.ins().return_(&[failed]);
        b.seal_all_blocks();
        b.finalize();
    }
    context
}
fn range_inputs() -> Vec<u64> {
    let mut inputs = vec![
        0,
        1,
        63,
        64,
        255,
        256,
        65535,
        65536,
        u32::MAX as u64,
        (1 << 63) - 1,
        1 << 63,
        u64::MAX - 1,
        u64::MAX,
    ];
    let mut seed = 0x243f6a8885a308d3u64;
    for _ in 0..128 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        inputs.push(seed);
    }
    inputs
}
#[test]
fn intrinsic_unsigned_ranges_preserve_extremes_random_inputs_and_real_narrow_loads() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let maxima = [
        255,
        u64::MAX >> 6,
        u64::MAX / 64,
        255,
        64,
        64,
        64,
        255,
        65535,
        u32::MAX as u64,
        255,
        255,
        255,
        355,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        63,
        0xff00000000000000,
        0xff00000000000000,
    ];
    let mut entries = Vec::new();
    for (mode, maximum) in maxima.into_iter().enumerate() {
        for immediate in [false, true] {
            for increment in [0, 1, 64, u64::MAX - 255, u64::MAX] {
                let mut context = intrinsic_function(mode, increment, immediate);
                assert_eq!(
                    optimize(&mut context),
                    usize::from(maximum <= u64::MAX - increment),
                    "mode={mode} add={increment} immediate={immediate}"
                );
                context.verify(module.isa()).unwrap();
                let id = module
                    .declare_function(
                        &format!("intrinsic_{mode}_{increment}_{immediate}"),
                        Linkage::Local,
                        &context.func.signature,
                    )
                    .unwrap();
                module.define_function(id, &mut context).unwrap();
                entries.push((mode, increment, id));
            }
        }
    }
    module.finalize_definitions().unwrap();
    for (mode, increment, id) in entries {
        // SAFETY: the module owns this exact two-word ABI entry; every narrow
        // load stays within eight live input bytes for the synchronous call.
        let run: unsafe extern "C" fn(u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for x in range_inputs() {
            let bytes = x.to_ne_bytes();
            let base = match mode {
                0 | 3 | 7 => x & 255,
                1 => x >> 6,
                2 => x / 64,
                4 => x.count_ones() as u64,
                5 => x.leading_zeros() as u64,
                6 => x.trailing_zeros() as u64,
                8 => x & 65535,
                9 => x & u32::MAX as u64,
                10 => {
                    if x == 0 {
                        x & 63
                    } else {
                        255
                    }
                }
                11 => (x & 255) | 255,
                12 => (x & 255) ^ 255,
                13 => (x & 255) + 100,
                14 => x,
                15 => (x as i8 as i64) as u64,
                16 => ((x as i64) / 64) as u64,
                17 => x % 64,
                18 => (x & 255) << 56,
                19 => x << 56,
                _ => unreachable!(),
            };
            assert_eq!(
                unsafe { run(x, bytes.as_ptr() as u64) },
                base.checked_add(increment).unwrap_or(u64::MAX - 1),
                "mode={mode} x={x} add={increment}"
            );
        }
    }
}
fn dynamic_function(mode: usize) -> Context {
    let mut context = Context::new();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let body = b.create_block();
        let success = b.create_block();
        let failed = b.create_block();
        let outside = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let x = b.block_params(entry)[0];
        let y = b.block_params(entry)[1];
        let condition = match mode {
            2 => Some(b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, x, -256)),
            3 => Some(b.ins().icmp_imm(IntCC::Equal, x, 0)),
            4 => Some(b.ins().icmp_imm(IntCC::SignedLessThanOrEqual, x, 0)),
            _ => None,
        };
        if let Some(condition) = condition {
            b.ins().brif(condition, body, &[], outside, &[]);
        } else {
            b.ins().jump(body, &[]);
        }
        b.switch_to_block(body);
        let base = if mode == 0 {
            b.ins().band_imm(x, 255)
        } else {
            x
        };
        let step = b.ins().band_imm(y, 255);
        let sum = b.ins().iadd(base, step);
        let good = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, sum, base);
        b.ins().brif(good, success, &[], failed, &[]);
        b.switch_to_block(success);
        b.ins().return_(&[sum]);
        b.switch_to_block(failed);
        let error = b.ins().iconst(types::I64, -2);
        b.ins().return_(&[error]);
        b.switch_to_block(outside);
        let skipped = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[skipped]);
        b.seal_all_blocks();
        b.finalize();
    }
    context
}
#[test]
fn dynamic_masked_addends_and_immediate_predicates_preserve_signed_and_overflow_failures() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut entries = Vec::new();
    for mode in 0..5 {
        let mut context = dynamic_function(mode);
        assert_eq!(
            optimize(&mut context),
            usize::from(matches!(mode, 0 | 2 | 3))
        );
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(
                &format!("dynamic_{mode}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        entries.push((mode, id));
    }
    module.finalize_definitions().unwrap();
    let inputs = range_inputs();
    for (mode, id) in entries {
        // SAFETY: exact live two-word JIT ABI owned by this module.
        let run: unsafe extern "C" fn(u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for &x in &inputs {
            for &y in &inputs {
                let admitted = match mode {
                    2 => x <= u64::MAX - 255,
                    3 => x == 0,
                    4 => (x as i64) <= 0,
                    _ => true,
                };
                let base = if mode == 0 { x & 255 } else { x };
                let expected = if admitted {
                    base.checked_add(y & 255).unwrap_or(u64::MAX - 1)
                } else {
                    u64::MAX
                };
                assert_eq!(unsafe { run(x, y) }, expected, "mode={mode} x={x} y={y}");
            }
        }
    }
}
