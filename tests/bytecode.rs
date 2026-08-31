#![cfg(any(feature = "distribution-standard", feature = "distribution-full"))]

#[path = "support/bytecode/catalog.rs"]
mod catalog;
#[path = "support/bytecode/dynamic_matrix_factory.rs"]
mod dynamic_matrix_factory;

use mech_core::{
    BytecodeInstruction, MResult, ParsedProgram, RuntimeType, SchemaBody, Value, ValueData,
    hash_str, snapshot::SequenceView,
};
use mech_engine::decode_program_artifact_bytecode_v1;
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder, RuntimeProgramRoute};

fn compile_source(source: &str) -> MResult<Vec<u8>> {
    RuntimeBuilder::new()
        .function_catalog(mech::stdlib::source_catalog())
        .build_compiler()?
        .compile_source(source)
        .map(|product| product.into_parts().1)
}

fn run_compiled_source(source: &str) -> MResult<(ParsedProgram, Value)> {
    let bytecode = compile_source(source)?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech::stdlib::source_catalog())
        .build()?;
    let loaded = runtime.load_bytecode_program(&bytecode, ResidentDurabilityPolicy::Volatile)?;
    Ok((parsed, loaded.initial_value.into_value()))
}

fn assert_f64(value: &Value, expected: f64) {
    assert!(
        matches!(value.data(), ValueData::F64(actual) if actual.to_f64() == expected),
        "expected canonical f64 {expected}, got {:?}",
        value.data(),
    );
}

fn assert_bool(value: &Value, expected: bool) {
    assert!(matches!(value.data(), ValueData::Bool(actual) if *actual == expected));
}

fn assert_f64_matrix(value: &Value, expected: &[f64], rows: u64, columns: u64) {
    let matrix = value.matrix_view().expect("expected canonical matrix");
    let SequenceView::F64(actual) = matrix.elements() else {
        panic!("expected canonical f64 matrix");
    };
    let schemas = value
        .schemas()
        .expect("canonical matrix retains its schema arena");
    let SchemaBody::Matrix { dimensions, .. } = schemas
        .get(value.schema())
        .expect("canonical matrix schema exists")
        .body()
    else {
        panic!("expected canonical matrix schema");
    };
    assert_eq!(
        (
            value
                .shape()
                .resolve_dimension(&dimensions[0])
                .expect("matrix row extent resolves"),
            value
                .shape()
                .resolve_dimension(&dimensions[1])
                .expect("matrix column extent resolves"),
        ),
        (rows, columns),
    );
    assert_eq!(
        actual
            .iter()
            .map(|element| element.to_f64())
            .collect::<Vec<_>>(),
        expected,
    );
}

fn return_register(program: &ParsedProgram) -> u32 {
    match program.instructions.last() {
        Some(BytecodeInstruction::Return { src }) => *src,
        instruction => panic!("expected one final Return, found {instruction:?}"),
    }
}

fn final_binary_register(program: &ParsedProgram) -> u32 {
    program
        .instructions
        .iter()
        .rev()
        .find_map(|instruction| match instruction {
            BytecodeInstruction::RuntimeBinary { dst, .. } => Some(*dst),
            _ => None,
        })
        .expect("expected a RuntimeBinary instruction")
}

#[test]
fn literal_only_source_returns_its_literal_register() -> MResult<()> {
    let (parsed, value) = run_compiled_source("10")?;
    assert_f64(&value, 10.0);
    assert_eq!(
        parsed
            .instructions
            .iter()
            .filter(|instruction| matches!(instruction, BytecodeInstruction::Return { .. }))
            .count(),
        1,
    );
    assert!(matches!(
        parsed.instructions.last(),
        Some(BytecodeInstruction::Return { .. })
    ));
    Ok(())
}

#[test]
fn scalar_add_returns_the_final_function_register() -> MResult<()> {
    let (parsed, value) = run_compiled_source("1 + 2")?;
    assert_f64(&value, 3.0);
    assert!(
        parsed
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, BytecodeInstruction::RuntimeBinary { .. }))
    );
    assert!(matches!(
        parsed.instructions.last(),
        Some(BytecodeInstruction::Return { .. })
    ));
    Ok(())
}

