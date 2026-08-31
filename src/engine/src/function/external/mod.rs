mod host_call;
mod resource_read;
mod resource_write;

pub use host_call::*;
pub use resource_read::*;
pub use resource_write::*;

#[cfg(feature = "semantic-compiler")]
use mech_core::{
    BytecodeCompilerContext, MResult, Register, ValueCell,
    compile_runtime_produced_value_cell_register, compile_value_cell_register,
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
