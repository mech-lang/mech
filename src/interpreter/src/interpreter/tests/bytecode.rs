#[cfg(all(
    test,
    feature = "program",
    feature = "functions",
    feature = "symbol_table",
    feature = "f64"
))]
mod bytecode_dependency_tests {
    use super::super::super::{
        FunctionArgs, MResult, MechFunction, MechFunctionImpl, ProgramState, ReactiveCellId,
        ReactiveDependencyKind, Ref, Value, register_bytecode_function,
    };

    #[cfg(feature = "compiler")]
    use super::super::super::{BytecodeCompilerContext, MechFunctionCompiler, Register};

    struct BytecodeDependencyTestFunction {
        output: Value,
    }

    impl MechFunctionImpl for BytecodeDependencyTestFunction {
        fn solve(&self) {}

        fn out(&self) -> Value {
            self.output.clone()
        }

        fn to_string(&self) -> String {
            "bytecode-dependency-test".to_string()
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for BytecodeDependencyTestFunction {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn bytecode_dependency_test_factory(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        let output = match args {
            FunctionArgs::Nullary(output)
            | FunctionArgs::Unary(output, _)
            | FunctionArgs::Binary(output, _, _)
            | FunctionArgs::Ternary(output, _, _, _)
            | FunctionArgs::Quaternary(output, _, _, _, _)
            | FunctionArgs::Variadic(output, _) => output,
        };

        Ok(Box::new(BytecodeDependencyTestFunction { output }))
    }

    fn scalar(value: f64) -> (Value, ReactiveCellId) {
        let cell = Ref::new(value);
        let id = ReactiveCellId::new(cell.id());
        (Value::F64(cell), id)
    }

    #[test]
    fn bytecode_nullary_registration_has_no_inputs() {
        let state = ProgramState::new();
        let (output, output_cell) = scalar(1.0);

        let result = register_bytecode_function(
            &state,
            bytecode_dependency_test_factory,
            FunctionArgs::Nullary(output.clone()),
        )
        .unwrap();

        let plan = state.plan.borrow();
        let node = plan.node(0).unwrap();
        assert_eq!(plan.len(), 1);
        assert!(node.inputs.is_empty());
        assert!(plan.reactive_consumers.is_empty());
        assert!(plan.sampled_consumers.is_empty());
        assert!(node.outputs.contains(&output_cell));
        assert_eq!(result.reactive_cell_ids(), output.reactive_cell_ids());
    }

    #[test]
    fn bytecode_unary_registration_indexes_operand() {
        let state = ProgramState::new();
        let (output, output_cell) = scalar(1.0);
        let (input, input_cell) = scalar(2.0);

        register_bytecode_function(
            &state,
            bytecode_dependency_test_factory,
            FunctionArgs::Unary(output, input),
        )
        .unwrap();

        let plan = state.plan.borrow();
        let node = plan.node(0).unwrap();
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.inputs[0].cell, input_cell);
        assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Reactive);
        assert_eq!(plan.reactive_consumers_for(input_cell), &[0]);
        assert!(
            !node
                .inputs
                .iter()
                .any(|dependency| dependency.cell == output_cell)
        );
        assert!(node.outputs.contains(&output_cell));
    }

    #[test]
    fn bytecode_binary_registration_preserves_operand_order() {
        let state = ProgramState::new();
        let (output, _) = scalar(1.0);
        let (lhs, lhs_cell) = scalar(2.0);
        let (rhs, rhs_cell) = scalar(3.0);

        register_bytecode_function(
            &state,
            bytecode_dependency_test_factory,
            FunctionArgs::Binary(output, lhs, rhs),
        )
        .unwrap();

        let plan = state.plan.borrow();
        let node = plan.node(0).unwrap();
        assert_eq!(
            node.inputs
                .iter()
                .map(|dependency| dependency.cell)
                .collect::<Vec<_>>(),
            vec![lhs_cell, rhs_cell],
        );
        assert!(
            node.inputs
                .iter()
                .all(|dependency| { dependency.kind == ReactiveDependencyKind::Reactive })
        );
        assert_eq!(plan.reactive_consumers_for(lhs_cell), &[0]);
        assert_eq!(plan.reactive_consumers_for(rhs_cell), &[0]);
    }

    #[test]
    fn bytecode_variadic_registration_preserves_all_inputs() {
        let state = ProgramState::new();
        let (output, _) = scalar(1.0);
        let (first, first_cell) = scalar(2.0);
        let (second, second_cell) = scalar(3.0);
        let (third, third_cell) = scalar(4.0);

        register_bytecode_function(
            &state,
            bytecode_dependency_test_factory,
            FunctionArgs::Variadic(output, vec![first, second, third]),
        )
        .unwrap();

        let plan = state.plan.borrow();
        let node = plan.node(0).unwrap();
        assert_eq!(
            node.inputs
                .iter()
                .map(|dependency| dependency.cell)
                .collect::<Vec<_>>(),
            vec![first_cell, second_cell, third_cell],
        );
    }

    #[test]
    fn bytecode_registration_deduplicates_alias_operands() {
        let state = ProgramState::new();
        let (output, _) = scalar(1.0);
        let (input, input_cell) = scalar(2.0);

        register_bytecode_function(
            &state,
            bytecode_dependency_test_factory,
            FunctionArgs::Binary(output, input.clone(), input),
        )
        .unwrap();

        let plan = state.plan.borrow();
        let node = plan.node(0).unwrap();
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.inputs[0].cell, input_cell);
        assert_eq!(plan.reactive_consumers_for(input_cell), &[0]);
    }
}
