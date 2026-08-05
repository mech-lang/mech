#[cfg(feature = "no_std")]
use alloc::{format, string::String, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{format, string::String, vec, vec::Vec};

use crate::{
    ApplicationRequirement, FunctionArgs, FunctionCatalog, MResult, MechError, MechErrorKind,
    ResourceIntent, RuntimeFunctionId, Value,
};

use super::{BytecodeInstruction, ParsedProgram};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeRuntimeContractViolation {
    pub instruction: u32,
    pub function_id: Option<u64>,
    pub function_name: Option<String>,
    pub reason: String,
}

impl MechErrorKind for BytecodeRuntimeContractViolation {
    fn name(&self) -> &str {
        "BytecodeRuntimeContractViolation"
    }

    fn message(&self) -> String {
        let function = match (&self.function_name, self.function_id) {
            (Some(name), Some(id)) => format!(" for runtime function {name} (0x{id:016x})"),
            (None, Some(id)) => format!(" for runtime function 0x{id:016x}"),
            _ => String::new(),
        };
        format!(
            "bytecode instruction {}{} violates its runtime contract: {}",
            self.instruction, function, self.reason
        )
    }
}

fn violation(
    instruction: usize,
    function_id: Option<u64>,
    function_name: Option<String>,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        BytecodeRuntimeContractViolation {
            instruction: u32::try_from(instruction).unwrap_or(u32::MAX),
            function_id,
            function_name,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn violation_with_source(
    instruction: usize,
    function_id: Option<u64>,
    function_name: Option<String>,
    source: MechError,
) -> MechError {
    violation(
        instruction,
        function_id,
        function_name,
        source.simple_message(),
    )
    .with_source(source)
}

impl ParsedProgram {
    /// Validates catalog-aware bytecode contracts without mutating interpreter
    /// or runtime state.
    pub fn validate_runtime_contracts(&self, catalog: &FunctionCatalog) -> MResult<()> {
        let constants = self
            .decode_constants()
            .map_err(|error| violation_with_source(0, None, None, error))?;
        let mut registers = vec![None::<Value>; self.header.register_count as usize];

        for (instruction_index, instruction) in self.instructions.iter().enumerate() {
            if let BytecodeInstruction::ConstLoad { dst, constant } = instruction {
                let value = constants
                    .get(*constant as usize)
                    .ok_or_else(|| {
                        violation(
                            instruction_index,
                            None,
                            None,
                            format!("constant {constant} is out of range"),
                        )
                    })?
                    .try_deep_snapshot()
                    .map_err(|error| violation_with_source(instruction_index, None, None, error))?;
                let destination = registers.get_mut(*dst as usize).ok_or_else(|| {
                    violation(
                        instruction_index,
                        None,
                        None,
                        format!("register {dst} is out of range"),
                    )
                })?;
                *destination = Some(value);
                continue;
            }

            let register = |index: u32| -> MResult<Value> {
                registers
                    .get(index as usize)
                    .and_then(Option::as_ref)
                    .cloned()
                    .ok_or_else(|| {
                        violation(
                            instruction_index,
                            instruction.runtime_function(),
                            None,
                            format!("register {index} has no detached constant seed"),
                        )
                    })
            };

            match instruction {
                BytecodeInstruction::ConstLoad { .. } => unreachable!(),
                BytecodeInstruction::RuntimeNullary { function, dst } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionArgs::Nullary(register(*dst)?),
                    )?;
                }
                BytecodeInstruction::RuntimeUnary { function, dst, src } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionArgs::Unary(register(*dst)?, register(*src)?),
                    )?;
                }
                BytecodeInstruction::RuntimeBinary {
                    function,
                    dst,
                    lhs,
                    rhs,
                } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionArgs::Binary(register(*dst)?, register(*lhs)?, register(*rhs)?),
                    )?;
                }
                BytecodeInstruction::RuntimeTernary {
                    function,
                    dst,
                    a,
                    b,
                    c,
                } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionArgs::Ternary(
                            register(*dst)?,
                            register(*a)?,
                            register(*b)?,
                            register(*c)?,
                        ),
                    )?;
                }
                BytecodeInstruction::RuntimeQuaternary {
                    function,
                    dst,
                    a,
                    b,
                    c,
                    d,
                } => {
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionArgs::Quaternary(
                            register(*dst)?,
                            register(*a)?,
                            register(*b)?,
                            register(*c)?,
                            register(*d)?,
                        ),
                    )?;
                }
                BytecodeInstruction::RuntimeVariadic {
                    function,
                    dst,
                    arguments,
                } => {
                    let arguments = arguments
                        .iter()
                        .map(|argument| register(*argument))
                        .collect::<MResult<Vec<_>>>()?;
                    self.validate_runtime_instruction(
                        catalog,
                        instruction_index,
                        *function,
                        FunctionArgs::Variadic(register(*dst)?, arguments),
                    )?;
                }
                BytecodeInstruction::HostCall {
                    requirement,
                    dst,
                    arguments,
                } => {
                    if !matches!(
                        self.requirements.get(*requirement as usize),
                        Some(ApplicationRequirement::HostFunction(_))
                    ) {
                        return Err(violation(
                            instruction_index,
                            None,
                            None,
                            format!("HostCall requirement {requirement} must be a HostFunction"),
                        ));
                    }
                    register(*dst)?;
                    for argument in arguments {
                        register(*argument)?;
                    }
                }
                BytecodeInstruction::ResourceRead { requirement, dst } => {
                    self.validate_resource_requirement(
                        instruction_index,
                        *requirement,
                        ResourceIntent::Read,
                    )?;
                    register(*dst)?;
                }
                BytecodeInstruction::ResourceWrite {
                    requirement,
                    dst,
                    src,
                } => {
                    self.validate_resource_requirement(
                        instruction_index,
                        *requirement,
                        ResourceIntent::Assign,
                    )?;
                    let output = register(*dst)?;
                    register(*src)?;
                    self.validate_resource_write_seed(instruction_index, *dst, output)?;
                }
                BytecodeInstruction::ResourceSend {
                    requirement,
                    dst,
                    src,
                } => {
                    self.validate_resource_requirement(
                        instruction_index,
                        *requirement,
                        ResourceIntent::Send,
                    )?;
                    let output = register(*dst)?;
                    register(*src)?;
                    self.validate_resource_write_seed(instruction_index, *dst, output)?;
                }
                BytecodeInstruction::Return { .. } => {}
            }
        }
        Ok(())
    }

    fn validate_runtime_instruction(
        &self,
        catalog: &FunctionCatalog,
        instruction: usize,
        function: u64,
        args: FunctionArgs,
    ) -> MResult<()> {
        let id = RuntimeFunctionId::from_raw(function);
        let entry = catalog.runtime_entry(id).ok_or_else(|| {
            violation(
                instruction,
                Some(function),
                None,
                "runtime function is absent from the trusted catalog",
            )
        })?;
        entry.validate_args(&args).map_err(|error| {
            violation_with_source(instruction, Some(function), Some(entry.name.clone()), error)
        })
    }

    fn validate_resource_requirement(
        &self,
        instruction: usize,
        requirement: u32,
        expected: ResourceIntent,
    ) -> MResult<()> {
        match self.requirements.get(requirement as usize) {
            Some(ApplicationRequirement::Resource(request)) if request.intent == expected => Ok(()),
            actual => Err(violation(
                instruction,
                None,
                None,
                format!(
                    "resource requirement {requirement} must have intent {expected:?}, found {actual:?}"
                ),
            )),
        }
    }

    fn validate_resource_write_seed(
        &self,
        instruction: usize,
        destination: u32,
        output: Value,
    ) -> MResult<()> {
        if output == Value::Empty {
            return Ok(());
        }
        Err(violation(
            instruction,
            None,
            None,
            format!(
                "resource write/send destination register {destination} must have an Empty seed, found {:?}",
                output.kind(),
            ),
        ))
    }
}

