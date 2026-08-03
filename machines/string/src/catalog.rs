use mech_core::{FunctionCatalogBuilder, MResult};
#[cfg(feature = "source")]
use mech_core::{FunctionExport, FunctionExposure};
#[cfg(feature = "concat")]
use paste::paste;
#[cfg(feature = "source")]
use std::sync::Arc;

#[cfg(feature = "concat")]
use crate::concat::*;

/// Installs the frozen named source-specializer surface for the string machine.
#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "concat")]
    {
        let canonical_name = "string/concat";
        let operation = builder.insert_specializer(canonical_name, Arc::new(StringConcat {}))?;

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
mech_core::declare_native_binop_runtime_factories! {
    package: "mech-string",
    crate_name: "mech_string",
    operation: Concat,
    operation_feature: "concat",
    additional_features: [],
    scalars: ("string", String, "string", string),
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    mech_core::export_native_binop_runtime_factories! {
        operation_feature: "concat",
        operation: Concat;
        ("string", String, "string", string),
    }
}

pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "concat")]
    mech_core::install_native_binop_runtime_factories!(
        builder,
        Concat;
        ("string", String, "string", string),
    )?;

    Ok(())
}

#[cfg(all(test, feature = "source", feature = "concat"))]
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
