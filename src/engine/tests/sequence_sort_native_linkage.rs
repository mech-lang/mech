#![cfg(all(feature = "sequence_sort", feature = "native-link"))]
use mech_core::{FunctionCatalogBuilder, RuntimeFunctionId};

#[test]
fn sort_indices_installer_preserves_exact_identity_without_source_features() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::__mech_native::install_sequence_sort_indices(&mut builder).unwrap();
    assert_eq!(
        mech_engine::__mech_native::install_sequence_sort_indices(&mut builder)
            .unwrap_err()
            .kind_name(),
        "FunctionCatalogDuplicateRuntimeCapability"
    );
    let catalog = builder.build().unwrap();
    assert_eq!(catalog.runtime_factory_count(), 1);
    assert_eq!(
        catalog
            .runtime_entry(RuntimeFunctionId::from_name("sequence/sort-indices"))
            .unwrap()
            .name,
        "sequence/sort-indices"
    );
}
