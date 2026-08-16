#![cfg(feature = "dynamic-modules")]

#[path = "support/intrinsic_catalog.rs"]
mod intrinsic_catalog;

fn assert_status_error(message: &str, function: &str, status: &str, code: i32) {
    assert!(message.contains(function), "missing function in {message}");
    assert!(message.contains(status), "missing status in {message}");
    assert!(
        message.contains(&format!("status {code}")),
        "missing numeric status in {message}"
    );
}

#[test]
fn unary_status_failure_reaches_the_caller() {
    let mut compiler = intrinsic_catalog::compiler().unwrap();
    let error = compiler
        .compile_source(
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
    compiler
        .compile_source("answer := 42.0\nanswer")
        .expect("a failed compilation must not poison the compiler workspace");
}

#[test]
fn scalar_binary_status_failure_reaches_the_caller() {
    let mut compiler = intrinsic_catalog::compiler().unwrap();
    let error = compiler
        .compile_source(
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
    compiler
        .compile_source("answer := 42.0\nanswer")
        .expect("a failed compilation must not poison the compiler workspace");
}

#[test]
fn view_status_failure_reaches_the_caller() {
    let mut compiler = intrinsic_catalog::compiler().unwrap();
    let error = compiler
        .compile_source(
            "+> status-test/view
y := view([1.0 2.0])
y",
        )
        .expect_err("a dynamic view status failure must abort eager execution");

    assert_status_error(&error.full_chain_message(), "status-test/view", "Panic", 6);
    compiler
        .compile_source("answer := 42.0\nanswer")
        .expect("a failed compilation must not poison the compiler workspace");
}
