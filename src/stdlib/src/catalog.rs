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
    #[cfg(feature = "semantic-compiler")]
    mech_engine::install_ekf_runtime(builder)?;

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

    mech_engine::install_intrinsic_resident(builder)?;

    Ok(())
}

/// Installs the source specializers selected for this distribution.
#[cfg(feature = "source")]
pub fn install_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    mech_engine::install_intrinsic_source(builder)?;
    #[cfg(feature = "semantic-compiler")]
    mech_engine::install_ekf_source(builder)?;

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
pub fn install_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    mech_engine::install_intrinsic_native_plan(builder)?;
    #[cfg(feature = "semantic-compiler")]
    mech_engine::install_ekf_runtime(builder)?;

    #[cfg(feature = "mech-math")]
    mech_math::install_native_plan(builder)?;
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

#[cfg(feature = "native-plan")]
pub fn build_native_plan_catalog() -> MResult<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    install_native_plan(&mut builder)?;
    builder.build()
}

#[cfg(feature = "source")]
pub fn build_source_catalog() -> MResult<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    #[cfg(feature = "native-plan")]
    {
        install_native_plan(&mut builder)?;
        // Native planning replaces the runtime-factory installation above,
        // but a source compiler also activates the ProgramArtifact it emits.
        // Preserve the resident binders normally installed by
        // `install_runtime` so the compiler catalog remains activation-ready.
        mech_engine::install_intrinsic_resident(&mut builder)?;
    }
    #[cfg(not(feature = "native-plan"))]
    install_runtime(&mut builder)?;
    install_source(&mut builder)?;
    builder.build()
}

/// Builds a source catalog that can also resolve compiler-emitted native-plan
/// bridge operations. This is intentionally separate from the frozen source
/// catalog used by the interpreter.
#[cfg(all(feature = "source", feature = "native-plan"))]
pub fn build_source_native_plan_catalog() -> MResult<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    install_native_plan(&mut builder)?;
    // Source compilation immediately activates the semantic artifact it
    // emits, so this catalog needs the same resident binders as the ordinary
    // source catalog in addition to native-plan linkage.
    mech_engine::install_intrinsic_resident(&mut builder)?;
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

#[cfg(all(feature = "source", feature = "native-plan"))]
pub fn source_native_plan_catalog() -> Arc<FunctionCatalog> {
    #[cfg(not(feature = "no_std"))]
    {
        static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();
        Arc::clone(CATALOG.get_or_init(|| {
            build_catalog_on_large_stack("source-native-plan", build_source_native_plan_catalog)
        }))
    }
    #[cfg(feature = "no_std")]
    {
        Arc::new(
            build_source_native_plan_catalog().expect("source native-plan catalog must be valid"),
        )
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

                    #[cfg(feature = "native-plan")]
                    {
                        let source_native_plan = source_native_plan_catalog();
                        let source_native_plan_again = source_native_plan_catalog();
                        assert!(Arc::ptr_eq(&source_native_plan, &source_native_plan_again));
                        assert!(!Arc::ptr_eq(&source, &source_native_plan));
                    }
                }
            })
            .expect("catalog cache test thread must spawn");

        if let Err(payload) = test.join() {
            std::panic::resume_unwind(payload);
        }
    }

    #[cfg(all(feature = "native-plan", feature = "full_runtime"))]
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
                assert_eq!(native_plan.runtime_factory_count(), 9_116);
                assert!(!Arc::ptr_eq(&runtime, &native_plan));
                let native_plan_entries = native_plan
                    .runtime_entries()
                    .map(|entry| (entry.id, entry.name.clone()))
                    .collect::<Vec<_>>();
                let mut expected_native_plan_only = [
                    "AddM1MD<i8>",
                    "AddM1MD<i16>",
                    "AddM1MD<i32>",
                    "AddM1MD<i64>",
                    "AddM1MD<i128>",
                    "AddM1MD<u8>",
                    "AddM1MD<u16>",
                    "AddM1MD<u32>",
                    "AddM1MD<u64>",
                    "AddM1MD<u128>",
                    "AddM1MD<f32>",
                    "AddM1MD<f64>",
                    "AddM1MD<rational>",
                    "AddM1MD<complex>",
                    "AddMDM1<i8>",
                    "AddMDM1<i16>",
                    "AddMDM1<i32>",
                    "AddMDM1<i64>",
                    "AddMDM1<i128>",
                    "AddMDM1<u8>",
                    "AddMDM1<u16>",
                    "AddMDM1<u32>",
                    "AddMDM1<u64>",
                    "AddMDM1<u128>",
                    "AddMDM1<f32>",
                    "AddMDM1<f64>",
                    "AddMDM1<rational>",
                    "AddMDM1<complex>",
                    "HorizontalConcatenateFourArgs<f64>",
                    "HorizontalConcatenateNArgs<f64>",
                    "RecordAccessField",
                    "RecordAccessSwizzle",
                    "TableAccessSwizzle",
                    "VariableDefineMatrix<u8Matrix1>",
                    "VariableDefineMatrix<u16Matrix1>",
                    "VariableDefineMatrix<u32Matrix1>",
                    "VariableDefineMatrix<u64Matrix1>",
                    "VariableDefineMatrix<u128Matrix1>",
                    "VariableDefineMatrix<i8Matrix1>",
                    "VariableDefineMatrix<i16Matrix1>",
                    "VariableDefineMatrix<i32Matrix1>",
                    "VariableDefineMatrix<i64Matrix1>",
                    "VariableDefineMatrix<i128Matrix1>",
                    "VariableDefineMatrix<f32Matrix1>",
                    "VariableDefineMatrix<f64Matrix1>",
                    "VariableDefineMatrix<rationalMatrix1>",
                    "VariableDefineMatrix<complexMatrix1>",
                    "VariableDefineMatrix<boolMatrix1>",
                    "VariableDefineMatrix<stringMatrix1>",
                ]
                .into_iter()
                .map(str::to_string)
                .collect::<std::collections::BTreeSet<_>>();
                for scalar in [
                    "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128", "f32",
                    "f64", "bool", "string", "rational", "complex",
                ] {
                    for shape in ["DMatrix", "DVector", "RowDVector"] {
                        expected_native_plan_only.insert(format!("Assign<{scalar}{shape}>"));
                    }
                }
                assert_eq!(
                    native_plan_entries
                        .iter()
                        .filter(|entry| !runtime_entries.contains(entry))
                        .map(|(_, name)| name.clone())
                        .collect::<std::collections::BTreeSet<_>>(),
                    expected_native_plan_only,
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
                for name in expected_native_plan_only {
                    assert!(
                        native_plan
                            .runtime_entry(RuntimeFunctionId::from_name(&name))
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
