//! Direct structural scalar reads from the session's existing typed word bank.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::CodegenReport;
use crate::native_artifact::compiler::{backend, guard, offset};
use crate::native_artifact::program::prepare::{ProgramMetadata, frame_scalars};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeScalarKind;

pub(in crate::native_artifact) fn emit(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    metadata: &ProgramMetadata,
    site: usize,
    error: Block,
    report: &mut CodegenReport,
) -> Result<bool, NativeCompileError> {
    let Some((word_at, kind, output)) =
        frame_scalars::read(&metadata.plan, &metadata.buffer_frames, site)
    else {
        return Ok(false);
    };
    let pointer = metadata
        .frame_scalar_pointer()
        .ok_or_else(|| backend("frame scalar read has no admitted loan lane"))?;
    // Helpers can update this bank between reads. Ordinary memory flags keep
    // those writes observable; this is not invariant or readonly memory.
    let bank = b
        .ins()
        .load(types::I64, MemFlags::new(), locals, offset(pointer)?);
    let valid = b
        .ins()
        .load(types::I64, MemFlags::new(), bank, offset(word_at + 1)?);
    let valid = b.ins().icmp_imm(IntCC::Equal, valid, 1);
    guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
    let word = b
        .ins()
        .load(types::I64, MemFlags::new(), bank, offset(word_at)?);
    let maximum = if kind == NativeScalarKind::Bool {
        Some(1)
    } else {
        kind.unsigned_maximum().filter(|&max| max != u64::MAX)
    };
    if let Some(maximum) = maximum {
        let valid = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, word, maximum as i64);
        guard(b, valid, error, crate::native_artifact::abi::STATUS_ABI);
    }
    b.ins().store(
        MemFlags::new(),
        word,
        locals,
        offset(output.get() as usize)?,
    );
    report.frame_scalar_reads.push((site, word_at));
    Ok(true)
}
