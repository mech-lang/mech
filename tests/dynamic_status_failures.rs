#![cfg(feature = "dynamic-modules")]

#[path = "support/intrinsic_catalog.rs"]
mod intrinsic_catalog;

use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};

fn assert_status_error(message: &str, function: &str, status: &str, code: i32) {
    assert!(message.contains(function), "missing function in {message}");
    assert!(message.contains(status), "missing status in {message}");
    assert!(
        message.contains(&format!("status {code}")),
        "missing numeric status in {message}"
    );
}

fn assert_resident_status_failure(source: &str, function: &str, status: &str, code: i32) {
    let mut compiler = intrinsic_catalog::compiler().unwrap();
    let product = compiler
        .compile_source(source)
        .expect("a valid dynamic invocation must compile to a resident artifact");
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(intrinsic_catalog::source_catalog())
        .build()
        .unwrap();
    let error = runtime
        .load_bytecode_program(product.bytecode(), ResidentDurabilityPolicy::Volatile)
        .expect_err("a dynamic provider status failure must abort resident initialization");

    assert_status_error(&error.full_chain_message(), function, status, code);

    let recovery = compiler
        .compile_source("answer := 42.0\nanswer")
        .expect("a provider failure must not poison the compiler workspace");
    runtime
        .load_bytecode_program(recovery.bytecode(), ResidentDurabilityPolicy::Volatile)
        .expect("a provider failure must not poison the runtime workspace");
}

#[test]
fn unary_status_failure_reaches_the_caller() {
    assert_resident_status_failure(
        "+> status-test/unary
y := unary(2.0)
y",
        "status-test/unary",
        "WrongShape",
        4,
    );
}

#[test]
fn scalar_binary_status_failure_reaches_the_caller() {
    assert_resident_status_failure(
        "+> status-test/binary
y := binary(2.0, 3.0)
y",
        "status-test/binary",
        "Unsupported",
        5,
    );
}

#[test]
fn view_status_failure_reaches_the_caller() {
    assert_resident_status_failure(
        "+> status-test/view
y := view([1.0 2.0])
y",
        "status-test/view",
        "Panic",
        6,
    );
}
