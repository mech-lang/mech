#![cfg(feature = "full-hosts")]
#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

pub mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use mech_core::{
    BytecodeInstruction, BytecodeProgram, EncodedConstant, MatrixStorage, ParsedProgram,
    RuntimeType, hash_str, write_bytecode,
};
use mech_runtime::RuntimeBuilder;
use support::*;

#[derive(Clone, Copy)]
enum RuntimeArity {
    Unary,
    Binary,
    Ternary,
    Quaternary,
    Variadic,
}

impl RuntimeArity {
    fn matches(self, instruction: &BytecodeInstruction) -> bool {
        matches!(
            (self, instruction),
            (Self::Unary, BytecodeInstruction::RuntimeUnary { .. })
                | (Self::Binary, BytecodeInstruction::RuntimeBinary { .. })
                | (Self::Ternary, BytecodeInstruction::RuntimeTernary { .. })
                | (
                    Self::Quaternary,
                    BytecodeInstruction::RuntimeQuaternary { .. }
                )
                | (Self::Variadic, BytecodeInstruction::RuntimeVariadic { .. })
        )
    }
}

fn boolean_constant(value: bool) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::Bool,
        alignment: 1,
        bytes: vec![u8::from(value)],
    }
}

fn f64_constant(value: f64) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::F64,
        alignment: 8,
        bytes: value.to_bits().to_le_bytes().to_vec(),
    }
}

fn f64_row_matrix_constant(values: &[f64]) -> EncodedConstant {
    let mut bytes = Vec::with_capacity(8 + values.len() * 8);
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&(values.len() as u32).to_le_bytes());
    for value in values {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(RuntimeType::F64),
            storage: MatrixStorage::MatrixD,
            rows: 1,
            cols: values.len() as u32,
        },
        alignment: 8,
        bytes,
    }
}

