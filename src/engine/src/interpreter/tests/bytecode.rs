#[cfg(all(
    test,
    feature = "program",
    feature = "functions",
    feature = "symbol_table",
    feature = "f64"
))]
mod bytecode_dependency_tests {
    use super::super::super::{
        BytecodeInstruction, BytecodeProgram, EncodedConstant, FunctionArgs, FunctionArgumentRole,
        FunctionCatalogBuilder, FunctionRuntimeType, FunctionValueRepresentation, Interpreter,
        MResult, MatrixStorage, MechError, MechFunction, MechFunctionFactory, MechFunctionImpl,
        NoMechExecutionServices, ProgramState, ReactiveCellId, ReactiveDependencyKind, Ref,
        RuntimeFunctionContract, RuntimeFunctionId, RuntimeFunctionSignature,
        RuntimeOutputAliasPolicy, RuntimeType, ToValue, Value, hash_str,
        register_bytecode_function, write_bytecode,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

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

    #[derive(Debug)]
    struct ExactF64Nullary {
        output: Ref<f64>,
    }

    impl MechFunctionFactory for ExactF64Nullary {
        const SIGNATURE: RuntimeFunctionSignature =
            RuntimeFunctionSignature::nullary(<f64 as FunctionRuntimeType>::REPRESENTATION);

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            match args {
                FunctionArgs::Nullary(output) => Ok(Box::new(Self {
                    output: output.try_function_ref(FunctionArgumentRole::Output)?,
                })),
                _ => Err(MechError::new(
                    super::super::super::IncorrectNumberOfArguments {
                        expected: 0,
                        found: args.len(),
                    },
                    None,
                )),
            }
        }
    }

    impl MechFunctionImpl for ExactF64Nullary {
        fn solve(&self) {}

        fn out(&self) -> Value {
            self.output.to_value()
        }

