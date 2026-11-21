//! Machine-executed witnesses for shared canonical comparison results.
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Type, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Run = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;
const CCS: [IntCC; 10] = [
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
const WORDS: [u64; 16] = [
    0,
    1,
    2,
    0x7f,
    0x80,
    0xff,
    0x7fff,
    0x8000,
    0xffff,
    0x7fff_ffff,
    0x8000_0000,
    0xffff_ffff,
    1 << 63,
    u64::MAX - 1,
    u64::MAX,
    0x1234_5678_9abc_def0,
];

#[derive(Clone, Copy)]
enum Shape {
    Ordinary,
    AcrossBlocks,
    InterveningStore,
    SharedLeaf,
    RawWords,
}

fn comparison(cc: IntCC, a: u64, b: u64, ty: Type) -> bool {
    let shift = 64 - ty.bits();
    let mask = u64::MAX >> shift;
    let (a, b) = (a & mask, b & mask);
    let signed = |v: u64| ((v << shift) as i64) >> shift;
    match cc {
        IntCC::Equal => a == b,
        IntCC::NotEqual => a != b,
        IntCC::SignedLessThan => signed(a) < signed(b),
        IntCC::SignedLessThanOrEqual => signed(a) <= signed(b),
        IntCC::SignedGreaterThan => signed(a) > signed(b),
        IntCC::SignedGreaterThanOrEqual => signed(a) >= signed(b),
        IntCC::UnsignedLessThan => a < b,
        IntCC::UnsignedLessThanOrEqual => a <= b,
        IntCC::UnsignedGreaterThan => a > b,
        IntCC::UnsignedGreaterThanOrEqual => a >= b,
    }
}

fn check(tys: [Type; 2], cc: [IntCC; 2], shape: Shape) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut context = module.make_context();
    context.set_disasm(true);
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut storage = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut storage);
        let entry = b.create_block();
        let middle = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [x, y, u, v, seed] = b.block_params(entry).try_into().unwrap();
        let narrow = |b: &mut FunctionBuilder<'_>, value, ty| {
            if ty == types::I64 {
                value
            } else {
                b.ins().ireduce(ty, value)
            }
        };
        let original_x = if matches!(shape, Shape::InterveningStore) {
            b.ins().load(types::I64, MemFlags::new(), x, 0)
        } else {
            x
        };
        let a = narrow(&mut b, original_x, tys[0]);
        let c = narrow(&mut b, y, tys[0]);
        let first = b.ins().icmp(cc[0], a, c);
        let first = b.ins().uextend(types::I64, first);
        if matches!(shape, Shape::AcrossBlocks | Shape::InterveningStore) {
            b.ins().jump(middle, &[]);
            b.switch_to_block(middle);
        }
        if matches!(shape, Shape::InterveningStore) {
            // The comparison must keep its already loaded operand when a
            // later block overwrites the original address independently.
            b.ins().store(MemFlags::new(), v, x, 0);
        }
        let a = narrow(&mut b, u, tys[1]);
        let c = narrow(&mut b, v, tys[1]);
        let second = b.ins().icmp(cc[1], a, c);
        let second = b.ins().uextend(types::I64, second);
        let root = if matches!(shape, Shape::RawWords) {
            b.ins().bor(x, u)
        } else {
            b.ins().bor(first, second)
        };
        // The result controls a branch and is also returned numerically.
        b.ins().brif(root, yes, &[], no, &[]);
        for (block, offset, shift) in [(yes, 17, 8), (no, -29, 16)] {
            b.switch_to_block(block);
            let shifted = b.ins().ishl_imm(root, shift);
            let result = b.ins().iadd(seed, shifted);
            let mut result = b.ins().iadd_imm(result, offset);
            if matches!(shape, Shape::SharedLeaf) {
                let extra = b.ins().ishl_imm(first, 10);
                result = b.ins().iadd(result, extra);
            }
            b.ins().return_(&[result]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    let id = module
        .declare_function("shared_boolean", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let code = context.compiled_code().unwrap().vcode.as_ref().unwrap();
    if std::env::var_os("MECH_ARM64_CCMP_SHARED_BOOLEAN").is_some() {
        let expected = usize::from(matches!(
            shape,
            Shape::Ordinary | Shape::AcrossBlocks | Shape::InterveningStore
        ));
        // The diagnostic also includes a second copy before allocation.
        let assembly = code.split(";; REGALLOC_INPUT_BEGIN").next().unwrap();
        assert_eq!(assembly.matches("ccmp").count(), expected, "{code}");
        if expected != 0 {
            assert_eq!(assembly.matches("cset").count(), 1, "{code}");
        }
    }
    module.finalize_definitions().unwrap();
    let run: Run = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    for &x in &WORDS {
        for &y in &WORDS {
            let u = if matches!(shape, Shape::RawWords) {
                x.rotate_left(13)
            } else {
                x.rotate_left(13) ^ 0x80
            };
            let v = y.wrapping_sub(1);
            let first = comparison(cc[0], x, y, tys[0]);
            let root = if matches!(shape, Shape::RawWords) {
                x | u
            } else {
                u64::from(first || comparison(cc[1], u, v, tys[1]))
            };
            let seed = x.wrapping_mul(0x1234_5678_9abc_def1);
            let (offset, shift) = if root != 0 {
                (17u64, 8)
            } else {
                (29u64.wrapping_neg(), 16)
            };
            let mut expected = seed
                .wrapping_add(root.wrapping_shl(shift))
                .wrapping_add(offset);
            if matches!(shape, Shape::SharedLeaf) {
                expected = expected.wrapping_add(u64::from(first) << 10);
            }
            let mut cell = x;
            let argument = if matches!(shape, Shape::InterveningStore) {
                std::ptr::addr_of_mut!(cell) as u64
            } else {
                x
            };
            assert_eq!(
                unsafe { run(argument, y, u, v, seed) },
                expected,
                "{tys:?} {cc:?} {x:x} {y:x}"
            );
            if matches!(shape, Shape::InterveningStore) {
                assert_eq!(cell, v);
            }
        }
    }
}

#[test]
fn shared_boolean_all_integer_conditions_and_mixed_widths() {
    for tys in [
        [types::I8, types::I64],
        [types::I16, types::I32],
        [types::I32, types::I8],
        [types::I64, types::I16],
    ] {
        for a in CCS {
            for b in CCS {
                check(tys, [a, b], Shape::Ordinary);
            }
        }
    }
}

#[test]
fn shared_boolean_retains_raw_words_shared_leaves_and_cross_block_values() {
    for shape in [
        Shape::AcrossBlocks,
        Shape::InterveningStore,
        Shape::SharedLeaf,
        Shape::RawWords,
    ] {
        check(
            [types::I8, types::I64],
            [IntCC::SignedLessThan, IntCC::UnsignedGreaterThan],
            shape,
        );
    }
}
