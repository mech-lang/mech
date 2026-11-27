//! Compose the maintained checked scalar map emitter inside one phase loan.
//! Native values compose directly in SSA; all pointers belong to admitted
//! phase roots. Failure discards the fold before the original range resumes.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::{numeric_phase::Fold, readonly_numeric::Proof};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{Block, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeProgramArtifactPlan as Plan;
use std::collections::BTreeMap;

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
    let first = words.read(b, source.first as usize);
    let last = words.read(b, source.last as usize);
    if fold.arrays.len() != source.region.matrices.len() {
        return Err(backend("phase fold lost its exact matrix inventory"));
    }
    let matrices = fold
        .arrays
        .iter()
        .map(|&array| std::array::from_fn(|lane| words.read(b, shadow + proof.view(array) + lane)))
        .collect::<Vec<[Value; 4]>>();
    let captures = source
        .region
        .captures
        .iter()
        .map(|&v| (v, words.read(b, v as usize)))
        .collect::<BTreeMap<_, _>>();
    let value = emit_from_views(b, source, plan, first, last, &matrices, &captures, miss)?;
    words.write(
        b,
        source
            .region
            .reduction
            .as_ref()
            .ok_or_else(|| backend("phase fold lost extraction"))?
            .scalar as usize,
        value,
    );
    Ok(())
}
/// Shared by a numeric phase and an enclosing native scalar map. Current
/// admitted views and SSA captures are supplied by the exact parent loan.
/// This emits only the existing checked map DAG and ordered F64 fold.
pub(super) fn emit_from_views(
    b: &mut FunctionBuilder<'_>,
    source: &crate::native_artifact::program::prepare::numeric_phase::Reduction,
    plan: &Plan,
    first: Value,
    last: Value,
    matrices: &[[Value; 4]],
    captures: &BTreeMap<u32, Value>,
    miss: Block,
) -> Result<Value, NativeCompileError> {
    let region = &source.region;
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
    let count = if source.constant_count {
        // Retain the exact guarded source trip count as a native constant.
        b.ins().iconst(types::I64, source.maximum as i64)
    } else {
        count
    };
    emit_region(
        b, region, plan, first, count, matrices, captures, None, miss,
    )
}

/// Shared checked map DAG emission from current admitted views. The optional
/// destination is caller-owned scratch, never an additional mutable root loan.
pub(super) fn emit_region(
    b: &mut FunctionBuilder<'_>,
    region: &crate::native_artifact::program::prepare::numeric_maps::Region,
    plan: &Plan,
    first: Value,
    count: Value,
    matrices: &[[Value; 4]],
    captures: &BTreeMap<u32, Value>,
    output: Option<(Value, Value)>,
    miss: Block,
) -> Result<Value, NativeCompileError> {
    if !region.range || matrices.len() != region.matrices.len() {
        return Err(backend(
            "native map composition lost its range or exact views",
        ));
    }
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let elements = if let Some(rectangle) = &region.rectangle {
        let rows = b.ins().iconst(types::I64, rectangle.rows as i64);
        let (elements, overflow) = b.ins().umul_overflow(count, rows);
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        require(b, good, miss);
        elements
    } else {
        count
    };
    let (output, capacity_bytes, output_count) = output
        .map_or((zero, zero, zero), |(pointer, capacity)| {
            (pointer, capacity, elements)
        });
    let input_width = b.ins().iconst(types::I64, i64::from(region.input_width));
    let output_width = b.ins().iconst(types::I64, i64::from(region.output_width));
    let inputs = super::numeric_map::Values {
        count: elements,
        source_count: count,
        input: zero,
        input_bytes: zero,
        output,
        capacity_bytes,
        output_count,
        input_width,
        output_width,
        epoch: one,
        issued: one,
        next: zero,
        fold: zero,
        range_start: first,
        matrices: matrices.to_vec(),
    };
    let complete = b.create_block();
    b.append_block_param(complete, types::I64);
    let arithmetic = b.create_block();
    b.append_block_param(arithmetic, types::I64);
    super::numeric_map::emit_values(
        b, &inputs, region, plan, captures, complete, arithmetic, miss,
    )?;
    b.switch_to_block(arithmetic);
    b.ins().jump(miss, &[]);
    b.switch_to_block(complete);
    Ok(b.block_params(complete)[0])
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "numeric_map_ssa_tests.rs"]
mod tests;