#[test]
#[cfg(feature = "distribution-full")]
fn completed_math_lowerings_compile_through_bytecode_v1() -> MResult<()> {
    for source in [
        "+> math\nmath/copysign(3.0, -2.0)",
        "+> math\nmath/fdim(5.0, 3.0)",
        "+> math\nmath/fmod(5.3, 2.0)",
        "+> math\nmath/nextafter(1.0, 2.0)",
        "+> math\nmath/remainder(5.3, 2.0)",
        "+> math\nmath/bessel/jn(2.0, 1.0)",
        "+> math\nmath/bessel/yn(2.0, 1.0)",
    ] {
        let parsed = ParsedProgram::from_bytes(&compile_source(source)?)?;
        assert!(
            parsed.instructions.iter().any(|instruction| matches!(
                instruction,
                BytecodeInstruction::RuntimeBinary { .. }
            )),
            "{source} did not lower to a binary runtime instruction",
        );
    }
    Ok(())
}

#[test]
fn dynamic_strict_equality_round_trips_through_bytecode() -> MResult<()> {
    let (parsed, value) = run_compiled_source("x := 1 + [4 5 6]\nx === [5 6 7]")?;
    assert_bool(&value, true);
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeBinary { function, .. }
            if *function == hash_str("compare/seq")
    )));
    Ok(())
}

#[test]
fn dynamic_strict_inequality_round_trips_through_bytecode() -> MResult<()> {
    let (parsed, value) = run_compiled_source("x := 1 + [4 5 6]\nx !== [5 6 8]")?;
    assert_bool(&value, true);
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeBinary { function, .. }
            if *function == hash_str("compare/sneq")
    )));
    Ok(())
}

#[test]
fn strict_comparison_preserves_scalar_and_matrix_shape_identity() -> MResult<()> {
    let (_, equal) = run_compiled_source("1.0 === [1.0]")?;
    assert_bool(&equal, false);
    let (_, not_equal) = run_compiled_source("1.0 !== [1.0]")?;
    assert_bool(&not_equal, true);
    Ok(())
}

#[test]
fn ordinary_set_elements_round_trip_through_bytecode() -> MResult<()> {
    let (inserted_program, inserted) = run_compiled_source("set/insert({1, 2}, 3)")?;
    assert!(
        inserted_program
            .instructions
            .iter()
            .any(|instruction| matches!(
                instruction,
                BytecodeInstruction::RuntimeBinary { function, .. }
                    if *function == hash_str("SetInsertFxn")
            ))
    );
    let inserted = inserted
        .set_view()
        .expect("set/insert must return a canonical set");
    assert!(
        inserted.elements().iter().any(
            |element| matches!(element.data(), ValueData::F64(value) if value.to_f64() == 3.0)
        )
    );

    let (_, removed) = run_compiled_source("set/remove({1}, 1)")?;
    assert!(
        removed
            .set_view()
            .expect("set/remove must return a canonical set")
            .elements()
            .is_empty()
    );

    let (_, member) = run_compiled_source("2 ∈ {1, 2, 3}")?;
    assert_bool(&member, true);
    let (_, not_member) = run_compiled_source("4 ∉ {1, 2, 3}")?;
    assert_bool(&not_member, true);
    Ok(())
}

#[test]
fn compiled_set_membership_round_trips_through_bytecode() -> MResult<()> {
    let (_, value) =
        run_compiled_source("x := 2\nitems := {1, 2, 3}\nmember := x ∈ items\nmember")?;
    assert_bool(&value, true);

    Ok(())
}

#[test]
fn set_membership_preserves_element_schema_identity() -> MResult<()> {
    let (_, value) = run_compiled_source("[2.0] ∈ {1.0, 2.0, 3.0}")?;
    assert_bool(&value, false);
    Ok(())
}