        fn to_string(&self) -> String {
            "ExactF64Nullary".into()
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for ExactF64Nullary {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
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

    macro_rules! dependency_factory {
        ($name:ident, $signature:expr) => {
            struct $name;

            impl MechFunctionFactory for $name {
                const SIGNATURE: RuntimeFunctionSignature = $signature;

                fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                    bytecode_dependency_test_factory(args)
                }
            }
        };
    }

    dependency_factory!(
        DependencyNullary,
        RuntimeFunctionSignature::nullary(FunctionValueRepresentation::AnyValue)
    );
    dependency_factory!(
        DependencyUnary,
        RuntimeFunctionSignature::unary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        )
    );
    dependency_factory!(
        DependencyBinary,
        RuntimeFunctionSignature::binary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        )
    );
    dependency_factory!(
        DependencyTernary,
        RuntimeFunctionSignature::ternary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        )
    );
    dependency_factory!(
        DependencyQuaternary,
        RuntimeFunctionSignature::quaternary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        )
    );
    dependency_factory!(
        DependencyVariadic,
        RuntimeFunctionSignature::variadic(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        )
    );

    fn register_dependency_test_function(
        state: &ProgramState,
        args: FunctionArgs,
    ) -> MResult<Value> {
        let mut builder = FunctionCatalogBuilder::new();
        let contract =
            RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias);
        let name = match &args {
            FunctionArgs::Nullary(_) => {
                builder
                    .insert_runtime_factory::<DependencyNullary>("DependencyNullary", contract)?;
                "DependencyNullary"
            }
            FunctionArgs::Unary(_, _) => {
                builder.insert_runtime_factory::<DependencyUnary>("DependencyUnary", contract)?;
                "DependencyUnary"
            }
            FunctionArgs::Binary(_, _, _) => {
                builder.insert_runtime_factory::<DependencyBinary>("DependencyBinary", contract)?;
                "DependencyBinary"
            }
            FunctionArgs::Ternary(_, _, _, _) => {
                builder
                    .insert_runtime_factory::<DependencyTernary>("DependencyTernary", contract)?;
                "DependencyTernary"
            }
            FunctionArgs::Quaternary(_, _, _, _, _) => {
                builder.insert_runtime_factory::<DependencyQuaternary>(
                    "DependencyQuaternary",
                    contract,
                )?;
                "DependencyQuaternary"
            }
            FunctionArgs::Variadic(_, _) => {
                builder
                    .insert_runtime_factory::<DependencyVariadic>("DependencyVariadic", contract)?;
                "DependencyVariadic"
            }
        };
        let catalog = builder.build()?;
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name(name))
            .expect("test factory was just registered");
        register_bytecode_function(state, entry, args)
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

        let result =
            register_dependency_test_function(&state, FunctionArgs::Nullary(output.clone()))
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

        register_dependency_test_function(&state, FunctionArgs::Unary(output, input)).unwrap();

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

        register_dependency_test_function(&state, FunctionArgs::Binary(output, lhs, rhs)).unwrap();

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

        register_dependency_test_function(
            &state,
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

        register_dependency_test_function(
            &state,
            FunctionArgs::Binary(output, input.clone(), input),
        )
        .unwrap();

        let plan = state.plan.borrow();
        let node = plan.node(0).unwrap();
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.inputs[0].cell, input_cell);
        assert_eq!(plan.reactive_consumers_for(input_cell), &[0]);
    }

    #[test]
    fn runtime_contract_preflight_preserves_plan_symbols_dictionary_and_register_state() {
        const NAME: &str = "AddMDMD<f64>";
        struct MustNotInstantiate;
        impl MechFunctionFactory for MustNotInstantiate {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                FunctionValueRepresentation::AnyValue,
                FunctionValueRepresentation::AnyValue,
                FunctionValueRepresentation::AnyValue,
            );

            fn new(_args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                panic!("malformed matrix relations must fail before factory construction")
            }
        }
        fn matrix(rows: u32, cols: u32) -> EncodedConstant {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&rows.to_le_bytes());
            bytes.extend_from_slice(&cols.to_le_bytes());
            for index in 0..rows.saturating_mul(cols) {
                bytes.extend_from_slice(&f64::from(index + 1).to_bits().to_le_bytes());
            }
            EncodedConstant {
                runtime_type: RuntimeType::Matrix {
                    element: Box::new(RuntimeType::F64),
                    storage: MatrixStorage::MatrixD,
                    rows,
                    cols,
                },
                alignment: 8,
                bytes,
            }
        }
        let mut catalog = FunctionCatalogBuilder::new();
        catalog
            .insert_runtime_factory::<MustNotInstantiate>(
                NAME,
                RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias),
            )
            .unwrap();
        let mut interpreter =
            Interpreter::with_function_catalog(7, 100, Arc::new(catalog.build().unwrap()));

        let prior_register = Value::F64(Ref::new(17.0));
        let prior_constant = Value::F64(Ref::new(23.0));
        interpreter.ip = 9;
        interpreter.registers = vec![prior_register.clone()];
        interpreter.constants = vec![prior_constant.clone()];
        interpreter.out = Value::F64(Ref::new(31.0));
        let prior_output = interpreter.out.clone();
        register_dependency_test_function(
            &interpreter.state.borrow(),
            FunctionArgs::Nullary(Value::F64(Ref::new(41.0))),
        )
        .unwrap();
        let symbol_id = hash_str("prior");
        interpreter
            .symbols()
            .borrow_mut()
            .insert(symbol_id, Value::F64(Ref::new(47.0)), false);
        interpreter
            .dictionary()
            .borrow_mut()
            .insert(symbol_id, "prior".into());

        let prior_plan_len = interpreter.plan_len();
        let prior_symbols = interpreter.symbols().borrow().snapshot();
        let prior_dictionary = interpreter.dictionary().borrow().clone();
        let program = super::super::super::ParsedProgram::from_bytes(
            &write_bytecode(&BytecodeProgram {
                register_count: 3,
                constants: vec![matrix(2, 2), matrix(2, 2), matrix(3, 3)],
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions: vec![
                    BytecodeInstruction::ConstLoad {
                        dst: 0,
                        constant: 0,
                    },
                    BytecodeInstruction::ConstLoad {
                        dst: 1,
                        constant: 1,
                    },
                    BytecodeInstruction::ConstLoad {
                        dst: 2,
                        constant: 2,
                    },
                    BytecodeInstruction::RuntimeBinary {
                        function: RuntimeFunctionId::from_name(NAME).raw(),
                        dst: 0,
                        lhs: 1,
                        rhs: 2,
                    },
                    BytecodeInstruction::Return { src: 0 },
                ],
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            })
            .unwrap(),
        )
        .unwrap();

        let mut services = NoMechExecutionServices;
        let error = interpreter
            .run_program_with_services(&program, &mut services)
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert_eq!(interpreter.ip, 9);
        assert_eq!(interpreter.registers, vec![prior_register]);
        assert_eq!(interpreter.constants, vec![prior_constant]);
        assert_eq!(interpreter.out, prior_output);
        assert_eq!(interpreter.plan_len(), prior_plan_len);
        assert_eq!(interpreter.symbols().borrow().snapshot(), prior_symbols);
        assert_eq!(*interpreter.dictionary().borrow(), prior_dictionary);
    }
}

