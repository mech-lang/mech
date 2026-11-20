use super::*;
use cranelift_codegen::{Context, ir::AbiParam};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Run = unsafe extern "C" fn(*mut u32, u64, u64) -> u64;

fn body(module: &JITModule) -> (Context, Value) {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let length;
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        let left = b.create_block();
        let right = b.create_block();
        let join = b.create_block();
        let read = b.create_block();
        let empty = b.create_block();
        let update = b.create_block();
        let fits = b.create_block();
        let write = b.create_block();
        let overflow = b.create_block();
        let invalid = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(join, types::I64);
        b.append_block_param(update, types::I64);
        b.switch_to_block(entry);
        let [pointer, current, tag] = b.block_params(entry).try_into().unwrap();
        length = current;
        // A visible source effect must still happen exactly once on refusal.
        let marker = b.ins().ireduce(types::I32, tag);
        b.ins()
            .store(cranelift_codegen::ir::MemFlags::new(), marker, pointer, 0);
        b.ins().brif(tag, left, &[], right, &[]);
        b.switch_to_block(left);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(join, &[zero.into()]);
        b.switch_to_block(right);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().jump(join, &[one.into()]);
        b.switch_to_block(join);
        let bit = b.block_params(join)[0];
        let index = b.ins().iadd_imm(bit, 8);
        let in_bounds = b.ins().icmp(IntCC::UnsignedLessThan, index, current);
        b.ins().brif(in_bounds, read, &[], empty, &[]);
        b.switch_to_block(read);
        let offset = b.ins().ishl_imm(index, 2);
        let address = b.ins().iadd(pointer, offset);
        let value = b
            .ins()
            .uload32(cranelift_codegen::ir::MemFlags::new(), address, 0);
        b.ins().jump(update, &[value.into()]);
        b.switch_to_block(empty);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(update, &[zero.into()]);
        b.switch_to_block(update);
        let value = b.block_params(update)[0];
        let valid = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThan, value, u32::MAX as i64);
        b.ins().brif(valid, fits, &[], overflow, &[]);
        b.switch_to_block(fits);
        let incremented = b.ins().iadd_imm(value, 1);
        let one_based = b.ins().iadd_imm(index, 1);
        let write_index = b.ins().iadd_imm(one_based, -1);
        let in_bounds = b.ins().icmp(IntCC::UnsignedLessThan, write_index, current);
        b.ins().brif(in_bounds, write, &[], invalid, &[]);
        b.switch_to_block(write);
        let offset = b.ins().ishl_imm(write_index, 2);
        let address = b.ins().iadd(pointer, offset);
        let narrow = b.ins().ireduce(types::I32, incremented);
        b.ins()
            .store(cranelift_codegen::ir::MemFlags::new(), narrow, address, 0);
        let success = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[success]);
        for (block, origin) in [(overflow, 101), (invalid, 202)] {
            b.switch_to_block(block);
            let error = b.ins().iconst(types::I64, origin);
            b.ins().return_(&[error]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    (context, length)
}

#[test]
fn guarded_small_table_preserves_partial_state_and_error_origins() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let (mut original, length) = body(&module);
    let mut fast = module.make_context();
    fast.func = original.func.clone();
    let floors = required_index_floors(&fast.func, &std::collections::BTreeSet::from([length]));
    assert_eq!(floors, BTreeMap::from([(length, 10)]));
    assert_eq!(fold_index_ranges(&mut fast.func, &floors), (0, 2));
    let mut ids = Vec::new();
    for (name, context) in [("original", &mut original), ("fast", &mut fast)] {
        context.compute_cfg();
        context.compute_domtree();
        context.verify(module.isa()).unwrap();
        let id = module
            .declare_function(name, Linkage::Local, &context.func.signature)
            .unwrap();
        module.define_function(id, context).unwrap();
        ids.push(id);
    }
    let mut wrapper = module.make_context();
    wrapper.func.signature = original.func.signature.clone();
    let functions = ids
        .iter()
        .map(|&id| module.declare_func_in_func(id, &mut wrapper.func))
        .collect::<Vec<_>>();
    let mut builder = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut wrapper.func, &mut builder);
        let entry = b.create_block();
        let admitted = b.create_block();
        let refused = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let enough = b.ins().icmp_imm(
            IntCC::UnsignedGreaterThanOrEqual,
            args[1],
            floors[&length] as i64,
        );
        b.ins().brif(enough, admitted, &[], refused, &[]);
        for (block, function) in [(admitted, functions[1]), (refused, functions[0])] {
            b.switch_to_block(block);
            let call = b.ins().call(function, &args);
            let result = b.inst_results(call)[0];
            b.ins().return_(&[result]);
        }
        b.seal_all_blocks();
        b.finalize();
    }
    let id = module
        .declare_function("guarded", Linkage::Local, &wrapper.func.signature)
        .unwrap();
    module.define_function(id, &mut wrapper).unwrap();
    module.finalize_definitions().unwrap();
    // Both declarations have the exact pointer/two-word ABI; keep the module alive.
    let old =
        unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(ids[0])) };
    let new = unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    for length in [0, 1, 7, 8, 9, 10, 11, 12, u64::MAX] {
        for tag in [0, 1, 8, 9, u64::MAX] {
            for seed in [0, 1, u32::MAX - 1, u32::MAX] {
                let mut a = [seed; 12];
                let mut b = a;
                // Only slots 0, 8 and 9 are touched, backed by both live arrays.
                let before = unsafe { old(a.as_mut_ptr(), length, tag) };
                let after = unsafe { new(b.as_mut_ptr(), length, tag) };
                assert_eq!((before, a), (after, b), "{length} {tag} {seed}");
                assert_eq!(b[0], tag as u32);
            }
        }
    }
}
