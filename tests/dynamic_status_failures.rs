#![cfg(feature = "dynamic-modules")]

use mech::program::{MechProgram, MechProgramConfig, ProgramInputId, ProgramInputUpdate};
use mech_core::{Ref, Value, hash_str, structures::matrix::Matrix};

fn unwrap_value(value: Value) -> Value {
    match value {
        Value::MutableReference(reference) => unwrap_value(reference.borrow().clone()),
        Value::Typed(value, _) => unwrap_value(*value),
        value => value,
    }
}

fn f64_output(value: Value) -> f64 {
    match unwrap_value(value) {
        Value::F64(value) => *value.borrow(),
        value => panic!("expected f64 output, got {value:?}"),
    }
}

fn assert_matrix_output(value: Value, expected: &[f64], rows: usize, cols: usize) {
    let Value::MatrixF64(matrix) = unwrap_value(value) else {
        panic!("expected f64 matrix output");
    };

    assert_eq!((matrix.rows(), matrix.cols()), (rows, cols));
    assert_eq!(
        (1..=rows * cols)
            .map(|index| matrix.index1d(index))
            .collect::<Vec<_>>(),
        expected,
    );
}

fn matrix_value(values: Vec<f64>, rows: usize, cols: usize) -> Value {
    Value::MatrixF64(Matrix::from_vec(values, rows, cols))
}

fn ensure_input(program: &mut MechProgram, name: &str, value: Value) -> ProgramInputId {
    program
        .ensure_input(program.interpreter().id, hash_str(name), name, value)
        .unwrap()
}

fn symbol_value(program: &MechProgram, name: &str) -> Value {
    let symbols = program.interpreter().symbols();
    let value = symbols
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol `{name}`"));
    value.borrow().clone()
}

fn assert_status_error(message: &str, function: &str, status: &str, code: i32) {
    assert!(message.contains(function), "missing function in {message}");
    assert!(message.contains(status), "missing status in {message}");
    assert!(
        message.contains(&format!("status {code}")),
        "missing numeric status in {message}"
    );
}

fn assert_no_dynamic_node(program: &MechProgram, function: &str) {
    let plan = program.interpreter().plan();
    assert!(
        !plan
            .borrow()
            .nodes
            .iter()
            .any(|node| node.function.to_string() == format!("dynamic {function}")),
        "failed dynamic function `{function}` remained registered"
    );
}

#[test]
fn unary_status_failure_reaches_the_caller() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let plan_len = program.interpreter().plan_len();
    let error = program
        .run_string(
            "+> status-test/unary
y := unary(2.0)
y",
        )
        .expect_err("a dynamic unary status failure must abort eager execution");

    assert_status_error(
        &error.full_chain_message(),
        "status-test/unary",
        "WrongShape",
        4,
    );
    assert_eq!(program.interpreter().plan_len(), plan_len);
}

#[test]
fn scalar_binary_status_failure_reaches_the_caller() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let plan_len = program.interpreter().plan_len();
    let error = program
        .run_string(
            "+> status-test/binary
y := binary(2.0, 3.0)
y",
        )
        .expect_err("a dynamic binary status failure must abort eager execution");

    assert_status_error(
        &error.full_chain_message(),
        "status-test/binary",
        "Unsupported",
        5,
    );
    assert_eq!(program.interpreter().plan_len(), plan_len);
}

#[test]
fn view_status_failure_reaches_the_caller() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let error = program
        .run_string(
            "+> status-test/view
y := view([1.0 2.0])
y",
        )
        .expect_err("a dynamic view status failure must abort eager execution");

    assert_status_error(&error.full_chain_message(), "status-test/view", "Panic", 6);
    assert_no_dynamic_node(&program, "status-test/view");
}

#[test]
fn unary_scalar_output_survives_failure() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input = ensure_input(&mut program, "x", Value::F64(Ref::new(1.0)));
    program
        .run_string(
            "+> status-test/unary
y := unary(x)
y",
        )
        .unwrap();
    assert_eq!(f64_output(symbol_value(&program, "y")), 10.0);

    let error = program
        .update_inputs_and_advance_turn(&[ProgramInputUpdate {
            input,
            value: Value::F64(Ref::new(2.0)),
        }])
        .expect_err("a failed reactive unary kernel must return an error");

    assert_status_error(
        &error.full_chain_message(),
        "status-test/unary",
        "WrongShape",
        4,
    );
    assert_eq!(f64_output(symbol_value(&program, "x")), 2.0);
    assert_eq!(f64_output(symbol_value(&program, "y")), 10.0);
}

#[test]
fn binary_scalar_output_survives_failure() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input = ensure_input(&mut program, "x", Value::F64(Ref::new(1.0)));
    program
        .run_string(
            "+> status-test/binary
y := binary(x, 3.0)
y",
        )
        .unwrap();
    assert_eq!(f64_output(symbol_value(&program, "y")), 4.0);

    let error = program
        .update_inputs_and_advance_turn(&[ProgramInputUpdate {
            input,
            value: Value::F64(Ref::new(2.0)),
        }])
        .expect_err("a failed reactive binary kernel must return an error");

    assert_status_error(
        &error.full_chain_message(),
        "status-test/binary",
        "Unsupported",
        5,
    );
    assert_eq!(f64_output(symbol_value(&program, "x")), 2.0);
    assert_eq!(f64_output(symbol_value(&program, "y")), 4.0);
}

#[test]
fn broadcast_output_is_all_or_nothing() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input = ensure_input(&mut program, "x", matrix_value(vec![1.0, 1.0], 1, 2));
    program
        .run_string(
            "+> status-test/binary
y := binary(x, 3.0)
y",
        )
        .unwrap();
    assert_matrix_output(symbol_value(&program, "y"), &[4.0, 4.0], 1, 2);

    let error = program
        .update_inputs_and_advance_turn(&[ProgramInputUpdate {
            input,
            value: matrix_value(vec![1.0, 2.0], 1, 2),
        }])
        .expect_err("a failed reactive broadcast kernel must return an error");

    assert_status_error(
        &error.full_chain_message(),
        "status-test/binary",
        "Unsupported",
        5,
    );
    assert_matrix_output(symbol_value(&program, "x"), &[1.0, 2.0], 1, 2);
    assert_matrix_output(symbol_value(&program, "y"), &[4.0, 4.0], 1, 2);
}

#[test]
fn view_output_is_all_or_nothing() {
    let mut program = MechProgram::new(MechProgramConfig::default());
    let input = ensure_input(&mut program, "x", matrix_value(vec![1.0, 1.0], 1, 2));
    program
        .run_string(
            "+> status-test/view
y := view(x)
y",
        )
        .unwrap();
    assert_matrix_output(symbol_value(&program, "y"), &[10.0, 10.0], 1, 2);

    let error = program
        .update_inputs_and_advance_turn(&[ProgramInputUpdate {
            input,
            value: matrix_value(vec![1.0, 2.0], 1, 2),
        }])
        .expect_err("a failed reactive view kernel must return an error");

    assert_status_error(&error.full_chain_message(), "status-test/view", "Panic", 6);
    assert_matrix_output(symbol_value(&program, "x"), &[1.0, 2.0], 1, 2);
    assert_matrix_output(symbol_value(&program, "y"), &[10.0, 10.0], 1, 2);
}
