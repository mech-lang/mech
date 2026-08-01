use mech_core::{FunctionCatalog, FunctionCatalogBuilder};
use std::sync::Arc;

/// Builds the explicit function catalog for the current standard composition.
///
/// PR1 intentionally contains only the migrated `math/add` slice. Standard
/// composition moves to `mech-stdlib` in a later transition.
pub fn default_function_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();

    #[cfg(feature = "math_add")]
    mech_math::install_math_add_catalog(&mut builder)
        .expect("static math/add catalog must be valid");

    Arc::new(
        builder
            .build()
            .expect("static function catalog must be valid"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::OperationId;

    #[cfg(feature = "math_add")]
    #[test]
    fn default_catalog_contains_only_the_migrated_math_add_specializer() {
        let catalog = default_function_catalog();
        let operation = OperationId::from_name("math/add");

        assert_eq!(catalog.specializer_count(), 1);
        assert_eq!(
            catalog.specializer(operation).unwrap().canonical_name,
            "math/add",
        );
        assert_eq!(
            catalog.module_export("math", "add").unwrap().operation,
            operation,
        );
        assert!(catalog.runtime_factory_count() > 0);
    }
}
