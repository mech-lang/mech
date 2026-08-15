#![cfg(feature = "full-hosts")]

mod support;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use mech_core::{
    BytecodeInstruction, BytecodeProgram, EncodedConstant, ParsedProgram, RuntimeType, hash_str,
    write_bytecode,
};
use mech_runtime::RuntimeBuilder;
use support::*;

#[derive(Clone, Copy)]
enum RuntimeArity {
    Nullary,
    Unary,
    Binary,
    Ternary,
    Quaternary,
    Variadic,
}

impl RuntimeArity {
    fn appears_in(self, parsed: &ParsedProgram) -> bool {
        parsed.instructions.iter().any(|instruction| {
            matches!(
                (self, instruction),
                (Self::Nullary, BytecodeInstruction::RuntimeNullary { .. })
                    | (Self::Unary, BytecodeInstruction::RuntimeUnary { .. })
                    | (Self::Binary, BytecodeInstruction::RuntimeBinary { .. })
                    | (Self::Ternary, BytecodeInstruction::RuntimeTernary { .. })
                    | (
                        Self::Quaternary,
                        BytecodeInstruction::RuntimeQuaternary { .. }
                    )
                    | (Self::Variadic, BytecodeInstruction::RuntimeVariadic { .. })
            )
        })
    }
}

fn compile_source(source: &str) -> Vec<u8> {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    compiler
        .compile_source(source)
        .map(|product| product.into_parts().1)
        .unwrap()
}

fn typed_nullary_set_comprehension() -> Vec<u8> {
    // Empty source comprehensions have no element value from which the source
    // planner can infer a schema.  Build the equivalent bytecode fixture with
    // its intended Set<f64> output type stated explicitly: this test owns
    // native output-seed recomputation, not source type inference.
    write_bytecode(&BytecodeProgram {
        register_count: 1,
        constants: vec![EncodedConstant {
            runtime_type: RuntimeType::Set {
                element: Box::new(RuntimeType::F64),
                max_len: None,
            },
            alignment: 8,
            bytes: 0_u32.to_le_bytes().to_vec(),
        }],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::RuntimeNullary {
                function: hash_str("set/comprehension"),
                dst: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

#[test]
fn poisoned_output_seeds_are_recomputed_for_every_runtime_arity() {
    let cases = [
        ("nullary", None, RuntimeArity::Nullary, "{}"),
        ("unary", Some("[9.0]"), RuntimeArity::Unary, "[9]"),
        ("binary", Some("1.0 + 2.0"), RuntimeArity::Binary, "3"),
        (
            "ternary",
            Some("1.0..1.0..=4.0"),
            RuntimeArity::Ternary,
            "[1 2 3 4]",
        ),
        (
            "quaternary",
            Some(concat!(
                "a := [1.0; 2.0]; b := [3.0; 4.0]; ",
                "c := [5.0; 6.0]; d := [7.0; 8.0]; [a b c d]",
            )),
            RuntimeArity::Quaternary,
            "[1 3 5 7; 2 4 6 8]",
        ),
        (
            "variadic",
            Some("[1.0 2.0 3.0 4.0 5.0]"),
            RuntimeArity::Variadic,
            "[1 2 3 4 5]",
        ),
    ];

    let temporary = tempfile::tempdir().unwrap();
    let compiled = cases
        .into_iter()
        .map(|(name, source, arity, expected)| {
            let bytecode = source
                .map(compile_source)
                .unwrap_or_else(typed_nullary_set_comprehension);
            (name, bytecode, arity, expected)
        })
        .collect::<Vec<_>>();
    for (name, bytecode, arity, _) in &compiled {
        let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
        assert!(
            arity.appears_in(&parsed),
            "{name} source missed its arity: {:?}",
            parsed.instructions
        );
    }
    for (name, bytecode, _, expected) in compiled {
        let fixture = temporary.path().join(format!("{name}.mecb"));
        fs::write(&fixture, bytecode).unwrap();

        let result = run_owner(
            OwnerProfile::Standard,
            RunnerAction::Build,
            name,
            fixture,
            &format!("native_output_seed_{name}"),
            true,
        );
        assert!(result.poisoned_output_seed);
        assert!(result.poisoned_output_seed_count > 0);
        assert_eq!(result.stdout.unwrap().trim(), expected, "{name}");
    }
}
