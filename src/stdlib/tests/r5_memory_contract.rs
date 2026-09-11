#![cfg(feature = "full_compiler")]

use mech_core::{
    FunctionMatrixElement, FunctionValueRepresentation, ImplementationMemoryClass, OperationId,
    RuntimeBindingSelector,
};

#[test]
fn every_maintained_runtime_entry_declares_one_closed_memory_class() {
    let catalog = mech_stdlib::source_catalog();
    assert_ne!(catalog.runtime_entries().len(), 0);
    for entry in catalog.runtime_entries() {
        match entry.implementation_memory_class() {
            ImplementationMemoryClass::NoAdditionalScratch
            | ImplementationMemoryClass::CloneInput { .. }
            | ImplementationMemoryClass::CanonicalCloneInput { .. }
            | ImplementationMemoryClass::AbiContiguousBridge { .. }
            | ImplementationMemoryClass::ExternalMarshalling
            | ImplementationMemoryClass::MatrixSolve
            | ImplementationMemoryClass::CanonicalFinalize
            | ImplementationMemoryClass::CanonicalSortUnique => {}
        }
    }
}

#[test]
fn maintained_scratch_families_are_declared_by_concrete_specialization() {
    let catalog = mech_stdlib::source_catalog();
    let expected = [
        ("matrix/solve", ImplementationMemoryClass::MatrixSolve),
        ("set/union", ImplementationMemoryClass::CanonicalSortUnique),
        (
            "set/intersection",
            ImplementationMemoryClass::CanonicalSortUnique,
        ),
    ];
    for (operation, class) in expected {
        let operation = OperationId::from_name(operation);
        let entries = catalog
            .runtime_entries_for_binding(
                RuntimeBindingSelector::Operation(operation),
                mech_core::ExecutionTarget::DirectRuntime,
            )
            .collect::<Vec<_>>();
        assert!(
            !entries.is_empty(),
            "missing operation 0x{:016x}",
            operation.raw()
        );
        assert!(
            entries
                .iter()
                .all(|entry| entry.implementation_memory_class() == class),
            "operation 0x{:016x} has inconsistent memory classes",
            operation.raw()
        );
    }

    // Transpose has one semantic operation but two physical memory obligations:
    // fixed-width elements write directly into their admitted output, while String
    // elements must construct and finalize canonical payload storage. The closed
    // implementation class therefore belongs to the concrete specialization, not
    // to the operation name alone.
    let operation = OperationId::from_name("matrix/transpose");
    let entries = catalog
        .runtime_entries_for_binding(
            RuntimeBindingSelector::Operation(operation),
            mech_core::ExecutionTarget::DirectRuntime,
        )
        .collect::<Vec<_>>();
    assert!(!entries.is_empty(), "missing matrix/transpose");
    let mut saw_fixed_width = false;
    let mut saw_string = false;
    for entry in entries {
        let expected = match entry.signature().output {
            FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::String,
                ..
            } => {
                saw_string = true;
                ImplementationMemoryClass::CanonicalFinalize
            }
            FunctionValueRepresentation::Matrix { .. } => {
                saw_fixed_width = true;
                ImplementationMemoryClass::NoAdditionalScratch
            }
            output => panic!("matrix/transpose specialization has non-matrix output {output:?}"),
        };
        assert_eq!(
            entry.implementation_memory_class(),
            expected,
            "matrix/transpose specialization {} declares the wrong memory class",
            entry.name,
        );
    }
    assert!(saw_fixed_width, "missing fixed-width matrix/transpose");
    assert!(saw_string, "missing String matrix/transpose");
}

#[test]
fn memory_class_catalog_projection_is_deterministic() {
    fn projection() -> Vec<(u64, String)> {
        let mut projection = mech_stdlib::source_catalog()
            .runtime_entries()
            .map(|entry| {
                (
                    entry.id.raw(),
                    format!("{:?}", entry.implementation_memory_class()),
                )
            })
            .collect::<Vec<_>>();
        projection.sort();
        projection
    }
    assert_eq!(projection(), projection());
}