#[test]
fn compiled_integrity_constraints_are_reconstructed() -> MResult<()> {
    let bytecode = compile_source("x := 1.0\nsafe! := x <= 2.0")?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    assert_eq!(
        parsed
            .dictionary
            .get(&hash_str("safe!"))
            .map(String::as_str),
        Some("safe!"),
    );
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeVariadic { function, arguments, .. }
            if *function == hash_str("integrity/constraint")
                && arguments.first() == parsed.symbols.get(&hash_str("safe!"))
    )));

    let artifact = decode_program_artifact_bytecode_v1(&bytecode)
        .expect("compiled bytecode must decode its authoritative artifact sections");
    assert_eq!(artifact.constraints().len(), 1);
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(mech::stdlib::source_catalog())
        .build()?;
    let loaded = runtime.load_bytecode_program(&bytecode, ResidentDurabilityPolicy::Volatile)?;
    assert_eq!(loaded.route, RuntimeProgramRoute::ResidentPure);

    Ok(())
}

#[test]
fn mixed_program_returns_trailing_literal_after_planned_work() -> MResult<()> {
    let (parsed, value) = run_compiled_source("x := 1.0 + 2.0\n42.0")?;
    assert_f64(&value, 42.0);
    assert_ne!(return_register(&parsed), final_binary_register(&parsed));
    Ok(())
}

#[test]
fn mixed_program_reuses_trailing_symbol_producer_register() -> MResult<()> {
    let (parsed, value) = run_compiled_source("x := 1.0 + 2.0\nx")?;
    assert_f64(&value, 3.0);
    assert_eq!(return_register(&parsed), final_binary_register(&parsed));
    Ok(())
}

#[test]
fn scalar_constants_round_trip_through_source_compilation() -> MResult<()> {
    let (_, value) = run_compiled_source("true")?;
    assert_bool(&value, true);
    let (_, value) = run_compiled_source(r#""bytecode-v1""#)?;
    assert!(matches!(value.data(), ValueData::String(actual) if actual.as_ref() == "bytecode-v1"));
    Ok(())
}

#[test]
fn dynamic_f64_matrix_add_round_trips() -> MResult<()> {
    let (_, value) = run_compiled_source("[1 2; 3 4] + [5 6; 7 8]")?;
    assert_f64_matrix(&value, &[6.0, 8.0, 10.0, 12.0], 2, 2);
    Ok(())
}

#[test]
fn variadic_f64_matrix_construction_round_trips() -> MResult<()> {
    let (parsed, value) = run_compiled_source("[1 2 3]")?;
    assert_f64_matrix(&value, &[1.0, 2.0, 3.0], 1, 3);
    assert!(
        parsed
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, BytecodeInstruction::RuntimeVariadic { .. }))
    );
    Ok(())
}

#[test]
fn source_compilation_is_byte_for_byte_deterministic() -> MResult<()> {
    let expected = compile_source("left := [1 2; 3 4]; right := [5 6; 7 8]; left + right")?;
    for _ in 0..4 {
        assert_eq!(
            compile_source("left := [1 2; 3 4]; right := [5 6; 7 8]; left + right")?,
            expected,
        );
    }
    Ok(())
}

#[test]
fn tuple_source_constant_is_encoded_by_bytecode_v1() -> MResult<()> {
    let compiled = compile_source("(1, 2)")?;
    let parsed = ParsedProgram::from_bytes(&compiled)?;
    assert!(
        parsed
            .types
            .iter()
            .any(|ty| matches!(ty, RuntimeType::Tuple(_)))
    );
    Ok(())
}

#[test]
fn outer_join_option_columns_compile_through_bytecode_v1() -> MResult<()> {
    let source = r#"
a := |id<u64> hw1<u8>| 1 10 | 2 20 | 3 30 |
b := |id<u64> hw2<u8>| 2 200 | 3 255 | 4 42 |
x := a ⟗ b
x
"#;

    let bytecode = compile_source(source)?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;

    parsed.decode_constants()?;
    assert!(parsed.types.iter().any(|runtime_type| {
        matches!(
            runtime_type,
            RuntimeType::Option(inner) if **inner == RuntimeType::U8
        )
    }));
    Ok(())
}