#[cfg(all(
    test,
    feature = "compiler",
    feature = "program",
    feature = "functions",
    feature = "symbol_table",
    feature = "f64"
))]
mod external_bytecode_tests {
    use super::super::super::{
        BytecodeInstruction, ExecutionHostFunctionRequest, ExecutionResourceRequest,
        ExternalHostCallFunction, ExternalResourceReadFunction, ExternalResourceWriteFunction,
        InitialSolvePolicy, MResult, MechExecutionServices, MechProgram, MechProgramConfig,
        ParsedProgram, Ref, ResourceDelivery, ResourceIntent, ValRef, Value,
    };

    #[derive(Default)]
    struct RecordingExternalServices {
        host_requests: Vec<ExecutionHostFunctionRequest>,
        read_requests: Vec<ExecutionResourceRequest>,
        writes: Vec<(ExecutionResourceRequest, Value)>,
        bindings: Vec<(u64, ExecutionResourceRequest, usize)>,
    }

    impl MechExecutionServices for RecordingExternalServices {
        fn invoke_host_function(
            &mut self,
            request: &ExecutionHostFunctionRequest,
            _arguments: &[Value],
        ) -> MResult<Value> {
            self.host_requests.push(request.clone());
            Ok(Value::F64(Ref::new(9.0)))
        }

        fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<Value> {
            self.read_requests.push(request.clone());
            Ok(Value::F64(Ref::new(8.0)))
        }

        fn write_resource(
            &mut self,
            request: &ExecutionResourceRequest,
            value: &Value,
        ) -> MResult<()> {
            self.writes.push((request.clone(), value.clone()));
            Ok(())
        }

        fn bind_live_resource(
            &mut self,
            interpreter_id: u64,
            request: &ExecutionResourceRequest,
            target: ValRef,
        ) -> MResult<()> {
            self.bindings
                .push((interpreter_id, request.clone(), target.addr()));
            Ok(())
        }
    }

