#![cfg(feature = "full_compiler")]

use mech_core::{
    ExecutionTarget, FunctionRuntimeType, OperationId, RuntimeBindingSelector,
    RuntimeFunctionSignature, RuntimeOperationBinding,
};
use nalgebra::DMatrix;

#[test]
fn runtime_catalog_declares_every_operation_or_compiler_family_explicitly() {
    let catalog = mech_stdlib::source_catalog();
    for entry in catalog.runtime_entries() {
        match entry.operation_binding() {
            RuntimeOperationBinding::Fixed(operations) => {
                assert!(!operations.is_empty(), "{} has no operations", entry.name);
                assert!(operations.windows(2).all(|pair| pair[0] < pair[1]));
            }
            RuntimeOperationBinding::CompilerResolved(family) => {
                assert_ne!(
                    family.raw(),
                    0,
                    "{} has an empty compiler family",
                    entry.name
                );
            }
        }
    }

    for operation in [
        "math/add",
        "matrix/transpose",
        "range/inclusive",
        "set/union",
        "stats/sum/row",
        "combinatorics/n-choose-k",
        "compare/eq",
        "logic/and",
        "string/concat",
    ] {
        let operation = OperationId::from_name(operation);
        let entries = catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(operation),
                ExecutionTarget::DirectRuntime,
            )
            .collect::<Vec<_>>();
        assert!(!entries.is_empty(), "operation 0x{:016x}", operation.raw());
        assert!(entries.iter().all(|entry| matches!(
            entry.operation_binding(),
            RuntimeOperationBinding::Fixed(operations)
                if operations.binary_search(&operation).is_ok()
        )));
    }
}

#[test]
fn exact_operation_lookup_never_falls_back_to_a_similar_name() {
    let catalog = mech_stdlib::source_catalog();
    let add = OperationId::from_name("math/add");
    let unrelated = OperationId::from_name("math/add-similar-but-not-declared");
    assert!(
        catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(add),
                ExecutionTarget::DirectRuntime,
            )
            .next()
            .is_some()
    );
    assert!(
        catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(unrelated),
                ExecutionTarget::DirectRuntime,
            )
            .next()
            .is_none()
    );
}

#[test]
fn scalar_and_matrix_negation_bind_to_distinct_exact_capabilities() {
    let catalog = mech_stdlib::source_catalog();
    let selector = RuntimeBindingSelector::Operation(OperationId::from_name("math/neg"));
    let exact = |signature| {
        catalog
            .runtime_entries_for_binding(selector, ExecutionTarget::DirectRuntime)
            .filter(|entry| entry.signature() == signature)
            .collect::<Vec<_>>()
    };

    let scalar = exact(RuntimeFunctionSignature::unary(
        <f64 as FunctionRuntimeType>::REPRESENTATION,
        <f64 as FunctionRuntimeType>::REPRESENTATION,
    ));
    assert_eq!(scalar.len(), 1);
    assert!(scalar[0].name.starts_with("NegateS<"));

    let matrix = exact(RuntimeFunctionSignature::unary(
        <DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
        <DMatrix<f64> as FunctionRuntimeType>::REPRESENTATION,
    ));
    assert_eq!(matrix.len(), 1);
    assert!(matrix[0].name.starts_with("NegateV<"));
}
