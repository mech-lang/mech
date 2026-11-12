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
    // The compact native contract has no finite work account. This is a real
    // unmetered specialization, not prepaid managed execution authority.
    Unmetered,
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
    classified: bool,
) -> cranelift_codegen::ir::Block {
    use crate::native_artifact::program::protocol::GlobalHelper;
    let zero = b.ins().iconst(types::I64, 0);
    let classes = b.ins().iconst(types::I64, i64::from(classified));
    hooks.call(
        b,
        site,
        GlobalHelper::HistogramPrepare,
        [classes, zero, zero],
        error,
    );
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
    if matches!(funding, Funding::Prepaid | Funding::Unmetered) {
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
/// Unmetered emission belongs only to the compact contract, which provides no
/// finite-work guarantees and obtains no managed execution authority here.
pub(in crate::native_artifact) fn define<M: Module>(
    module: &mut M,
    symbol: &str,
    vectorized: bool,
    funding: Funding,
    halfwords: bool,
    classified: bool,
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
        emit(&mut b, frame, vectorized, funding, halfwords, classified);
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

fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    vectorized: bool,
    funding: Funding,
    halfwords: bool,
    classified: bool,
) {
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
    let representatives = classified.then(|| get(b, frame, offset_of!(Frame, representatives)));
    if let Some(pointer) = representatives {
        let nonzero = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
        let alignment = b.ins().band_imm(pointer, 7);
        let aligned = b.ins().icmp_imm(IntCC::Equal, alignment, 0);
        let valid = b.ins().band(nonzero, aligned);
        require(b, valid, invalid);
    }
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
    spend(b, fuel, if classified { 24 } else { 16 }, cold, funding);
    let offset = b.ins().imul_imm(at, size_of::<Cell>() as i64);
    let cell = b.ins().iadd(cells, offset);
    let address = get(b, cell, offset_of!(Cell, address));
    let bytes = get(b, cell, offset_of!(Cell, bytes));
    // The private short-key bank stores every byte in one word. Equal packed
    // words and equal lengths are exact equality, not a probabilistic hash hit.
    // Long keys keep the maintained chunked hash and full byte comparison.
    let short_key = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, bytes, 8);
    let packed = b.create_block();
    let long = b.create_block();
    let hashed_key = b.create_block();
    b.append_block_param(hashed_key, types::I64);
    b.append_block_param(hashed_key, types::I64);
    b.ins().brif(short_key, packed, &[], long, &[]);
    b.switch_to_block(packed);
    let word = emit_packed_short_key(b, address, bytes, fuel, cold, funding);
    let bucket_hash = short_key_bucket_hash(b, word, bytes);
    let first = b.ins().band(bucket_hash, mask);
    b.ins().jump(hashed_key, &[word.into(), first.into()]);
    b.switch_to_block(long);
    let word = if matches!(funding, Funding::Prepaid | Funding::Unmetered) {
        emit_hash(b, address, bytes)
    } else {
        let hash_head = b.create_block();
        let hash_read = b.create_block();
        let hashed = b.create_block();
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
        word
    };
    let first = b.ins().band(word, mask);
    b.ins().jump(hashed_key, &[word.into(), first.into()]);
    b.switch_to_block(hashed_key);
    let word = b.block_params(hashed_key)[0];
    let first = b.block_params(hashed_key)[1];
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
    if matches!(funding, Funding::Prepaid | Funding::Unmetered) {
        // Both words came from complete, bounded short-key loads in this leaf.
        // Their checked equal lengths distinguish trailing NULs and zero padding.
        // Prepaid managed entry retains the original complete pass charges;
        // the metered test specialization still executes its exact debit ladder.
        let needs_bytes = b.create_block();
        b.ins().brif(short_key, hit, &[], needs_bytes, &[]);
        b.switch_to_block(needs_bytes);
    }
    let old_address = get(b, old_cell, offset_of!(Cell, address));
    b.def_var(equal_at, zero);
    b.ins().jump(compare_head, &[]);
    b.switch_to_block(compare_head);
    let offset = b.use_var(equal_at);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, bytes);
    let tail = b.create_block();
    b.ins().brif(more, tail, &[], hit, &[]);
    b.switch_to_block(tail);
    for width in if vectorized && halfwords {
        &[16i64, 8, 4, 2, 1][..]
    } else if vectorized {
        &[16i64, 8, 4, 1][..]
    } else {
        &[1i64][..]
    } {
        let read = b.create_block();
        let next = b.create_block();
        // Full vectors loop; after a short width succeeds, every smaller
        // exact tail is considered once without restarting the width ladder.
        let offset = b.use_var(equal_at);
        let remaining = b.ins().isub(bytes, offset);
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
            2 => types::I16,
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
        b.ins().jump(
            if vectorized && halfwords && *width < 16 {
                next
            } else {
                compare_head
            },
            &[],
        );
        b.switch_to_block(next);
    }
    // The straight tail has exhausted the equal byte extent. The legacy
    // ladder instead requires one width to jump back to its checked header.
    b.ins().jump(
        if vectorized && halfwords {
            hit
        } else {
            invalid
        },
        &[],
    );
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
    if let Some(representatives) = representatives {
        let first = get(b, slot, offset_of!(Slot, row_plus_one));
        let first = b.ins().iadd_imm(first, -1);
        let offset = b.ins().imul_imm(at, 8);
        let destination = b.ins().iadd(representatives, offset);
        b.ins().store(MemFlags::new(), first, destination, 0);
    }
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
    if let Some(representatives) = representatives {
        let offset = b.ins().imul_imm(at, 8);
        let destination = b.ins().iadd(representatives, offset);
        b.ins().store(MemFlags::new(), at, destination, 0);
    }
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

/// Lossless private representation for an arbitrary UTF8 byte string of at
/// most eight bytes. Each load is within the exact source allocation; an empty
/// input performs no load. This does not assume ASCII or accessible padding.
fn emit_packed_short_key(
    b: &mut FunctionBuilder<'_>,
    address: Value,
    bytes: Value,
    fuel: Variable,
    cold: cranelift_codegen::ir::Block,
    funding: Funding,
) -> Value {
    if matches!(funding, Funding::Prepaid | Funding::Unmetered) {
        return emit_packed_short_overlap(b, address, bytes);
    }
    let zero = b.ins().iconst(types::I64, 0);
    let packed = b.declare_var(types::I64);
    let position = b.declare_var(types::I64);
    b.def_var(packed, zero);
    b.def_var(position, zero);
    for (width, ty) in [
        (8i64, types::I64),
        (4, types::I32),
        (2, types::I16),
        (1, types::I8),
    ] {
        let read = b.create_block();
        let next = b.create_block();
        let at = b.use_var(position);
        let remaining = b.ins().isub(bytes, at);
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, width);
        b.ins().brif(enough, read, &[], next, &[]);
        b.switch_to_block(read);
        // Exactly the maintained hash's per-chunk debit and refusal point.
        spend(b, fuel, width * 2, cold, funding);
        let pointer = b.ins().iadd(address, at);
        let flags = MemFlags::new().with_endianness(cranelift_codegen::ir::Endianness::Little);
        let chunk = b.ins().load(ty, flags, pointer, 0);
        let chunk = if width == 8 {
            chunk
        } else {
            b.ins().uextend(types::I64, chunk)
        };
        let shift = b.ins().ishl_imm(at, 3);
        let chunk = b.ins().ishl(chunk, shift);
        let prior = b.use_var(packed);
        let combined = b.ins().bor(prior, chunk);
        b.def_var(packed, combined);
        let after = b.ins().iadd_imm(at, width);
        b.def_var(position, after);
        b.ins().jump(next, &[]);
        b.switch_to_block(next);
    }
    b.use_var(packed)
}

