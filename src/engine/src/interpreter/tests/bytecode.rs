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
        LegacyValue, MResult, MatrixStorage, MechError, MechFunction, MechFunctionFactory,
        MechFunctionImpl, NoMechExecutionServices, ProgramState, ReactiveCellId,
        ReactiveDependencyKind, Ref, RuntimeFunctionContract, RuntimeFunctionId,
        RuntimeFunctionSignature, RuntimeOutputAliasPolicy, RuntimeType, ToValue, hash_str,
        register_bytecode_function, write_bytecode,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    #[cfg(feature = "compiler")]
    use super::super::super::{BytecodeCompilerContext, MechFunctionCompiler, Register};

    struct BytecodeDependencyTestFunction {
        output: LegacyValue,
    }

    impl MechFunctionImpl for BytecodeDependencyTestFunction {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            self.output.clone()
        }

        fn to_string(&self) -> String {
            "bytecode-dependency-test".to_string()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
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
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            self.output.to_value()
        }

        fn to_string(&self) -> String {
            "ExactF64Nullary".into()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
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
    ) -> MResult<LegacyValue> {
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

    fn scalar(value: f64) -> (LegacyValue, ReactiveCellId) {
        let cell = Ref::new(value);
        let id = ReactiveCellId::new(cell.id());
        (LegacyValue::F64(cell), id)
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
        fn matrix(rows: u32, cols: u32, first: u32) -> EncodedConstant {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&rows.to_le_bytes());
            bytes.extend_from_slice(&cols.to_le_bytes());
            for index in 0..rows.saturating_mul(cols) {
                bytes.extend_from_slice(&f64::from(index + first).to_bits().to_le_bytes());
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

        let prior_register = LegacyValue::F64(Ref::new(17.0));
        let prior_constant = LegacyValue::F64(Ref::new(23.0));
        interpreter.ip = 9;
        interpreter.bytecode_registers = super::super::super::BytecodeRegisterFile::new(1);
        interpreter
            .bytecode_registers
            .load(0, prior_register.clone())
            .unwrap();
        interpreter.constants = vec![prior_constant.clone()];
        interpreter.out = LegacyValue::F64(Ref::new(31.0));
        let prior_output = interpreter.out.clone();
        register_dependency_test_function(
            &interpreter.state.borrow(),
            FunctionArgs::Nullary(LegacyValue::F64(Ref::new(41.0))),
        )
        .unwrap();
        let symbol_id = hash_str("prior");
        interpreter.symbols().borrow_mut().insert(
            symbol_id,
            LegacyValue::F64(Ref::new(47.0)),
            false,
        );
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
                constants: vec![matrix(2, 2, 1), matrix(2, 2, 10), matrix(3, 3, 20)],
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
        assert_eq!(
            interpreter.bytecode_registers.value(0).unwrap(),
            prior_register
        );
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
    feature = "map",
    feature = "set",
    feature = "u8"
))]
mod hashed_composite_pack_tests {
    use super::super::super::{
        BytecodeCompilerContext, BytecodeInstruction, BytecodeProgram, EncodedConstant,
        FunctionArgs, FunctionArgumentRole, FunctionCatalogBuilder, FunctionRuntimeType,
        Interpreter, LegacyValue, MResult, MechError, MechFunction, MechFunctionCompiler,
        MechFunctionFactory, MechFunctionImpl, ParsedProgram, Ref, Register,
        RuntimeFunctionContract, RuntimeFunctionId, RuntimeFunctionSignature,
        RuntimeOutputAliasPolicy, RuntimeType, ToValue, write_bytecode,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    const PRODUCER: &str = "BytecodeComputedU8";
    const INCREMENTING_PRODUCER: &str = "BytecodeIncrementingU8";

    #[derive(Debug)]
    struct ComputedU8 {
        output: Ref<u8>,
    }

    impl MechFunctionFactory for ComputedU8 {
        const SIGNATURE: RuntimeFunctionSignature =
            RuntimeFunctionSignature::nullary(<u8 as FunctionRuntimeType>::REPRESENTATION);

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

    impl MechFunctionImpl for ComputedU8 {
        fn solve_result(&self) -> MResult<()> {
            *self.output.borrow_mut() = 7;
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            self.output.to_value()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            PRODUCER.to_string()
        }
    }

    impl MechFunctionCompiler for ComputedU8 {
        fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    #[derive(Debug)]
    struct IncrementingU8 {
        output: Ref<u8>,
    }

    impl MechFunctionFactory for IncrementingU8 {
        const SIGNATURE: RuntimeFunctionSignature =
            RuntimeFunctionSignature::nullary(<u8 as FunctionRuntimeType>::REPRESENTATION);

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

    impl MechFunctionImpl for IncrementingU8 {
        fn solve_result(&self) -> MResult<()> {
            *self.output.borrow_mut() += 1;
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            self.output.to_value()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            INCREMENTING_PRODUCER.to_string()
        }
    }

    impl MechFunctionCompiler for IncrementingU8 {
        fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn constant(runtime_type: RuntimeType, alignment: u8, bytes: Vec<u8>) -> EncodedConstant {
        EncodedConstant {
            runtime_type,
            alignment,
            bytes,
        }
    }

    fn child(bytes: &mut Vec<u8>, payload: &[u8]) {
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
    }

    fn run_interpreter(program: BytecodeProgram) -> MResult<(Interpreter, LegacyValue)> {
        let mut catalog = FunctionCatalogBuilder::new();
        catalog
            .insert_runtime_factory::<ComputedU8>(
                PRODUCER,
                RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            )
            .unwrap();
        catalog
            .insert_runtime_factory::<IncrementingU8>(
                INCREMENTING_PRODUCER,
                RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            )
            .unwrap();
        let mut interpreter =
            Interpreter::with_function_catalog(1, 100, Arc::new(catalog.build().unwrap()));
        let parsed = ParsedProgram::from_bytes(&write_bytecode(&program).unwrap()).unwrap();
        let output = interpreter.run_program(&parsed)?;
        Ok((interpreter, output))
    }

    fn run_result(program: BytecodeProgram) -> MResult<LegacyValue> {
        run_interpreter(program).map(|(_, output)| output)
    }

    fn run(program: BytecodeProgram) -> LegacyValue {
        run_result(program).unwrap()
    }

    #[test]
    fn computed_map_keys_are_rehashed_after_their_producer_solves() {
        let mut template = 1_u32.to_le_bytes().to_vec();
        child(&mut template, &[1]);
        child(&mut template, &[9]);
        let output = run(BytecodeProgram {
            register_count: 3,
            constants: vec![
                constant(RuntimeType::U8, 1, vec![1]),
                constant(RuntimeType::U8, 1, vec![9]),
                constant(
                    RuntimeType::Map {
                        key: Box::new(RuntimeType::U8),
                        value: Box::new(RuntimeType::U8),
                    },
                    4,
                    template,
                ),
            ],
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::RuntimeNullary {
                    function: RuntimeFunctionId::from_name(PRODUCER).raw(),
                    dst: 0,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 1,
                    constant: 1,
                },
                BytecodeInstruction::CompositePack {
                    dst: 2,
                    template: 2,
                    children: vec![0, 1],
                },
                BytecodeInstruction::Return { src: 2 },
            ],
            dictionary: BTreeMap::new(),
            requirements: Vec::new(),
        });

        let LegacyValue::Map(output) = output else {
            panic!("expected map output");
        };
        let key = LegacyValue::U8(Ref::new(7));
        assert_eq!(
            output.borrow().map.get(&key),
            Some(&LegacyValue::U8(Ref::new(9)))
        );
        assert!(
            !output
                .borrow()
                .map
                .contains_key(&LegacyValue::U8(Ref::new(1)))
        );
    }

    #[test]
    fn captured_map_values_remain_attached_across_reactive_rebuilds() {
        let mut template = 1_u32.to_le_bytes().to_vec();
        child(&mut template, &[1]);
        child(&mut template, &[2]);
        let (mut interpreter, output) = run_interpreter(BytecodeProgram {
            register_count: 3,
            constants: vec![
                constant(RuntimeType::U8, 1, vec![1]),
                constant(RuntimeType::U8, 1, vec![2]),
                constant(
                    RuntimeType::Map {
                        key: Box::new(RuntimeType::U8),
                        value: Box::new(RuntimeType::U8),
                    },
                    4,
                    template,
                ),
            ],
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
                BytecodeInstruction::RuntimeNullary {
                    function: RuntimeFunctionId::from_name(INCREMENTING_PRODUCER).raw(),
                    dst: 1,
                },
                BytecodeInstruction::CompositePack {
                    dst: 2,
                    template: 2,
                    children: vec![0, 1],
                },
                BytecodeInstruction::Return { src: 2 },
            ],
            dictionary: BTreeMap::new(),
            requirements: Vec::new(),
        })
        .unwrap();

        let LegacyValue::Map(output) = output else {
            panic!("expected map output");
        };
        let key = LegacyValue::U8(Ref::new(1));
        let captured = output.borrow().map.get(&key).unwrap().clone();
        assert_eq!(captured, LegacyValue::U8(Ref::new(3)));

        interpreter.solve_plan().unwrap();

        assert_eq!(captured, LegacyValue::U8(Ref::new(4)));
        assert_eq!(
            output.borrow().map.get(&key),
            Some(&LegacyValue::U8(Ref::new(4)))
        );
    }

    #[test]
    fn computed_set_elements_are_rehashed_after_their_producer_solves() {
        let mut template = 1_u32.to_le_bytes().to_vec();
        child(&mut template, &[1]);
        let output = run(BytecodeProgram {
            register_count: 2,
            constants: vec![
                constant(RuntimeType::U8, 1, vec![1]),
                constant(
                    RuntimeType::Set {
                        element: Box::new(RuntimeType::U8),
                        max_len: Some(1),
                    },
                    4,
                    template,
                ),
            ],
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::RuntimeNullary {
                    function: RuntimeFunctionId::from_name(PRODUCER).raw(),
                    dst: 0,
                },
                BytecodeInstruction::CompositePack {
                    dst: 1,
                    template: 1,
                    children: vec![0],
                },
                BytecodeInstruction::Return { src: 1 },
            ],
            dictionary: BTreeMap::new(),
            requirements: Vec::new(),
        });

        let LegacyValue::Set(output) = output else {
            panic!("expected set output");
        };
        assert!(output.borrow().set.contains(&LegacyValue::U8(Ref::new(7))));
        assert!(!output.borrow().set.contains(&LegacyValue::U8(Ref::new(1))));
    }

    #[test]
    fn computed_map_key_collisions_fail_instead_of_overwriting_an_entry() {
        let mut template = 2_u32.to_le_bytes().to_vec();
        for payload in [[1], [9], [2], [10]] {
            child(&mut template, &payload);
        }
        let error = run_result(BytecodeProgram {
            register_count: 5,
            constants: vec![
                constant(RuntimeType::U8, 1, vec![1]),
                constant(RuntimeType::U8, 1, vec![2]),
                constant(RuntimeType::U8, 1, vec![9]),
                constant(RuntimeType::U8, 1, vec![10]),
                constant(
                    RuntimeType::Map {
                        key: Box::new(RuntimeType::U8),
                        value: Box::new(RuntimeType::U8),
                    },
                    4,
                    template,
                ),
            ],
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::RuntimeNullary {
                    function: RuntimeFunctionId::from_name(PRODUCER).raw(),
                    dst: 0,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 1,
                    constant: 1,
                },
                BytecodeInstruction::RuntimeNullary {
                    function: RuntimeFunctionId::from_name(PRODUCER).raw(),
                    dst: 1,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 2,
                    constant: 2,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 3,
                    constant: 3,
                },
                BytecodeInstruction::CompositePack {
                    dst: 4,
                    template: 4,
                    children: vec![0, 2, 1, 3],
                },
                BytecodeInstruction::Return { src: 4 },
            ],
            dictionary: BTreeMap::new(),
            requirements: Vec::new(),
        })
        .unwrap_err();

        assert_eq!(error.kind_name(), "BytecodeValidation");
        assert!(
            error
                .kind_message()
                .contains("duplicate-equal hashed children")
        );
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
        ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction, BytecodeProgram,
        EncodedConstant, ExecutionHostFunctionRequest, ExecutionResourceRequest,
        ExternalHostCallFunction, ExternalResourceReadFunction, ExternalResourceWriteFunction,
        FunctionArgs, FunctionArgumentRole, FunctionCatalog, FunctionCatalogBuilder,
        FunctionRuntimeType, InitialSolvePolicy, LegacyValue, MResult, MechError,
        MechExecutionServices, MechFunction, MechFunctionCompiler, MechFunctionFactory,
        MechFunctionImpl, MechProgram, MechProgramConfig, ParsedProgram, ReactiveCellId, Ref,
        Register, ResourceDelivery, ResourceIntent, RuntimeFunctionContract, RuntimeFunctionId,
        RuntimeFunctionSignature, RuntimeOutputAliasPolicy, RuntimeType, ToValue, ValRef,
        apply_stable_value_update, hash_str, write_bytecode,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    #[derive(Debug)]
    struct CopyString {
        output: Ref<String>,
        input: Ref<String>,
    }

    impl MechFunctionFactory for CopyString {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
            <String as FunctionRuntimeType>::REPRESENTATION,
            <String as FunctionRuntimeType>::REPRESENTATION,
        );

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            match args {
                FunctionArgs::Unary(output, input) => Ok(Box::new(Self {
                    output: output.try_function_ref(FunctionArgumentRole::Output)?,
                    input: input.try_function_ref(FunctionArgumentRole::Input(0))?,
                })),
                _ => Err(MechError::new(
                    super::super::super::IncorrectNumberOfArguments {
                        expected: 1,
                        found: args.len(),
                    },
                    None,
                )),
            }
        }
    }

