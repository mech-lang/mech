use std::sync::Arc;

use mech_core::{FunctionCatalog, FunctionCatalogBuilder, MResult};
use mech_runtime::{ProgramCompiler, RuntimeBuilder};

pub fn source_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    mech::engine::install_intrinsic_runtime(&mut builder)
        .expect("engine intrinsic runtime catalog must be valid");
    mech::engine::install_intrinsic_source(&mut builder)
        .expect("engine intrinsic source catalog must be valid");
    #[cfg(feature = "dynamic-modules")]
    for module in ["math", "combinatorics", "status-test"] {
        mech::engine::install_dynamic_source_module(&mut builder, module).unwrap_or_else(|error| {
            panic!("dynamic source module `{module}` must load: {error:?}")
        });
    }
    Arc::new(
        builder
            .build()
            .expect("engine intrinsic catalog must be valid"),
    )
}

pub fn compiler() -> MResult<ProgramCompiler> {
    RuntimeBuilder::new()
        .function_catalog(source_catalog())
        .build_compiler()
}
