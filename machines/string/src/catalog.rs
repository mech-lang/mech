use mech_core::{
    FunctionCatalogBuilder, FunctionExport, FunctionExposure, MResult, legacy_source_specializer,
};
#[cfg(feature = "concat")]
use paste::paste;

#[cfg(feature = "concat")]
use crate::concat::*;

/// Installs the frozen named source-specializer surface for the string machine.
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "concat")]
    {
        let canonical_name = "string/concat";
        let operation = builder
            .insert_specializer(canonical_name, legacy_source_specializer(StringConcat {}))?;

        builder.insert_export(FunctionExport {
            operation,
            canonical_name: canonical_name.to_string(),
            module: Some("string".to_string()),
            item: Some("concat".to_string()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
        builder.insert_export(FunctionExport {
            operation,
            canonical_name: canonical_name.to_string(),
            module: None,
            item: None,
            exposure: FunctionExposure::Internal,
        })?;
    }

    Ok(())
}

/// Installs every concrete string concatenation runtime factory enabled by
/// the selected value kinds and matrix shapes.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "concat")]
    mech_core::install_binop_runtime_factories!(
        builder,
        Concat;
        ("string", String, "string"),
    )?;

    Ok(())
}

pub fn install_catalog(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_runtime(builder)?;
    install_source(builder)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod runtime_tests {
    use super::*;
    use mech_core::FunctionDescriptor;
    use std::collections::BTreeMap;

    #[test]
    fn explicit_runtime_factories_match_the_linked_string_inventory() {
        let mut builder = FunctionCatalogBuilder::new();
        install_runtime(&mut builder).unwrap();
        let catalog = builder.build().unwrap();

        let mut legacy = BTreeMap::new();
        for descriptor in inventory::iter::<FunctionDescriptor>
            .into_iter()
            .filter(|descriptor| descriptor.name.starts_with("Concat"))
        {
            if let Some(existing) = legacy.insert(descriptor.name, descriptor.ptr) {
                assert_eq!(
                    existing as usize, descriptor.ptr as usize,
                    "conflicting legacy string factory {}",
                    descriptor.name,
                );
            }
        }

        assert_eq!(catalog.runtime_factory_count(), legacy.len());
        for entry in catalog.runtime_entries() {
            let legacy_factory = legacy
                .remove(entry.name.as_str())
                .unwrap_or_else(|| panic!("missing legacy string factory {}", entry.name));
            assert_eq!(
                entry.factory as usize, legacy_factory as usize,
                "{}",
                entry.name
            );
        }
        assert!(
            legacy.is_empty(),
            "unmigrated legacy string factories: {legacy:?}"
        );
    }
}

#[cfg(all(test, feature = "concat"))]
mod tests {
    use super::*;
    use mech_core::OperationId;

    #[test]
    fn concat_has_one_specializer_and_both_frozen_exports() {
        let mut builder = FunctionCatalogBuilder::new();
        install_source(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let operation = OperationId::from_name("string/concat");
        let exports = catalog.exports_for_operation(operation);

        assert_eq!(catalog.specializer_count(), 1);
        assert_eq!(exports.len(), 2);
        assert!(
            exports
                .iter()
                .any(|export| export.exposure == FunctionExposure::Internal)
        );
        let module_export = catalog.module_export("string", "concat").unwrap();
        assert_eq!(module_export.operation, operation);
        assert_eq!(module_export.exposure, FunctionExposure::ModuleOnly);
    }
}
