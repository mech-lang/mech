use mech_core::MResult;
use mech_core::{FunctionCatalog, FunctionCatalogBuilder};
use std::sync::Arc;

/// Installs the concrete factories owned by the engine's intrinsic fragment.
///
/// Standard distribution composition lives outside the engine; this narrow
/// installer exists so composition crates can include engine-owned operations.
pub fn install_intrinsic_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::intrinsics::catalog::install_runtime(builder)
}

/// Installs the prebound dense-numeric resident factory surface. This is kept
/// separate from both legacy runtime construction and source specialization.
pub fn install_intrinsic_resident(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    #[cfg(feature = "resident-artifact")]
    crate::resident::composite::install(builder)?;
    #[cfg(feature = "resident-artifact")]
    crate::resident::numeric::install(builder)?;
    #[cfg(feature = "resident-artifact")]
    crate::resident::set::install(builder)?;
    #[cfg(feature = "resident-artifact")]
    crate::resident::text::install(builder)?;
    Ok(())
}

/// Installs the fixed-shape EKF runtime operations used by resident robotics
/// applications. The operations live in the engine because their allocation-
/// free kernels are also shared by the resident execution substrate.
#[cfg(feature = "ekf")]
pub fn install_ekf_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::efficacy::ekf::catalog::install_runtime(builder)
}

/// Installs source specializers for the public `ekf/*` module without the
/// frozen efficacy fixture's private compatibility entries.
#[cfg(all(feature = "ekf", feature = "semantic-compiler"))]
pub fn install_ekf_source(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    crate::efficacy::ekf::catalog::install_source_operations(builder)
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
