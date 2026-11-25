use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{Linkage, Module};

fn module() -> JITModule {
    crate::native_artifact::codegen::backend::jit(&[
        ("opt_level", "speed"),
        ("regalloc_checker", "true"),
    ])
    .unwrap()
}

#[test]
fn arithmetic_word_extensions_keep_wrapping_signed_shared_and_dirty_inputs() {
    type Call = unsafe extern "C" fn(u64, u64, u64, *const u8, *mut u64) -> u64;
    let mut module = module();
    let mut definitions = Vec::new();
    for kind in 0..13 {
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 5];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut fc = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [a, rhs, salt, input, output] = b.block_params(entry).try_into().unwrap();
        let x = b.ins().ireduce(types::I32, a);
        let y = b.ins().ireduce(types::I32, rhs);
        let z = b.ins().ireduce(types::I32, salt);
        let value = match kind {
            0 => b.ins().iadd(x, y),
            1 => b.ins().isub(x, y),
            2 => b.ins().imul(x, y),
            3 => b.ins().iadd_imm(x, 4095),
            4 => b.ins().iadd_imm(x, -4095),
            5 => {
                let shifted = b.ins().ishl_imm(y, 5);
                b.ins().iadd(x, shifted)
            }
            6 => {
                let product = b.ins().imul(y, z);
                b.ins().iadd(x, product)
            }
            7 => {
                let product = b.ins().imul(y, z);
                b.ins().isub(x, product)
            }
            8 => x,
            9 => {
                let left = b.create_block();
                let right = b.create_block();
                let join = b.create_block();
                let joined = b.append_block_param(join, types::I32);
                let choice = b.ins().band_imm(a, 1);
                b.ins().brif(choice, left, &[], right, &[]);
                b.switch_to_block(left);
                b.ins().jump(join, &[x.into()]);
                b.switch_to_block(right);
                b.ins().jump(join, &[y.into()]);
                b.switch_to_block(join);
                joined
            }
            10 => b.ins().sload8(types::I32, MemFlags::new(), input, 0),
            11 => {
                let sum = b.ins().iadd(a, rhs);
                b.ins().ireduce(types::I32, sum)
            }
            12 => b.ins().iconst(types::I32, -1),
            _ => unreachable!(),
        };
        let wide = b.ins().uextend(types::I64, value);
        let signed = b.ins().sextend(types::I64, value);
        let high = b.ins().ishl_imm(salt, 32);
        let packed = b.ins().bor(wide, high);
        let mixed = b.ins().bxor(value, z);
        let mixed = b.ins().uextend(types::I64, mixed);
        for (at, value) in [wide, signed, packed, mixed].into_iter().enumerate() {
            b.ins()
                .store(MemFlags::new(), value, output, (at * 8) as i32);
        }
        let result = b.ins().bxor(wide, a);
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(
                &format!("arithmetic_word_extensions_{kind}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        definitions.push((kind, id));
    }
    module.finalize_definitions().unwrap();
    let edges = [
        0,
        1,
        2,
        3,
        0x7fffffff,
        0x80000000,
        0xffffffff,
        0x100000000,
        u64::MAX,
        0xfedcba9800000001,
        1 << 63,
        0xaaaaaaaa55555555,
    ];
    let mut calls = 0;
    let mut seed = 0x8ce4a187d035b62fu64;
    for (kind, id) in definitions {
        // The exact declared ABI is three words, input/output pointers -> word.
        // The module and initialized input/output storage remain live.
        let call: Call = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let mut check = |a: u64, rhs: u64, salt: u64| {
            let x = a as u32;
            let y = rhs as u32;
            let z = salt as u32;
            let value = match kind {
                0 | 11 => x.wrapping_add(y),
                1 => x.wrapping_sub(y),
                2 => x.wrapping_mul(y),
                3 => x.wrapping_add(4095),
                4 => x.wrapping_sub(4095),
                5 => x.wrapping_add(y.wrapping_shl(5)),
                6 => x.wrapping_add(y.wrapping_mul(z)),
                7 => x.wrapping_sub(y.wrapping_mul(z)),
                8 => x,
                9 => {
                    if a & 1 != 0 {
                        x
                    } else {
                        y
                    }
                }
                10 => (a as u8 as i8 as i32) as u32,
                12 => u32::MAX,
                _ => unreachable!(),
            };
            let expected = [
                u64::from(value),
                (value as i32 as i64) as u64,
                u64::from(value) | salt.wrapping_shl(32),
                u64::from(value ^ z),
            ];
            let input = [a as u8];
            let mut output = [0xa5a5a5a5a5a5a5a5; 4];
            let got = unsafe { call(a, rhs, salt, input.as_ptr(), output.as_mut_ptr()) };
            assert_eq!(
                output, expected,
                "kind={kind} a={a:#x} rhs={rhs:#x} salt={salt:#x}"
            );
            assert_eq!(got, u64::from(value) ^ a);
            calls += 1;
        };
        for a in edges {
            for rhs in edges {
                for salt in [0, 1, u32::MAX as u64, u64::MAX] {
                    check(a, rhs, salt);
                }
            }
        }
        for _ in 0..512 {
            let mut word = || {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                seed
            };
            check(word(), word(), word());
        }
    }
    eprintln!("arithmetic word extension differential native calls: {calls}");
}

#[test]
fn arithmetic_word_extensions_remove_only_the_proven_add_extension_move() {
    let mut module = module();
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [a, rhs] = b.block_params(entry).try_into().unwrap();
    let x = b.ins().ireduce(types::I32, a);
    let y = b.ins().ireduce(types::I32, rhs);
    let sum = b.ins().iadd(x, y);
    let wide = b.ins().uextend(types::I64, sum);
    b.ins().return_(&[wide]);
    b.seal_all_blocks();
    b.finalize();
    let id = module
        .declare_function(
            "arithmetic_word_extension_probe",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let bytes = context.compiled_code().unwrap().buffer.data();
    let word_moves = bytes
        .chunks_exact(4)
        .filter(|bytes| {
            let word = u32::from_le_bytes((*bytes).try_into().unwrap());
            word & 0xffe0ffe0 == 0x2a0003e0
        })
        .count();
    let enabled = std::env::var("MECH_ARM64_REUSE_I32_ARITHMETIC")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "all" | "add"));
    assert_eq!(word_moves, usize::from(!enabled));
    eprintln!(
        "arithmetic word extension add probe bytes: {} word moves: {word_moves}",
        bytes.len()
    );
}
