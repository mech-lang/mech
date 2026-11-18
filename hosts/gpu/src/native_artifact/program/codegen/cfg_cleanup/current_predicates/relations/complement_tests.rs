use super::fold_with_mode;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Same,
    Store,
    FreshLoad,
    Bypass,
    ChangedWord,
    Signed,
}

type Entry = unsafe extern "C" fn(*mut u64, u64, u64, u64) -> u64;

fn compile(first: IntCC, shape: Shape, optimize: bool) -> (cranelift_jit::JITModule, Entry) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 4];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut scratch = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut scratch);
        let entry = b.create_block();
        let test = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [storage, right, replacement, bypass] = b.block_params(entry).try_into().unwrap();
        let left = b.ins().load(types::I64, MemFlags::new(), storage, 0);
        if shape == Shape::Bypass {
            b.ins().brif(bypass, yes, &[], test, &[]);
        } else {
            b.ins().jump(test, &[]);
        }
        b.switch_to_block(test);
        let condition = b.ins().icmp(first, left, right);
        let widened = b.ins().uextend(types::I64, condition);
        b.ins().brif(widened, yes, &[], no, &[]);
        for (arm, tag) in [(yes, 0x100), (no, 0x200)] {
            b.switch_to_block(arm);
            if matches!(shape, Shape::Store | Shape::FreshLoad) {
                b.ins().store(MemFlags::new(), replacement, storage, 0);
            }
            let left = match shape {
                Shape::FreshLoad => b.ins().load(types::I64, MemFlags::new(), storage, 0),
                Shape::ChangedWord => b.ins().iadd_imm(left, 1),
                _ => left,
            };
            let second = if shape == Shape::Signed {
                IntCC::SignedGreaterThanOrEqual
            } else {
                first.complement()
            };
            let condition = b.ins().icmp(second, left, right);
            let good = b.create_block();
            let bad = b.create_block();
            b.ins().brif(condition, good, &[], bad, &[]);
            for (block, value) in [(good, tag + 1), (bad, tag + 2)] {
                b.switch_to_block(block);
                let result = b.ins().iconst(types::I64, value);
                b.ins().return_(&[result]);
            }
        }
        b.seal_all_blocks();
        b.finalize();
    }
    context.compute_cfg();
    context.compute_domtree();
    if optimize {
        let changed = fold_with_mode(&mut context.func, &context.cfg, &context.domtree, true);
        let expected = match shape {
            Shape::Same | Shape::Store => 2,
            Shape::Bypass => 1,
            Shape::FreshLoad | Shape::ChangedWord | Shape::Signed => 0,
        };
        assert_eq!(changed, expected, "{first:?}, {shape:?}");
    } else {
        assert_eq!(
            fold_with_mode(&mut context.func, &context.cfg, &context.domtree, false),
            0
        );
    }
    context.compute_cfg();
    context.compute_domtree();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(
            "complemented_edges",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // The verified entry accesses one live, initialized U64 through storage.
    let entry =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, entry)
}

fn compare(condition: IntCC, left: u64, right: u64) -> bool {
    match condition {
        IntCC::Equal => left == right,
        IntCC::NotEqual => left != right,
        IntCC::UnsignedLessThan => left < right,
        IntCC::UnsignedLessThanOrEqual => left <= right,
        IntCC::UnsignedGreaterThan => left > right,
        IntCC::UnsignedGreaterThanOrEqual => left >= right,
        _ => unreachable!(),
    }
}

#[test]
fn complementary_edges_preserve_current_words_effects_and_original_outcomes() {
    let values = [
        0,
        1,
        2,
        31,
        32767,
        32768,
        u32::MAX as u64,
        1 << 63,
        u64::MAX - 1,
        u64::MAX,
    ];
    let mut calls = 0;
    for first in [
        IntCC::Equal,
        IntCC::NotEqual,
        IntCC::UnsignedLessThan,
        IntCC::UnsignedLessThanOrEqual,
        IntCC::UnsignedGreaterThan,
        IntCC::UnsignedGreaterThanOrEqual,
    ] {
        for shape in [
            Shape::Same,
            Shape::Store,
            Shape::FreshLoad,
            Shape::Bypass,
            Shape::ChangedWord,
            Shape::Signed,
        ] {
            let (_old_module, old) = compile(first, shape, false);
            let (_new_module, new) = compile(first, shape, true);
            for left in values {
                for right in values {
                    for bypass in [0, 1] {
                        let replacement = left.rotate_left(13) ^ right;
                        let first_truth =
                            (shape == Shape::Bypass && bypass != 0) || compare(first, left, right);
                        let current = match shape {
                            Shape::FreshLoad => replacement,
                            Shape::ChangedWord => left.wrapping_add(1),
                            _ => left,
                        };
                        let second_truth = if shape == Shape::Signed {
                            (current as i64) >= (right as i64)
                        } else {
                            !compare(first, current, right)
                        };
                        let expected = if first_truth { 0x100 } else { 0x200 }
                            + if second_truth { 1 } else { 2 };
                        let memory = if matches!(shape, Shape::Store | Shape::FreshLoad) {
                            replacement
                        } else {
                            left
                        };
                        for entry in [old, new] {
                            let mut storage = left;
                            assert_eq!(
                                unsafe { entry(&mut storage, right, replacement, bypass) },
                                expected,
                                "{first:?}, {shape:?}, {left}, {right}"
                            );
                            assert_eq!(storage, memory);
                            calls += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(calls, 14_400);
}
