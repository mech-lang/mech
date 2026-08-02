use std::sync::Arc;

use mech::{MechProgram, MechProgramConfig};
use mech_core::{FunctionCatalog, FunctionCatalogBuilder};

fn source_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    mech::engine::install_intrinsic_runtime(&mut builder)
        .expect("engine intrinsic runtime catalog must be valid");
    mech::engine::install_intrinsic_source(&mut builder)
        .expect("engine intrinsic source catalog must be valid");
    Arc::new(
        builder
            .build()
            .expect("engine intrinsic catalog must be valid"),
    )
}

pub fn program() -> MechProgram {
    MechProgram::with_function_catalog(MechProgramConfig::default(), source_catalog())
}
