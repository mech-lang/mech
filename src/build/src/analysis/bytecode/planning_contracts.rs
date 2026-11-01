//! Legacy instruction contracts remain directly testable without treating
//! instruction-only bytecode as an executable native application.

use super::*;
use mech_core::{
    BytecodeCompilerContext, BytecodeInstruction, BytecodeProgram, EncodedConstant,
    FunctionCatalogBuilder, FunctionInvocation, FunctionRuntimeType, FunctionValueOutput,
    FunctionValueRepresentation, MatrixStorage, MechFunction, MechFunctionCompiler,
    MechFunctionFactory, MechFunctionImpl, Register, RuntimeFamilyId, RuntimeFunctionContract,
    RuntimeFunctionSignature, RuntimeOutputAliasPolicy, RuntimeType, write_bytecode,
};
use std::collections::{BTreeMap, BTreeSet};

fn validate_legacy_contracts(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> MResult<Vec<PlannedRuntimeFunction>> {
    let program = ParsedProgram::from_bytes(bytecode)?;
    program.validate_runtime_contracts(catalog)?;
    analyze_runtime_functions(&program, catalog, None, None)
}

#[test]
fn unknown_runtime_ids_fail_contract_validation() {
    let bytecode = runtime_nullary_bytecode(0x0123_4567_89ab_cdef);
    let error = validate_legacy_contracts(&bytecode, &FunctionCatalog::empty()).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
}

#[derive(Debug)]
struct PlanningFunction {
    _output: FunctionValueOutput,
}

impl MechFunctionImpl for PlanningFunction {
    fn solve_managed(
        &self,
        _frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        (|| -> MResult<()> { Ok(()) })()?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn to_string(&self) -> String {
        "PlanningFunction".into()
    }
}

impl MechFunctionCompiler for PlanningFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

impl MechFunctionFactory for PlanningFunction {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(<f64 as FunctionRuntimeType>::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let output = invocation.expect_nullary()?;
        Ok(Box::new(PlanningFunction {
            _output: output.value(),
        }))
    }
}

#[test]
fn known_runtime_ids_without_native_metadata_fail_linkage_analysis() {
    const NAME: &str = "KnownButUnlinked";
    let mut catalog = FunctionCatalogBuilder::new();
    catalog
        .insert_runtime_factory::<PlanningFunction>(
            NAME,
            RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name(NAME),
        )
        .unwrap();
    let bytecode = runtime_nullary_bytecode(hash_str(NAME));

    let error = validate_legacy_contracts(&bytecode, &catalog.build().unwrap()).unwrap_err();
    assert_eq!(error.kind_name(), "NativeRuntimeFunctionLinkageMissing");
}

#[test]
fn malicious_runtime_type_mismatch_fails_before_native_analysis() {
    const NAME: &str = "LinkedPlanningFunction";
    let mut catalog = FunctionCatalogBuilder::new();
    catalog
        .insert_runtime_factory_with_linkage::<PlanningFunction>(
            NAME,
            RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            NativeFunctionLinkage {
                package: "mech-test",
                crate_name: "mech_test",
                installer_path: "mech_test::__mech_native::install",
                cargo_features: vec!["native-link", "runtime"],
            },
            RuntimeFamilyId::from_name(NAME),
        )
        .unwrap();
    let bytecode = runtime_nullary_bytecode_with_constant(
        hash_str(NAME),
        EncodedConstant {
            runtime_type: RuntimeType::I8,
            alignment: 1,
            bytes: vec![7],
        },
    );

    let error = validate_legacy_contracts(&bytecode, &catalog.build().unwrap()).unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
    assert!(error.kind_message().contains(NAME));
}

#[test]
fn malicious_matrix_relations_and_aliases_fail_before_factory_construction() {
    struct MustNotInstantiate;

    impl MechFunctionFactory for MustNotInstantiate {
        fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
            mech_core::ImplementationMemoryClass::NoAdditionalScratch
        }

        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        );

        fn new_invocation(_invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
            panic!("malformed matrix relations must fail before factory construction")
        }
    }
    let mut catalog = FunctionCatalogBuilder::new();
    for (name, installer_path, contract) in [
        (
            "AddMDMD<f64>",
            "mech_test::__mech_native::install_add_mdmd",
            RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
        (
            "AddM2M2<f64>",
            "mech_test::__mech_native::install_add_m2m2",
            RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
        (
            "MatMulMDMD<f64>",
            "mech_test::__mech_native::install_matmul_mdmd",
            RuntimeFunctionContract::matrix_product(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
        (
            "MatrixSolveMDVD<f64>",
            "mech_test::__mech_native::install_solve_mdvd",
            RuntimeFunctionContract::linear_solve(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
    ] {
        catalog
            .insert_runtime_factory_with_linkage::<MustNotInstantiate>(
                name,
                contract,
                NativeFunctionLinkage {
                    package: "mech-test",
                    crate_name: "mech_test",
                    installer_path,
                    cargo_features: vec!["native-link", "runtime"],
                },
                RuntimeFamilyId::from_name(name),
            )
            .unwrap();
    }
    let catalog = catalog.build().unwrap();

    let dynamic = |rows, cols| f64_matrix_constant(MatrixStorage::MatrixD, rows, cols);
    let vector = |rows| f64_matrix_constant(MatrixStorage::VectorD, rows, 1);
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
        (
            "AddM2M2<f64>",
            vec![dynamic(2, 2), dynamic(2, 2)],
            (0, 0, 1),
        ),
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
            vec![vector(3), dynamic(2, 3), vector(3)],
            (0, 1, 2),
        ),
        (
            "MatrixSolveMDVD<f64>",
            vec![vector(2), dynamic(2, 2), vector(3)],
            (0, 1, 2),
        ),
    ];

    for (name, constants, (dst, lhs, rhs)) in cases {
        let malformed = runtime_binary_bytecode(hash_str(name), constants, dst, lhs, rhs);
        let error = validate_legacy_contracts(&malformed, &catalog).unwrap_err();
        assert_eq!(
            error.kind_name(),
            "BytecodeRuntimeContractViolation",
            "{name}"
        );
        assert!(error.kind_message().contains(name));
    }
}

#[test]
fn unrelated_program_types_do_not_become_machine_features() {
    let bytes = string_and_scalar_add_bytecode();
    let program = ParsedProgram::from_bytes(&bytes).unwrap();
    let catalog = mech_stdlib::source_native_plan_catalog();
    program.validate_runtime_contracts(&catalog).unwrap();
    let runtime_types = super::super::runtime_types::analyze_runtime_types(
        &program.referenced_runtime_types().unwrap(),
    )
    .unwrap();
    assert!(
        runtime_types
            .cargo_features
            .iter()
            .any(|feature| feature == "string")
    );
    let functions = analyze_runtime_functions(&program, &catalog, None, None).unwrap();
    let math = functions
        .iter()
        .find(|function| function.package == "mech-math")
        .unwrap();
    assert!(math.cargo_features.iter().any(|feature| feature == "f64"));
    assert!(
        math.cargo_features
            .iter()
            .all(|feature| feature != "string")
    );
}

fn runtime_nullary_bytecode(function: u64) -> Vec<u8> {
    runtime_nullary_bytecode_with_constant(
        function,
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: 0.0_f64.to_bits().to_le_bytes().to_vec(),
        },
    )
}

fn f64_matrix_constant(storage: MatrixStorage, rows: u32, cols: u32) -> EncodedConstant {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&rows.to_le_bytes());
    bytes.extend_from_slice(&cols.to_le_bytes());
    for index in 0..rows.saturating_mul(cols) {
        bytes.extend_from_slice(&f64::from(index + 1).to_bits().to_le_bytes());
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

fn runtime_binary_bytecode(
    function: u64,
    constants: Vec<EncodedConstant>,
    dst: u32,
    lhs: u32,
    rhs: u32,
) -> Vec<u8> {
    let register_count = constants.len() as u32;
    let mut canonical_constants = Vec::new();
    let mut instructions = constants
        .into_iter()
        .enumerate()
        .map(|(register, constant)| {
            let constant_id = canonical_constants
                .iter()
                .position(|existing| existing == &constant)
                .unwrap_or_else(|| {
                    canonical_constants.push(constant);
                    canonical_constants.len() - 1
                }) as u32;
            BytecodeInstruction::ConstLoad {
                dst: register as u32,
                constant: constant_id,
            }
        })
        .collect::<Vec<_>>();
    instructions.push(BytecodeInstruction::RuntimeBinary {
        function,
        dst,
        lhs,
        rhs,
    });
    instructions.push(BytecodeInstruction::Return { src: dst });
    write_bytecode(&BytecodeProgram {
        register_count,
        constants: canonical_constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

fn runtime_nullary_bytecode_with_constant(function: u64, output: EncodedConstant) -> Vec<u8> {
    write_bytecode(&BytecodeProgram {
        register_count: 1,
        constants: vec![output],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::RuntimeNullary { function, dst: 0 },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

fn string_and_scalar_add_bytecode() -> Vec<u8> {
    write_bytecode(&BytecodeProgram {
        register_count: 4,
        constants: vec![
            EncodedConstant {
                runtime_type: RuntimeType::String,
                alignment: 1,
                bytes: b"unrelated".to_vec(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::F64,
                alignment: 8,
                bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::F64,
                alignment: 8,
                bytes: 2.0_f64.to_bits().to_le_bytes().to_vec(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::F64,
                alignment: 8,
                bytes: 0.0_f64.to_bits().to_le_bytes().to_vec(),
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
            BytecodeInstruction::ConstLoad {
                dst: 2,
                constant: 2,
            },
            BytecodeInstruction::ConstLoad {
                dst: 3,
                constant: 3,
            },
            BytecodeInstruction::RuntimeBinary {
                function: hash_str("AddSS<f64>"),
                dst: 3,
                lhs: 1,
                rhs: 2,
            },
            BytecodeInstruction::Return { src: 3 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}
