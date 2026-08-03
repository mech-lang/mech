#[cfg(feature = "no_std")]
use alloc::sync::Arc;
use mech_core::{FunctionCatalog, FunctionCatalogBuilder, MResult};
#[cfg(not(feature = "no_std"))]
use std::sync::{Arc, OnceLock};

#[cfg(all(not(feature = "no_std"), not(target_family = "wasm")))]
fn build_catalog_on_large_stack(
    name: &'static str,
    build: fn() -> MResult<FunctionCatalog>,
) -> Arc<FunctionCatalog> {
    let thread = std::thread::Builder::new()
        .name(format!("mech-stdlib-{name}-catalog"))
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            Arc::new(
                build().unwrap_or_else(|error| panic!("{name} catalog must be valid: {error:?}")),
            )
        })
        .unwrap_or_else(|error| panic!("failed to start {name} catalog builder: {error}"));
    match thread.join() {
        Ok(catalog) => catalog,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[cfg(all(not(feature = "no_std"), target_family = "wasm"))]
fn build_catalog_on_large_stack(
    name: &'static str,
    build: fn() -> MResult<FunctionCatalog>,
) -> Arc<FunctionCatalog> {
    Arc::new(build().unwrap_or_else(|error| panic!("{name} catalog must be valid: {error:?}")))
}

/// Installs the concrete runtime factories selected for this distribution.
pub fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    mech_engine::install_intrinsic_runtime(builder)?;

    #[cfg(feature = "mech-math")]
    mech_math::install_runtime(builder)?;
    #[cfg(feature = "mech-compare")]
    mech_compare::install_runtime(builder)?;
    #[cfg(feature = "mech-logic")]
    mech_logic::install_runtime(builder)?;
    #[cfg(feature = "mech-range")]
    mech_range::install_runtime(builder)?;
    #[cfg(feature = "mech-matrix")]
    mech_matrix::install_runtime(builder)?;
    #[cfg(feature = "mech-set")]
    mech_set::install_runtime(builder)?;
    #[cfg(feature = "mech-string")]
    mech_string::install_runtime(builder)?;
    #[cfg(feature = "mech-stats")]
    mech_stats::install_runtime(builder)?;
    #[cfg(feature = "mech-combinatorics")]
    mech_combinatorics::install_runtime(builder)?;

    Ok(())
}

/// Installs the source specializers selected for this distribution.
#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    mech_engine::install_intrinsic_source(builder)?;

    #[cfg(feature = "mech-math")]
    mech_math::install_source(builder)?;
    #[cfg(feature = "mech-compare")]
    mech_compare::install_source(builder)?;
    #[cfg(feature = "mech-logic")]
    mech_logic::install_source(builder)?;
    #[cfg(feature = "mech-range")]
    mech_range::install_source(builder)?;
    #[cfg(feature = "mech-matrix")]
    mech_matrix::install_source(builder)?;
    #[cfg(feature = "mech-set")]
    mech_set::install_source(builder)?;
    #[cfg(feature = "mech-string")]
    mech_string::install_source(builder)?;
    #[cfg(feature = "mech-stats")]
    mech_stats::install_source(builder)?;
    #[cfg(feature = "mech-combinatorics")]
    mech_combinatorics::install_source(builder)?;

    Ok(())
}

pub fn build_runtime_catalog() -> MResult<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    install_runtime(&mut builder)?;
    builder.build()
}

/// Builds the selected catalog for native application planning.
///
/// It begins with the frozen standard runtime surface and adds only
/// compiler-emitted factories required to resolve native application
/// bytecode. Those planning-only entries must not leak into
/// [`build_runtime_catalog`].
#[cfg(feature = "native-plan")]
pub fn build_native_plan_catalog() -> MResult<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_native_plan(&mut builder)?;

    #[cfg(feature = "mech-math")]
    mech_math::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-compare")]
    mech_compare::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-logic")]
    mech_logic::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-range")]
    mech_range::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-matrix")]
    mech_matrix::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-set")]
    mech_set::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-string")]
    mech_string::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-stats")]
    mech_stats::install_runtime(&mut builder)?;
    #[cfg(feature = "mech-combinatorics")]
    mech_combinatorics::install_runtime(&mut builder)?;

    builder.build()
}

#[cfg(feature = "source")]
pub fn build_source_catalog() -> MResult<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    install_runtime(&mut builder)?;
    install_source(&mut builder)?;
    builder.build()
}

pub fn runtime_catalog() -> Arc<FunctionCatalog> {
    #[cfg(not(feature = "no_std"))]
    {
        static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();
        Arc::clone(
            CATALOG.get_or_init(|| build_catalog_on_large_stack("runtime", build_runtime_catalog)),
        )
    }
    #[cfg(feature = "no_std")]
    {
        Arc::new(build_runtime_catalog().expect("runtime catalog must be valid"))
    }
}

