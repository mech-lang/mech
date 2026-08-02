#[path = "support/bytecode/catalog.rs"]
mod catalog;
#[path = "support/bytecode/dynamic_matrix_factory.rs"]
mod dynamic_matrix_factory;

use mech_core::matrix::Matrix;
use mech_core::{BytecodeInstruction, MResult, ParsedProgram, Ref, Value};
use mech_engine::{MechProgram, MechProgramConfig};

fn standard_program() -> MechProgram {
    MechProgram::with_function_catalog(MechProgramConfig::default(), mech::stdlib::source_catalog())
}

fn compile_source(source: &str) -> MResult<Vec<u8>> {
    let mut program = standard_program();
    program.run_string(source)?;
    program.compile_bytecode()
}

fn run_compiled_source(source: &str) -> MResult<(ParsedProgram, Value)> {
    let bytecode = compile_source(source)?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let value = standard_program().run_bytecode_program(&parsed)?;
    Ok((parsed, value))
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
    assert_eq!(value, Value::F64(Ref::new(10.0)));
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
    assert_eq!(value, Value::F64(Ref::new(3.0)));
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
fn mixed_program_returns_trailing_literal_after_planned_work() -> MResult<()> {
    let (parsed, value) = run_compiled_source("x := 1.0 + 2.0\n42.0")?;
    assert_eq!(value, Value::F64(Ref::new(42.0)));
    assert_ne!(return_register(&parsed), final_binary_register(&parsed));
    Ok(())
}

#[test]
fn mixed_program_reuses_trailing_symbol_producer_register() -> MResult<()> {
    let (parsed, value) = run_compiled_source("x := 1.0 + 2.0\nx")?;
    assert_eq!(value, Value::F64(Ref::new(3.0)));
    assert_eq!(return_register(&parsed), final_binary_register(&parsed));
    Ok(())
}

#[test]
fn phase1_scalar_constants_round_trip_through_source_compilation() -> MResult<()> {
    for (source, expected) in [
        ("true", Value::Bool(Ref::new(true))),
        (
            r#""bytecode-v1""#,
            Value::String(Ref::new("bytecode-v1".to_string())),
        ),
    ] {
        let (_, value) = run_compiled_source(source)?;
        assert_eq!(value, expected);
    }
    Ok(())
}

#[test]
fn fixed_f64_matrix_add_round_trips() -> MResult<()> {
    let (_, value) = run_compiled_source("[1 2; 3 4] + [5 6; 7 8]")?;
    assert_eq!(
        value,
        Value::MatrixF64(Matrix::from_vec(vec![6.0, 10.0, 8.0, 12.0], 2, 2)),
    );
    Ok(())
}

#[test]
fn variadic_f64_matrix_construction_round_trips() -> MResult<()> {
    let (parsed, value) = run_compiled_source("[1 2 3]")?;
    assert_eq!(
        value,
        Value::MatrixF64(Matrix::from_vec(vec![1.0, 2.0, 3.0], 1, 3)),
    );
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
fn unsupported_phase1_source_constant_is_structured_error() -> MResult<()> {
    let mut program = standard_program();
    program.run_string("(1, 2)")?;
    let error = program
        .compile_bytecode()
        .expect_err("tuple constants are frozen for Phase 2, not encoded by Phase 1");
    assert_eq!(error.kind_name(), "BytecodeConstantUnsupported");
    assert!(error.kind_message().contains("Tuple"));
    Ok(())
}
