//! Exercise the exact vendored builder without the upstream x86 dev features.
extern crate alloc;
pub use cranelift_frontend::Variable;
#[allow(dead_code)]
#[path = "../../../vendor/cranelift-frontend/src/ssa.rs"]
mod ssa;

/// A known single-predecessor success chain can be sealed while its enclosing
/// join is still waiting for forward predecessors. Only the join needs a phi.
#[test]
fn sealed_success_chain_keeps_the_enclosing_join_open_without_temporary_phis() {
    use cranelift_codegen::ir::{
        AbiParam, Block, Function, InstBuilder, InstructionData, Value, types,
    };
    use cranelift_codegen::{settings, verifier::verify_function};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    fn build(early: bool, diamond: bool) -> (Function, Block, Value) {
        let mut function = Function::new();
        function.signature.params = vec![AbiParam::new(types::I64); 3];
        function.signature.returns.push(AbiParam::new(types::I64));
        let mut context = FunctionBuilderContext::new();
        let source;
        let returned;
        {
            let mut b = FunctionBuilder::new(&mut function, &mut context);
            let entry = b.create_block();
            let left = b.create_block();
            let right = b.create_block();
            source = b.create_block();
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            b.seal_block(entry);
            let args = b.block_params(entry).to_vec();
            let variable = b.declare_var(types::I64);
            b.ins().brif(args[0], left, &[], right, &[]);

            // Emit the join and its success continuations before either of
            // the join's incoming definitions has been translated.
            b.switch_to_block(source);
            for _ in 0..128 {
                let value = b.use_var(variable);
                let valid =
                    b.ins()
                        .icmp_imm(cranelift_codegen::ir::condcodes::IntCC::NotEqual, value, 0);
                let good = b.create_block();
                if diamond {
                    let done = b.create_block();
                    b.ins().brif(valid, good, &[], done, &[]);
                    if early {
                        b.seal_block(good);
                    }
                    b.switch_to_block(good);
                    b.ins().jump(done, &[]);
                    if early {
                        b.seal_block_with_forwarding(done);
                    }
                    b.switch_to_block(done);
                } else {
                    let status = b.ins().iconst(types::I64, 9);
                    b.ins().brif(valid, good, &[], error, &[status.into()]);
                    if early {
                        b.seal_block(good);
                        assert!(b.is_block_sealed(good));
                    }
                    b.switch_to_block(good);
                }
                assert!(!b.is_block_sealed(source));
            }
            returned = b.use_var(variable);
            b.ins().return_(&[returned]);
            b.switch_to_block(left);
            b.def_var(variable, args[1]);
            b.ins().jump(source, &[]);
            b.switch_to_block(right);
            b.def_var(variable, args[2]);
            b.ins().jump(source, &[]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            b.ins().return_(&[status]);
            b.seal_all_blocks();
            assert!(b.is_block_sealed(source));
            b.finalize();
        }
        verify_function(&function, &settings::Flags::new(settings::builder())).unwrap();
        assert_eq!(function.dfg.block_params(source).len(), 1);
        assert_eq!(
            function.dfg.resolve_aliases(returned),
            function.dfg.block_params(source)[0]
        );
        // Its two forward predecessors must pass the actual two distinct
        // inputs, rather than a default value or a premature definition.
        let incoming = function
            .layout
            .blocks()
            .flat_map(|block| function.layout.block_insts(block))
            .filter_map(|inst| match function.dfg.insts[inst] {
                InstructionData::Jump { destination, .. }
                    if destination.block(&function.dfg.value_lists) == source =>
                {
                    Some(
                        destination
                            .args(&function.dfg.value_lists)
                            .next()
                            .unwrap()
                            .as_value()
                            .unwrap(),
                    )
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let entry = function.layout.entry_block().unwrap();
        assert_eq!(incoming, function.dfg.block_params(entry)[1..]);
        (function, source, returned)
    }

    for diamond in [false, true] {
        let (late, _, _) = build(false, diamond);
        let (early, _, _) = build(true, diamond);
        assert_eq!(late.layout.blocks().count(), early.layout.blocks().count());
        assert_eq!(late.dfg.num_insts(), early.dfg.num_insts());
        assert!(late.dfg.num_values() >= early.dfg.num_values() + 128);
    }
}

/// Unchanged diamonds must forward the existing SSA value without allocating
/// aliases. A real assignment on one arm must retain its merge parameter.
#[test]
fn unchanged_diamonds_forward_values_but_distinct_assignments_keep_phis() {
    use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, types};
    use cranelift_codegen::{settings, verifier::verify_function};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    for assign in [false, true] {
        let mut function = Function::new();
        function.signature.params = vec![AbiParam::new(types::I64); 2];
        function.signature.returns.push(AbiParam::new(types::I64));
        let mut context = FunctionBuilderContext::new();
        let joins;
        let returned;
        let seed;
        {
            let mut b = FunctionBuilder::new(&mut function, &mut context);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            b.seal_block(entry);
            let condition = b.block_params(entry)[0];
            seed = b.block_params(entry)[1];
            let variable = b.declare_var(types::I64);
            b.def_var(variable, seed);
            let mut all_joins = Vec::new();
            for _ in 0..128 {
                let yes = b.create_block();
                let done = b.create_block();
                b.ins().brif(condition, yes, &[], done, &[]);
                b.seal_block(yes);
                b.switch_to_block(yes);
                if assign {
                    let different = b.ins().iconst(types::I64, 37);
                    b.def_var(variable, different);
                }
                b.ins().jump(done, &[]);
                b.seal_block_with_forwarding(done);
                b.switch_to_block(done);
                let _ = b.use_var(variable);
                all_joins.push(done);
            }
            returned = b.use_var(variable);
            b.ins().return_(&[returned]);
            b.seal_all_blocks();
            b.finalize();
            joins = all_joins;
        }
        verify_function(&function, &settings::Flags::new(settings::builder())).unwrap();
        for &join in &joins {
            assert_eq!(function.dfg.block_params(join).len(), usize::from(assign));
        }
        if assign {
            assert_ne!(function.dfg.resolve_aliases(returned), seed);
            assert_eq!(
                function.dfg.resolve_aliases(returned),
                function.dfg.block_params(*joins.last().unwrap())[0]
            );
        } else {
            assert_eq!(function.dfg.resolve_aliases(returned), seed);
            assert_eq!(function.dfg.num_values(), 2);
        }
    }
}
