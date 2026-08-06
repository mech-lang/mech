#[path = "support/bytecode/catalog.rs"]
mod catalog;
#[path = "support/bytecode/dynamic_matrix_factory.rs"]
mod dynamic_matrix_factory;

use mech_core::matrix::Matrix;
use mech_core::{
    BytecodeInstruction, ExecutionHostFunctionRequest, ExecutionResourceRequest, MResult,
    MechExecutionServices, ParsedProgram, Ref, RuntimeType, ValRef, Value, ValueKind, hash_str,
};
use mech_engine::{MechProgram, MechProgramConfig, ProgramInputId, ProgramInputUpdate};
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
fn dynamic_strict_equality_round_trips_through_bytecode() -> MResult<()> {
    let (parsed, value) = run_compiled_source("x := 1 + [4 5 6]\nx === [5 6 7]")?;
    assert_eq!(value, Value::Bool(Ref::new(true)));
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
    assert_eq!(value, Value::Bool(Ref::new(true)));
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeBinary { function, .. }
            if *function == hash_str("compare/sneq")
    )));
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
    let Value::Set(inserted) = inserted else {
        panic!("set/insert must return a set");
    };
    assert_eq!(inserted.borrow().kind, ValueKind::F64);
    assert!(inserted.borrow().set.contains(&Value::F64(Ref::new(3.0))));

    let (_, removed) = run_compiled_source("set/remove({1}, 1)")?;
    let Value::Set(removed) = removed else {
        panic!("set/remove must return a set");
    };
    assert!(removed.borrow().set.is_empty());
    assert_eq!(removed.borrow().kind, ValueKind::F64);

    let (_, member) = run_compiled_source("2 ∈ {1, 2, 3}")?;
    assert_eq!(member, Value::Bool(Ref::new(true)));
    let (_, not_member) = run_compiled_source("4 ∉ {1, 2, 3}")?;
    assert_eq!(not_member, Value::Bool(Ref::new(true)));
    Ok(())
}

#[test]
fn compiled_set_membership_retains_reactive_element_cells() -> MResult<()> {
    let bytecode = compile_source("x := 2\nitems := {1, 2, 3}\nmember := x ∈ items\nmember")?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let mut compiled = standard_program();
    assert_eq!(
        compiled.run_bytecode_program(&parsed)?,
        Value::Bool(Ref::new(true))
    );

    compiled.update_inputs_and_advance_turn(&[ProgramInputUpdate {
        input: ProgramInputId {
            interpreter_id: compiled.interpreter().id,
            symbol_id: hash_str("x"),
        },
        value: Value::F64(Ref::new(4.0)),
    }])?;
    assert_eq!(
        compiled.root_symbol_value("member")?,
        Value::Bool(Ref::new(false))
    );
    Ok(())
}

#[test]
fn compiled_integrity_constraints_are_reconstructed_and_enforced() -> MResult<()> {
    let mut source_program = standard_program();
    source_program.run_string("x := 1.0\nsafe! := x <= 2.0")?;
    let source_report = source_program.integrity_constraint_report()?;
    let bytecode = source_program.compile_bytecode()?;
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

    let mut compiled = standard_program();
    compiled.run_bytecode_program(&parsed)?;
    let report = compiled.integrity_constraint_report()?;
    assert_eq!(report.evaluations.len(), 1);
    assert_eq!(report.evaluations[0].name, "safe!");
    assert_eq!(
        report.evaluations[0].expression,
        source_report.evaluations[0].expression,
    );
    assert_eq!(
        report.evaluations[0].actual,
        source_report.evaluations[0].actual,
    );
    assert_eq!(
        report.evaluations[0].expected,
        source_report.evaluations[0].expected,
    );
    assert!(report.evaluations[0].passed);

    let error = compiled
        .update_inputs_and_advance_turn(&[ProgramInputUpdate {
            input: ProgramInputId {
                interpreter_id: compiled.interpreter().id,
                symbol_id: hash_str("x"),
            },
            value: Value::F64(Ref::new(3.0)),
        }])
        .unwrap_err();
    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert_eq!(compiled.root_symbol_value("x")?, Value::F64(Ref::new(1.0)));
    assert_eq!(
        compiled.root_symbol_value("safe!")?,
        Value::Bool(Ref::new(true)),
    );
    Ok(())
}

#[test]
fn malformed_compiled_integrity_constraint_metadata_is_rejected() -> MResult<()> {
    let bytecode = compile_source("x := 1.0\nsafe! := x <= 2.0")?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;

    let mut missing_marker = parsed.clone();
    missing_marker.instructions.retain(|instruction| {
        !matches!(
            instruction,
            BytecodeInstruction::RuntimeVariadic { function, .. }
                if *function == hash_str("integrity/constraint")
        )
    });
    let error = standard_program()
        .run_bytecode_program(&missing_marker)
        .unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(
        error
            .display_message()
            .contains("missing its runtime marker")
    );

    let mut mutable_constraint = parsed;
    mutable_constraint.mutable_symbols.insert(hash_str("safe!"));
    let error = standard_program()
        .run_bytecode_program(&mutable_constraint)
        .unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeValidation");
    assert!(error.display_message().contains("must be immutable"));
    Ok(())
}

#[test]
fn compiled_integrity_constraints_preserve_absent_operands() -> MResult<()> {
    let bytecode = compile_source("always! := true")?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let mut compiled = standard_program();
    compiled.run_bytecode_program(&parsed)?;
    let state = compiled.interpreter().state.borrow();
    let constraint = state
        .integrity_constraints
        .get(&hash_str("always!"))
        .expect("compiled integrity constraint");
    assert!(constraint.operator.is_none());
    assert!(constraint.lhs.is_none());
    assert!(constraint.rhs.is_none());
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

#[test]
fn outer_join_option_columns_compile_through_bytecode_v1() -> MResult<()> {
    let source = r#"
a := |id<u64> hw1<u8>| 1 10 | 2 20 | 3 30 |
b := |id<u64> hw2<u8>| 2 200 | 3 255 | 4 42 |
x := a ⟗ b
x
"#;

    let mut source_program = standard_program();
    source_program.run_string(source)?;
    let bytecode = source_program.compile_bytecode()?;
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
