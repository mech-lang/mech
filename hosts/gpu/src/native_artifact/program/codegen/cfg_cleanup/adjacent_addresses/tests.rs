use crate::native_artifact::codegen::options::{CompilationControls, OptimizationOptions};
use crate::native_artifact::program::codegen::cfg_cleanup::reuse_adjacent_addresses;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Path {
    Adjacent,
    Displaced,
    Distant,
    DifferentPointer,
    InterveningRead,
    Join,
}

type Entry = unsafe extern "C" fn(*mut u16, *mut u16, u64) -> u64;

fn address(b: &mut FunctionBuilder<'_>, base: Value, index: Value, one_based: i64) -> Value {
    let index = b.ins().iadd_imm(index, one_based);
    let index = b.ins().iadd_imm(index, -1);
    let bytes = b.ins().imul_imm(index, 2);
    b.ins().iadd(base, bytes)
}

fn compile(path: Path) -> (cranelift_jit::JITModule, Entry) {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        let next = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [table, other, index] = b.block_params(entry).try_into().unwrap();
        let displaced = path == Path::Displaced;
        let first = address(&mut b, table, index, if displaced { 1 } else { 2 });
        let first_offset = if displaced { 2 } else { 0 };
        let flags = MemFlags::new();
        let left = b.ins().load(types::I16, flags, first, first_offset);
        let barrier = if path == Path::InterveningRead {
            b.ins().load(types::I16, flags, other, 0)
        } else {
            b.ins().iconst(types::I16, 0)
        };
        if path == Path::Join {
            let from_true = b.create_block();
            let from_false = b.create_block();
            let check = b.ins().icmp_imm(IntCC::Equal, index, -1);
            b.ins().brif(check, from_true, &[], from_false, &[]);
            b.switch_to_block(from_true);
            b.ins().jump(next, &[]);
            b.switch_to_block(from_false);
            b.ins().jump(next, &[]);
        } else {
            b.ins().jump(next, &[]);
        }
        b.switch_to_block(next);
        let second_table = if path == Path::DifferentPointer {
            other
        } else {
            table
        };
        let second_slot = if path == Path::Distant { 4 } else { 3 };
        let second = address(&mut b, second_table, index, second_slot);
        let right = b.ins().load(types::I16, flags, second, 0);
        let first_store = address(&mut b, table, index, 2);
        let second_store = address(&mut b, second_table, index, second_slot);
        let replacement = b.ins().iconst(types::I16, 900);
        b.ins().store(flags, replacement, first_store, 0);
        b.ins().store(flags, left, second_store, 0);
        let left = b.ins().uextend(types::I64, left);
        let right = b.ins().uextend(types::I64, right);
        let barrier = b.ins().uextend(types::I64, barrier);
        let right = b.ins().ishl_imm(right, 16);
        let barrier = b.ins().ishl_imm(barrier, 32);
        let result = b.ins().bor(left, right);
        let result = b.ins().bor(result, barrier);
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
    }
    let controls = CompilationControls::new(OptimizationOptions::default());
    let changed = reuse_adjacent_addresses(&controls, &mut context, module.isa()).unwrap();
    let expected = match path {
        Path::Adjacent | Path::Displaced => 3,
        Path::InterveningRead | Path::Join => 2,
        Path::Distant | Path::DifferentPointer => 0,
    };
    assert_eq!(changed, expected, "{path:?}");
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("adjacent_accesses", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // The verified function takes two live eight-element tables and an index
    // whose two selected cells are in those tables.
    let call =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, call)
}

#[test]
fn adjacent_typed_addresses_keep_values_order_and_alternate_paths() {
    let mut calls = 0;
    for path in [
        Path::Adjacent,
        Path::Displaced,
        Path::Distant,
        Path::DifferentPointer,
        Path::InterveningRead,
        Path::Join,
    ] {
        let (_module, call) = compile(path);
        for index in [0u64, 1, 2, 3, u64::MAX] {
            for seed in [0u16, 1, 7, 1234, u16::MAX] {
                let mut first: [u16; 8] = core::array::from_fn(|i| seed.wrapping_add(i as u16));
                let mut second: [u16; 8] = core::array::from_fn(|i| seed.wrapping_sub(i as u16));
                let left = index.wrapping_add(1) as usize;
                let step = if path == Path::Distant { 3 } else { 2 };
                let right = index.wrapping_add(step) as usize;
                let mut wanted_first = first;
                let mut wanted_second = second;
                let old_left = first[left];
                let old_right = if path == Path::DifferentPointer {
                    second[right]
                } else {
                    first[right]
                };
                wanted_first[left] = 900;
                if path == Path::DifferentPointer {
                    wanted_second[right] = old_left;
                } else {
                    wanted_first[right] = old_left;
                }
                let barrier = if path == Path::InterveningRead {
                    second[0]
                } else {
                    0
                };
                let expected =
                    u64::from(old_left) | (u64::from(old_right) << 16) | (u64::from(barrier) << 32);
                assert_eq!(
                    unsafe { call(first.as_mut_ptr(), second.as_mut_ptr(), index) },
                    expected
                );
                assert_eq!(first, wanted_first);
                assert_eq!(second, wanted_second);
                calls += 1;
            }
        }
    }
    assert_eq!(calls, 150);
}
