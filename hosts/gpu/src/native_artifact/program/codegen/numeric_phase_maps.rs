//! Simultaneous whole-map replacement through the maintained native map DAG.
//! Old current views remain read-only until every output cell succeeds. Only an
//! already exclusive phase bank receives the final non-failing typed copy.
//! Source-proved identity masks keep the entire checked scratch boundary.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::prepare::{
    numeric_phase::matrix_maps::Replacement, readonly_numeric::Proof,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{
    Block, InstBuilder, MemFlags, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeProgramArtifactPlan as Plan;
use std::collections::BTreeMap;
/// Descriptive emission information; authority remains in the source mask.
pub(super) struct MapEmission {
    pub mask: Option<(u32, u8)>,
    pub source_selector: u32,
    pub guarded_initialization_bytes: Option<u32>,
    pub compound: Option<(usize, Vec<(u8, u32, u8)>)>,
}
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
    masked_maps: bool,
    selective_initialization: bool,
    compound_domains: bool,
    typed_folds: bool,
    miss: Block,
) -> Result<MapEmission, NativeCompileError> {
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
    let bytes = source
        .elements
        .checked_mul(u64::from(width))
        .and_then(|n| u32::try_from(n).ok())
        .filter(|&n| n > 0 && n <= 65536)
        .ok_or_else(|| backend("phase map lost finite scratch geometry"))?;
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, bytes, 3));
    let scratch = b.ins().stack_addr(types::I64, slot, 0);
    let capacity = b.ins().iconst(types::I64, i64::from(bytes));
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
    let mask = if masked_maps {
        crate::native_artifact::program::prepare::numeric_phase::masks::Mask::derive(
            plan,
            proof,
            replacement,
        )
    } else {
        None
    };
    let domain = if masked_maps && compound_domains {
        crate::native_artifact::program::prepare::numeric_phase::masks::domains::Domain::derive(
            plan,
            proof,
            replacement,
        )
        .filter(|domain| domain.strictly_refines(mask.as_ref()))
    } else {
        None
    };
    let identity = domain
        .as_ref()
        .map(|domain| domain.base())
        .or_else(|| mask.as_ref().map(|mask| mask.base()));
    let mut whole_output_initialized = None;
    let old = if let Some(identity) = identity {
        // The source proof uses literal one-based coordinates. Check the
        // actual current first word too, not only the range cardinality.
        let one = b.ins().icmp_imm(IntCC::Equal, first, 1);
        require(b, one, miss);
        let base_view = shadow + proof.view(identity);
        for (lane, expected) in [
            (1, source.elements),
            (2, source.extents[0]),
            (3, source.extents[1]),
        ] {
            let actual = words.read(b, base_view + lane);
            let good = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
            require(b, good, miss);
        }
        let old = words.read(b, base_view);
        span(b, old, bytes, miss);
        let size = b.ins().iconst(types::I64, i64::from(bytes));
        let old_end = b.ins().iadd(old, size);
        let scratch_end = b.ins().iadd(scratch, size);
        let before = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, old_end, scratch);
        let after = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, scratch_end, old);
        let disjoint = b.ins().bor(before, after);
        require(b, disjoint, miss);
        if selective_initialization {
            // The body reads only the captured immutable source views. When
            // selected publication targets the identity source itself, no
            // scratch complement will be read. Keep the complete capacity and
            // bounds/disjointness guards, but omit this dead copy pass.
            let destination = words.read(b, owned.map_or(view, |bank| shadow + bank));
            let different = b.ins().icmp(IntCC::NotEqual, destination, old);
            let initialize = b.create_block();
            let ready = b.create_block();
            b.ins().brif(different, initialize, &[], ready, &[]);
            b.switch_to_block(initialize);
            initialize_scratch(b, old, scratch, bytes, width)?;
            b.ins().jump(ready, &[]);
            b.switch_to_block(ready);
            whole_output_initialized = Some(different);
        } else {
            initialize_scratch(b, old, scratch, bytes, width)?;
        }
        Some(old)
    } else {
        None
    };
    let selection = if let Some(domain) = &domain {
        Some(super::numeric_map::selection::Selection::compound(
            b,
            domain,
            mask.as_ref(),
            &captures,
        )?)
    } else {
        mask.as_ref()
            .map(|mask| super::numeric_map::selection::Selection::new(b, mask, &captures))
            .transpose()?
    };
    super::numeric_phase_reductions::emit_region_with_selection(
        b,
        &source.region,
        plan,
        first,
        count,
        &matrices,
        &captures,
        typed_folds,
        Some((scratch, capacity)),
        selection.as_ref(),
        whole_output_initialized,
        miss,
    )?;
    // The complete checked map succeeded. No fallible source operation or
    // helper occurs after the first destination store, so reverse/overlapping
    // source reads observe the entire old vector and failures publish nothing.
    let base = words.read(b, owned.map_or(view, |bank| shadow + bank));
    if let (Some(selection), Some(old)) = (&selection, old) {
        span(b, base, bytes, miss);
        let partial = b.create_block();
        let full = b.create_block();
        let published = b.create_block();
        let current = b.ins().icmp(IntCC::Equal, base, old);
        b.ins().brif(current, partial, &[], full, &[]);
        b.switch_to_block(partial);
        publish_selected(b, selection, scratch, base, width)?;
        b.ins().jump(published, &[]);
        b.switch_to_block(full);
        if let Some(initialized) = whole_output_initialized {
            // Recheck after checked evaluation and before any publication.
            // A skipped initialization can never reach complete publication,
            // even if a future body implementation changes its destination.
            require(b, initialized, miss);
        }
        publish_complete(b, scratch, base, width, bytes)?;
        b.ins().jump(published, &[]);
        b.switch_to_block(published);
    } else {
        publish_complete(b, scratch, base, width, bytes)?;
    }
    if owned.is_some() {
        words.write(b, view, base);
        let one = b.ins().iconst(types::I64, 1);
        words.write(b, shadow + proof.private_flag(target), one);
    }
    Ok(MapEmission {
        source_selector: mask
            .as_ref()
            .map_or(source.region.header, |mask| mask.selector()),
        mask: mask.map(|mask| {
            (mask.selector(), match mask.axis() {
            crate::native_artifact::program::prepare::numeric_phase::masks::Axis::Row => 0,
            crate::native_artifact::program::prepare::numeric_phase::masks::Axis::Column => 1,
        })
        }),
        guarded_initialization_bytes: (identity.is_some() && selective_initialization)
            .then_some(bytes),
        compound: domain.map(|domain| (domain.base(), domain.description())),
    })
}

