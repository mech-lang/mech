//! Retain the existing confined byte-expression proof and machine word load.
//! A failed whole-view guard executes the original checked source chain.
use super::{arrays::Arrays, layout::Layout, owner_flow::OwnerFlow};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::{
    codegen::numeric_read_packs::{self, Pack},
    prepare::PreparedNativeProgram,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::{BTreeMap, BTreeSet};
pub(super) fn derive(prepared: &PreparedNativeProgram) -> Result<Vec<Pack>, NativeCompileError> {
    let facts = prepared.metadata().plan.facts();
    let mut candidates = BTreeMap::new();
    for region in &prepared.metadata().closed_tails.regions {
        let Some(proof) = &region.numeric else {
            continue;
        };
        for pack in numeric_read_packs::derive(&facts, proof) {
            match candidates.entry(pack.first) {
                std::collections::btree_map::Entry::Vacant(e) => {
                    e.insert(pack);
                }
                std::collections::btree_map::Entry::Occupied(mut e) => {
                    if pack.width > e.get().width {
                        e.insert(pack);
                    }
                }
            }
        }
    }
    let mut covered = BTreeSet::new();
    let mut selected = vec![];
    for (_, pack) in candidates {
        if covered.iter().any(|&site| pack.contains(site)) {
            continue;
        }
        covered.extend(pack.sites());
        selected.push(pack);
    }
    Ok(selected)
}
pub(super) fn emit<M: cranelift_module::Module>(
    module: &mut M,
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &Layout,
    owners: &OwnerFlow,
    storage: Option<&Arrays>,
    observe: bool,
    pack: &Pack,
    next: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    let observer = super::phases::observer(module, b, observe)?;
    let base_slot = layout.slot(pack.source.get());
    let element = layout.ty(pack.source.get()).element()?;
    if element.packed_bytes != Some(1) || layout.ty(pack.output.get()).words != 1 {
        return Err(backend(
            "compact shared word pack requires the current byte view and scalar output",
        ));
    }
    let base = words.read(b, layout.slot(pack.base.get()));
    let length = words.read(b, base_slot + 1);
    let rows = words.read(b, base_slot + 3);
    let columns = words.read(b, base_slot + 4);
    let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
    let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
    let mut valid = b.ins().bor(row, column);
    let end = b.ins().iconst(types::I64, pack.end as i64);
    let enough = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, length, end);
    let available = b.ins().isub(length, end);
    let fits = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, base, available);
    valid = b.ins().band(valid, enough);
    valid = b.ins().band(valid, fits);
    if pack.offset == 0 {
        let nonzero = b.ins().icmp_imm(IntCC::NotEqual, base, 0);
        valid = b.ins().band(valid, nonzero);
    }
    let fast = b.create_block();
    let fallback = b.create_block();
    b.ins().brif(valid, fast, &[], fallback, &[]);
    b.switch_to_block(fast);
    if let (Some(observer), Some(storage)) = (observer, storage) {
        let at = b
            .ins()
            .iconst(types::I64, (pack.first as u64 | (1 << 63)) as i64);
        let yes = b.ins().iconst(types::I64, 1);
        b.ins().call(observer, &[storage.heap, at, yes]);
    }
    let ordinal = b.ins().iadd_imm(base, pack.offset.wrapping_sub(1) as i64);
    let pointer = words.read(b, base_slot);
    let address = b.ins().iadd(pointer, ordinal);
    let value = numeric_read_packs::load_word(b, Default::default(), address, pack.width);
    words.write(b, layout.slot(pack.output.get()), value);
    if let Some(storage) = storage {
        storage.retire(
            b,
            words,
            &owners.retire_bypass(pack.first, pack.next as usize, &[]),
            error,
        );
    }
    b.ins().jump(next, &[]);
    b.switch_to_block(fallback);
    if let (Some(observer), Some(storage)) = (observer, storage) {
        let at = b
            .ins()
            .iconst(types::I64, (pack.first as u64 | (1 << 63)) as i64);
        let no = b.ins().iconst(types::I64, 0);
        b.ins().call(observer, &[storage.heap, at, no]);
    }
    Ok(())
}
