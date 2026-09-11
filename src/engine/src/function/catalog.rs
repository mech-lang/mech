#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::sync::Arc;
use mech_core::MResult;
use mech_core::{FunctionCatalog, FunctionCatalogBuilder};
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::sync::Arc;

/// Installs the concrete factories owned by the engine's intrinsic fragment.
///
/// Standard distribution composition lives outside the engine; this narrow
/// installer exists so composition crates can include engine-owned operations.
pub fn install_intrinsic_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_runtime(builder)
}

/// Installs compiler-emitted engine factories into a source compiler catalog
/// without expanding the runtime-only distribution surface.
#[cfg(feature = "semantic-compiler")]
pub fn install_intrinsic_compiler_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_compiler_runtime(builder)
}

/// Installs the prebound dense-numeric resident factory surface. This is kept
/// separate from direct runtime construction and source specialization.
pub fn install_intrinsic_resident(
    #[cfg(feature = "resident-artifact")] builder: &mut FunctionCatalogBuilder,
    #[cfg(not(feature = "resident-artifact"))] _: &mut FunctionCatalogBuilder,
) -> MResult<()> {
    #[cfg(feature = "resident-artifact")]
    crate::resident::composite::install(builder)?;
    #[cfg(all(feature = "resident-artifact", feature = "convert"))]
    crate::resident::conversion::install(builder)?;
    #[cfg(feature = "resident-artifact")]
    crate::resident::matrix_literal::install(builder)?;
    #[cfg(feature = "resident-artifact")]
    crate::resident::numeric::install(builder)?;
    #[cfg(feature = "resident-artifact")]
    crate::resident::set::install(builder)?;
    #[cfg(all(feature = "resident-artifact", feature = "table"))]
    crate::resident::table::install(builder)?;
    #[cfg(feature = "resident-artifact")]
    crate::resident::text::install(builder)?;
    Ok(())
}

/// Installs the engine runtime fragment plus compiler-emitted factories needed
/// only while planning native applications.
#[cfg(feature = "native-plan")]
pub fn install_intrinsic_native_plan(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    install_intrinsic_runtime(builder)?;
    crate::intrinsics::catalog::install_native_plan(builder)
}

/// Installs the source specializers owned by the engine's intrinsic fragment.
#[cfg(feature = "semantic-compiler")]
pub fn install_intrinsic_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_source(builder)
}

/// Returns a new empty catalog for a bare engine instance.
pub fn empty_function_catalog() -> Arc<FunctionCatalog> {
    Arc::new(FunctionCatalog::empty())
}