#[cfg(all(test, feature = "f64"))]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::{
        ApplicationRequirement, BytecodeProgram, EncodedConstant, ExecutionHostFunctionRequest,
        ExecutionResourceRequest, FunctionArgumentRole, FunctionCatalogBuilder,
        FunctionRuntimeType, FunctionValueRepresentation, MechFunction, MechFunctionFactory,
        MechFunctionImpl, Ref, ResourceDelivery, RuntimeFunctionSignature, RuntimeType, ToValue,
        write_bytecode,
    };
    #[cfg(feature = "compiler")]
    use crate::{BytecodeCompilerContext, MechFunctionCompiler, Register};

    #[derive(Debug)]
    struct ExactF64Binary {
        out: Ref<f64>,
    }

    impl MechFunctionFactory for ExactF64Binary {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
            <f64 as FunctionRuntimeType>::REPRESENTATION,
            <f64 as FunctionRuntimeType>::REPRESENTATION,
            <f64 as FunctionRuntimeType>::REPRESENTATION,
        );

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            match args {
                FunctionArgs::Binary(out, lhs, rhs) => {
                    let out = out.try_function_ref(FunctionArgumentRole::Output)?;
                    let _: Ref<f64> = lhs.try_function_ref(FunctionArgumentRole::Input(0))?;
                    let _: Ref<f64> = rhs.try_function_ref(FunctionArgumentRole::Input(1))?;
                    Ok(Box::new(Self { out }))
                }
                _ => Err(MechError::new(
                    crate::IncorrectNumberOfArguments {
                        expected: 2,
                        found: args.len(),
                    },
                    None,
                )),
            }
        }
    }

    impl MechFunctionImpl for ExactF64Binary {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn out(&self) -> Value {
            self.out.to_value()
        }

        fn to_string(&self) -> String {
            "ExactF64Binary".into()
        }

        fn transaction_state_values(&self) -> MResult<Vec<Value>> {
            Ok(self.reactive_output_values())
        }
    }

    #[cfg(feature = "compiler")]
    impl MechFunctionCompiler for ExactF64Binary {
        fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    fn scalar(runtime_type: RuntimeType, bytes: Vec<u8>) -> EncodedConstant {
        let alignment = if runtime_type == RuntimeType::I8 {
            1
        } else {
            8
        };
        EncodedConstant {
            runtime_type,
            alignment,
            bytes,
        }
    }

    fn parsed_runtime_program(
        constants: Vec<EncodedConstant>,
        instruction: BytecodeInstruction,
    ) -> ParsedProgram {
        let mut instructions = constants
            .iter()
            .enumerate()
            .map(|(register, _)| BytecodeInstruction::ConstLoad {
                dst: register as u32,
                constant: register as u32,
            })
            .collect::<Vec<_>>();
        instructions.push(instruction);
        instructions.push(BytecodeInstruction::Return { src: 0 });
        ParsedProgram::from_bytes(
            &write_bytecode(&BytecodeProgram {
                register_count: constants.len() as u32,
                constants,
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions,
                dictionary: BTreeMap::new(),
                requirements: Vec::new(),
            })
            .unwrap(),
        )
        .unwrap()
    }

    fn catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory::<ExactF64Binary>(
                "ExactF64Binary",
                crate::RuntimeFunctionContract::no_matrix(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            )
            .unwrap();
        builder.build().unwrap()
    }

    fn assert_contract_violation(program: &ParsedProgram, expected: &str) {
        let error = program.validate_runtime_contracts(&catalog()).unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(
            error.kind_message().contains(expected),
            "expected `{expected}` in `{}`",
            error.kind_message()
        );
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    fn f64_matrix_constant(storage: crate::MatrixStorage, rows: u32, cols: u32) -> EncodedConstant {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&rows.to_le_bytes());
        bytes.extend_from_slice(&cols.to_le_bytes());
        for index in 0..rows.saturating_mul(cols) {
            let value = f64::from(index + 1);
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        EncodedConstant {
            runtime_type: RuntimeType::Matrix {
                element: Box::new(RuntimeType::F64),
                storage,
                rows,
                cols,
            },
            alignment: 8,
            bytes,
        }
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    struct FactoryMustNotRun;

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    impl MechFunctionFactory for FactoryMustNotRun {
        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        );

        fn new(_args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            panic!("shape and alias contracts must reject before invoking the factory")
        }
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd", feature = "vectord"))]
    #[test]
    fn malicious_matrix_relations_and_output_aliases_fail_as_bytecode_contracts() {
        use crate::MatrixStorage;

        let mut catalog = FunctionCatalogBuilder::new();
        for (name, contract) in [
            (
                "AddMDMD<f64>",
                crate::RuntimeFunctionContract::same_shape(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
            (
                "AddM2M2<f64>",
                crate::RuntimeFunctionContract::same_shape(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
            (
                "MatMulMDMD<f64>",
                crate::RuntimeFunctionContract::matrix_product(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
            (
                "MatrixSolveMDVD<f64>",
                crate::RuntimeFunctionContract::linear_solve(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            ),
        ] {
            catalog
                .insert_runtime_factory::<FactoryMustNotRun>(name, contract)
                .unwrap();
        }
        let catalog = catalog.build().unwrap();

        let dynamic = |rows, cols| f64_matrix_constant(MatrixStorage::MatrixD, rows, cols);
        let fixed2 = || f64_matrix_constant(MatrixStorage::Matrix2, 2, 2);
        let cases = [
            (
                "AddMDMD<f64>",
                vec![dynamic(2, 2), dynamic(2, 2), dynamic(3, 3)],
                (0, 1, 2),
            ),
            (
                "AddMDMD<f64>",
                vec![dynamic(3, 3), dynamic(2, 2), dynamic(2, 2)],
                (0, 1, 2),
            ),
            (
                "AddMDMD<f64>",
                vec![dynamic(2, 2), dynamic(2, 2)],
                (0, 0, 1),
            ),
            (
                "AddMDMD<f64>",
                vec![dynamic(2, 2), dynamic(2, 2)],
                (1, 0, 1),
            ),
            ("AddM2M2<f64>", vec![fixed2(), fixed2()], (0, 0, 1)),
            (
                "MatMulMDMD<f64>",
                vec![dynamic(2, 4), dynamic(2, 3), dynamic(2, 4)],
                (0, 1, 2),
            ),
            (
                "MatMulMDMD<f64>",
                vec![dynamic(3, 4), dynamic(2, 3), dynamic(3, 4)],
                (0, 1, 2),
            ),
            (
                "MatrixSolveMDVD<f64>",
                vec![dynamic(3, 1), dynamic(2, 3), dynamic(3, 1)],
                (0, 1, 2),
            ),
            (
                "MatrixSolveMDVD<f64>",
                vec![dynamic(2, 1), dynamic(2, 2), dynamic(3, 1)],
                (0, 1, 2),
            ),
        ];

        for (name, constants, (dst, lhs, rhs)) in cases {
            let program = parsed_runtime_program(
                constants,
                BytecodeInstruction::RuntimeBinary {
                    function: RuntimeFunctionId::from_name(name).raw(),
                    dst,
                    lhs,
                    rhs,
                },
            );
            let error = program.validate_runtime_contracts(&catalog).unwrap_err();
            assert_eq!(
                error.kind_name(),
                "BytecodeRuntimeContractViolation",
                "{name}"
            );
            assert!(error.kind_message().contains(name));
        }
    }

    #[test]
    fn rejects_valid_runtime_id_with_wrong_scalar_inputs_and_output() {
        let function = RuntimeFunctionId::from_name("ExactF64Binary").raw();
        let f64_constant = || scalar(RuntimeType::F64, 1.0_f64.to_bits().to_le_bytes().to_vec());
        let i8_constant = || scalar(RuntimeType::I8, vec![1]);

        let wrong_input = parsed_runtime_program(
            vec![f64_constant(), i8_constant(), f64_constant()],
            BytecodeInstruction::RuntimeBinary {
                function,
                dst: 0,
                lhs: 1,
                rhs: 2,
            },
        );
        assert_contract_violation(&wrong_input, "Input(0)");

        let wrong_output = parsed_runtime_program(
            vec![i8_constant(), f64_constant(), f64_constant()],
            BytecodeInstruction::RuntimeBinary {
                function,
                dst: 0,
                lhs: 1,
                rhs: 2,
            },
        );
        assert_contract_violation(&wrong_output, "Output");
    }

    #[test]
    fn rejects_opcode_and_factory_arity_mismatch() {
        let function = RuntimeFunctionId::from_name("ExactF64Binary").raw();
        let program = parsed_runtime_program(
            vec![
                scalar(RuntimeType::F64, 1.0_f64.to_bits().to_le_bytes().to_vec()),
                scalar(RuntimeType::F64, 2.0_f64.to_bits().to_le_bytes().to_vec()),
            ],
            BytecodeInstruction::RuntimeUnary {
                function,
                dst: 0,
                src: 1,
            },
        );
        assert_contract_violation(&program, "IncorrectNumberOfArguments");
    }

    #[test]
    fn external_opcodes_require_the_exact_requirement_kind_and_intent() {
        fn parsed(
            instruction: BytecodeInstruction,
            requirement: ApplicationRequirement,
        ) -> ParsedProgram {
            ParsedProgram::from_bytes(
                &write_bytecode(&BytecodeProgram {
                    register_count: 2,
                    constants: vec![
                        EncodedConstant {
                            runtime_type: RuntimeType::Empty,
                            alignment: 1,
                            bytes: Vec::new(),
                        },
                        EncodedConstant {
                            runtime_type: RuntimeType::F64,
                            alignment: 8,
                            bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
                        },
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
                        instruction,
                        BytecodeInstruction::Return { src: 0 },
                    ],
                    dictionary: BTreeMap::new(),
                    requirements: vec![requirement],
                })
                .unwrap(),
            )
            .unwrap()
        }

        fn resource(intent: ResourceIntent) -> ApplicationRequirement {
            ApplicationRequirement::Resource(ExecutionResourceRequest {
                base_uri: "test://provider".into(),
                path: "value".into(),
                context_name: "test".into(),
                operation: "operate".into(),
                intent,
                delivery: ResourceDelivery::Snapshot,
            })
        }

        let empty_catalog = FunctionCatalog::empty();
        let cases = [
            (
                parsed(
                    BytecodeInstruction::HostCall {
                        requirement: 0,
                        dst: 0,
                        arguments: vec![1],
                    },
                    resource(ResourceIntent::Read),
                ),
                "HostFunction",
            ),
            (
                parsed(
                    BytecodeInstruction::ResourceRead {
                        requirement: 0,
                        dst: 0,
                    },
                    ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
                        name: "test/host".into(),
                    }),
                ),
                "Read",
            ),
            (
                parsed(
                    BytecodeInstruction::ResourceWrite {
                        requirement: 0,
                        dst: 0,
                        src: 1,
                    },
                    resource(ResourceIntent::Send),
                ),
                "Assign",
            ),
            (
                parsed(
                    BytecodeInstruction::ResourceSend {
                        requirement: 0,
                        dst: 0,
                        src: 1,
                    },
                    resource(ResourceIntent::Assign),
                ),
                "Send",
            ),
        ];

        for (program, expected) in cases {
            let error = program
                .validate_runtime_contracts(&empty_catalog)
                .unwrap_err();
            assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
            assert!(error.kind_message().contains(expected));
        }
    }

    #[test]
    fn resource_writes_require_empty_output_seeds() {
        let program = ParsedProgram::from_bytes(
            &write_bytecode(&BytecodeProgram {
                register_count: 2,
                constants: vec![
                    EncodedConstant {
                        runtime_type: RuntimeType::F64,
                        alignment: 8,
                        bytes: 0.0_f64.to_bits().to_le_bytes().to_vec(),
                    },
                    EncodedConstant {
                        runtime_type: RuntimeType::String,
                        alignment: 1,
                        bytes: b"payload".to_vec(),
                    },
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
                    BytecodeInstruction::ResourceSend {
                        requirement: 0,
                        dst: 0,
                        src: 1,
                    },
                    BytecodeInstruction::Return { src: 0 },
                ],
                dictionary: BTreeMap::new(),
                requirements: vec![ApplicationRequirement::Resource(ExecutionResourceRequest {
                    base_uri: "test://provider/output".into(),
                    path: "line".into(),
                    context_name: "output".into(),
                    operation: "write".into(),
                    intent: ResourceIntent::Send,
                    delivery: ResourceDelivery::Snapshot,
                })],
            })
            .unwrap(),
        )
        .unwrap();

        assert_contract_violation(&program, "must have an Empty seed");
    }

    #[cfg(all(feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn rejects_fixed_and_dynamic_matrix_storage_mismatch() {
        use crate::structures::matrix::Matrix as ValueMatrix;
        use nalgebra::Matrix2;

        #[derive(Debug)]
        struct ExactMatrix2Binary {
            out: Ref<Matrix2<f64>>,
        }
        impl MechFunctionFactory for ExactMatrix2Binary {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
                <Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
                <Matrix2<f64> as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, lhs, rhs) => {
                        let out = out.try_function_ref(FunctionArgumentRole::Output)?;
                        let _: Ref<Matrix2<f64>> =
                            lhs.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let _: Ref<Matrix2<f64>> =
                            rhs.try_function_ref(FunctionArgumentRole::Input(1))?;
                        Ok(Box::new(Self { out }))
                    }
                    _ => unreachable!(),
                }
            }
        }
        impl MechFunctionImpl for ExactMatrix2Binary {
            fn solve_result(&self) -> MResult<()> {
                Ok(())
            }
            fn out(&self) -> Value {
                Value::MatrixF64(ValueMatrix::Matrix2(self.out.clone()))
            }
            fn to_string(&self) -> String {
                "ExactMatrix2Binary".into()
            }
            fn transaction_state_values(&self) -> MResult<Vec<Value>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "compiler")]
        impl MechFunctionCompiler for ExactMatrix2Binary {
            fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                Ok(0)
            }
        }

        const NAME: &str = "ExactMatrix2Binary";
        let function = RuntimeFunctionId::from_name(NAME).raw();
        let program = parsed_runtime_program(
            vec![
                f64_matrix_constant(crate::MatrixStorage::Matrix2, 2, 2),
                f64_matrix_constant(crate::MatrixStorage::MatrixD, 2, 2),
                f64_matrix_constant(crate::MatrixStorage::Matrix2, 2, 2),
            ],
            BytecodeInstruction::RuntimeBinary {
                function,
                dst: 0,
                lhs: 1,
                rhs: 2,
            },
        );
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory::<ExactMatrix2Binary>(
                NAME,
                crate::RuntimeFunctionContract::same_shape(
                    crate::RuntimeOutputAliasPolicy::DisallowInputAlias,
                ),
            )
            .unwrap();
        let error = program
            .validate_runtime_contracts(&builder.build().unwrap())
            .unwrap_err();
        assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
        assert!(error.kind_message().contains("Input(0)"));
    }
}
