#[cfg(feature = "no_std")]
use alloc::sync::Arc;
use mech_core::{FunctionCatalog, FunctionCatalogBuilder, MResult};
#[cfg(not(feature = "no_std"))]
use std::sync::{Arc, OnceLock};

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
        Arc::clone(CATALOG.get_or_init(|| {
            Arc::new(build_runtime_catalog().expect("runtime catalog must be valid"))
        }))
    }
    #[cfg(feature = "no_std")]
    {
        Arc::new(build_runtime_catalog().expect("runtime catalog must be valid"))
    }
}

#[cfg(feature = "source")]
pub fn source_catalog() -> Arc<FunctionCatalog> {
    #[cfg(not(feature = "no_std"))]
    {
        static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();
        Arc::clone(CATALOG.get_or_init(|| {
            Arc::new(build_source_catalog().expect("source catalog must be valid"))
        }))
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
            .stack_size(1024 * 1024)
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
}
