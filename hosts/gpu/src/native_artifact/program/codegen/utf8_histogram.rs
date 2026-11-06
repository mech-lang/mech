//! Generated hash/count work over pinned UTF8 cells and a private slot bank.
//! This kernel grants no selected-call, publication or source-visit authority.
//! Entry/finalization must admit and retain both owners for the entire call.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::compiler::backend;
use crate::native_artifact::program::runtime::maps::utf8_read::{Cell, HASH_OFFSET};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Linkage, Module};
use std::mem::{offset_of, size_of};

use crate::native_artifact::program::runtime::maps::utf8_histogram::{
    COLD, COMPLETE, FULL, Frame, INVALID, Slot,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(in crate::native_artifact) enum Funding {
    #[cfg(test)]
    Metered,
    // Only the genuine original-call loan may choose this specialization.
    // Its checked pass_work bound has already been spent on the same account.
    Prepaid,
}

/// Static eligibility is the exact same maintained selected-call proof that
/// runtime rechecks before lending a source. No extra image authority exists.
pub(in crate::native_artifact) fn selected(
    metadata: &crate::native_artifact::program::prepare::ProgramMetadata,
    site: usize,
) -> bool {
    matches!(metadata.plan.blocks()[site].action(),
        mech_engine::native_artifact::NativeIterationAction::LibraryCall { operands, .. }
        if operands.len() == 1)
        && metadata.calls[site]
            .as_ref()
            .is_some_and(|call| call.supports_compiled_string_histogram())
}

/// Run the checked private leaf only between its genuine original-call loans.
/// A declined entry/finalization keeps the untouched ordinary library action.
pub(in crate::native_artifact) fn entry<M: Module>(
    b: &mut FunctionBuilder<'_>,
    module: &mut M,
    function: FuncId,
    hooks: &crate::native_artifact::program::codegen::hooks::Hooks,
    site: usize,
    error: cranelift_codegen::ir::Block,
) -> cranelift_codegen::ir::Block {
    use crate::native_artifact::program::protocol::GlobalHelper;
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, GlobalHelper::HistogramPrepare, [zero; 3], error);
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let native = b.create_block();
    b.append_block_param(native, types::I64);
    let ordinary = b.create_block();
    let done = b.create_block();
    b.ins().brif(frame, native, &[frame.into()], ordinary, &[]);
    b.switch_to_block(native);
    let frame = b.block_params(native)[0];
    let function = module.declare_func_in_func(function, b.func);
    let call = b.ins().call(function, &[frame]);
    let status = b.inst_results(call)[0];
    hooks.call(
        b,
        site,
        GlobalHelper::HistogramFinish,
        [frame, status, zero],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let finished = b.ins().icmp_imm(IntCC::Equal, ready, 1);
    let retry = b.create_block();
    b.ins().brif(finished, done, &[], retry, &[]);
    b.switch_to_block(retry);
    b.ins().brif(ready, native, &[ready.into()], ordinary, &[]);
    b.switch_to_block(ordinary);
    done
}

fn get(b: &mut FunctionBuilder<'_>, base: Value, offset: usize) -> Value {
    b.ins()
        .load(types::I64, MemFlags::new(), base, offset as i32)
}
fn require(b: &mut FunctionBuilder<'_>, condition: Value, failed: cranelift_codegen::ir::Block) {
    let next = b.create_block();
    b.ins().brif(condition, next, &[], failed, &[]);
    b.switch_to_block(next);
}
fn spend(
    b: &mut FunctionBuilder<'_>,
    variable: Variable,
    units: i64,
    cold: cranelift_codegen::ir::Block,
    funding: Funding,
) {
    if funding == Funding::Prepaid {
        return;
    }
    let available = b.use_var(variable);
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, available, units);
    require(b, enough, cold);
    let remaining = b.ins().iadd_imm(available, -units);
    b.def_var(variable, remaining);
}

