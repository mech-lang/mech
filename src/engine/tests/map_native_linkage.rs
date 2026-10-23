#![cfg(all(feature = "map_ops", feature = "native-link"))]
use mech_core::{FunctionCatalogBuilder, RuntimeFunctionId};

#[test]
fn map_installers_preserve_exact_identity_without_source_compiler_features() {
    type Installer = fn(&mut FunctionCatalogBuilder) -> mech_core::MResult<()>;
    for (name, install) in [
        (
            "map/new",
            mech_engine::__mech_native::install_map_new as Installer,
        ),
        (
            "map/histogram",
            mech_engine::__mech_native::install_map_histogram as Installer,
        ),
        (
            "map/keys",
            mech_engine::__mech_native::install_map_keys as Installer,
        ),
        (
            "map/values",
            mech_engine::__mech_native::install_map_values as Installer,
        ),
    ] {
        let mut builder = FunctionCatalogBuilder::new();
        install(&mut builder).unwrap();
        assert_eq!(
            install(&mut builder).unwrap_err().kind_name(),
            "FunctionCatalogDuplicateRuntimeCapability"
        );
        let catalog = builder.build().unwrap();
        assert_eq!(catalog.runtime_factory_count(), 1);
        assert_eq!(
            catalog
                .runtime_entry(RuntimeFunctionId::from_name(name))
                .unwrap()
                .name,
            name
        );
    }
}
