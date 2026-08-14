#![cfg(feature = "full-hosts")]

mod support;

use std::fs;

use mech_core::{BytecodeInstruction, ParsedProgram};
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
    let mut runtime = RuntimeBuilder::new()
        .planning()
        .function_catalog(mech_stdlib::source_catalog())
        .build()
        .unwrap();
    runtime.compile_source_program_bytecode(source).unwrap()
}

#[test]
fn poisoned_output_seeds_are_recomputed_for_every_runtime_arity() {
    let cases = [
        (
            "nullary",
            "{x | x <- {1.0}, false}",
            RuntimeArity::Nullary,
            "{}",
        ),
        ("unary", "[9.0]", RuntimeArity::Unary, "[9]"),
        ("binary", "1.0 + 2.0", RuntimeArity::Binary, "3"),
        (
            "ternary",
            "1.0..1.0..=4.0",
            RuntimeArity::Ternary,
            "[1 2 3 4]",
        ),
        (
            "quaternary",
            concat!(
                "a := [1.0; 2.0]; b := [3.0; 4.0]; ",
                "c := [5.0; 6.0]; d := [7.0; 8.0]; [a b c d]",
            ),
            RuntimeArity::Quaternary,
            "[1 3 5 7; 2 4 6 8]",
        ),
        (
            "variadic",
            "[1.0 2.0 3.0 4.0 5.0]",
            RuntimeArity::Variadic,
            "[1 2 3 4 5]",
        ),
    ];

    let temporary = tempfile::tempdir().unwrap();
    let compiled = cases
        .into_iter()
        .map(|(name, source, arity, expected)| (name, compile_source(source), arity, expected))
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
