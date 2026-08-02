use mech_core::{FunctionCatalog, FunctionCatalogBuilder};
use std::sync::{Arc, OnceLock};

/// Installs the concrete factories owned by the engine's intrinsic fragment.
///
/// Standard distribution composition lives outside the engine; this narrow
/// installer exists so composition crates can include engine-owned operations.
pub fn install_intrinsic_runtime(builder: &mut FunctionCatalogBuilder) -> mech_core::MResult<()> {
    crate::stdlib::catalog::install_runtime(builder)
}

/// Installs the source specializers owned by the engine's intrinsic fragment.
pub fn install_intrinsic_source(builder: &mut FunctionCatalogBuilder) -> mech_core::MResult<()> {
    crate::stdlib::catalog::install_source(builder)
}

fn build_default_function_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();

    crate::stdlib::catalog::install_runtime(&mut builder)
        .expect("engine function catalog fragment must be valid");
    #[cfg(feature = "compiler")]
    crate::stdlib::catalog::install_source(&mut builder)
        .expect("engine source catalog fragment must be valid");
    #[cfg(feature = "mech-math")]
    mech_math::install_runtime(&mut builder).expect("math runtime catalog fragment must be valid");
    #[cfg(all(feature = "mech-math", feature = "compiler"))]
    mech_math::install_source(&mut builder).expect("math source catalog fragment must be valid");
    #[cfg(feature = "mech-compare")]
    mech_compare::install_runtime(&mut builder)
        .expect("compare runtime catalog fragment must be valid");
    #[cfg(all(feature = "mech-compare", feature = "compiler"))]
    mech_compare::install_source(&mut builder)
        .expect("compare source catalog fragment must be valid");
    #[cfg(feature = "mech-logic")]
    mech_logic::install_runtime(&mut builder)
        .expect("logic runtime catalog fragment must be valid");
    #[cfg(all(feature = "mech-logic", feature = "compiler"))]
    mech_logic::install_source(&mut builder).expect("logic source catalog fragment must be valid");
    #[cfg(feature = "mech-range")]
    mech_range::install_runtime(&mut builder)
        .expect("range runtime catalog fragment must be valid");
    #[cfg(all(feature = "mech-range", feature = "compiler"))]
    mech_range::install_source(&mut builder).expect("range source catalog fragment must be valid");
    #[cfg(feature = "mech-matrix")]
    mech_matrix::install_runtime(&mut builder)
        .expect("matrix runtime catalog fragment must be valid");
    #[cfg(all(feature = "mech-matrix", feature = "compiler"))]
    mech_matrix::install_source(&mut builder)
        .expect("matrix source catalog fragment must be valid");
    #[cfg(feature = "mech-set")]
    mech_set::install_runtime(&mut builder).expect("set runtime catalog fragment must be valid");
    #[cfg(all(feature = "mech-set", feature = "compiler"))]
    mech_set::install_source(&mut builder).expect("set source catalog fragment must be valid");
    #[cfg(feature = "mech-string")]
    mech_string::install_runtime(&mut builder)
        .expect("string runtime catalog fragment must be valid");
    #[cfg(all(feature = "mech-string", feature = "compiler"))]
    mech_string::install_source(&mut builder)
        .expect("string source catalog fragment must be valid");
    #[cfg(feature = "stats")]
    mech_stats::install_runtime(&mut builder)
        .expect("stats runtime catalog fragment must be valid");
    #[cfg(all(feature = "stats", feature = "compiler"))]
    mech_stats::install_source(&mut builder).expect("stats source catalog fragment must be valid");
    #[cfg(feature = "combinatorics")]
    mech_combinatorics::install_runtime(&mut builder)
        .expect("combinatorics runtime catalog fragment must be valid");
    #[cfg(all(feature = "combinatorics", feature = "compiler"))]
    mech_combinatorics::install_source(&mut builder)
        .expect("combinatorics source catalog fragment must be valid");

    Arc::new(
        builder
            .build()
            .expect("static function catalog must be valid"),
    )
}

/// Returns the immutable function catalog for the standard distribution.
///
/// Standard composition moves to `mech-stdlib` in PR3. PR2 keeps the explicit
/// composition here while eliminating all discovery through registries.
pub fn default_function_catalog() -> Arc<FunctionCatalog> {
    static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();
    Arc::clone(CATALOG.get_or_init(build_default_function_catalog))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn static_exports(catalog: &FunctionCatalog) -> Vec<&mech_core::FunctionExport> {
        catalog
            .specializer_entries()
            .flat_map(|entry| catalog.exports_for_operation(entry.operation))
            .collect()
    }

    #[cfg(feature = "linked_stdlib")]
    #[test]
    fn linked_standard_catalog_matches_the_frozen_source_surface_counts() {
        let catalog = default_function_catalog();
        let exports = static_exports(&catalog);

        assert_eq!(catalog.specializer_count(), 119);
        assert_eq!(catalog.intrinsic_specializer_count(), 10);
        assert_eq!(
            exports
                .iter()
                .filter(|export| export.exposure == mech_core::FunctionExposure::Prelude)
                .count(),
            52,
        );
        assert_eq!(
            exports
                .iter()
                .filter(|export| export.module.is_some())
                .count(),
            50,
        );
        assert_eq!(exports.len(), 120);
    }

    #[test]
    fn default_catalog_accessor_reuses_the_same_cached_catalog() {
        let first = default_function_catalog();
        let second = default_function_catalog();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn default_catalog_builds_on_a_windows_sized_stack() {
        const WINDOWS_DEFAULT_STACK_SIZE: usize = 1024 * 1024;

        std::thread::Builder::new()
            .name(String::from("default-function-catalog-small-stack"))
            .stack_size(WINDOWS_DEFAULT_STACK_SIZE)
            .spawn(build_default_function_catalog)
            .expect("catalog construction thread must start")
            .join()
            .expect("catalog construction must fit on the Windows default stack");
    }
}
