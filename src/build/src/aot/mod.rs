//! Build-time lowering of a complete resident numeric turn into Rust source.
//!
//! This prototype is deliberately all-or-nothing. A generated application
//! uses the ordinary bytecode runtime unless every turn node is representable
//! by the typed numeric IR. That keeps fallback behavior explicit while the
//! runtime/executor boundary for mixed compiled regions is still being
//! designed.

mod codegen;
mod kernel_ir;

use mech_core::{CellSlotId, FunctionCatalog, ParsedProgram, ReactiveInstanceId};
use mech_engine::__resident::{ActivationFacts, activate};
use mech_engine::artifact::decode_program_artifact_sections;

/// A complete numeric turn that can be emitted into a generated native app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeAotProgram {
    pub source: String,
    pub input_len: usize,
    pub state_len: usize,
    pub instruction_count: usize,
}

/// Try to lower the entire bytecode turn. Rejection is data, not a build
/// failure: callers retain the normal interpreter/runtime as the fallback.
pub fn lower_bytecode(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeAotProgram, String> {
    let parsed = ParsedProgram::from_bytes(bytecode)
        .map_err(|error| format!("bytecode decode failed: {}", error.display_message()))?;
    let artifact = decode_program_artifact_sections(&parsed.artifact)
        .map_err(|error| format!("semantic artifact decode failed: {error:?}"))?;
    let instance = activate(
        ReactiveInstanceId::new(0, 0),
        &artifact,
        catalog,
        &ActivationFacts::default(),
    )
    .map_err(|error| format!("resident activation failed: {error:?}"))?;
    let input_slots = instance
        .plan
        .inputs
        .iter()
        .map(|input| CellSlotId::new(input.slot.get()))
        .collect::<Vec<_>>();
    let kernel = kernel_ir::KernelIr::lower(&artifact, &instance, &input_slots)
        .map_err(|error| error.to_string())?;
    if kernel.instructions.is_empty() {
        return Err("program has no numeric turn instructions".to_owned());
    }
    let source = codegen::emit_rust(&kernel)?;
    Ok(NativeAotProgram {
        source,
        input_len: kernel.input_len,
        state_len: kernel.state_len,
        instruction_count: kernel.instructions.len(),
    })
}
