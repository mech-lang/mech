use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn product_function(mode: usize) -> Context {
    let mut c = Context::new();
    c.func.signature.params = vec![AbiParam::new(types::I64); 4];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let range = b.create_block();
        let body = b.create_block();
        let success = b.create_block();
        let outside = b.create_block();
        let overflow = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        b.ins().store(MemFlags::new(), args[0], args[3], 0);
        let a = b.ins().band_imm(args[0], 255);
        let y = b.ins().band_imm(args[1], 255);
        // Compute the pure wrapping difference before the optional shared
        // entry. Only a dominating successful unsigned test can bound it.
        let three = b.ins().iconst(types::I64, 3);
        let difference = b.ins().isub(a, three);
        if mode == 3 {
            b.ins().brif(args[2], body, &[], range, &[]);
        } else {
            b.ins().jump(range, &[]);
        }
        b.switch_to_block(range);
        if matches!(mode, 1..=3) {
            let cc = if mode == 2 {
                IntCC::SignedGreaterThanOrEqual
            } else {
                IntCC::UnsignedGreaterThanOrEqual
            };
            let admitted = b.ins().icmp(cc, a, three);
            b.ins().brif(admitted, body, &[], outside, &[]);
        } else {
            b.ins().jump(body, &[]);
        }
        b.switch_to_block(body);
        b.ins().store(MemFlags::new(), args[1], args[3], 8);
        let product = match mode {
            0 => b.ins().imul(a, y),
            1..=3 => b.ins().imul(difference, y),
            4 => {
                let a = b.ins().band_imm(args[0], i64::from(u32::MAX));
                let y = b.ins().band_imm(args[1], i64::from(u32::MAX));
                b.ins().imul(a, y)
            }
            5 => {
                let high = b.ins().umulhi(a, y);
                let valid = b.ins().icmp_imm(IntCC::Equal, high, 0);
                let low = b.ins().imul(a, y);
                b.ins().brif(valid, success, &[], overflow, &[]);
                b.switch_to_block(success);
                b.ins().store(MemFlags::new(), low, args[3], 16);
                b.ins().return_(&[low]);
                b.switch_to_block(outside);
                let error = b.ins().iconst(types::I64, -1);
                b.ins().return_(&[error]);
                b.switch_to_block(overflow);
                let error = b.ins().iconst(types::I64, -2);
                b.ins().return_(&[error]);
                b.seal_all_blocks();
                b.finalize();
                return c;
            }
            6 => {
                let a = b.ins().band_imm(args[0], 65535);
                let y = b.ins().band_imm(args[1], 65535);
                let a = b.ins().ireduce(types::I32, a);
                let y = b.ins().ireduce(types::I32, y);
                let product = b.ins().imul(a, y);
                let divided = b.ins().udiv_imm(product, 6);
                let wide = b.ins().uextend(types::I64, divided);
                b.ins().iadd_imm(wide, 255)
            }
            7 => {
                let a = b.ins().ireduce(types::I8, args[0]);
                let y = b.ins().ireduce(types::I8, args[1]);
                let product = b.ins().imul(a, y);
                b.ins().uextend(types::I64, product)
            }
            _ => unreachable!(),
        };
        let valid = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, product, i64::from(u32::MAX));
        b.ins().brif(valid, success, &[], overflow, &[]);
        b.switch_to_block(success);
        b.ins().store(MemFlags::new(), product, args[3], 16);
        b.ins().return_(&[product]);
        for (block, word) in [(outside, -1), (overflow, -2)] {
            b.switch_to_block(block);
            let error = b.ins().iconst(types::I64, word);
            b.ins().return_(&[error]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    c
}

#[test]
fn arithmetic_intervals_preserve_checked_products_effects_and_shared_or_signed_entries() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut entries = Vec::new();
    for mode in 0..8 {
        for enabled in [false, true] {
            let mut c = product_function(mode);
            if enabled {
                assert_eq!(
                    optimize(&mut c),
                    usize::from(matches!(mode, 0 | 1 | 5 | 6 | 7)),
                    "mode={mode}"
                );
            }
            c.verify(module.isa()).unwrap();
            let id = module
                .declare_function(
                    &format!("product_{mode}_{enabled}"),
                    Linkage::Local,
                    &c.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut c).unwrap();
            entries.push((mode, id));
        }
    }
    module.finalize_definitions().unwrap();
    let mut values = vec![
        0,
        1,
        2,
        3,
        4,
        127,
        128,
        254,
        255,
        256,
        65535,
        65536,
        u32::MAX as u64,
        1u64 << 63,
        u64::MAX,
    ];
    let mut seed = 0x243f6a8885a308d3u64;
    for _ in 0..32 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        values.push(seed);
    }
    for (mode, id) in entries {
        // SAFETY: module owns the live entry with exactly these four words;
        // stores are bounded to the three live synchronous output words.
        let run: unsafe extern "C" fn(u64, u64, u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for &x in &values {
            for &y in &values {
                for bypass in [0, 1, u64::MAX] {
                    let a = x & 255;
                    let admitted = !matches!(mode, 1..=3) || (mode == 3 && bypass != 0) || a >= 3;
                    let product = match mode {
                        0 | 5 => a * (y & 255),
                        1..=3 => a.wrapping_sub(3).wrapping_mul(y & 255),
                        4 => (x & u32::MAX as u64) * (y & u32::MAX as u64),
                        6 => ((x & 65535) * (y & 65535)) / 6 + 255,
                        7 => (x as u8).wrapping_mul(y as u8) as u64,
                        _ => unreachable!(),
                    };
                    let valid = product <= u32::MAX as u64;
                    let expected = if !admitted {
                        u64::MAX
                    } else if !valid {
                        u64::MAX - 1
                    } else {
                        product
                    };
                    let mut effects = [0xfeedu64; 3];
                    assert_eq!(
                        unsafe { run(x, y, bypass, effects.as_mut_ptr() as u64) },
                        expected,
                        "mode={mode} x={x} y={y} bypass={bypass}"
                    );
                    assert_eq!(
                        effects,
                        [
                            x,
                            if admitted { y } else { 0xfeed },
                            if admitted && valid { product } else { 0xfeed }
                        ]
                    );
                }
            }
        }
    }
}

