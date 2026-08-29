#![cfg(feature = "full-hosts")]

pub mod support;

use std::fs;

use mech_core::{
    BytecodeInstruction, BytecodeProgram, EncodedConstant, ParsedProgram, hash_str,
    write_bytecode_with_artifact,
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
    fn appears_in(self, parsed: &ParsedProgram) -> bool {
        parsed.instructions.iter().any(|instruction| {
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

fn quaternary_bytecode() -> Vec<u8> {
    let bytes = compile_source(concat!(
        "a := [1.0; 2.0]; b := [3.0; 4.0]; ",
        "c := [5.0; 6.0]; d := [7.0; 8.0]; [a b c d]",
    ));
    let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
    let constants = parsed
        .constants
        .iter()
        .map(|constant| {
            let start = constant.offset as usize;
            let end = start + constant.length as usize;
            EncodedConstant {
                runtime_type: parsed.types[constant.type_id as usize].clone(),
                alignment: constant.alignment,
                bytes: parsed.constant_blob[start..end].to_vec(),
            }
        })
        .collect();
    let mut instructions = parsed.instructions;
    let instruction = instructions
        .iter_mut()
        .rev()
        .find(|instruction| {
            matches!(
                instruction,
                BytecodeInstruction::RuntimeVariadic { arguments, .. }
                    if arguments.len() == 4
            )
        })
        .expect("four-input concatenation emits a variadic instruction");
    let BytecodeInstruction::RuntimeVariadic { dst, arguments, .. } = instruction else {
        unreachable!("four-input concatenation instruction was selected above")
    };
    let [a, b, c, d] = arguments.as_slice() else {
        unreachable!("four-input concatenation instruction has four arguments")
    };
    *instruction = BytecodeInstruction::RuntimeQuaternary {
        function: hash_str("HorizontalConcatenateFourArgs<f64>"),
        dst: *dst,
        a: *a,
        b: *b,
        c: *c,
        d: *d,
    };
    write_bytecode_with_artifact(
        &BytecodeProgram {
            register_count: parsed.header.register_count,
            constants,
            symbols: parsed.symbols,
            mutable_symbols: parsed.mutable_symbols,
            instructions,
            dictionary: parsed.dictionary,
            requirements: parsed.requirements,
        },
        &parsed.artifact,
    )
    .unwrap()
}

#[test]
fn poisoned_output_seeds_are_recomputed_for_every_resident_runtime_arity() {
    let compiled = vec![
        (
            "unary",
            compile_source("¬false"),
            RuntimeArity::Unary,
            "true",
        ),
        (
            "binary",
            compile_source("1.0 + 2.0"),
            RuntimeArity::Binary,
            "3",
        ),
        (
            "ternary",
            compile_source("1.0..1.0..=4.0"),
            RuntimeArity::Ternary,
            "[1 2 3 4]",
        ),
        (
            "quaternary",
            quaternary_bytecode(),
            RuntimeArity::Quaternary,
            "[1 3 5 7; 2 4 6 8]",
        ),
        (
            "variadic",
            compile_source("[1.0 2.0 3.0 4.0 5.0]"),
            RuntimeArity::Variadic,
            "[1 2 3 4 5]",
        ),
    ];

    let temporary = tempfile::tempdir().unwrap();
    for (name, bytecode, arity, _) in &compiled {
        let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
        assert!(
            arity.appears_in(&parsed),
            "{name} fixture missed its arity: {:?}",
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
