#[cfg(feature = "matrix_comprehensions")]
use super::super::ValueMatrixComprehension;
#[cfg(feature = "set_comprehensions")]
use super::super::ValueSetComprehension;
use crate::{FunctionArgs, MechFunctionFactory, MechFunctionImpl, MechSet, Ref, Value};
#[cfg(feature = "compiler")]
use mech_bytecode::CompileCtx;
#[cfg(feature = "compiler")]
use mech_core::{BytecodeInstruction, MechFunctionCompiler, ParsedProgram, hash_str};
#[cfg(feature = "matrix_comprehensions")]
use nalgebra::DMatrix;

#[cfg(feature = "matrix_comprehensions")]
#[test]
fn transaction_state_retains_matrix_comprehension_outer_output_ref() {
    let out = Ref::new(Value::Empty);
    let function = ValueMatrixComprehension {
        arguments: Vec::new(),
        out: out.clone(),
    };

    let values = function.transaction_state_values().unwrap();
    assert_eq!(values.len(), 1);
    match &values[0] {
        Value::MutableReference(root) => assert_eq!(root.addr(), out.addr()),
        other => panic!("expected mutable-reference transaction root, got {other:?}"),
    }
}

#[cfg(feature = "matrix_comprehensions")]
#[test]
fn matrix_comprehension_factory_reconstructs_variadic_inputs() {
    let first = Value::from(1.0f64);
    let second = Value::from(2.0f64);
    let function = ValueMatrixComprehension::new(FunctionArgs::Variadic(
        Value::MatrixF64(crate::Matrix::DMatrix(Ref::new(DMatrix::zeros(1, 2)))),
        vec![first, second],
    ))
    .unwrap();
    function.solve_result().unwrap();
    let Value::MatrixF64(matrix) = function.out() else {
        panic!("expected matrix comprehension output")
    };
    assert_eq!(matrix.as_vec(), vec![1.0, 2.0]);
}

#[cfg(feature = "set_comprehensions")]
#[test]
fn set_comprehension_factory_preserves_checked_set_output() {
    let output = Ref::new(MechSet::from_vec(Vec::new()));
    let function = ValueSetComprehension::new(FunctionArgs::Variadic(
        Value::Set(output.clone()),
        vec![Value::from(7u8)],
    ))
    .unwrap();
    function.solve_result().unwrap();
    let Value::Set(actual) = function.out() else {
        panic!("expected set comprehension output")
    };
    assert_eq!(actual.addr(), output.addr());
    assert_eq!(actual.borrow().set.len(), 1);
}

#[cfg(feature = "set_comprehensions")]
#[test]
fn set_comprehension_factory_rejects_non_set_output() {
    let result = ValueSetComprehension::new(FunctionArgs::Variadic(Value::Empty, Vec::new()));
    let error = match result {
        Ok(_) => panic!("non-set bytecode output should be rejected"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind_name(),
        "SetComprehensionOutputKindMismatch".to_string()
    );
}

#[cfg(all(feature = "set_comprehensions", feature = "compiler"))]
#[test]
fn set_comprehension_bytecode_encodes_ordered_child_registers() {
    let first = Value::from(1u8);
    let second = Value::from(2u8);
    let function = ValueSetComprehension {
        arguments: vec![first, second],
        out: Ref::new(MechSet::from_vec(Vec::new())),
    };
    let mut context = CompileCtx::new();
    let output = function.compile(&mut context).unwrap();
    let parsed = ParsedProgram::from_bytes(&context.finish(output).unwrap()).unwrap();
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeVariadic { function, arguments, .. }
            if *function == hash_str("set/comprehension")
                && arguments.len() == 2
                && arguments[0] != arguments[1]
    )));
}

#[cfg(all(feature = "matrix_comprehensions", feature = "compiler"))]
#[test]
fn matrix_comprehension_bytecode_reuses_repeated_child_registers() {
    let repeated = Value::from(3.0f64);
    let function = ValueMatrixComprehension {
        arguments: vec![repeated.clone(), repeated],
        out: Ref::new(Value::MatrixF64(crate::Matrix::DMatrix(Ref::new(
            DMatrix::zeros(1, 2),
        )))),
    };
    let mut context = CompileCtx::new();
    let output = function.compile(&mut context).unwrap();
    let parsed = ParsedProgram::from_bytes(&context.finish(output).unwrap()).unwrap();
    assert!(parsed.instructions.iter().any(|instruction| matches!(
        instruction,
        BytecodeInstruction::RuntimeVariadic { function, arguments, .. }
            if *function == hash_str("matrix/comprehension")
                && arguments.len() == 2
                && arguments[0] == arguments[1]
    )));
}