/// Define a leaf with no calls, allocations or per-key host dispatch. All exits
/// preserve a complete committed input prefix. FULL/COLD leave the current row
/// uncommitted; the owner may retain/grow the bank and resume at `completed`.
/// The caller must provide valid, aligned, disjoint banks, immutable cells and
/// UTF8 byte owners, and truthful prefix state. Header checks are defensive;
/// they do not establish the provenance of arbitrary native pointers.
/// Prepaid emission additionally requires the complete checked pass allowance
/// to have been spent before entry. Its frame counters are not reusable credit.
pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    symbol: &str,
    vectorized: bool,
    funding: Funding,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    if module.target_config().pointer_type() != types::I64 {
        return Err(NativeCompileError::UnsupportedHost);
    }
    let mut context = module.make_context();
    context
        .func
        .signature
        .params
        .push(AbiParam::new(types::I64));
    context
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    let id = module
        .declare_function(symbol, Linkage::Local, &context.func.signature)
        .map_err(backend)?;
    let mut fc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let frame = b.block_params(entry)[0];
        emit(&mut b, frame, vectorized, funding);
        b.seal_all_blocks();
        b.finalize();
    }
    for block in context.func.layout.blocks() {
        for inst in context.func.layout.block_insts(block) {
            if matches!(
                context.func.dfg.insts[inst].opcode(),
                cranelift_codegen::ir::Opcode::Call | cranelift_codegen::ir::Opcode::CallIndirect
            ) {
                return Err(backend("UTF8 histogram work must not dispatch a host call"));
            }
        }
    }
    context.verify(module.isa()).map_err(backend)?;
    module.define_function(id, &mut context).map_err(backend)?;
    let code = context
        .compiled_code()
        .ok_or_else(|| backend("UTF8 histogram code missing"))?;
    Ok((
        id,
        crate::native_artifact::compiler::compiled_stack_bytes(&code.buffer)?,
        code.code_info().total_size as u64,
    ))
}