    fn request(intent: ResourceIntent, delivery: ResourceDelivery) -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: "test://provider".into(),
            path: match intent {
                ResourceIntent::Read => "input".into(),
                ResourceIntent::Assign => "assigned".into(),
                ResourceIntent::Send => "sent".into(),
            },
            context_name: "test".into(),
            operation: match intent {
                ResourceIntent::Read => "read".into(),
                ResourceIntent::Assign => "write".into(),
                ResourceIntent::Send => "send".into(),
            },
            intent,
            delivery,
        }
    }

    fn plan_shape(program: &MechProgram) -> Vec<(usize, usize, String)> {
        let plan = program.interpreter().plan();
        let plan = plan.borrow();
        plan.nodes
            .iter()
            .map(|node| {
                (
                    node.inputs.len(),
                    node.outputs.len(),
                    node.function
                        .to_string()
                        .split("::")
                        .next()
                        .unwrap()
                        .to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn external_source_plan_compiles_and_reconstructs_equivalent_executable_nodes() {
        let mut source = MechProgram::new(MechProgramConfig::default());
        let plan = source.interpreter().plan();
        let host_argument = Value::F64(Ref::new(3.0));
        let host_request = ExecutionHostFunctionRequest {
            name: "test/host".into(),
        };
        let read_request = request(ResourceIntent::Read, ResourceDelivery::Live);
        let assign_request = request(ResourceIntent::Assign, ResourceDelivery::Snapshot);
        let send_request = request(ResourceIntent::Send, ResourceDelivery::Snapshot);

        let host_output = Ref::new(Value::F64(Ref::new(1.0)));
        plan.register_function(
            Box::new(ExternalHostCallFunction {
                request: host_request.clone(),
                arguments: vec![host_argument.clone()],
                output: host_output.clone(),
                initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
            }),
            &[host_argument.clone()],
        )
        .unwrap();
        let read_output = Ref::new(Value::F64(Ref::new(2.0)));
        plan.register_function(
            Box::new(ExternalResourceReadFunction {
                interpreter_id: source.interpreter().id,
                request: read_request.clone(),
                output: read_output.clone(),
                initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
            }),
            &[],
        )
        .unwrap();
        let assigned = host_output.borrow().clone();
        plan.register_function(
            Box::new(ExternalResourceWriteFunction {
                request: assign_request.clone(),
                input: assigned.clone(),
                output: Ref::new(Value::Empty),
                initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
            }),
            &[assigned],
        )
        .unwrap();
        let sent = read_output.borrow().clone();
        plan.register_function(
            Box::new(ExternalResourceWriteFunction {
                request: send_request.clone(),
                input: sent.clone(),
                output: Ref::new(Value::Empty),
                initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
            }),
            &[sent],
        )
        .unwrap();

        let source_shape = plan_shape(&source);
        let bytecode = source.compile_bytecode().unwrap();
        let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
        assert!(
            parsed
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, BytecodeInstruction::HostCall { .. }))
        );
        assert!(
            parsed
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, BytecodeInstruction::ResourceRead { .. }))
        );
        assert!(
            parsed.instructions.iter().any(|instruction| matches!(
                instruction,
                BytecodeInstruction::ResourceWrite { .. }
            ))
        );
        assert!(
            parsed
                .instructions
                .iter()
                .any(|instruction| matches!(instruction, BytecodeInstruction::ResourceSend { .. }))
        );

        let mut loaded = MechProgram::new(MechProgramConfig::default());
        let loaded_interpreter_id = loaded.interpreter().id;
        let mut services = RecordingExternalServices::default();
        let result = loaded
            .run_bytecode_program_with_services(&parsed, &mut services)
            .unwrap();

        assert_eq!(result, Value::Empty);
        assert_eq!(services.host_requests, vec![host_request]);
        assert_eq!(services.read_requests, vec![read_request.clone()]);
        assert_eq!(services.writes.len(), 2);
        assert_eq!(services.writes[0].0, assign_request);
        assert_eq!(services.writes[1].0, send_request);
        assert!(matches!(&services.writes[0].1, Value::F64(value) if *value.borrow() == 9.0));
        assert!(matches!(&services.writes[1].1, Value::F64(value) if *value.borrow() == 8.0));
        assert_eq!(services.bindings.len(), 1);
        assert_eq!(services.bindings[0].0, loaded_interpreter_id);
        assert_eq!(services.bindings[0].1, read_request);
        assert_eq!(plan_shape(&loaded), source_shape);

        let live_target = services.bindings[0].2;
        loaded
            .interpreter_mut()
            .solve_plan_with_services(&mut services)
            .unwrap();
        assert_eq!(services.bindings.len(), 2);
        assert_eq!(services.bindings[1].2, live_target);
        assert!(matches!(&services.writes[2].1, Value::F64(value) if *value.borrow() == 9.0));
        assert!(matches!(&services.writes[3].1, Value::F64(value) if *value.borrow() == 8.0));
    }
}
