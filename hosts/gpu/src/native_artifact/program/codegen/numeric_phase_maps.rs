//! Simultaneous whole-map replacement through the maintained native map DAG.
//! Proved total same-cell/disjoint maps write an already exclusive bank.
//! Other maps retain the complete scratch result before a non-failing copy.
//! Current view/ownership publication still follows complete source success.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::{
    numeric_phase::matrix_maps::{MatrixMap, Replacement},
    readonly_numeric::Proof,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{
    Block, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeProgramArtifactPlan as Plan;
use std::collections::BTreeMap;
#[path = "numeric_phase_direct_maps.rs"]
mod direct;
fn require(b: &mut FunctionBuilder<'_>, good: cranelift_codegen::ir::Value, miss: Block) {
    let next = b.create_block();
    b.ins().brif(good, next, &[], miss, &[]);
    b.switch_to_block(next);
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    proof: &Proof,
    replacement: &Replacement,
    plan: &Plan,
    miss: Block,
    direct_enabled: bool,
) -> Result<Option<Value>, NativeCompileError> {
    let source = &replacement.source;
    let target = replacement.target;
    let width = proof.arrays[target].width;
    if replacement.arrays.len() != source.region.matrices.len()
        || width != source.region.output_width
    {
        return Err(backend("phase map lost exact views or output width"));
    }
    let view = shadow + proof.view(target);
    let owned = proof.map_bank(target);
    if owned.is_none() {
        let private = words.read(b, shadow + proof.private_flag(target));
        let good = b.ins().icmp_imm(IntCC::NotEqual, private, 0);
        require(b, good, miss);
    }
    for (lane, expected) in [
        (1, source.elements),
        (2, source.extents[0]),
        (3, source.extents[1]),
    ] {
        let actual = words.read(b, view + lane);
        let good = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
        require(b, good, miss);
    }
    if let Some(bank) = owned {
        let pointer = words.read(b, shadow + bank);
        let good = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
        require(b, good, miss);
    }
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
    let good = b.ins().icmp_imm(IntCC::Equal, count, source.count as i64);
    require(b, good, miss);
    // The equality guard above proves this exact fixed source cardinality.
    let count = b.ins().iconst(types::I64, source.count as i64);
    let matrices = replacement
        .arrays
        .iter()
        .map(|&array| std::array::from_fn(|lane| words.read(b, shadow + proof.view(array) + lane)))
        .collect::<Vec<[_; 4]>>();
    let captures = source
        .region
        .captures
        .iter()
        .map(|&v| (v, words.read(b, v as usize)))
        .collect::<BTreeMap<_, _>>();
    let base = words.read(b, owned.map_or(view, |bank| shadow + bank));
    let direct = emit_map(
        b,
        source,
        plan,
        base,
        first,
        count,
        &matrices,
        &captures,
        direct_enabled,
        miss,
    )?;
    if owned.is_some() {
        words.write(b, view, base);
        let one = b.ins().iconst(types::I64, 1);
        words.write(b, shadow + proof.private_flag(target), one);
    }
    Ok(direct)
}

/// The caller already owns the exact bounded destination. Entry selection
/// permits in-place writes only when every possible source body is total and
/// every read is either disjoint or reads its own current output cell. No
/// fallible source operation or helper can then observe a partial result.
fn emit_map(
    b: &mut FunctionBuilder<'_>,
    source: &MatrixMap,
    plan: &Plan,
    base: Value,
    first: Value,
    count: Value,
    matrices: &[[Value; 4]],
    captures: &BTreeMap<u32, Value>,
    direct_enabled: bool,
    miss: Block,
) -> Result<Option<Value>, NativeCompileError> {
    let width = source.region.output_width;
    let bytes = source
        .elements
        .checked_mul(u64::from(width))
        .and_then(|n| u32::try_from(n).ok())
        .filter(|&n| n > 0 && n <= 65536)
        .ok_or_else(|| backend("phase map lost finite scratch geometry"))?;
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, bytes, 3));
    let scratch = b.ins().stack_addr(types::I64, slot, 0);
    let capacity = b.ins().iconst(types::I64, i64::from(bytes));
    let direct = direct_enabled
        .then(|| direct::guard(b, source, plan, base, first, matrices, captures))
        .flatten();
    let destination = direct.map_or(scratch, |allowed| b.ins().select(allowed, base, scratch));
    super::numeric_phase_reductions::emit_region(
        b,
        &source.region,
        plan,
        first,
        count,
        matrices,
        captures,
        Some((destination, capacity)),
        miss,
    )?;
    let copy = b.create_block();
    let done = b.create_block();
    b.append_block_param(copy, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    if let Some(allowed) = direct {
        b.ins().brif(allowed, done, &[], copy, &[zero.into()]);
    } else {
        b.ins().jump(copy, &[zero.into()]);
    }
    b.switch_to_block(copy);
    let offset = b.block_params(copy)[0];
    let input = b.ins().iadd(scratch, offset);
    let output = b.ins().iadd(base, offset);
    let ty = match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => return Err(backend("phase map lost its typed scalar width")),
    };
    let cell = b.ins().load(ty, MemFlags::new(), input, 0);
    b.ins().store(MemFlags::new(), cell, output, 0);
    let next = b.ins().iadd_imm(offset, i64::from(width));
    let more = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, next, i64::from(bytes));
    b.ins().brif(more, copy, &[next.into()], done, &[]);
    b.switch_to_block(done);
    Ok(direct)
}

#[cfg(all(test, feature = "jit"))]
#[path = "numeric_phase_direct_maps_tests.rs"]
mod direct_tests;
