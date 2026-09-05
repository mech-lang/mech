mod host_call;
mod resource_read;
mod resource_write;

pub use host_call::*;
pub use resource_read::*;
pub use resource_write::*;

use mech_core::{MResult, Value, ValueCell};

/// Installs one host/resource result only after its semantic descriptor and
/// current storage satisfy the declaration already owned by the stable cell.
pub(super) fn install_external_value(output: &ValueCell, value: Value) -> MResult<()> {
    let expected = output.resolved_descriptor()?;
    let candidate = ValueCell::from_snapshot(value.clone())?;
    candidate.validate_descriptor(&expected)?;
    output.replace(&value)
}

#[cfg(feature = "semantic-compiler")]
use mech_core::{
    BytecodeCompilerContext, Register, compile_runtime_produced_value_cell_register,
    compile_value_cell_register,
};

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_external_output(
    output: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_value_cell_register(output, context)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_external_cell(
    value: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_value_cell_register(value, context)
}

#[cfg(feature = "semantic-compiler")]
pub(super) fn compile_runtime_produced_external_output(
    output: &ValueCell,
    context: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    compile_runtime_produced_value_cell_register(output, context)
}