/// The complete short-key span was bounded by the caller before this entry.
/// For four through eight bytes, the first and last four-byte reads cover every
/// byte. Their overlap contains identical bits, so OR forms the exact little-
/// endian word. The two/three-byte case uses the first two and last one byte.
/// No read crosses either end, including an empty inaccessible byte pointer.
/// Logical metered chunk refusals remain in emit_packed_short_key; only entry
/// which already owns its complete pass allowance uses these overlapping loads.
fn emit_packed_short_overlap(b: &mut FunctionBuilder<'_>, address: Value, bytes: Value) -> Value {
    let flags = MemFlags::new().with_endianness(cranelift_codegen::ir::Endianness::Little);
    let four = b.create_block();
    let small = b.create_block();
    let two = b.create_block();
    let tiny = b.create_block();
    let one = b.create_block();
    let empty = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, bytes, 4);
    b.ins().brif(enough, four, &[], small, &[]);
    b.switch_to_block(four);
    let tail_at = b.ins().iadd_imm(bytes, -4);
    let tail = b.ins().iadd(address, tail_at);
    let first = b.ins().load(types::I32, flags, address, 0);
    let last = b.ins().load(types::I32, flags, tail, 0);
    let first = b.ins().uextend(types::I64, first);
    let last = b.ins().uextend(types::I64, last);
    let shift = b.ins().ishl_imm(tail_at, 3);
    let last = b.ins().ishl(last, shift);
    let packed = b.ins().bor(first, last);
    b.ins().jump(done, &[packed.into()]);
    b.switch_to_block(small);
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, bytes, 2);
    b.ins().brif(enough, two, &[], tiny, &[]);
    b.switch_to_block(two);
    let tail_at = b.ins().iadd_imm(bytes, -1);
    let tail = b.ins().iadd(address, tail_at);
    let first = b.ins().load(types::I16, flags, address, 0);
    let last = b.ins().load(types::I8, flags, tail, 0);
    let first = b.ins().uextend(types::I64, first);
    let last = b.ins().uextend(types::I64, last);
    let shift = b.ins().ishl_imm(tail_at, 3);
    let last = b.ins().ishl(last, shift);
    let packed = b.ins().bor(first, last);
    b.ins().jump(done, &[packed.into()]);
    b.switch_to_block(tiny);
    b.ins().brif(bytes, one, &[], empty, &[]);
    b.switch_to_block(one);
    let first = b.ins().load(types::I8, flags, address, 0);
    let first = b.ins().uextend(types::I64, first);
    b.ins().jump(done, &[first.into()]);
    b.switch_to_block(empty);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(done, &[zero.into()]);
    b.switch_to_block(done);
    b.block_params(done)[0]
}

