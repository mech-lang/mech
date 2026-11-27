//! Source-ordered complete matrix expressions over one current checked loan.
//! Existing kernels retain their FP term order and independent SIMD outputs.
use super::hooks::Hooks;
use crate::native_artifact::program::{
    prepare::{
        ProgramMetadata,
        numeric_regions::{Source, Step},
    },
    protocol::{GlobalHelper, abi::NumericBinaryFrame},
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, guard},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeMatchValue;

pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    hooks: &Hooks,
    m: &ProgramMetadata,
    site: usize,
    locals: Value,
    is_scalar: impl Fn(NativeMatchValue) -> bool,
    blocks: &[Option<Block>],
    error: Block,
) -> Result<(), NativeCompileError> {
    let Some((_, candidate)) = m.numeric_regions.at_entry(site) else {
        return Ok(());
    };
    let Some((steps, exit)) = candidate.body(&m.plan) else {
        return Ok(());
    };
    if steps
        .iter()
        .any(|s| m.map_admission.group(s.site() as usize).is_some())
    {
        return Ok(());
    }
    let target = blocks
        .get(exit as usize)
        .copied()
        .flatten()
        .ok_or_else(|| backend("matrix region exit is unreachable"))?;
    // Managed captures are read from their actual current arena generation.
    // Only scalar values need transport from generated word homes.
    let scalar_inputs = candidate
        .external
        .iter()
        .copied()
        .filter(|v| is_scalar(*v))
        .collect::<Vec<_>>();
    hooks.library_operation_mode(
        b,
        site,
        GlobalHelper::NumericBinaryPrepare.raw(),
        &scalar_inputs,
        locals,
        is_scalar,
        1,
        error,
    )?;
    let frames = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(frames, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    for step in steps {
        if let Step::Operator {
            kind, inputs, bank, ..
        } = step
        {
            let offset = bank
                .checked_mul(core::mem::size_of::<NumericBinaryFrame>())
                .ok_or_else(|| backend("matrix region frame offset overflows"))?;
            let frame = b.ins().iadd_imm(frames, offset as i64);
            let mut bank_inputs = [false; 2];
            for (at, input) in inputs.iter().enumerate() {
                bank_inputs[at] = matches!(input, Source::Bank(_));
            }
            super::numeric_binary::emit_kernel(b, *kind, frame, bank_inputs, error);
        }
    }
    let count = b
        .ins()
        .iconst(types::I64, candidate.storage_slots.len() as i64);
    let mode = b.ins().iconst(types::I64, 1);
    hooks.call(
        b,
        site,
        GlobalHelper::NumericBinaryFinish,
        [frames, count, mode],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let valid = b.ins().icmp_imm(IntCC::Equal, ready, 1);
    guard(
        b,
        valid,
        error,
        crate::native_artifact::abi::STATUS_INTEGRITY,
    );
    b.ins().jump(target, &[]);
    b.switch_to_block(ordinary);
    Ok(())
}
