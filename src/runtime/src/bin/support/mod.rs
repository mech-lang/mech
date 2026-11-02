use std::sync::{Arc, OnceLock};

use mech_core::{FunctionCatalog, FunctionCatalogBuilder};
use mech_runtime::RuntimeBuilder;

pub fn intrinsic_source_catalog() -> Arc<FunctionCatalog> {
    static CATALOG: OnceLock<Arc<FunctionCatalog>> = OnceLock::new();

    Arc::clone(CATALOG.get_or_init(|| {
        let mut builder = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_runtime(&mut builder)
            .expect("engine intrinsic runtime fragment must be valid");
        mech_engine::install_intrinsic_source(&mut builder)
            .expect("engine intrinsic source fragment must be valid");
        Arc::new(
            builder
                .build()
                .expect("engine intrinsic source catalog must be valid"),
        )
    }))
}

pub fn source_runtime_builder() -> RuntimeBuilder {
    RuntimeBuilder::new().function_catalog(intrinsic_source_catalog())
}
