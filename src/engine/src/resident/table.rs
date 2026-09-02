use mech_core::{FunctionCatalogBuilder, MResult};

/// Table joins remain source-runtime and bytecode-v1 operations in R1, but
/// their current implementation constructs several independently owned
/// recursive table representations before publication. Until that legacy
/// materializer is replaced by one proof-carrying admitted plan, the Resident
/// target must report the family unavailable during preflight instead of
/// accepting an execution path whose peak recursive liveness is not
/// structurally bounded.
pub(crate) fn install(_builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    Ok(())
}
