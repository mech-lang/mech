#![cfg(feature = "full_compiler")]

use mech_core::{ImplementationMemoryClass, OperationId, RuntimeBindingSelector};

#[test]
fn every_maintained_runtime_entry_declares_one_closed_memory_class() {
    let catalog = mech_stdlib::source_catalog();
    assert_ne!(catalog.runtime_entries().len(), 0);
    for entry in catalog.runtime_entries() {
        match entry.implementation_memory_class() {
            ImplementationMemoryClass::NoAdditionalScratch
            | ImplementationMemoryClass::CloneInput { .. }
            | ImplementationMemoryClass::MatrixSolve
            | ImplementationMemoryClass::CanonicalFinalize
            | ImplementationMemoryClass::CanonicalSortUnique => {}
        }
    }
}

#[test]
fn maintained_scratch_families_are_declared_by_semantic_operation() {
    let catalog = mech_stdlib::source_catalog();
    let expected = [
        ("matrix/solve", ImplementationMemoryClass::MatrixSolve),
        ("set/union", ImplementationMemoryClass::CanonicalSortUnique),
        (
            "set/intersection",
            ImplementationMemoryClass::CanonicalSortUnique,
        ),
        (
            "matrix/transpose",
            ImplementationMemoryClass::NoAdditionalScratch,
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
