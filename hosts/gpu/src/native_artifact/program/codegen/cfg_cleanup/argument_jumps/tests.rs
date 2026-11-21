use super::*;
use cranelift_codegen::{
    ir::{AbiParam, Inst, InstBuilder, types},
    settings, verify_function,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn verify(function: &Function) {
    verify_function(function, &settings::Flags::new(settings::builder())).unwrap();
}

fn edges(function: &Function, inst: Inst) -> Vec<(Block, Vec<Value>)> {
    function.dfg.insts[inst]
        .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables)
        .iter()
        .map(|&call| {
            (
                call.block(&function.dfg.value_lists),
                arguments(function, call).unwrap(),
            )
        })
        .collect()
}

#[test]
fn composes_distinct_parallel_edges_and_transitive_permutations() {
    let mut function = Function::new();
    function.signature.params.extend([
        AbiParam::new(types::I8),
        AbiParam::new(types::I64),
        AbiParam::new(types::I64),
    ]);
    function
        .signature
        .returns
        .extend([AbiParam::new(types::I64); 3]);
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut state);
    let entry = b.create_block();
    let first = b.create_block();
    let second = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    for block in [first, second] {
        b.append_block_param(block, types::I64);
        b.append_block_param(block, types::I64);
    }
    for _ in 0..3 {
        b.append_block_param(done, types::I64);
    }
    b.switch_to_block(entry);
    let input = b.block_params(entry).to_vec();
    let branch = b.ins().brif(
        input[0],
        first,
        &[input[1].into(), input[2].into()],
        first,
        &[input[2].into(), input[1].into()],
    );
    b.switch_to_block(first);
    let a = b.block_params(first).to_vec();
    b.ins().jump(second, &[a[1].into(), a[0].into()]);
    b.switch_to_block(second);
    let a = b.block_params(second).to_vec();
    b.ins().jump(done, &[a[0].into(), a[1].into(), a[0].into()]);
    b.switch_to_block(done);
    let result = b.block_params(done).to_vec();
    b.ins().return_(&result);
    b.seal_all_blocks();
    b.finalize();
    verify(&function);

    assert_eq!(forward(&mut function), 3);
    verify(&function);
    assert_eq!(
        edges(&function, branch),
        [
            (done, vec![input[2], input[1], input[2]]),
            (done, vec![input[1], input[2], input[1]]),
        ]
    );
}

#[test]
fn preserves_parameters_used_outside_the_jump_including_aliases() {
    let mut function = Function::new();
    function.signature.params.push(AbiParam::new(types::I64));
    function.signature.returns.push(AbiParam::new(types::I64));
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut state);
    let entry = b.create_block();
    let via = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    let param = b.append_block_param(via, types::I64);
    b.switch_to_block(entry);
    let input = b.block_params(entry)[0];
    let incoming = b.ins().jump(via, &[input.into()]);
    b.switch_to_block(via);
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    let unused = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[unused]);
    b.seal_all_blocks();
    b.finalize();
    let ValueDef::Result(inst, _) = function.dfg.value_def(unused) else {
        unreachable!()
    };
    function.dfg.detach_inst_results(inst);
    function.layout.remove_inst(inst);
    function.dfg.change_to_alias(unused, param);
    verify(&function);
    assert_eq!(forward(&mut function), 0);
    verify(&function);
    assert_eq!(edges(&function, incoming), [(via, vec![input])]);
}

#[test]
fn retains_the_cold_boundary_and_debug_contract() {
    let mut function = Function::new();
    function.signature.params.push(AbiParam::new(types::F64));
    function.signature.returns.push(AbiParam::new(types::F64));
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut state);
    let entry = b.create_block();
    let via = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(via, types::F64);
    b.append_block_param(done, types::F64);
    b.set_cold_block(via);
    b.switch_to_block(entry);
    let input = b.block_params(entry)[0];
    let incoming = b.ins().jump(via, &[input.into()]);
    b.switch_to_block(via);
    let value = b.block_params(via)[0];
    b.ins().jump(done, &[value.into()]);
    b.switch_to_block(done);
    let value = b.block_params(done)[0];
    b.ins().return_(&[value]);
    b.seal_all_blocks();
    b.finalize();
    verify(&function);
    assert_eq!(forward(&mut function), 0);
    function.layout.set_cold(done);
    function.preferred_fallthrough = Some((entry, via));
    assert_eq!(forward(&mut function), 0);
    function.preferred_fallthrough = None;
    function.dfg.values_labels = Some(Default::default());
    assert_eq!(forward(&mut function), 0);
    function.dfg.values_labels = None;
    assert_eq!(forward(&mut function), 1);
    verify(&function);
    assert_eq!(edges(&function, incoming), [(done, vec![input])]);
}

#[test]
fn stops_at_backward_edges_and_preserves_loop_arguments() {
    let mut function = Function::new();
    function.signature.params.push(AbiParam::new(types::I64));
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut state);
    let entry = b.create_block();
    let first = b.create_block();
    let second = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(first, types::I64);
    b.append_block_param(second, types::I64);
    b.switch_to_block(entry);
    let input = b.block_params(entry)[0];
    let incoming = b.ins().jump(first, &[input.into()]);
    b.switch_to_block(first);
    let param = b.block_params(first)[0];
    b.ins().jump(second, &[param.into()]);
    b.switch_to_block(second);
    let param = b.block_params(second)[0];
    let back = b.ins().jump(first, &[param.into()]);
    b.seal_all_blocks();
    b.finalize();
    verify(&function);
    assert_eq!(forward(&mut function), 2);
    verify(&function);
    assert_eq!(edges(&function, incoming), [(second, vec![input])]);
    assert_eq!(edges(&function, back), [(second, vec![param])]);
    assert_eq!(forward(&mut function), 0);
}

#[test]
fn refuses_unextracted_union_uses_of_a_transport_parameter() {
    let mut function = Function::new();
    function.signature.params.push(AbiParam::new(types::I64));
    function.signature.returns.push(AbiParam::new(types::I64));
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut function, &mut state);
    let entry = b.create_block();
    let via = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    let param = b.append_block_param(via, types::I64);
    b.switch_to_block(entry);
    let input = b.block_params(entry)[0];
    let incoming = b.ins().jump(via, &[input.into()]);
    b.switch_to_block(via);
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    let result = b.ins().return_(&[param]);
    b.seal_all_blocks();
    b.finalize();
    let union = function.dfg.union(param, param);
    function.dfg.inst_args_mut(result)[0] = union;
    // Unextracted egraph values have no final SSA attachment. The pass must
    // leave this intermediate representation alone instead of hiding a use.
    assert_eq!(forward(&mut function), 0);
    assert_eq!(edges(&function, incoming), [(via, vec![input])]);
}
