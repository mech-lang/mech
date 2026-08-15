use std::sync::Arc;

use mech_core::{FunctionCatalog, FunctionCatalogBuilder, LegacyValue, MResult};
use mech_engine::ProgramCompilationProduct;
use mech_runtime::{ProgramCompiler, ResidentDurabilityPolicy, RuntimeBuilder};

pub fn source_catalog() -> Arc<FunctionCatalog> {
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

pub fn compiler() -> MResult<ProgramCompiler> {
    RuntimeBuilder::new()
        .function_catalog(source_catalog())
        .build_compiler()
}

pub fn compile(source: &str) -> MResult<ProgramCompilationProduct> {
    compiler()?.compile_source(source)
}

pub fn run(source: &str) -> MResult<LegacyValue> {
    let product = compile(source)?;
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(source_catalog())
        .build()?;
    Ok(runtime
        .load_bytecode_program(product.bytecode(), ResidentDurabilityPolicy::Volatile)?
        .initial_value
        .into_value())
}
