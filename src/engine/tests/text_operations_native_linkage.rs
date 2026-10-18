#![cfg(all(feature = "pure_text", feature = "native-link"))]

use mech_core::{FunctionCatalogBuilder, RuntimeFunctionId};

#[test]
fn text_installers_register_without_source_features() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::__mech_native::install_text_lines(&mut builder).unwrap();
    mech_engine::__mech_native::install_text_join(&mut builder).unwrap();
    assert_eq!(
        mech_engine::__mech_native::install_text_lines(&mut builder)
            .unwrap_err()
            .kind_name(),
        "FunctionCatalogDuplicateRuntimeCapability"
    );
    let catalog = builder.build().unwrap();
    assert_eq!(catalog.runtime_factory_count(), 2);
    for name in ["text/lines", "text/join"] {
        assert_eq!(
            catalog
                .runtime_entry(RuntimeFunctionId::from_name(name))
                .unwrap()
                .name,
            name
        );
    }
}
