use super::{Candidate, share};
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use std::collections::BTreeMap;

fn define(module: &mut JITModule, enabled: bool) -> FuncId {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [choose, path, value] = *b.block_params(entry) else {
        unreachable!()
    };
    let zero = b.ins().iconst(types::I64, 0);
    let seed = b.ins().iconst(types::I64, 11);
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    b.ins().stack_store(seed, slot, 0);
    let first = b.create_block();
    let second = b.create_block();
    for guard in [first, second] {
        b.append_block_param(guard, types::I64);
    }
    b.ins()
        .brif(choose, second, &[zero.into()], first, &[zero.into()]);
    let mut parts = Vec::new();
    for (guard, header) in [(first, 30), (second, 90)] {
        let start = b.create_block();
        let ready = b.create_block();
        let error = b.create_block();
        let choice = b.create_block();
        let exit = b.create_block();
        parts.push(Candidate {
            guard,
            start,
            exit,
            header,
            span: 12,
        });
        b.append_block_param(exit, types::I64);
        b.switch_to_block(guard);
        let carry = b.block_params(guard)[0];
        b.ins().brif(carry, ready, &[], start, &[]);
        b.switch_to_block(ready);
        let old = b.ins().stack_load(types::I64, slot, 0);
        let sum = b.ins().iadd(old, carry);
        b.ins().jump(exit, &[sum.into()]);
        b.switch_to_block(start);
        let initial = b.ins().stack_load(types::I64, slot, 0);
        let advanced = b.ins().iadd(initial, value);
        b.ins().stack_store(advanced, slot, 0);
        let reject = b.ins().icmp_imm(IntCC::Equal, path, 2);
        b.ins().brif(reject, error, &[], choice, &[]);
        b.switch_to_block(error);
        b.set_cold_block(error);
        let code = b.ins().iconst(types::I64, 17);
        let origin = b.ins().iconst(types::I64, (header + 4) as i64);
        b.ins().return_(&[code, origin]);
        b.switch_to_block(choice);
        b.ins()
            .brif(path, guard, &[advanced.into()], exit, &[advanced.into()]);
        b.switch_to_block(exit);
        let result = b.block_params(exit)[0];
        b.ins().return_(&[zero, result]);
    }
    b.seal_all_blocks();
    b.finalize();

    let id = module
        .declare_function(
            if enabled {
                "outlined_test"
            } else {
                "original_test"
            },
            Linkage::Export,
            &ctx.func.signature,
        )
        .unwrap();
    if enabled {
        let report = share(module, &mut ctx.func, BTreeMap::from([(id, parts)]), false)
            .unwrap()
            .expect("both traversals selected");
        assert_eq!(report[0]["headers"], serde_json::json!([30, 90]));
    }
    ctx.verify(module.isa()).unwrap();
    module.define_function(id, &mut ctx).unwrap();
    id
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
struct ResultPair(u64, u64);

#[test]
fn shared_traversal_preserves_exit_resume_and_failure_origin() {
    let builder = JITBuilder::new(default_libcall_names()).unwrap();
    let mut module = JITModule::new(builder);
    let original = define(&mut module, false);
    let outlined = define(&mut module, true);
    module.finalize_definitions().unwrap();
    // Both functions were defined above with the host C calling convention
    // and a pair of 64-bit integer return values.
    let bind = |id| unsafe {
        std::mem::transmute::<_, extern "C" fn(u64, u64, u64) -> ResultPair>(
            module.get_finalized_function(id),
        )
    };
    let old = bind(original);
    let new = bind(outlined);
    for (choice, origin) in [(0, 34), (1, 94)] {
        for (path, expected) in [
            (0, ResultPair(0, 16)),
            (1, ResultPair(0, 32)),
            (2, ResultPair(17, origin)),
        ] {
            assert_eq!(old(choice, path, 5), expected);
            assert_eq!(new(choice, path, 5), expected);
        }
    }
}
