#![cfg(feature = "functions")]
use mech_core::{
    BoundResidentKernel, FunctionCatalogBuilder, ImplementationMemoryClass,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelFactoryEntry,
    ResidentOperationKey,
};
use std::sync::Arc;

#[derive(Debug, Eq, PartialEq)]
struct Marker(u64);

fn factory(
    _: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    panic!("factory lifecycle tests must not bind or execute a kernel")
}

fn key() -> ResidentOperationKey {
    ResidentOperationKey::new(Box::new(["test".into()]), "factory".into()).unwrap()
}

fn entry() -> ResidentKernelFactoryEntry {
    ResidentKernelFactoryEntry::new(
        key(),
        ImplementationMemoryClass::NoAdditionalScratch,
        factory,
    )
}

#[test]
fn constructors_and_unchanged_registration_begin_without_lowering_metadata() {
    let plain = entry();
    assert_eq!(plain.key(), &key());
    assert_eq!(
        plain.implementation_memory(),
        ImplementationMemoryClass::NoAdditionalScratch
    );
    assert!(plain.lowering_witness::<Marker>().is_none());
    let mut builder = FunctionCatalogBuilder::new();
    builder
        .insert_resident_factory(
            ["test"],
            "factory",
            ImplementationMemoryClass::NoAdditionalScratch,
            factory,
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    assert!(
        catalog
            .resident_factory(&["test".into()], "factory")
            .unwrap()
            .lowering_witness::<Marker>()
            .is_none()
    );
}

#[test]
fn typed_attachment_unchanged_clone_and_catalog_clone_preserve_private_owner() {
    let original = entry().with_lowering_witness(Arc::new(Marker(41)));
    let cloned = original.clone();
    assert_eq!(cloned.lowering_witness::<Marker>(), Some(&Marker(41)));
    assert!(cloned.lowering_witness::<u64>().is_none());
    assert!(std::ptr::eq(
        original.lowering_witness::<Marker>().unwrap(),
        cloned.lowering_witness::<Marker>().unwrap()
    ));
    let mut builder = FunctionCatalogBuilder::new();
    builder.insert_resident_factory_entry(cloned).unwrap();
    let catalog = builder.build().unwrap();
    let copy = FunctionCatalogBuilder::from_catalog(&catalog)
        .build()
        .unwrap();
    let copy = copy.resident_factory(&["test".into()], "factory").unwrap();
    assert!(std::ptr::eq(
        original.lowering_witness::<Marker>().unwrap(),
        copy.lowering_witness::<Marker>().unwrap()
    ));
    let debug = format!("{copy:?}");
    assert!(debug.contains("lowering_witness: true"));
    assert!(!debug.contains("Marker"));
    assert!(!debug.contains("41"));
}

#[test]
fn all_factory_semantic_and_cost_mutations_invalidate_even_same_values() {
    let original = entry().with_lowering_witness(Arc::new(Marker(7)));
    for changed in [
        original.clone().with_key(original.key().clone()),
        original
            .clone()
            .with_implementation_memory(original.implementation_memory()),
        original.clone().with_factory(original.factory()),
    ] {
        assert!(changed.lowering_witness::<Marker>().is_none());
        assert!(changed.clone().lowering_witness::<Marker>().is_none());
    }
    let replaced = original.clone().with_lowering_witness(Arc::new(7u64));
    assert!(replaced.lowering_witness::<Marker>().is_none());
    assert_eq!(replaced.lowering_witness::<u64>(), Some(&7));
    assert_eq!(original.lowering_witness::<Marker>(), Some(&Marker(7)));
}

#[test]
fn entry_insertion_preserves_name_validation_and_duplicate_rejection() {
    let mut builder = FunctionCatalogBuilder::new();
    builder.insert_resident_factory_entry(entry()).unwrap();
    assert!(builder.insert_resident_factory_entry(entry()).is_err());
    let invalid = entry().with_key(ResidentOperationKey {
        module_path: Box::new([]),
        operation_name: "invalid".into(),
    });
    assert!(builder.insert_resident_factory_entry(invalid).is_err());
    let invalid = entry().with_key(ResidentOperationKey {
        module_path: Box::new(["test".into()]),
        operation_name: String::new(),
    });
    assert!(builder.insert_resident_factory_entry(invalid).is_err());
}
