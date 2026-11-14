use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, default_libcall_names};

#[test]
fn endpoint_addresses_allow_commutation_and_preserve_distinct_bases() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    let (entry, left, commuted, distinct);
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        left = b.ins().iadd(args[0], args[1]);
        commuted = b.ins().iadd(args[1], args[0]);
        distinct = b.ins().iadd(args[2], args[1]);
        b.ins().return_(&[left]);
        b.seal_all_blocks();
        b.finalize();
    }
    context.cfg.compute(&context.func);
    let resolve = Resolve {
        context: &context,
        header: entry,
    };
    assert!(same_address(&resolve, left, left));
    assert!(same_address(&resolve, left, commuted));
    assert!(!same_address(&resolve, left, distinct));
}

#[test]
fn endpoint_handoff_extends_only_its_selected_edge_and_preserves_existing_arguments() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    let (join, selected, other, end, original);
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        let yes = b.create_block();
        let no = b.create_block();
        join = b.create_block();
        b.append_block_param(join, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        original = args[0];
        end = args[1];
        b.ins().brif(original, yes, &[], no, &[]);
        b.switch_to_block(yes);
        selected = b.ins().jump(join, &[original.into()]);
        b.switch_to_block(no);
        other = b.ins().jump(join, &[original.into()]);
        b.switch_to_block(join);
        let value = b.block_params(join)[0];
        b.ins().return_(&[value]);
        b.seal_all_blocks();
        b.finalize();
    }
    let handoff = Handoff {
        header: 7,
        entry: join,
        selected,
        predecessors: BTreeSet::from([selected, other]),
    };
    let parameter = handoff.emit(&mut context.func, end);
    assert_eq!(
        context.func.dfg.block_params(join),
        &[context.func.dfg.block_params(join)[0], parameter]
    );
    for inst in [selected, other] {
        let edge = context.func.dfg.insts[inst].branch_destination(
            &context.func.dfg.jump_tables,
            &context.func.dfg.exception_tables,
        )[0];
        let args = edge
            .args(&context.func.dfg.value_lists)
            .map(|v| v.as_value().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(args[0], original);
        if inst == selected {
            assert_eq!(args[1], end);
        } else {
            let ValueDef::Result(zero, 0) = context.func.dfg.value_def(args[1]) else {
                panic!("fallback endpoint is not zero")
            };
            assert!(
                matches!(context.func.dfg.insts[zero], InstructionData::UnaryImm { opcode: Opcode::Iconst, imm } if imm.bits() == 0)
            );
        }
    }
    cranelift_codegen::verify_function(&context.func, module.isa()).unwrap();
}
