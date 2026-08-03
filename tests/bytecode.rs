#[path = "support/bytecode/catalog.rs"]
mod catalog;
#[path = "support/bytecode/dynamic_matrix_factory.rs"]
mod dynamic_matrix_factory;

use mech_core::matrix::Matrix;
use mech_core::{
    BytecodeInstruction, ExecutionHostFunctionRequest, ExecutionResourceRequest, MResult,
    MechExecutionServices, ParsedProgram, Ref, RuntimeType, ValRef, Value,
};
use mech_engine::{MechProgram, MechProgramConfig};
use nalgebra::DMatrix;

#[derive(Default)]
struct RecordingExecutionServices {
    writes: Vec<Value>,
}

impl MechExecutionServices for RecordingExecutionServices {
    fn invoke_host_function(
        &mut self,
        _request: &ExecutionHostFunctionRequest,
        _arguments: &[Value],
    ) -> MResult<Value> {
        panic!("mixed-program test did not expect a host call")
    }

    fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<Value> {
        panic!("mixed-program test did not expect a resource read")
    }

    fn write_resource(
        &mut self,
        _request: &ExecutionResourceRequest,
        value: &Value,
    ) -> MResult<()> {
        self.writes.push(value.clone());
        Ok(())
    }

    fn bind_live_resource(
        &mut self,
        _interpreter_id: u64,
        _request: &ExecutionResourceRequest,
        _target: ValRef,
    ) -> MResult<()> {
        panic!("mixed-program test did not expect a live resource binding")
    }
}

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
fn mixed_program_returns_literal_after_external_send() -> MResult<()> {
    let source = "@out/line <- \"message\"\n\"final\"";
    let message = Value::String(Ref::new("message".to_string()));
    let final_value = Value::String(Ref::new("final".to_string()));

    let mut source_program = standard_program();
    source_program.run_string("@out := test://effects/output{:write(line)}")?;
    let mut source_services = RecordingExecutionServices::default();
    assert_eq!(
        source_program.run_string_with_services(source, &mut source_services)?,
        final_value,
    );
    assert_eq!(source_services.writes, vec![message.clone()]);

    let parsed = ParsedProgram::from_bytes(&source_program.compile_bytecode()?)?;
    assert!(
        parsed
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, BytecodeInstruction::ResourceSend { .. }))
    );
    let mut compiled_services = RecordingExecutionServices::default();
    let compiled_value =
        standard_program().run_bytecode_program_with_services(&parsed, &mut compiled_services)?;
    assert_eq!(compiled_value, final_value);
    assert_eq!(compiled_services.writes, vec![message]);
    Ok(())
}

#[test]
fn scalar_constants_round_trip_through_source_compilation() -> MResult<()> {
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
fn dynamic_f64_matrix_add_round_trips() -> MResult<()> {
    let (_, value) = run_compiled_source("[1 2; 3 4] + [5 6; 7 8]")?;
    assert_eq!(
        value,
        Value::MatrixF64(Matrix::DMatrix(Ref::new(DMatrix::from_vec(
            2,
            2,
            vec![6.0, 10.0, 8.0, 12.0],
        )))),
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
fn tuple_source_constant_is_encoded_by_bytecode_v1() -> MResult<()> {
    let mut program = standard_program();
    program.run_string("(1, 2)")?;
    let compiled = program.compile_bytecode()?;
    let parsed = ParsedProgram::from_bytes(&compiled)?;
    assert!(
        parsed
            .types
            .iter()
            .any(|ty| matches!(ty, RuntimeType::Tuple(_)))
    );
    Ok(())
}
