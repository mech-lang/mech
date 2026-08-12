//! Build-time lowering of a complete resident numeric turn into Rust source.
//!
//! This prototype is deliberately all-or-nothing. A generated application
//! uses the ordinary bytecode runtime unless every turn node is representable
//! by the typed numeric IR. That keeps fallback behavior explicit while the
//! runtime/executor boundary for mixed compiled regions is still being
//! designed.

mod codegen;
mod kernel_ir;
mod mlir_codegen;
mod mlir_gpu_codegen;
mod mlir_spirv_codegen;

use mech_core::{CellSlotId, FunctionCatalog, ParsedProgram, ReactiveInstanceId};
use mech_engine::__resident::{ActivationFacts, ResidentActivationError, activate};
use mech_engine::artifact::decode_program_artifact_sections;

/// A complete numeric turn that can be emitted into a generated native app.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeAotProgram {
    pub source: String,
    pub input_len: usize,
    pub state_len: usize,
    pub instruction_count: usize,
}

/// A complete numeric turn expressed as standalone textual MLIR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMlirProgram {
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
    let kernel = lower_kernel(bytecode, catalog)?;
    let source = codegen::emit_rust(&kernel)?;
    Ok(NativeAotProgram {
        source,
        input_len: kernel.input_len,
        state_len: kernel.state_len,
        instruction_count: kernel.instructions.len(),
    })
}

/// Lower the same activated numeric kernel used by the Rust AOT backend into
/// an MLIR module with C-callable initialize, one-turn, and resident-loop APIs.
pub fn lower_bytecode_mlir(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeMlirProgram, String> {
    let kernel = lower_kernel(bytecode, catalog)?;
    let source = mlir_codegen::emit_mlir(&kernel)?;
    Ok(NativeMlirProgram {
        source,
        input_len: kernel.input_len,
        state_len: kernel.state_len,
        instruction_count: kernel.instructions.len(),
    })
}

/// Lower a materialized lane-wise numeric kernel into MLIR's GPU dialect.
/// The emitted host wrapper launches one GPU thread per lane; target-specific
/// NVVM/PTX lowering remains an explicit subsequent MLIR toolchain step.
pub fn lower_bytecode_mlir_gpu(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeMlirProgram, String> {
    let kernel = lower_kernel(bytecode, catalog)?;
    let source = mlir_gpu_codegen::emit_mlir(&kernel)?;
    Ok(NativeMlirProgram {
        source,
        input_len: kernel.input_len,
        state_len: kernel.state_len,
        instruction_count: kernel.instructions.len(),
    })
}

/// Lower a lane-wise numeric kernel for accelerators without native f64.
/// This is an explicit relaxed-precision profile: Mech f64 values and
/// constants become f32 in the emitted GPU module.
pub fn lower_bytecode_mlir_gpu_f32(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeMlirProgram, String> {
    let kernel = lower_kernel(bytecode, catalog)?;
    let source = mlir_gpu_codegen::emit_mlir_f32(&kernel)?;
    Ok(NativeMlirProgram {
        source,
        input_len: kernel.input_len,
        state_len: kernel.state_len,
        instruction_count: kernel.instructions.len(),
    })
}

/// Lower a lane-wise numeric kernel directly into f32 SPIR-V dialect MLIR.
/// This is the portable GPU interchange used by the Apple Metal prototype.
pub fn lower_bytecode_mlir_spirv_f32(
    bytecode: &[u8],
    catalog: &FunctionCatalog,
) -> Result<NativeMlirProgram, String> {
    let kernel = lower_kernel(bytecode, catalog)?;
    let source = mlir_spirv_codegen::emit_spirv_mlir_f32(&kernel)?;
    Ok(NativeMlirProgram {
        source,
        input_len: kernel.input_len,
        state_len: kernel.state_len,
        instruction_count: kernel.instructions.len(),
    })
}

fn lower_kernel(bytecode: &[u8], catalog: &FunctionCatalog) -> Result<kernel_ir::KernelIr, String> {
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
    .map_err(|error| activation_error(&artifact, error))?;
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
    Ok(kernel)
}

fn activation_error(
    artifact: &mech_engine::artifact::ProgramArtifact,
    error: ResidentActivationError,
) -> String {
    let node = match &error {
        ResidentActivationError::MissingResidentFactory { node }
        | ResidentActivationError::KernelBind { node, .. }
        | ResidentActivationError::ActivationKernel { node }
        | ResidentActivationError::UnsupportedConstruction { node }
        | ResidentActivationError::UnsupportedChangeDetection { node }
        | ResidentActivationError::InvalidAlias { node }
        | ResidentActivationError::InvalidNodeOutput { node }
        | ResidentActivationError::InvalidDependency { node } => Some(*node),
        _ => None,
    };
    let operation = node.and_then(|node| {
        artifact
            .nodes()
            .iter()
            .find(|declaration| declaration.node == node)
            .map(|declaration| {
                format!(
                    "{}::{}",
                    declaration.operation.module_path.join("/"),
                    declaration.operation.operation_name,
                )
            })
    });
    match operation {
        Some(operation) => {
            format!("resident activation failed for `{operation}`: {error:?}")
        }
        None => format!("resident activation failed: {error:?}"),
    }
}