/// Bucket mixing is independent of the exact full key retained in Slot.hash.
/// Two avalanche rounds distribute short inputs without adding a byte loop;
/// neither these bucket bits nor a long-key hash certify byte equality.
fn short_key_bucket_hash(b: &mut FunctionBuilder<'_>, packed: Value, bytes: Value) -> Value {
    let seed = b.ins().iconst(types::I64, HASH_OFFSET as i64);
    let word = b.ins().bxor(packed, seed);
    let word = b.ins().bxor(word, bytes);
    let upper = b.ins().ushr_imm(word, 33);
    let word = b.ins().bxor(word, upper);
    let word = b.ins().imul_imm(word, 0xff51afd7ed558ccdu64 as i64);
    let upper = b.ins().ushr_imm(word, 33);
    let word = b.ins().bxor(word, upper);
    let word = b.ins().imul_imm(word, 0xc4ceb9fe1a85ec53u64 as i64);
    let upper = b.ins().ushr_imm(word, 33);
    b.ins().bxor(word, upper)
}

#[cfg(test)]
mod tests;

/// Shared chunked UTF8 hash instructions for compact Map indexing and the
/// prepaid/unmetered histogram. Each load is bounded by the current byte span.
pub(in crate::native_artifact) fn emit_hash(
    b: &mut FunctionBuilder<'_>,
    address: Value,
    bytes: Value,
) -> Value {
    let zero = b.ins().iconst(types::I64, 0);
    let seed = b.ins().iconst(types::I64, HASH_OFFSET as i64);
    let seed = b.ins().bxor(seed, bytes);
    let head = b.create_block();
    b.append_block_param(head, types::I64);
    b.append_block_param(head, types::I64);
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    b.ins().jump(head, &[zero.into(), seed.into()]);
    b.switch_to_block(head);
    let at = b.block_params(head)[0];
    let prior = b.block_params(head)[1];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, bytes);
    b.ins().brif(more, body, &[], done, &[prior.into()]);
    b.switch_to_block(body);
    let remaining = b.ins().isub(bytes, at);
    for (width, ty) in [
        (8, types::I64),
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
        let pointer = b.ins().iadd(address, at);
        let word = b.ins().load(ty, MemFlags::new(), pointer, 0);
        let word = if width == 8 {
            word
        } else {
            b.ins().uextend(types::I64, word)
        };
        let mixed = b.ins().bxor(prior, word);
        let mixed = b.ins().imul_imm(mixed, 0x9e3779b185ebca87u64 as i64);
        let mixed = b.ins().rotl_imm(mixed, 27);
        let after = b.ins().iadd_imm(at, width);
        b.ins().jump(head, &[after.into(), mixed.into()]);
        b.switch_to_block(next);
    }
    // At least one width succeeds for every nonempty remaining byte span.
    b.ins()
        .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
    b.switch_to_block(done);
    let word = b.block_params(done)[0];
    let upper = b.ins().ushr_imm(word, 33);
    let word = b.ins().bxor(word, upper);
    let word = b.ins().imul_imm(word, 0xff51afd7ed558ccdu64 as i64);
    let upper = b.ins().ushr_imm(word, 33);
    b.ins().bxor(word, upper)
}
