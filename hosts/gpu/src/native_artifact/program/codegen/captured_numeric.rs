//! Typed immutable reads inside an already funded scalar-state Tail.
use super::{hooks::Hooks, numeric_words, tail};
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::codegen::options::OptimizationOptions;
use crate::native_artifact::compiler::{backend, set_origin};
use crate::native_artifact::program::prepare::{
    captured_numeric::{Operation, Proof},
    tail::Layout,
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeIterationAction as Action;

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    pub header: usize,
    pub captures: Vec<(u32, u8)>,
    pub typed_loads: Vec<(usize, u8)>,
    pub length_sites: Vec<usize>,
    pub successful_backedge_helpers: usize,
}

pub(super) fn emit(
    options: OptimizationOptions,
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    locals: Value,
    words: &Words,
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    proof: &Proof,
    action: &Operation,
    site: usize,
    hooks: &Hooks,
    error: Block,
) -> Result<Option<Block>, NativeCompileError> {
    let shadow = closed.regions[id]
        .shadow
        .ok_or_else(|| backend("numeric capture lost scratch home"))?;
    let (capture, output) = match *action {
        Operation::Read {
            capture, output, ..
        }
        | Operation::Length { capture, output } => (capture, output),
    };
    let view = shadow + capture * 4;
    let length = words.read(b, view + 1);
    let Operation::Read { index, .. } = action else {
        words.write(b, output as usize, length);
        return Ok(None);
    };
    let index = words.read(b, *index as usize);
    let ordinal = b.ins().iadd_imm(index, -1);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, length);
    let good = b.create_block();
    let bad = b.create_block();
    b.set_cold_block(bad);
    b.ins().brif(valid, good, &[], bad, &[]);
    b.switch_to_block(bad);
    set_origin(b, frame, site as u64);
    words.spill_all(b, locals)?;
    tail::failure(options, b, locals, closed, site, hooks, error)?;
    // Reconcile the same account, then let the original selected leaf produce
    // the maintained zero/empty/oversized-index error at its original origin.
    let Action::LibraryCall { operands, .. } = code.blocks()[site].action() else {
        return Err(backend("numeric capture read lost selected call"));
    };
    hooks.library(b, site, operands, locals, |v| code.kind(v).is_some(), error)?;
    let status = b
        .ins()
        .iconst(types::I64, crate::native_artifact::abi::STATUS_ABI as i64);
    b.ins().jump(error, &[status.into()]);
    b.switch_to_block(good);
    let rows = words.read(b, view + 2);
    let columns = words.read(b, view + 3);
    let width = proof.captures[capture].width;
    let Action::LibraryCall { template, .. } =
        code.blocks()[proof.captures[capture].witness].action()
    else {
        return Err(backend("numeric capture lost exact entry witness"));
    };
    let (address, ty) = numeric_words::linear_address(
        b,
        ordinal,
        rows,
        columns,
        width,
        template.inputs()[0].body(),
        |b| words.read(b, view),
    )?;
    let value = numeric_words::load_unsigned_word(b, ty, MemFlags::new(), address, 0);
    words.write(b, output as usize, value);
    Ok(Some(bad))
}