fn emit(b: &mut FunctionBuilder<'_>, frame: Value, vectorized: bool, funding: Funding) {
    let complete = b.create_block();
    let cold = b.create_block();
    let full = b.create_block();
    let invalid = b.create_block();
    let exit = b.create_block();
    b.append_block_param(exit, types::I64);
    for block in [cold, full, invalid] {
        b.set_cold_block(block);
    }
    let row = b.declare_var(types::I64);
    let fuel = b.declare_var(types::I64);
    let comparison = b.declare_var(types::I64);
    let distinct = b.declare_var(types::I64);
    let payload = b.declare_var(types::I64);
    for (variable, field) in [
        (row, offset_of!(Frame, completed)),
        (fuel, offset_of!(Frame, fuel)),
        (comparison, offset_of!(Frame, comparison)),
        (distinct, offset_of!(Frame, distinct)),
        (payload, offset_of!(Frame, payload)),
    ] {
        let value = get(b, frame, field);
        b.def_var(variable, value);
    }
    let cells = get(b, frame, offset_of!(Frame, cells));
    let count = get(b, frame, offset_of!(Frame, count));
    let slots = get(b, frame, offset_of!(Frame, slots));
    let capacity = get(b, frame, offset_of!(Frame, capacity));
    let limit = get(b, frame, offset_of!(Frame, limit));
    let nonzero_limit = b.ins().icmp_imm(IntCC::NotEqual, limit, 0);
    let bounded_limit = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, limit, capacity);
    let good_limit = b.ins().band(nonzero_limit, bounded_limit);
    require(b, good_limit, invalid);
    let mask = b.ins().iadd_imm(capacity, -1);
    let power = b.ins().band(capacity, mask);
    let power = b.ins().icmp_imm(IntCC::Equal, power, 0);
    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, capacity, 0);
    let good = b.ins().band(power, nonzero);
    require(b, good, invalid);
    let fits = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        capacity,
        (isize::MAX as usize / size_of::<Slot>()) as i64,
    );
    require(b, fits, invalid);
    let fits = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        count,
        (isize::MAX as usize / size_of::<Cell>()) as i64,
    );
    require(b, fits, invalid);
    let at = b.use_var(row);
    let good = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, at, count);
    require(b, good, invalid);
    let n = b.use_var(distinct);
    let good = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, n, limit);
    require(b, good, invalid);
    let good = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, n, at);
    require(b, good, invalid);
    let aligned = b.ins().band_imm(slots, 7);
    let aligned = b.ins().icmp_imm(IntCC::Equal, aligned, 0);
    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, slots, 0);
    let good = b.ins().band(aligned, nonzero);
    require(b, good, invalid);
    let aligned = b.ins().band_imm(cells, 7);
    let aligned = b.ins().icmp_imm(IntCC::Equal, aligned, 0);
    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, cells, 0);
    let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
    let present = b.ins().bor(nonzero, empty);
    let good = b.ins().band(aligned, present);
    require(b, good, invalid);

    let header = b.create_block();
    let query = b.create_block();
    let hash_head = b.create_block();
    let hash_read = b.create_block();
    let hashed = b.create_block();
    let probe = b.create_block();
    let inspect = b.create_block();
    let occupied = b.create_block();
    let compare = b.create_block();
    let compare_head = b.create_block();
    let advance = b.create_block();
    let hit = b.create_block();
    let insert = b.create_block();
    let commit = b.create_block();
    let byte_at = b.declare_var(types::I64);
    let hash = b.declare_var(types::I64);
    let probe_at = b.declare_var(types::I64);
    let probes = b.declare_var(types::I64);
    let equal_at = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at = b.use_var(row);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, query, &[], complete, &[]);
    b.switch_to_block(query);
    spend(b, fuel, 16, cold, funding);
    let offset = b.ins().imul_imm(at, size_of::<Cell>() as i64);
    let cell = b.ins().iadd(cells, offset);
    let address = get(b, cell, offset_of!(Cell, address));
    let bytes = get(b, cell, offset_of!(Cell, bytes));
    let seed = b.ins().iconst(types::I64, HASH_OFFSET as i64);
    let first = b.ins().bxor(seed, bytes);
    b.def_var(hash, first);
    b.def_var(byte_at, zero);
    b.ins().jump(hash_head, &[]);
    b.switch_to_block(hash_head);
    let at_byte = b.use_var(byte_at);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at_byte, bytes);
    b.ins().brif(more, hash_read, &[], hashed, &[]);
    b.switch_to_block(hash_read);
    let remaining = b.ins().isub(bytes, at_byte);
    for (width, ty) in [
        (8i64, types::I64),
        (4, types::I32),
        (2, types::I16),
        (1, types::I8),
    ] {
        let read = b.create_block();
        let next = b.create_block();
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, width);
        b.ins().brif(enough, read, &[], next, &[]);
        b.switch_to_block(read);
        spend(b, fuel, width * 2, cold, funding);
        let pointer = b.ins().iadd(address, at_byte);
        let word = b.ins().load(ty, MemFlags::new(), pointer, 0);
        let word = if width == 8 {
            word
        } else {
            b.ins().uextend(types::I64, word)
        };
        let prior = b.use_var(hash);
        let mixed = b.ins().bxor(prior, word);
        let mixed = b.ins().imul_imm(mixed, 0x9e3779b185ebca87u64 as i64);
        let mixed = b.ins().rotl_imm(mixed, 27);
        b.def_var(hash, mixed);
        let after = b.ins().iadd_imm(at_byte, width);
        b.def_var(byte_at, after);
        b.ins().jump(hash_head, &[]);
        b.switch_to_block(next);
    }
    b.ins().jump(invalid, &[]);
    b.switch_to_block(hashed);
    let word = b.use_var(hash);
    let upper = b.ins().ushr_imm(word, 33);
    let word = b.ins().bxor(word, upper);
    let word = b.ins().imul_imm(word, 0xff51afd7ed558ccdu64 as i64);
    let upper = b.ins().ushr_imm(word, 33);
    let word = b.ins().bxor(word, upper);
    let first = b.ins().band(word, mask);
    b.def_var(probe_at, first);
    b.def_var(probes, zero);
    b.ins().jump(probe, &[]);
    b.switch_to_block(probe);
    let tried = b.use_var(probes);
    let room = b.ins().icmp(IntCC::UnsignedLessThan, tried, capacity);
    b.ins().brif(room, inspect, &[], full, &[]);
    b.switch_to_block(inspect);
    spend(b, fuel, 8, cold, funding);
    let position = b.use_var(probe_at);
    let offset = b.ins().imul_imm(position, size_of::<Slot>() as i64);
    let slot = b.ins().iadd(slots, offset);
    let old_row = get(b, slot, offset_of!(Slot, row_plus_one));
    let present = b.ins().icmp_imm(IntCC::NotEqual, old_row, 0);
    b.ins().brif(present, occupied, &[], insert, &[]);
    b.switch_to_block(occupied);
    let good = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, old_row, at);
    require(b, good, invalid);
    let old_hash = get(b, slot, offset_of!(Slot, hash));
    let same = b.ins().icmp(IntCC::Equal, old_hash, word);
    b.ins().brif(same, compare, &[], advance, &[]);
    b.switch_to_block(compare);
    let index = b.ins().iadd_imm(old_row, -1);
    let offset = b.ins().imul_imm(index, size_of::<Cell>() as i64);
    let old_cell = b.ins().iadd(cells, offset);
    let old_bytes = get(b, old_cell, offset_of!(Cell, bytes));
    let same = b.ins().icmp(IntCC::Equal, old_bytes, bytes);
    let comparable = b.create_block();
    b.ins().brif(same, comparable, &[], advance, &[]);
    b.switch_to_block(comparable);
    let old_address = get(b, old_cell, offset_of!(Cell, address));
    b.def_var(equal_at, zero);
    b.ins().jump(compare_head, &[]);
    b.switch_to_block(compare_head);
    let offset = b.use_var(equal_at);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, bytes);
    let tail = b.create_block();
    b.ins().brif(more, tail, &[], hit, &[]);
    b.switch_to_block(tail);
    let remaining = b.ins().isub(bytes, offset);
    for width in if vectorized {
        &[16i64, 8, 4, 1][..]
    } else {
        &[1i64][..]
    } {
        let read = b.create_block();
        let next = b.create_block();
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, *width);
        b.ins().brif(enough, read, &[], next, &[]);
        b.switch_to_block(read);
        spend(b, fuel, *width, cold, funding);
        spend(b, comparison, *width, cold, funding);
        let source = b.ins().iadd(address, offset);
        let target = b.ins().iadd(old_address, offset);
        let ty = match width {
            16 => types::I8X16,
            8 => types::I64,
            4 => types::I32,
            _ => types::I8,
        };
        let left = b.ins().load(ty, MemFlags::new(), source, 0);
        let right = b.ins().load(ty, MemFlags::new(), target, 0);
        let same = if *width == 16 {
            let different = b.ins().bxor(left, right);
            let any = b.ins().vany_true(different);
            b.ins().icmp_imm(IntCC::Equal, any, 0)
        } else {
            b.ins().icmp(IntCC::Equal, left, right)
        };
        let equal = b.create_block();
        b.ins().brif(same, equal, &[], advance, &[]);
        b.switch_to_block(equal);
        let after = b.ins().iadd_imm(offset, *width);
        b.def_var(equal_at, after);
        b.ins().jump(compare_head, &[]);
        b.switch_to_block(next);
    }
    // Offset<bytes proves the final one-byte case; retain a defensive failure.
    b.ins().jump(invalid, &[]);
    b.switch_to_block(advance);
    let position = b.use_var(probe_at);
    let next = b.ins().iadd_imm(position, 1);
    let next = b.ins().band(next, mask);
    b.def_var(probe_at, next);
    let tried = b.use_var(probes);
    let next = b.ins().iadd_imm(tried, 1);
    b.def_var(probes, next);
    b.ins().jump(probe, &[]);
    b.switch_to_block(hit);
    let old = get(b, slot, offset_of!(Slot, count));
    let valid = b.ins().icmp_imm(IntCC::NotEqual, old, -1);
    require(b, valid, invalid);
    let next = b.ins().iadd_imm(old, 1);
    b.ins()
        .store(MemFlags::new(), next, slot, offset_of!(Slot, count) as i32);
    b.ins().jump(commit, &[]);
    b.switch_to_block(insert);
    let current_distinct = b.use_var(distinct);
    let room = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, current_distinct, limit);
    require(b, room, full);
    let old_payload = b.use_var(payload);
    let (next_payload, overflow) = b.ins().uadd_overflow(old_payload, bytes);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, invalid);
    let old_distinct = b.use_var(distinct);
    let next_distinct = b.ins().iadd_imm(old_distinct, 1);
    let one = b.ins().iconst(types::I64, 1);
    b.ins()
        .store(MemFlags::new(), word, slot, offset_of!(Slot, hash) as i32);
    b.ins()
        .store(MemFlags::new(), one, slot, offset_of!(Slot, count) as i32);
    let marker = b.ins().iadd_imm(at, 1);
    b.ins().store(
        MemFlags::new(),
        marker,
        slot,
        offset_of!(Slot, row_plus_one) as i32,
    );
    b.def_var(distinct, next_distinct);
    b.def_var(payload, next_payload);
    b.ins().jump(commit, &[]);
    b.switch_to_block(commit);
    let next = b.ins().iadd_imm(at, 1);
    b.def_var(row, next);
    b.ins().jump(header, &[]);
    for (block, status) in [
        (complete, COMPLETE),
        (cold, COLD),
        (full, FULL),
        (invalid, INVALID),
    ] {
        b.switch_to_block(block);
        let status = b.ins().iconst(types::I64, status as i64);
        b.ins().jump(exit, &[status.into()]);
    }
    b.switch_to_block(exit);
    for (variable, field) in [
        (row, offset_of!(Frame, completed)),
        (fuel, offset_of!(Frame, fuel)),
        (comparison, offset_of!(Frame, comparison)),
        (distinct, offset_of!(Frame, distinct)),
        (payload, offset_of!(Frame, payload)),
    ] {
        let value = b.use_var(variable);
        b.ins().store(MemFlags::new(), value, frame, field as i32);
    }
    let status = b.block_params(exit)[0];
    b.ins().return_(&[status]);
}

#[cfg(test)]
mod tests;