fn span(
    b: &mut FunctionBuilder<'_>,
    pointer: cranelift_codegen::ir::Value,
    bytes: u32,
    miss: Block,
) {
    let nonnull = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
    require(b, nonnull, miss);
    let size = b.ins().iconst(types::I64, i64::from(bytes));
    let (_, overflow) = b.ins().uadd_overflow(pointer, size);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, miss);
}
fn cell_type(width: u8) -> Result<cranelift_codegen::ir::Type, NativeCompileError> {
    Ok(match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => return Err(backend("masked map lost its exact cell width")),
    })
}
fn initialize_scratch(
    b: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    scratch: cranelift_codegen::ir::Value,
    bytes: u32,
    width: u8,
) -> Result<(), NativeCompileError> {
    // Same existing full scratch capacity; integer vectors preserve every bit
    // including NaNs and signed zeros. Both complete spans are disjoint.
    let packed = bytes / 16 * 16;
    if packed != 0 {
        let copy = b.create_block();
        let done = b.create_block();
        b.append_block_param(copy, types::I64);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(copy, &[zero.into()]);
        b.switch_to_block(copy);
        let offset = b.block_params(copy)[0];
        let input = b.ins().iadd(source, offset);
        let output = b.ins().iadd(scratch, offset);
        let bits = b.ins().load(types::I8X16, MemFlags::new(), input, 0);
        b.ins().store(MemFlags::new(), bits, output, 0);
        let next = b.ins().iadd_imm(offset, 16);
        let more = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThan, next, i64::from(packed));
        b.ins().brif(more, copy, &[next.into()], done, &[]);
        b.switch_to_block(done);
    }
    for offset in (packed..bytes).step_by(width as usize) {
        let bits = b
            .ins()
            .load(cell_type(width)?, MemFlags::new(), source, offset as i32);
        b.ins().store(MemFlags::new(), bits, scratch, offset as i32);
    }
    Ok(())
}
fn publish_selected(
    b: &mut FunctionBuilder<'_>,
    selection: &super::numeric_map::selection::Selection,
    scratch: cranelift_codegen::ir::Value,
    base: cranelift_codegen::ir::Value,
    width: u8,
) -> Result<(), NativeCompileError> {
    let head = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(head, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(head, &[zero.into()]);
    b.switch_to_block(head);
    let ordinal = b.block_params(head)[0];
    let more = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, ordinal, selection.count());
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let index = selection.output_index(b, ordinal);
    let offset = b.ins().imul_imm(index, i64::from(width));
    let input = b.ins().iadd(scratch, offset);
    let output = b.ins().iadd(base, offset);
    let bits = b.ins().load(cell_type(width)?, MemFlags::new(), input, 0);
    b.ins().store(MemFlags::new(), bits, output, 0);
    let next = b.ins().iadd_imm(ordinal, 1);
    b.ins().jump(head, &[next.into()]);
    b.switch_to_block(done);
    Ok(())
}
fn publish_complete(
    b: &mut FunctionBuilder<'_>,
    scratch: cranelift_codegen::ir::Value,
    base: cranelift_codegen::ir::Value,
    width: u8,
    bytes: u32,
) -> Result<(), NativeCompileError> {
    let copy = b.create_block();
    let done = b.create_block();
    b.append_block_param(copy, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(copy, &[zero.into()]);
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
    Ok(())
}
