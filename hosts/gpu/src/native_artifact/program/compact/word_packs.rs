//! Retain the existing confined byte-expression proof and machine word load.
//! A fresh source window can also admit one shared matrix read.
use super::{arrays::Arrays, layout::Layout, owner_flow::OwnerFlow};
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::program::{
    codegen::numeric_read_packs::{self, Pack},
    prepare::PreparedNativeProgram,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};
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
/// The caller's source window admits every logical byte and the current
/// view's descriptor proves its physical backing. Width is always 2, 4 or 8.
pub(super) fn define_matrix_word(module: &mut impl Module) -> Result<FuncId, NativeCompileError> {
    let mut context = module.make_context();
    let mut signature = module.make_signature();
    signature.params = vec![AbiParam::new(types::I64); 5];
    signature.returns = vec![AbiParam::new(types::I64)];
    context.func.signature = signature;
    let function = module
        .declare_function("mech_compact_matrix_word", Linkage::Local, &context.func.signature)
        .map_err(backend)?;
    let mut builder = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        let read = b.create_block();
        let done = b.create_block();
        b.append_block_params_for_function_params(entry);
        for _ in 0..4 {
            b.append_block_param(read, types::I64);
        }
        b.append_block_param(done, types::I64);
        b.switch_to_block(entry);
        let [pointer, ordinal, rows, columns, width]: [Value; 5] =
            b.block_params(entry).try_into().unwrap();
        let column = b.ins().udiv(ordinal, rows);
        let base = b.ins().imul(column, rows);
        let row = b.ins().isub(ordinal, base);
        let row_start = b.ins().imul(row, columns);
        let physical = b.ins().iadd(row_start, column);
        let address = b.ins().iadd(pointer, physical);
        let remaining = b.ins().isub(rows, row);
        // The caller admits a non-flat view, so a pair crosses at most one
        // column boundary. Crossing replaces one whole column stride with
        // the advance to the next column's first row.
        let view_length = b.ins().imul(rows, columns);
        let wrap_adjustment = b.ins().irsub_imm(view_length, 1);
        let single_wrap = b.ins().iadd(columns, wrap_adjustment);
        let pair_step = b.ins().ishl_imm(columns, 1);
        let pair_wrap = b.ins().iadd(pair_step, wrap_adjustment);
        let limit = b.ins().ishl_imm(width, 3);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(read, &[address.into(), remaining.into(), zero.into(), zero.into()]);
        b.switch_to_block(read);
        let [address, remaining, shift, word]: [Value; 4] =
            b.block_params(read).try_into().unwrap();
        let first_wrap = b.ins().icmp_imm(IntCC::Equal, remaining, 1);
        let second_step = b.ins().select(first_wrap, single_wrap, columns);
        let second_address = b.ins().iadd(address, second_step);
        let first = b.ins().uload8(types::I64, MemFlags::new(), address, 0);
        let second = b.ins().uload8(types::I64, MemFlags::new(), second_address, 0);
        let hi = b.ins().ishl_imm(second, 8);
        let pair = b.ins().bor(first, hi);
        let part = b.ins().ishl(pair, shift);
        let word = b.ins().bor(word, part);
        let wraps = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, remaining, 2);
        let step = b.ins().select(wraps, pair_wrap, pair_step);
        let next_address = b.ins().iadd(address, step);
        let row_adjustment = b.ins().select(wraps, rows, zero);
        let remaining = b.ins().iadd(remaining, row_adjustment);
        let next_remaining = b.ins().iadd_imm(remaining, -2);
        let next_shift = b.ins().iadd_imm(shift, 16);
        let complete = b.ins().icmp(IntCC::Equal, next_shift, limit);
        b.ins().brif(
            complete,
            done,
            &[word.into()],
            read,
            &[next_address.into(), next_remaining.into(), next_shift.into(), word.into()],
        );
        b.switch_to_block(done);
        let word = b.block_params(done)[0];
        b.ins().return_(&[word]);
        b.seal_all_blocks();
        b.finalize();
    }
    module.define_function(function, &mut context).map_err(backend)?;
    Ok(function)
}

fn admitted_matrix_word(
    module: &mut impl Module,
    b: &mut FunctionBuilder<'_>,
    helper: FuncId,
    pointer: Value,
    ordinal: Value,
    rows: Value,
    columns: Value,
    flat: Value,
    width: u8,
) -> Value {
    let contiguous = b.create_block();
    let matrix = b.create_block();
    let next = b.create_block();
    b.append_block_param(next, types::I64);
    b.set_cold_block(matrix);
    b.ins().brif(flat, contiguous, &[], matrix, &[]);
    b.switch_to_block(contiguous);
    let address = b.ins().iadd(pointer, ordinal);
    let value = numeric_read_packs::load_word(b, Default::default(), address, width);
    b.ins().jump(next, &[value.into()]);
    b.switch_to_block(matrix);
    let function = module.declare_func_in_func(helper, b.func);
    let width = b.ins().iconst(types::I64, i64::from(width));
    let call = b.ins().call(function, &[pointer, ordinal, rows, columns, width]);
    let value = b.inst_results(call)[0];
    b.ins().jump(next, &[value.into()]);
    b.switch_to_block(next);
    b.block_params(next)[0]
}

pub(super) fn emit<M: Module>(
    module: &mut M,
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &Layout,
    owners: &OwnerFlow,
    storage: Option<&Arrays>,
    observe: bool,
    matrix_word: Option<FuncId>,
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
    if let Some(helper) = matrix_word.filter(|_| pack.source_window) {
        let ordinal = b.ins().iadd_imm(base, pack.offset.wrapping_sub(1) as i64);
        let pointer = words.read(b, base_slot);
        let value = admitted_matrix_word(
            module, b, helper, pointer, ordinal, rows, columns, valid, pack.width,
        );
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
        let original = b.create_block();
        b.switch_to_block(original);
        return Ok(());
    }
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
