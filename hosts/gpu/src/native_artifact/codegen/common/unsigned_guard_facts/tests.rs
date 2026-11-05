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
