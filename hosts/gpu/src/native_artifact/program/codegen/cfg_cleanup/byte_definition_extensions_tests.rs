use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn module() -> JITModule {
    JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    )
}

const CONDITIONS: [IntCC; 10] = [
    IntCC::Equal,
    IntCC::NotEqual,
    IntCC::SignedLessThan,
    IntCC::SignedLessThanOrEqual,
    IntCC::SignedGreaterThan,
    IntCC::SignedGreaterThanOrEqual,
    IntCC::UnsignedLessThan,
    IntCC::UnsignedLessThanOrEqual,
    IntCC::UnsignedGreaterThan,
    IntCC::UnsignedGreaterThanOrEqual,
];

#[test]
fn byte_definition_extensions_preserve_shared_values_and_dirty_upper_bits() {
    type Call = unsafe extern "C" fn(u64, u64, u64, *const u8, *mut u64) -> u64;
    let mut module = module();
    let mut definitions = Vec::new();
    for kind in 0..45 {
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 5];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut fc = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [a, rhs, salt, input, output] = b.block_params(entry).try_into().unwrap();
        let x = b.ins().ireduce(types::I8, a);
        let y = b.ins().ireduce(types::I8, rhs);
        let value = if kind < 40 {
            let ty = [types::I8, types::I16, types::I32, types::I64][kind / 10];
            let lhs = if ty == types::I64 {
                a
            } else {
                b.ins().ireduce(ty, a)
            };
            let rhs = if ty == types::I64 {
                rhs
            } else {
                b.ins().ireduce(ty, rhs)
            };
            b.ins().icmp(CONDITIONS[kind % 10], lhs, rhs)
        } else {
            match kind {
                40 => b.ins().load(types::I8, MemFlags::new(), input, 0),
                41 => x,
                42 => {
                    let sum = b.ins().iadd(a, rhs);
                    b.ins().ireduce(types::I8, sum)
                }
                43 => {
                    let left = b.create_block();
                    let right = b.create_block();
                    let join = b.create_block();
                    let joined = b.append_block_param(join, types::I8);
                    let choice = b.ins().band_imm(salt, 1);
                    b.ins().brif(choice, left, &[], right, &[]);
                    b.switch_to_block(left);
                    b.ins().jump(join, &[x.into()]);
                    b.switch_to_block(right);
                    b.ins().jump(join, &[y.into()]);
                    b.switch_to_block(join);
                    joined
                }
                44 => {
                    let signed = b.ins().sload8(types::I32, MemFlags::new(), input, 0);
                    b.ins().ireduce(types::I8, signed)
                }
                _ => unreachable!(),
            }
        };
        let wide = b.ins().uextend(types::I64, value);
        let signed = b.ins().sextend(types::I64, value);
        let salt8 = b.ins().ireduce(types::I8, salt);
        let shared = b.ins().bxor(value, salt8);
        let shared = b.ins().uextend(types::I64, shared);
        let high = b.ins().ishl_imm(salt, 8);
        let packed = b.ins().bor(wide, high);
        for (at, value) in [wide, signed, shared, packed].into_iter().enumerate() {
            b.ins()
                .store(MemFlags::new(), value, output, (at * 8) as i32);
        }
        // This low-byte use keeps shared loads and compares observable too.
        b.ins().store(MemFlags::new(), value, output, 32);
        let result = b.ins().bxor(wide, a);
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(
                &format!("byte_definition_extensions_{kind}"),
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
        0x7f,
        0x80,
        0xff,
        0x100,
        0x7fff,
        0x8000,
        0xffffffff,
        u64::MAX,
        1 << 63,
        0xfedcba9800000001,
    ];
    let mut calls = 0;
    let mut seed = 0x61bfe3d9a28c0475u64;
    for (kind, id) in definitions {
        // Declared ABI, JIT module, initialized input and output all stay live.
        let call: Call = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let mut check = |a: u64, rhs: u64, salt: u64| {
            let value = if kind < 40 {
                let bits = [8, 16, 32, 64][kind / 10];
                let mask = u64::MAX >> (64 - bits);
                let (x, y) = (a & mask, rhs & mask);
                let sx = ((x << (64 - bits)) as i64) >> (64 - bits);
                let sy = ((y << (64 - bits)) as i64) >> (64 - bits);
                u8::from(match CONDITIONS[kind % 10] {
                    IntCC::Equal => x == y,
                    IntCC::NotEqual => x != y,
                    IntCC::SignedLessThan => sx < sy,
                    IntCC::SignedLessThanOrEqual => sx <= sy,
                    IntCC::SignedGreaterThan => sx > sy,
                    IntCC::SignedGreaterThanOrEqual => sx >= sy,
                    IntCC::UnsignedLessThan => x < y,
                    IntCC::UnsignedLessThanOrEqual => x <= y,
                    IntCC::UnsignedGreaterThan => x > y,
                    IntCC::UnsignedGreaterThanOrEqual => x >= y,
                })
            } else {
                match kind {
                    40 | 41 | 44 => a as u8,
                    42 => a.wrapping_add(rhs) as u8,
                    43 => (if salt & 1 != 0 { a } else { rhs }) as u8,
                    _ => unreachable!(),
                }
            };
            let expected = [
                u64::from(value),
                (value as i8 as i64) as u64,
                u64::from(value ^ salt as u8),
                u64::from(value) | salt.wrapping_shl(8),
            ];
            let input = [a as u8];
            let mut output = [0xa5a5a5a5a5a5a5a5; 5];
            let got = unsafe { call(a, rhs, salt, input.as_ptr(), output.as_mut_ptr()) };
            assert_eq!(&output[..4], &expected, "kind={kind} a={a:#x} rhs={rhs:#x}");
            assert_eq!(output[4], 0xa5a5a5a5a5a5a500 | u64::from(value));
            assert_eq!(got, u64::from(value) ^ a);
            calls += 1;
        };
        for a in edges {
            for rhs in edges {
                for salt in [0, 1, 0xff, u64::MAX] {
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
    eprintln!("byte definition extension differential native calls: {calls}");
}

#[test]
fn byte_definition_extensions_remove_only_the_proven_compare_extension() {
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
    let value = b.ins().icmp(IntCC::UnsignedLessThan, a, rhs);
    let wide = b.ins().uextend(types::I64, value);
    b.ins().return_(&[wide]);
    b.seal_all_blocks();
    b.finalize();
    let id = module
        .declare_function(
            "byte_extension_probe",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let bytes = context.compiled_code().unwrap().buffer.data();
    let byte_extensions = bytes
        .chunks_exact(4)
        .filter(|bytes| u32::from_le_bytes((*bytes).try_into().unwrap()) & 0xfffffc00 == 0x53001c00)
        .count();
    let enabled = std::env::var("MECH_ARM64_REUSE_I8_DEFINITIONS")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "all" | "compare"));
    assert_eq!(byte_extensions, usize::from(!enabled));
    eprintln!(
        "byte definition extension compare probe bytes: {} byte extensions: {byte_extensions}",
        bytes.len()
    );
}
