#[path = "intrinsic_catalog.rs"]
mod intrinsic_catalog;

use mech_core::{LegacyValue, MResult};
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};

pub fn run(source: &str) -> MResult<LegacyValue> {
    let product = intrinsic_catalog::compiler()?.compile_source(source)?;
    let mut runtime = RuntimeBuilder::new()
        .function_catalog(intrinsic_catalog::source_catalog())
        .build()?;
    Ok(runtime
        .load_bytecode_program(product.bytecode(), ResidentDurabilityPolicy::Volatile)?
        .initial_value
        .into_value())
}
