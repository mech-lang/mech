#![cfg(feature = "dynamic-modules")]

#[path = "support/intrinsic_catalog.rs"]
mod intrinsic_catalog;

use mech::MechProgram;

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
    let mut program = intrinsic_catalog::program();
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
    let mut program = intrinsic_catalog::program();
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
    let mut program = intrinsic_catalog::program();
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
