//! Immutable caller storage for owned outlined loops. Only this private call
//! receives its pointer; helpers and source Values never receive or mutate it.
use super::*;
use crate::native_artifact::compilation::{CompilationControls, OptimizationPass};
use crate::native_artifact::program_packed_map::{HASH_FIRST, HASH_SECOND};
use cranelift_codegen::ir::{StackSlotData, StackSlotKind};

pub(super) const BYTES: u32 = 24;

pub(super) fn enabled(controls: &CompilationControls) -> bool {
    controls.enabled(OptimizationPass::LoopConstants)
}

pub(super) fn endpoint_words(
    controls: &CompilationControls,
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: &BTreeSet<usize>,
) -> Option<[usize; 2]> {
    if !controls.enabled(OptimizationPass::TagThreading)
        || !controls.enabled(OptimizationPass::LoopBounds)
    {
        return None;
    }
    let graph = tag_threading::derive(code, closed, id, owner, aggregates)?;
    let proof = loop_bounds::derive(code, closed, id, owner, aggregates, &graph, slots)?;
    Some([proof.width_word, proof.limit_word])
}
fn source_word(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    code: CodePlan<'_>,
    slot: usize,
) -> Result<Value, NativeCompileError> {
    // Constants need not have materialized local slots. Match Words::new's
    // original initialization rather than reading an uninitialized literal.
    for index in 0..code.constant_count() {
        let (value, word) = code.constant(index)?;
        if value.get() as usize == slot
            && let Some(word) = word
        {
            return Ok(b.ins().iconst(types::I64, word as i64));
        }
    }
    Ok(b.ins()
        .load(types::I64, MemFlags::new(), locals, offset(slot)?))
}
pub(super) fn bind(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    code: CodePlan<'_>,
    endpoint: Option<[usize; 2]>,
) -> Result<Value, NativeCompileError> {
    // Actual stack storage is included in the root's compiled stack census.
    // All three words are initialized before the synchronous private call;
    // no subsequent write aliases them during that callee's lifetime.
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, BYTES, 3));
    let pointer = b.ins().stack_addr(types::I64, slot, 0);
    let first = b.ins().iconst(types::I64, HASH_FIRST as i64);
    let second = b.ins().iconst(types::I64, HASH_SECOND as i64);
    let end = if let Some([width, limit]) = endpoint {
        let width = source_word(b, locals, code, width)?;
        let limit = source_word(b, locals, code, limit)?;
        // Wrapping preparation is unobservable on declined bounds. Only the
        // original guarded loop may use this endpoint after its selector.
        let end = b.ins().isub(limit, width);
        b.ins().iadd_imm(end, 1)
    } else {
        b.ins().iconst(types::I64, 0)
    };
    for (index, value) in [first, second, end].into_iter().enumerate() {
        b.ins()
            .store(MemFlags::new(), value, pointer, (index * 8) as i32);
    }

    Ok(pointer)
}
