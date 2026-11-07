use super::*;
use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, MemFlags, Type, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn verify(function: &Function) {
    let isa = cranelift_native::builder()
        .unwrap()
        .finish(cranelift_codegen::settings::Flags::new(
            cranelift_codegen::settings::builder(),
        ))
        .unwrap();
    Context::for_function(function.clone())
        .verify(isa.as_ref())
        .unwrap();
}

fn cycle(ty: Type) -> (Function, Block) {
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 2];
    function.signature.returns.push(AbiParam::new(types::I64));
    let mut context = FunctionBuilderContext::new();
    let header;
    {
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        let entry = b.create_block();
        header = b.create_block();
        let body = b.create_block();
        let done = b.create_block();
        b.append_block_params_for_function_params(entry);
        for ty in [types::I64, types::I64, ty] {
            b.append_block_param(header, ty);
        }
        b.switch_to_block(entry);
        let pointer = b.block_params(entry)[0];
        let count = b.block_params(entry)[1];
        let zero = b.ins().iconst(types::I64, 0);
        let dead = b.ins().load(ty, MemFlags::new().with_notrap(), pointer, 8);
        b.ins()
            .jump(header, &[zero.into(), zero.into(), dead.into()]);
        b.switch_to_block(header);
        let args = b.block_params(header).to_vec();
        let more = b.ins().icmp(IntCC::UnsignedLessThan, args[0], count);
        b.ins().brif(more, body, &[], done, &[]);
        b.switch_to_block(body);
        let k = b.ins().iadd_imm(args[0], 1);
        let sum = b.ins().iadd(args[1], args[0]);
        let next = if ty.is_float() {
            b.ins().fadd(args[2], args[2])
        } else {
            b.ins().iadd_imm(args[2], 1)
        };
        b.ins().jump(header, &[k.into(), sum.into(), next.into()]);
        b.switch_to_block(done);
        b.ins().return_(&[args[1]]);
        b.seal_all_blocks();
        b.finalize();
    }
    (function, header)
}

#[test]
fn dead_values_retire_unused_cycles_and_nontrapping_loads_at_all_scalar_widths() {
    for ty in [
        types::I8,
        types::I16,
        types::I32,
        types::I64,
        types::F32,
        types::F64,
    ] {
        let (mut function, header) = cycle(ty);
        verify(&function);
        let removed = eliminate(&mut function);
        assert_eq!(removed.parameters, 1);
        assert_eq!(removed.loads, 1);
        assert_eq!(function.dfg.block_params(header).len(), 2);
        assert_eq!(function.signature.params.len(), 2);
        verify(&function);
        let again = eliminate(&mut function);
        assert_eq!((again.parameters, again.instructions), (0, 0));
    }
}

#[test]
fn dead_values_keep_trapping_reads_arithmetic_calls_stores_and_their_order() {
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 3];
    function.signature.returns.push(AbiParam::new(types::I64));
    let mut context = FunctionBuilderContext::new();
    let effects;
    {
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let _safe = b
            .ins()
            .load(types::I64, MemFlags::new().with_notrap(), args[0], 8);
        let _trapping = b.ins().load(types::I64, MemFlags::new(), args[0], 16);
        let seven = b.ins().iconst(types::I64, 7);
        let _divide = b.ins().udiv(seven, args[1]);
        let mut signature =
            cranelift_codegen::ir::Signature::new(cranelift_codegen::isa::CallConv::SystemV);
        signature.params.push(AbiParam::new(types::I64));
        let signature = b.func.import_signature(signature);
        b.ins().call_indirect(signature, args[2], &[seven]);
        b.ins().store(MemFlags::new(), seven, args[0], 0);
        b.ins().return_(&[seven]);
        b.seal_all_blocks();
        b.finalize();
        effects = function
            .layout
            .block_insts(entry)
            .skip(1)
            .collect::<Vec<_>>();
    }
    verify(&function);
    let removed = eliminate(&mut function);
    assert_eq!((removed.instructions, removed.loads), (1, 1));
    assert_eq!(
        function
            .layout
            .block_insts(function.layout.entry_block().unwrap())
            .collect::<Vec<_>>(),
        effects
    );
    verify(&function);
}

#[test]
fn dead_values_follow_each_current_argument_when_both_branches_share_a_target() {
    let mut function = Function::new();
    function.signature.params = vec![AbiParam::new(types::I64); 3];
    function.signature.returns.push(AbiParam::new(types::I64));
    let mut context = FunctionBuilderContext::new();
    let (entry, join, original);
    {
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        entry = b.create_block();
        join = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(join, types::I64);
        b.append_block_param(join, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        original = args.clone();
        b.ins().brif(
            args[0],
            join,
            &[args[1].into(), args[2].into()],
            join,
            &[args[2].into(), args[1].into()],
        );
        b.switch_to_block(join);
        let selected = b.block_params(join)[0];
        b.ins().return_(&[selected]);
        b.seal_all_blocks();
        b.finalize();
    }
    verify(&function);
    let removed = eliminate(&mut function);
    assert_eq!(removed.parameters, 1);
    let inst = function.layout.last_inst(entry).unwrap();
    let calls = function.dfg.insts[inst]
        .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables);
    for (call, wanted) in calls.iter().zip([original[1], original[2]]) {
        assert_eq!(
            call.args(&function.dfg.value_lists)
                .filter_map(|v| v.as_value())
                .collect::<Vec<_>>(),
            vec![wanted]
        );
    }
    assert_eq!(function.dfg.block_params(join).len(), 1);
    verify(&function);
}

#[test]
fn dead_values_do_not_remove_an_unobservable_infinite_loop() {
    let mut function = Function::new();
    let mut context = FunctionBuilderContext::new();
    let (entry, header);
    {
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        entry = b.create_block();
        header = b.create_block();
        b.append_block_param(header, types::I64);
        b.switch_to_block(entry);
        let seed = b.ins().iconst(types::I64, 19);
        b.ins().jump(header, &[seed.into()]);
        b.switch_to_block(header);
        let current = b.block_params(header)[0];
        let next = b.ins().iadd_imm(current, 1);
        b.ins().jump(header, &[next.into()]);
        b.seal_all_blocks();
        b.finalize();
    }
    verify(&function);
    let removed = eliminate(&mut function);
    assert_eq!(removed.parameters, 1);
    assert_eq!(
        function.layout.blocks().collect::<Vec<_>>(),
        vec![entry, header]
    );
    for block in [entry, header] {
        assert_eq!(function.layout.block_insts(block).count(), 1);
        assert_eq!(
            function.dfg.insts[function.layout.last_inst(block).unwrap()].opcode(),
            Opcode::Jump
        );
    }
    verify(&function);
}