#[test]
fn narrow_wrapping_arithmetic_does_not_supply_a_false_nonzero_proof() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut entries = Vec::new();
    for bits in [8, 16, 32, 64] {
        for multiply in [false, true] {
            let mut c = Context::new();
            c.func.signature.params = vec![AbiParam::new(types::I64)];
            c.func.signature.returns = vec![AbiParam::new(types::I64)];
            let mut fc = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
                let entry = b.create_block();
                let success = b.create_block();
                let zero = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.switch_to_block(entry);
                let input = b.block_params(entry)[0];
                let ty = match bits {
                    8 => types::I8,
                    16 => types::I16,
                    32 => types::I32,
                    _ => types::I64,
                };
                let input = if bits == 64 {
                    input
                } else {
                    b.ins().ireduce(ty, input)
                };
                let value = if multiply {
                    b.ins().imul_imm(input, 2)
                } else {
                    b.ins().iadd_imm(input, 1)
                };
                let wide = if bits == 64 {
                    value
                } else {
                    b.ins().uextend(types::I64, value)
                };
                let valid = b.ins().icmp_imm(IntCC::NotEqual, wide, 0);
                b.ins().brif(valid, success, &[], zero, &[]);
                b.switch_to_block(success);
                b.ins().return_(&[wide]);
                b.switch_to_block(zero);
                let error = b.ins().iconst(types::I64, -1);
                b.ins().return_(&[error]);
                b.seal_all_blocks();
                b.finalize();
            }
            assert_eq!(optimize(&mut c), 0);
            c.verify(module.isa()).unwrap();
            let id = module
                .declare_function(
                    &format!("wrapping_{bits}_{multiply}"),
                    Linkage::Local,
                    &c.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut c).unwrap();
            entries.push((bits, multiply, id));
        }
    }
    module.finalize_definitions().unwrap();
    for (bits, multiply, id) in entries {
        let mask = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        // SAFETY: exact one-word ABI owned by the live module.
        let run: unsafe extern "C" fn(u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for x in [
            0,
            1,
            127,
            128,
            255,
            256,
            65535,
            65536,
            u32::MAX as u64,
            1u64 << 63,
            u64::MAX,
        ] {
            let word = if multiply {
                x.wrapping_mul(2)
            } else {
                x.wrapping_add(1)
            } & mask;
            assert_eq!(unsafe { run(x) }, if word == 0 { u64::MAX } else { word });
        }
    }
}
