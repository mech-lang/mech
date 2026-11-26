//! Compose the maintained checked scalar map emitter inside one phase loan.
//! Only a fixed stack frame is local; all pointers belong to admitted phase
//! roots. Failure discards the fold and resumes before the original range.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::{numeric_phase::Fold, readonly_numeric::Proof};
use crate::native_artifact::program::runtime::{
    NativeNumericMapFrame as Frame, NativeNumericMapMatrixView as Matrix,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{
    Block, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeProgramArtifactPlan as Plan;
use std::{
    collections::BTreeMap,
    mem::{offset_of, size_of},
};

fn require(b: &mut FunctionBuilder<'_>, good: Value, miss: Block) {
    let next = b.create_block();
    b.ins().brif(good, next, &[], miss, &[]);
    b.switch_to_block(next);
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    proof: &Proof,
    fold: &Fold,
    plan: &Plan,
    miss: Block,
) -> Result<(), NativeCompileError> {
    let source = &fold.source;
    let region = &source.region;
    let first = words.read(b, source.first as usize);
    let last = words.read(b, source.last as usize);
    let (mut count, overflow) = b.ins().usub_overflow(last, first);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, miss);
    if source.inclusive {
        let one = b.ins().iconst(types::I64, 1);
        let (next, overflow) = b.ins().uadd_overflow(count, one);
        count = next;
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        require(b, good, miss);
    }
    let good = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
    require(b, good, miss);
    // The maximum bounds an optimization attempt, never the source range.
    let good = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, count, source.maximum as i64);
    require(b, good, miss);
    if source.constant_count {
        let good = b.ins().icmp_imm(IntCC::Equal, count, source.maximum as i64);
        require(b, good, miss);
    }
    let slot = b.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        size_of::<Frame>() as u32,
        3,
    ));
    let frame = b.ins().stack_addr(types::I64, slot, 0);
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    for at in [
        offset_of!(Frame, input),
        offset_of!(Frame, input_bytes),
        offset_of!(Frame, output),
        offset_of!(Frame, capacity_bytes),
        offset_of!(Frame, output_count),
        offset_of!(Frame, fold),
        offset_of!(Frame, next),
    ] {
        b.ins().store(MemFlags::new(), zero, frame, at as i32);
    }
    for at in [offset_of!(Frame, count), offset_of!(Frame, source_count)] {
        b.ins().store(MemFlags::new(), count, frame, at as i32);
    }
    for at in [offset_of!(Frame, epoch), offset_of!(Frame, issued)] {
        b.ins().store(MemFlags::new(), one, frame, at as i32);
    }
    b.ins().store(
        MemFlags::new(),
        first,
        frame,
        offset_of!(Frame, range_start) as i32,
    );
    for (at, width) in [
        (offset_of!(Frame, input_width), region.input_width),
        (offset_of!(Frame, output_width), region.output_width),
    ] {
        let value = b.ins().iconst(types::I64, i64::from(width));
        b.ins().store(MemFlags::new(), value, frame, at as i32);
    }
    if fold.arrays.len() != region.matrices.len() {
        return Err(backend("phase fold lost its exact matrix inventory"));
    }
    for (index, &array) in fold.arrays.iter().enumerate() {
        for (lane, field) in [
            offset_of!(Matrix, input),
            offset_of!(Matrix, count),
            offset_of!(Matrix, rows),
            offset_of!(Matrix, columns),
        ]
        .into_iter()
        .enumerate()
        {
            let value = words.read(b, shadow + proof.view(array) + lane);
            let at = offset_of!(Frame, matrices) + index * size_of::<Matrix>() + field;
            b.ins().store(MemFlags::new(), value, frame, at as i32);
        }
    }
    let captures = region
        .captures
        .iter()
        .map(|&v| (v, words.read(b, v as usize)))
        .collect::<BTreeMap<_, _>>();
    let complete = b.create_block();
    let arithmetic = b.create_block();
    b.append_block_param(arithmetic, types::I64);
    super::numeric_map::emit(
        b, frame, region, plan, &captures, complete, arithmetic, miss,
    )?;
    b.switch_to_block(arithmetic);
    b.ins().jump(miss, &[]);
    b.switch_to_block(complete);
    let value = b.ins().load(
        types::I64,
        MemFlags::new(),
        frame,
        offset_of!(Frame, fold) as i32,
    );
    words.write(
        b,
        region
            .reduction
            .as_ref()
            .ok_or_else(|| backend("phase fold lost scalar extraction"))?
            .scalar as usize,
        value,
    );
    Ok(())
}
