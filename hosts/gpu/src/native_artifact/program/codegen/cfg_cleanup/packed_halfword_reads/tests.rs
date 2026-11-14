use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Case {
    Exact,
    Offset,
    TooShort,
    DifferentBase,
    InterveningRead,
    InterveningWrite,
    AlternateEntry,
    Nonadjacent,
    WrongStoreValue,
    WrongMemoryFlags,
    NoFloor,
}

type Entry = unsafe extern "C" fn(*mut u16, *mut u16, u64, u64, u64) -> u64;

fn scaled(b: &mut FunctionBuilder<'_>, base: Value, index: Value, bias: i64) -> Value {
    let one = b.ins().iadd_imm(index, bias + 1);
    let zero = b.ins().iadd_imm(one, -1);
    let bytes = b.ins().imul_imm(zero, 2);
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
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 5];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let table;
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fc);
        let entry = b.create_block();
        let first = b.create_block();
        let second = b.create_block();
        let invalid = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [a, other, input, length, replacement] = b.block_params(entry).try_into().unwrap();
        table = a;
        let enough = b.ins().icmp_imm(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            length,
            8,
        );
        b.ins().brif(enough, first, &[], invalid, &[]);
        b.switch_to_block(invalid);
        let invalid_result = b.ins().iconst(types::I64, -1);
        b.ins().return_(&[invalid_result]);
        b.switch_to_block(first);
        let mask = b.ins().band_imm(input, 3);
        let stride = b.ins().iconst(types::I64, 2);
        let index = b.ins().imul(mask, stride);
        let lo = scaled(&mut b, a, index, 0);
        let flags = MemFlags::new();
        let low = b.ins().uload16(types::I64, flags, lo, 0);
        let extra = if case == Case::InterveningRead {
            b.ins().uload16(types::I64, flags, other, 0)
        } else {
            b.ins().iconst(types::I64, 0)
        };
        let replacement = b.ins().ireduce(types::I16, replacement);
        if case == Case::InterveningWrite {
            b.ins().store(flags, replacement, lo, 2);
        }
        if case == Case::AlternateEntry {
            let alternate = b.create_block();
            let pick = b
                .ins()
                .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, input, 0);
            b.ins().brif(pick, second, &[], alternate, &[]);
            b.switch_to_block(alternate);
            b.ins().jump(second, &[]);
        } else {
            b.ins().jump(second, &[]);
        }
        b.switch_to_block(second);
        let high_base = if case == Case::DifferentBase {
            other
        } else {
            a
        };
        let high_bias = if case == Case::Nonadjacent {
            2
        } else if case == Case::Offset {
            0
        } else {
            1
        };
        let high_disp = if case == Case::Offset { 2 } else { 0 };
        let hi = scaled(&mut b, high_base, index, high_bias);
        let high_flags = if case == Case::WrongMemoryFlags {
            flags.with_notrap()
        } else {
            flags
        };
        let high = b.ins().uload16(types::I64, high_flags, hi, high_disp);
        b.ins().store(flags, replacement, lo, 0);
        let update = b.ins().ireduce(
            types::I16,
            if case == Case::WrongStoreValue {
                high
            } else {
                low
            },
        );
        b.ins().store(flags, update, hi, high_disp);
        let shifted = b.ins().ishl_imm(high, 16);
        let pair = b.ins().bor(low, shifted);
        let extra = b.ins().ishl_imm(extra, 32);
        let result = b.ins().bor(pair, extra);
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
    }
    let bytes = match case {
        Case::TooShort => 14,
        Case::NoFloor => 0,
        _ => 16,
    };
    ctx.compute_cfg();
    ctx.compute_domtree();
    if optimize {
        let floors = if bytes == 0 {
            BTreeMap::new()
        } else {
            BTreeMap::from([(table, bytes)])
        };
        let changed = fold(&mut ctx.func, &ctx.cfg, &ctx.domtree, &floors);
        assert_eq!(
            changed,
            usize::from(matches!(case, Case::Exact | Case::Offset)),
            "{case:?}"
        );
    }
    ctx.verify(module.isa()).unwrap();
    let id = module
        .declare_function("halfwords", Linkage::Local, &ctx.func.signature)
        .unwrap();
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    // Each invocation passes two live initialized arrays large enough for the
    // inspected path, an admitted logical length and a scalar replacement.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}

#[test]
fn private_packed_halfword_reads_keep_both_values_stores_and_recovery() {
    let mut calls = 0;
    for kind in [
        Case::Exact,
        Case::Offset,
        Case::TooShort,
        Case::DifferentBase,
        Case::InterveningRead,
        Case::InterveningWrite,
        Case::AlternateEntry,
        Case::Nonadjacent,
        Case::WrongStoreValue,
        Case::WrongMemoryFlags,
        Case::NoFloor,
    ] {
        let (_before, a) = emit(kind, false);
        let (_after, b) = emit(kind, true);
        for index in [0u64, 1, 2, 3, 4, u64::MAX, u64::MAX - 1] {
            for seed in [0u16, 1, 17, 32767, 32768, 65534, 65535] {
                for shift in [0usize, 1] {
                    let old: [u16; 11] =
                        core::array::from_fn(|i| seed.wrapping_add((i as u16).wrapping_mul(127)));
                    let other: [u16; 11] = core::array::from_fn(|i| seed.rotate_left(i as u32));
                    let mut left = old;
                    let mut right = old;
                    let mut l2 = other;
                    let mut r2 = other;
                    let old_left = left;
                    let old_right = right;
                    for (len, replacement) in [(8, 65535), (7, 0), (8, 1)] {
                        let before = unsafe {
                            a(
                                left.as_mut_ptr().add(shift),
                                l2.as_mut_ptr().add(shift),
                                index,
                                len,
                                replacement,
                            )
                        };
                        let after = unsafe {
                            b(
                                right.as_mut_ptr().add(shift),
                                r2.as_mut_ptr().add(shift),
                                index,
                                len,
                                replacement,
                            )
                        };
                        assert_eq!((left, l2, before), (right, r2, after), "{kind:?}");
                        if len == 7 {
                            assert_eq!(after, u64::MAX);
                        }
                        if len == 8
                            && replacement == 65535
                            && matches!(kind, Case::Exact | Case::Offset)
                        {
                            let slot = shift + ((index & 3) as usize) * 2;
                            assert_eq!(before, old[slot] as u64 | ((old[slot + 1] as u64) << 16));
                            assert_eq!((left[slot], left[slot + 1]), (65535, old[slot]));
                        }
                        calls += 2;
                    }
                    assert_eq!(left[0..shift], old_left[0..shift]);
                    assert_eq!(right[0..shift], old_right[0..shift]);
                }
            }
        }
    }
    assert_eq!(calls, 6468);
    println!("{calls} paired compiled invocations");
}
