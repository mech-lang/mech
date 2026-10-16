//! Experimental typed collection AOT backend for the v0.4 canonical syntax.
//!
//! Source expressions become native calls; packed-window traversal is emitted
//! by Cranelift. Byte storage, IO, formatting and hashbrown are library services.
//! This is a separate admitted subset, not the general resident artifact path.
mod bytes;
mod compiler;
mod counts;
mod numeric;
mod runtime;
pub use compiler::{Constant, JitProgram, Program, compile, compile_parsed};
pub use runtime::{Context, NativeEntry};

/// Emit a scheduled, fused source program and its checked constant table.
/// This is shared by native application generation and build-time consumers.
pub fn emit_program(
    source: &str,
    directory: &std::path::Path,
    batch: usize,
    pack8: bool,
    simd: bool,
    schedule: bool,
    fuse: bool,
) -> Result<Program, String> {
    emit_program_with_array_simd(source, directory, batch, pack8, simd, schedule, fuse, simd)
}
/// Array and symbol-packing SIMD are independent compiler policies.
pub fn emit_program_with_array_simd(
    source: &str,
    directory: &std::path::Path,
    batch: usize,
    pack8: bool,
    simd: bool,
    schedule: bool,
    fuse: bool,
    array_simd: bool,
) -> Result<Program, String> {
    let mut program = compile(source, batch)?;
    if schedule {
        program = program.scheduled()?;
    }
    if fuse {
        program = program.fuse_record_encoding();
    }
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join("mech-sequence.o"),
        program.object_with_array_simd(batch, pack8, simd, array_simd)?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join("constants.json"),
        serde_json::to_vec(&program.constants).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(program)
}