/// Returns the selected runtime catalog with trusted native linkage metadata.
///
/// Source-only catalog entries intentionally omit linkage metadata; native
/// planning rejects only an actually referenced entry whose metadata is absent.
#[cfg(feature = "native-plan")]
pub fn native_plan_catalog() -> Arc<FunctionCatalog> {
    #[cfg(not(feature = "no_std"))]
    {
        static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();
        Arc::clone(
            CATALOG.get_or_init(|| {
                build_catalog_on_large_stack("native-plan", build_native_plan_catalog)
            }),
        )
    }
    #[cfg(feature = "no_std")]
    {
        Arc::new(build_native_plan_catalog().expect("native-plan catalog must be valid"))
    }
}

#[cfg(feature = "source")]
pub fn source_catalog() -> Arc<FunctionCatalog> {
    #[cfg(not(feature = "no_std"))]
    {
        static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();
        Arc::clone(
            CATALOG.get_or_init(|| build_catalog_on_large_stack("source", build_source_catalog)),
        )
    }
    #[cfg(feature = "no_std")]
    {
        Arc::new(build_source_catalog().expect("source catalog must be valid"))
    }
}

#[cfg(all(test, not(feature = "no_std")))]
mod tests {
    use super::*;

    #[test]
    fn runtime_and_source_catalogs_use_separate_caches() {
        let test = std::thread::Builder::new()
            .name("stdlib-catalog-cache-test".to_string())
            // The complete standard catalog contains thousands of factory
            // declarations; construction recursively visits enough generic
            // metadata that the platform default test stack is insufficient.
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let runtime = runtime_catalog();
                let runtime_again = runtime_catalog();
                assert!(Arc::ptr_eq(&runtime, &runtime_again));

                #[cfg(feature = "source")]
                {
                    let source = source_catalog();
                    let source_again = source_catalog();
                    assert!(Arc::ptr_eq(&source, &source_again));
                    assert!(!Arc::ptr_eq(&runtime, &source));
                }
            })
            .expect("catalog cache test thread must spawn");

        if let Err(payload) = test.join() {
            std::panic::resume_unwind(payload);
        }
    }

    #[cfg(all(feature = "native-plan", feature = "standard_runtime"))]
    #[test]
    fn native_plan_catalog_adds_only_compiler_emitted_application_factories() {
        use mech_core::RuntimeFunctionId;

        let test = std::thread::Builder::new()
            .name("stdlib-native-plan-catalog-test".to_string())
            // The complete standard catalog contains thousands of factory
            // declarations; construction recursively visits enough generic
            // metadata that the platform default test stack is insufficient.
            .stack_size(64 * 1024 * 1024)
            .spawn(|| {
                let runtime = runtime_catalog();
                assert_eq!(runtime.runtime_factory_count(), 9_019);
                let runtime_entries = runtime
                    .runtime_entries()
                    .map(|entry| (entry.id, entry.name.clone()))
                    .collect::<Vec<_>>();
                assert!(
                    runtime
                        .runtime_entry(RuntimeFunctionId::from_name(
                            "HorizontalConcatenateRDN<f64>"
                        ))
                        .is_some()
                );

                let native_plan = native_plan_catalog();
                assert_eq!(native_plan.runtime_factory_count(), 9_021);
                assert!(!Arc::ptr_eq(&runtime, &native_plan));
                let native_plan_entries = native_plan
                    .runtime_entries()
                    .map(|entry| (entry.id, entry.name.clone()))
                    .collect::<Vec<_>>();
                assert_eq!(
                    native_plan_entries
                        .iter()
                        .filter(|entry| !runtime_entries.contains(entry))
                        .map(|(_, name)| name.as_str())
                        .collect::<Vec<_>>(),
                    [
                        "HorizontalConcatenateFourArgs<f64>",
                        "HorizontalConcatenateNArgs<f64>",
                    ],
                );
                assert!(
                    runtime
                        .runtime_entry(RuntimeFunctionId::from_name(
                            "HorizontalConcatenateFourArgs<f64>",
                        ))
                        .is_none()
                );
                let entry = native_plan
                    .runtime_entry(RuntimeFunctionId::from_name(
                        "HorizontalConcatenateRDN<f64>",
                    ))
                    .expect("native-plan catalog must include compiler-emitted variadic horzcat");
                assert!(entry.native_linkage.is_some());
                let entry = native_plan
                    .runtime_entry(RuntimeFunctionId::from_name(
                        "VerticalConcatenateNArgs<f64>",
                    ))
                    .expect("native-plan catalog must include dynamic matrix construction");
                assert!(entry.native_linkage.is_some());
                for name in [
                    "HorizontalConcatenateFourArgs<f64>",
                    "HorizontalConcatenateNArgs<f64>",
                ] {
                    assert!(
                        native_plan
                            .runtime_entry(RuntimeFunctionId::from_name(name))
                            .expect("native-plan catalog must include compiler-emitted horzcat")
                            .native_linkage
                            .is_some()
                    );
                }
            })
            .expect("native-plan catalog test thread must spawn");

        if let Err(payload) = test.join() {
            std::panic::resume_unwind(payload);
        }
    }
}