fn legacy_runtime_bytecode(
    constants: Vec<EncodedConstant>,
    runtime_instruction: BytecodeInstruction,
    output: u32,
) -> Vec<u8> {
    let mut instructions = (0..constants.len() as u32)
        .map(|register| BytecodeInstruction::ConstLoad {
            dst: register,
            constant: register,
        })
        .collect::<Vec<_>>();
    instructions.push(runtime_instruction);
    instructions.push(BytecodeInstruction::Return { src: output });
    write_bytecode(&BytecodeProgram {
        register_count: constants.len() as u32,
        constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

fn unary_bytecode() -> Vec<u8> {
    legacy_runtime_bytecode(
        vec![boolean_constant(false), boolean_constant(true)],
        BytecodeInstruction::RuntimeUnary {
            function: hash_str("NotS<bool>"),
            dst: 1,
            src: 0,
        },
        1,
    )
}

fn binary_bytecode() -> Vec<u8> {
    legacy_runtime_bytecode(
        vec![f64_constant(1.0), f64_constant(2.0), f64_constant(99.0)],
        BytecodeInstruction::RuntimeBinary {
            function: hash_str("AddSS<f64>"),
            dst: 2,
            lhs: 0,
            rhs: 1,
        },
        2,
    )
}

fn matrix_input_constants(count: u32) -> Vec<EncodedConstant> {
    let mut constants = (1..=count)
        .map(|value| f64_row_matrix_constant(&[f64::from(value)]))
        .collect::<Vec<_>>();
    constants.push(f64_row_matrix_constant(&vec![99.0; count as usize]));
    constants
}

fn ternary_bytecode() -> Vec<u8> {
    legacy_runtime_bytecode(
        matrix_input_constants(3),
        BytecodeInstruction::RuntimeTernary {
            function: hash_str("HorizontalConcatenateThreeArgs<f64>"),
            dst: 3,
            a: 0,
            b: 1,
            c: 2,
        },
        3,
    )
}

fn quaternary_bytecode() -> Vec<u8> {
    legacy_runtime_bytecode(
        matrix_input_constants(4),
        BytecodeInstruction::RuntimeQuaternary {
            function: hash_str("HorizontalConcatenateFourArgs<f64>"),
            dst: 4,
            a: 0,
            b: 1,
            c: 2,
            d: 3,
        },
        4,
    )
}

fn variadic_bytecode() -> Vec<u8> {
    legacy_runtime_bytecode(
        matrix_input_constants(5),
        BytecodeInstruction::RuntimeVariadic {
            function: hash_str("HorizontalConcatenateNArgs<f64>"),
            dst: 5,
            arguments: vec![0, 1, 2, 3, 4],
        },
        5,
    )
}

#[test]
fn legacy_runtime_arities_remain_plannable_with_poisoned_output_seeds() {
    let compiled = vec![
        (
            "unary",
            unary_bytecode(),
            RuntimeArity::Unary,
            "NotS<bool>",
            "mech-logic",
            "mech_logic",
            "mech_logic::__mech_native::install_logic_not_s",
        ),
        (
            "binary",
            binary_bytecode(),
            RuntimeArity::Binary,
            "AddSS<f64>",
            "mech-math",
            "mech_math",
            "mech_math::__mech_native::install_add_ss_f64",
        ),
        (
            "ternary",
            ternary_bytecode(),
            RuntimeArity::Ternary,
            "HorizontalConcatenateThreeArgs<f64>",
            "mech-engine",
            "mech_engine",
            "mech_engine::__mech_native::install_horizontal_concatenate_three_args_f64",
        ),
        (
            "quaternary",
            quaternary_bytecode(),
            RuntimeArity::Quaternary,
            "HorizontalConcatenateFourArgs<f64>",
            "mech-engine",
            "mech_engine",
            "mech_engine::__mech_native::install_horizontal_concatenate_four_args_f64",
        ),
        (
            "variadic",
            variadic_bytecode(),
            RuntimeArity::Variadic,
            "HorizontalConcatenateNArgs<f64>",
            "mech-engine",
            "mech_engine",
            "mech_engine::__mech_native::install_horizontal_concatenate_n_args_f64",
        ),
    ];

    let temporary = tempfile::tempdir().unwrap();
    // Legacy-only bytecode remains a planning input, but the generated runtime
    // must not regain an instruction fallback. Canonical execution evidence for
    // the same semantic arities is exercised separately below.
    for (name, bytecode, arity, runtime_name, package, crate_name, installer_path) in compiled {
        let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
        assert!(parsed.artifact.is_empty(), "{name}");
        let runtime_instructions = parsed
            .instructions
            .iter()
            .filter(|instruction| instruction.runtime_function().is_some())
            .collect::<Vec<_>>();
        assert_eq!(runtime_instructions.len(), 1, "{name}");
        assert!(arity.matches(runtime_instructions[0]), "{name}");
        assert_eq!(
            runtime_instructions[0].runtime_function(),
            Some(hash_str(runtime_name)),
            "{name}"
        );

        let fixture = temporary.path().join(format!("{name}.mecb"));
        fs::write(&fixture, bytecode).unwrap();

        let result = run_owner(
            OwnerProfile::Standard,
            RunnerAction::Plan,
            name,
            fixture,
            &format!("native_output_seed_{name}"),
            true,
        );
        assert!(result.poisoned_output_seed, "{name}");
        assert_eq!(result.poisoned_output_seed_count, 1, "{name}");
        let [planned] = result.plan.runtime_functions.as_slice() else {
            panic!(
                "{name} must plan exactly one runtime function, found {:?}",
                result.plan.runtime_functions
            )
        };
        assert_eq!(planned.runtime_id, hash_str(runtime_name), "{name}");
        assert_eq!(planned.runtime_name, runtime_name, "{name}");
        assert_eq!(planned.package, package, "{name}");
        assert_eq!(planned.crate_name, crate_name, "{name}");
        assert_eq!(planned.installer_path, installer_path, "{name}");
    }
}

fn canonical_artifact_bytecode(source: &str) -> Vec<u8> {
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .unwrap();
    compiler
        .compile_canonical_source(source)
        .unwrap()
        .into_native_parts()
        .1
}

#[test]
fn canonical_resident_arities_build_and_execute_without_legacy_seeds() {
    let cases = [
        (
            "resident-unary",
            "value := false\n¬value\n",
            "logic/not",
            1,
            "true",
        ),
        (
            "resident-binary",
            "left := 1.0\nright := 2.0\nleft + right\n",
            "math/add",
            2,
            "3",
        ),
        (
            "resident-ternary",
            "a := [1.0]\nb := [2.0]\nc := [3.0]\nmatrix/horzcat(a, b, c)\n",
            "matrix/horzcat",
            3,
            "[1 2 3]",
        ),
        (
            "resident-quaternary",
            "a := [1.0]\nb := [2.0]\nc := [3.0]\nd := [4.0]\nmatrix/horzcat(a, b, c, d)\n",
            "matrix/horzcat",
            4,
            "[1 2 3 4]",
        ),
        (
            "resident-variadic",
            "a := [1.0]\nb := [2.0]\nc := [3.0]\nd := [4.0]\ne := [5.0]\nmatrix/horzcat(a, b, c, d, e)\n",
            "matrix/horzcat",
            5,
            "[1 2 3 4 5]",
        ),
    ];

    let temporary = tempfile::tempdir().unwrap();
    for (name, source, operation, input_count, expected) in cases {
        let bytecode = canonical_artifact_bytecode(source);
        let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
        assert!(parsed.instructions.is_empty(), "{name}");
        assert!(parsed.constants.is_empty(), "{name}");
        assert_eq!(parsed.header.register_count, 0, "{name}");
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytecode).unwrap();
        let matching = artifact
            .nodes()
            .iter()
            .filter_map(|node| node.as_operation())
            .filter(|node| node.operation.canonical_name() == operation)
            .collect::<Vec<_>>();
        let [node] = matching.as_slice() else {
            panic!("{name} must retain exactly one {operation} node, found {matching:#?}")
        };
        assert_eq!(node.input_bindings.len(), input_count, "{name}");

        let fixture = temporary.path().join(format!("{name}.mecb"));
        fs::write(&fixture, bytecode).unwrap();
        let result = run_owner(
            OwnerProfile::Standard,
            RunnerAction::Build,
            name,
            fixture,
            &format!("native_{name}"),
            false,
        );
        assert!(!result.poisoned_output_seed, "{name}");
        assert_eq!(result.poisoned_output_seed_count, 0, "{name}");
        assert!(result.plan.runtime_functions.is_empty(), "{name}");
        assert!(result.plan.runtime_types.is_empty(), "{name}");
        assert_exact_mech_packages(&result.plan, &["mech-core", "mech-engine", "mech-runtime"]);
        assert_eq!(
            result.stdout.as_deref().map(str::trim),
            Some(expected),
            "{name}"
        );
    }
}