    impl MechFunctionImpl for CopyString {
        fn solve_result(&self) -> MResult<()> {
            *self.output.borrow_mut() = self.input.borrow().clone();
            Ok(())
        }

        fn out(&self) -> LegacyValue {
            self.output.to_value()
        }

        fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
            Ok(self.reactive_output_values())
        }

        fn to_string(&self) -> String {
            "CopyString".into()
        }
    }

    impl MechFunctionCompiler for CopyString {
        fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn copy_string_catalog() -> Arc<FunctionCatalog> {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory::<CopyString>(
                "CopyString",
                RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            )
            .unwrap();
        Arc::new(builder.build().unwrap())
    }

    #[derive(Default)]
    struct RecordingExternalServices {
        host_requests: Vec<ExecutionHostFunctionRequest>,
        host_arguments: Vec<Vec<LegacyValue>>,
        read_requests: Vec<ExecutionResourceRequest>,
        writes: Vec<(ExecutionResourceRequest, LegacyValue)>,
        bindings: Vec<(u64, ExecutionResourceRequest, usize)>,
        binding_targets: Vec<ValRef>,
        host_result: Option<LegacyValue>,
        read_result: Option<LegacyValue>,
    }

    impl MechExecutionServices for RecordingExternalServices {
        fn invoke_host_function(
            &mut self,
            request: &ExecutionHostFunctionRequest,
            arguments: &[LegacyValue],
        ) -> MResult<LegacyValue> {
            self.host_requests.push(request.clone());
            self.host_arguments.push(
                arguments
                    .iter()
                    .map(LegacyValue::try_deep_snapshot)
                    .collect::<MResult<Vec<_>>>()?,
            );
            Ok(self
                .host_result
                .clone()
                .unwrap_or_else(|| LegacyValue::F64(Ref::new(9.0))))
        }

        fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
            self.read_requests.push(request.clone());
            Ok(self
                .read_result
                .clone()
                .unwrap_or_else(|| LegacyValue::F64(Ref::new(8.0))))
        }

        fn write_resource(
            &mut self,
            request: &ExecutionResourceRequest,
            value: &LegacyValue,
        ) -> MResult<()> {
            self.writes
                .push((request.clone(), value.try_deep_snapshot()?));
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
            self.binding_targets.push(target);
            Ok(())
        }
    }

    fn string_constant(value: &str) -> EncodedConstant {
        EncodedConstant {
            runtime_type: RuntimeType::String,
            alignment: 1,
            bytes: value.as_bytes().to_vec(),
        }
    }

    fn parse_program(
        register_count: u32,
        constants: Vec<EncodedConstant>,
        instructions: Vec<BytecodeInstruction>,
        requirements: Vec<ApplicationRequirement>,
        symbol_bindings: &[(&str, u32)],
    ) -> ParsedProgram {
        let symbols = symbol_bindings
            .iter()
            .map(|(name, register)| (hash_str(name), *register))
            .collect::<BTreeMap<_, _>>();
        let dictionary = symbol_bindings
            .iter()
            .map(|(name, _)| (hash_str(name), (*name).to_owned()))
            .collect();
        ParsedProgram::from_bytes(
            &write_bytecode(&BytecodeProgram {
                register_count,
                constants,
                symbols,
                mutable_symbols: BTreeSet::new(),
                instructions,
                dictionary,
                requirements,
            })
            .unwrap(),
        )
        .unwrap()
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

    fn string_value(value: &str) -> LegacyValue {
        LegacyValue::String(Ref::new(value.to_owned()))
    }

    fn assert_string(value: &LegacyValue, expected: &str) {
        assert!(
            matches!(value, LegacyValue::String(value) if value.borrow().as_str() == expected),
            "expected String({expected:?}), found {value:?}",
        );
    }

    fn symbol_cell(program: &MechProgram, name: &str) -> ValRef {
        program
            .interpreter()
            .symbols()
            .borrow()
            .get(hash_str(name))
            .unwrap_or_else(|| panic!("missing bytecode symbol {name:?}"))
    }

    fn host_program(
        register_count: u32,
        instructions: Vec<BytecodeInstruction>,
        symbols: &[(&str, u32)],
    ) -> ParsedProgram {
        parse_program(
            register_count,
            vec![string_constant("seed")],
            instructions,
            vec![ApplicationRequirement::HostFunction(
                ExecutionHostFunctionRequest {
                    name: "test/host".into(),
                },
            )],
            symbols,
        )
    }

    fn resource_program(
        register_count: u32,
        delivery: ResourceDelivery,
        instructions: Vec<BytecodeInstruction>,
        symbols: &[(&str, u32)],
    ) -> ParsedProgram {
        parse_program(
            register_count,
            vec![string_constant("seed")],
            instructions,
            vec![ApplicationRequirement::Resource(request(
                ResourceIntent::Read,
                delivery,
            ))],
            symbols,
        )
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
        let host_argument = LegacyValue::F64(Ref::new(3.0));
        let host_request = ExecutionHostFunctionRequest {
            name: "test/host".into(),
        };
        let read_request = request(ResourceIntent::Read, ResourceDelivery::Live);
        let assign_request = request(ResourceIntent::Assign, ResourceDelivery::Snapshot);
        let send_request = request(ResourceIntent::Send, ResourceDelivery::Snapshot);

        let host_output = Ref::new(LegacyValue::F64(Ref::new(1.0)));
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
        let read_output = Ref::new(LegacyValue::F64(Ref::new(2.0)));
        plan.register_function(
            Box::new(ExternalResourceReadFunction {
                interpreter_id: source.interpreter().id,
                request: read_request.clone(),
                output: read_output.clone(),
                initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
                semantic_contract: None,
            }),
            &[],
        )
        .unwrap();
        let assigned = host_output.borrow().clone();
        plan.register_function(
            Box::new(ExternalResourceWriteFunction {
                request: assign_request.clone(),
                input: assigned.clone(),
                output: Ref::new(LegacyValue::Empty),
                initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
                semantic_contract: None,
            }),
            &[assigned],
        )
        .unwrap();
        let sent = read_output.borrow().clone();
        plan.register_function(
            Box::new(ExternalResourceWriteFunction {
                request: send_request.clone(),
                input: sent.clone(),
                output: Ref::new(LegacyValue::Empty),
                initial_solve_policy: InitialSolvePolicy::PreserveSpecializedOutput,
                semantic_contract: None,
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

        assert_eq!(result, LegacyValue::Empty);
        assert_eq!(services.host_requests, vec![host_request]);
        assert!(matches!(
            services.host_arguments.as_slice(),
            [arguments]
                if matches!(arguments.as_slice(), [LegacyValue::F64(value)] if *value.borrow() == 3.0)
        ));
        assert_eq!(services.read_requests, vec![read_request.clone()]);
        assert_eq!(services.writes.len(), 2);
        assert_eq!(services.writes[0].0, assign_request);
        assert_eq!(services.writes[1].0, send_request);
        assert!(matches!(&services.writes[0].1, LegacyValue::F64(value) if *value.borrow() == 9.0));
        assert!(matches!(&services.writes[1].1, LegacyValue::F64(value) if *value.borrow() == 8.0));
        assert_eq!(services.bindings.len(), 1);
        assert_eq!(services.bindings[0].0, loaded_interpreter_id);
        assert_eq!(services.bindings[0].1, read_request);
        assert_eq!(
            source_shape,
            vec![
                (1, 1, "ExternalHostCallFunction".into()),
                (0, 1, "ExternalResourceReadFunction".into()),
                (1, 0, "ExternalResourceWriteFunction".into()),
                (1, 0, "ExternalResourceWriteFunction".into()),
            ]
        );
        // Functions retain stable register wrappers for execution, while the
        // reactive graph records the logical payload dependencies represented
        // by the source plan.
        assert_eq!(plan_shape(&loaded), source_shape);

        let live_target = services.bindings[0].2;
        loaded
            .interpreter_mut()
            .solve_plan_with_services(&mut services)
            .unwrap();
        assert_eq!(services.bindings.len(), 2);
        assert_eq!(services.bindings[1].2, live_target);
        assert!(matches!(&services.writes[2].1, LegacyValue::F64(value) if *value.borrow() == 9.0));
        assert!(matches!(&services.writes[3].1, LegacyValue::F64(value) if *value.borrow() == 8.0));
    }

    #[test]
    fn host_return_and_symbols_observe_the_actual_external_result() {
        let parsed = host_program(
            1,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::HostCall {
                    requirement: 0,
                    dst: 0,
                    arguments: Vec::new(),
                },
                BytecodeInstruction::Return { src: 0 },
            ],
            &[("host-result", 0), ("host-alias", 0)],
        );
        let mut program = MechProgram::new(MechProgramConfig::default());
        let mut services = RecordingExternalServices {
            host_result: Some(string_value("host-actual")),
            ..Default::default()
        };

        let result = program
            .run_bytecode_program_with_services(&parsed, &mut services)
            .unwrap();

        assert_string(&result, "host-actual");
        let register = program.interpreter().bytecode_registers.cell(0).unwrap();
        let result_symbol = symbol_cell(&program, "host-result");
        let alias_symbol = symbol_cell(&program, "host-alias");
        assert!(result_symbol.same_handle(&register));
        assert!(alias_symbol.same_handle(&register));
        assert_string(&result_symbol.borrow(), "host-actual");
    }

    #[test]
    fn resource_return_and_symbol_observe_the_actual_external_result() {
        let parsed = resource_program(
            1,
            ResourceDelivery::Snapshot,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::ResourceRead {
                    requirement: 0,
                    dst: 0,
                },
                BytecodeInstruction::Return { src: 0 },
            ],
            &[("resource-result", 0)],
        );
        let mut program = MechProgram::new(MechProgramConfig::default());
        let mut services = RecordingExternalServices {
            read_result: Some(string_value("resource-actual")),
            ..Default::default()
        };

        let result = program
            .run_bytecode_program_with_services(&parsed, &mut services)
            .unwrap();

        assert_string(&result, "resource-actual");
        let register = program.interpreter().bytecode_registers.cell(0).unwrap();
        let symbol = symbol_cell(&program, "resource-result");
        assert!(symbol.same_handle(&register));
        assert_string(&symbol.borrow(), "resource-actual");
    }

    #[test]
    fn later_runtime_nodes_consume_host_and_resource_results() {
        let cases = [
            (
                host_program(
                    2,
                    vec![
                        BytecodeInstruction::ConstLoad {
                            dst: 0,
                            constant: 0,
                        },
                        BytecodeInstruction::ConstLoad {
                            dst: 1,
                            constant: 0,
                        },
                        BytecodeInstruction::HostCall {
                            requirement: 0,
                            dst: 0,
                            arguments: Vec::new(),
                        },
                        BytecodeInstruction::RuntimeUnary {
                            function: RuntimeFunctionId::from_name("CopyString").raw(),
                            dst: 1,
                            src: 0,
                        },
                        BytecodeInstruction::Return { src: 1 },
                    ],
                    &[("host-input", 0), ("host-copy", 1)],
                ),
                true,
                "host-pipeline",
            ),
            (
                resource_program(
                    2,
                    ResourceDelivery::Snapshot,
                    vec![
                        BytecodeInstruction::ConstLoad {
                            dst: 0,
                            constant: 0,
                        },
                        BytecodeInstruction::ConstLoad {
                            dst: 1,
                            constant: 0,
                        },
                        BytecodeInstruction::ResourceRead {
                            requirement: 0,
                            dst: 0,
                        },
                        BytecodeInstruction::RuntimeUnary {
                            function: RuntimeFunctionId::from_name("CopyString").raw(),
                            dst: 1,
                            src: 0,
                        },
                        BytecodeInstruction::Return { src: 1 },
                    ],
                    &[("resource-input", 0), ("resource-copy", 1)],
                ),
                false,
                "resource-pipeline",
            ),
        ];

        for (parsed, is_host, actual) in cases {
            let mut program = MechProgram::with_function_catalog(
                MechProgramConfig::default(),
                copy_string_catalog(),
            );
            let mut services = RecordingExternalServices {
                host_result: Some(string_value(actual)),
                read_result: Some(string_value(actual)),
                ..Default::default()
            };

            let result = program
                .run_bytecode_program_with_services(&parsed, &mut services)
                .unwrap();

            assert_string(&result, actual);
            let copied = symbol_cell(
                &program,
                if is_host {
                    "host-copy"
                } else {
                    "resource-copy"
                },
            );
            assert_string(&copied.borrow(), actual);
        }
    }

    #[test]
    fn live_resource_updates_keep_register_identity_and_rerun_dependents() {
        let parsed = resource_program(
            2,
            ResourceDelivery::Live,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 1,
                    constant: 0,
                },
                BytecodeInstruction::ResourceRead {
                    requirement: 0,
                    dst: 0,
                },
                BytecodeInstruction::RuntimeUnary {
                    function: RuntimeFunctionId::from_name("CopyString").raw(),
                    dst: 1,
                    src: 0,
                },
                BytecodeInstruction::Return { src: 1 },
            ],
            &[("live-input", 0), ("live-copy", 1)],
        );
        let mut program =
            MechProgram::with_function_catalog(MechProgramConfig::default(), copy_string_catalog());
        let interpreter_id = program.interpreter().id;
        let mut services = RecordingExternalServices {
            read_result: Some(string_value("live-initial")),
            ..Default::default()
        };

        let result = program
            .run_bytecode_program_with_services(&parsed, &mut services)
            .unwrap();
        assert_string(&result, "live-initial");
        let input_register = program.interpreter().bytecode_registers.cell(0).unwrap();
        let output_register = program.interpreter().bytecode_registers.cell(1).unwrap();
        let input_symbol = symbol_cell(&program, "live-input");
        let output_symbol = symbol_cell(&program, "live-copy");
        let live_target = services.binding_targets.first().unwrap().clone();
        assert!(live_target.same_handle(&input_register));
        assert!(input_symbol.same_handle(&input_register));
        assert!(output_symbol.same_handle(&output_register));
        let input_value = input_register.borrow().as_string().unwrap();
        let output_value = output_register.borrow().as_string().unwrap();

        apply_stable_value_update(live_target.clone(), string_value("live-updated")).unwrap();
        let dirty_cells = live_target.borrow().reactive_root_cell_ids();
        program
            .advance_reactive_turn_with_services(interpreter_id, &dirty_cells, &mut services)
            .unwrap();

        assert!(
            input_register.same_handle(&program.interpreter().bytecode_registers.cell(0).unwrap())
        );
        assert!(
            output_register.same_handle(&program.interpreter().bytecode_registers.cell(1).unwrap())
        );
        assert_eq!(
            input_value.addr(),
            input_register.borrow().as_string().unwrap().addr()
        );
        assert_eq!(
            output_value.addr(),
            output_register.borrow().as_string().unwrap().addr()
        );
        assert_string(&output_symbol.borrow(), "live-updated");
    }

    #[test]
    fn failed_bytecode_install_restores_register_and_symbol_cells() {
        let prior = parse_program(
            1,
            vec![string_constant("prior")],
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::Return { src: 0 },
            ],
            Vec::new(),
            &[("prior", 0)],
        );
        let failing = host_program(
            1,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::HostCall {
                    requirement: 0,
                    dst: 0,
                    arguments: Vec::new(),
                },
                BytecodeInstruction::Return { src: 0 },
            ],
            &[("replacement", 0)],
        );
        let mut program = MechProgram::new(MechProgramConfig::default());
        program.run_bytecode_program(&prior).unwrap();
        let prior_register = program.interpreter().bytecode_registers.cell(0).unwrap();
        let prior_symbol = symbol_cell(&program, "prior");
        assert!(prior_symbol.same_handle(&prior_register));
        let mut services = RecordingExternalServices {
            host_result: Some(LegacyValue::F64(Ref::new(9.0))),
            ..Default::default()
        };

        let error = program
            .run_bytecode_program_with_services(&failing, &mut services)
            .unwrap_err();

        assert_eq!(error.kind_name(), "StableValueUpdateContractViolation");
        let restored_register = program.interpreter().bytecode_registers.cell(0).unwrap();
        let restored_symbol = symbol_cell(&program, "prior");
        assert!(restored_register.same_handle(&prior_register));
        assert!(restored_symbol.same_handle(&prior_symbol));
        assert!(restored_symbol.same_handle(&restored_register));
        assert_string(&restored_register.borrow(), "prior");
        assert!(
            program
                .interpreter()
                .symbols()
                .borrow()
                .get(hash_str("replacement"))
                .is_none()
        );
    }
}
