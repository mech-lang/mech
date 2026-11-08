//! Native UTF8 lookup WORK over an admitted, pinned source/table/output bank.
//! Physical preflight has no logical source visit or publication authority.
//! Missing keys leave only private writes, so the caller can discard the bank
//! and resume its unchanged ordinary CFG without replaying charged producers.
use super::hooks::Hooks;
use crate::native_artifact::compiler::{guard, offset};
use crate::native_artifact::program::protocol::GlobalHelper;
use crate::native_artifact::program::runtime::maps::utf8_read::{
    COLD, COMPLETE, Cell, Frame, HASH_OFFSET, HASH_PRIME, MISSING, Slot,
};
use crate::native_artifact::{NativeCompileError, abi::STATUS_ABI};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use std::mem::{offset_of, size_of};

/// Attempt at the ordinary Collection header after its original admission.
/// A decline leaves the ordinary header action and terminator intact. A
/// complete preflight commits once, then uses the ordinary finalization edge.
pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    site: usize,
    hooks: &Hooks,
    counter_slot: usize,
    exit: Block,
    error: Block,
    word_chunks: bool,
) -> Result<(), NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(b, site, GlobalHelper::Utf8ReadPrepare, [zero; 3], error);
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let selected = b.create_block();
    let ordinary = b.create_block();
    b.ins().brif(frame, selected, &[], ordinary, &[]);
    b.switch_to_block(selected);
    let complete = b.create_block();
    let declined = b.create_block();
    let missing = b.create_block();
    let settle = b.create_block();
    b.append_block_param(settle, types::I64);
    // Prepare has paid the complete physical worst case before lending the
    // private bank. Logical body fees remain uncommitted until full success.
    emit(
        b,
        frame,
        complete,
        declined,
        missing,
        true,
        true,
        true,
        word_chunks,
    );
    for (block, status) in [(complete, COMPLETE), (declined, COLD), (missing, MISSING)] {
        b.switch_to_block(block);
        let status = b.ins().iconst(types::I64, status as i64);
        b.ins().jump(settle, &[status.into()]);
    }
    b.switch_to_block(settle);
    let status = b.block_params(settle)[0];
    hooks.call(
        b,
        site,
        GlobalHelper::Utf8ReadFinish,
        [frame, status, zero],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, ready, 1);
    guard(b, valid, error, STATUS_ABI);
    let committed = b.create_block();
    b.ins().brif(ready, committed, &[], ordinary, &[]);
    b.switch_to_block(committed);
    b.ins()
        .store(MemFlags::new(), zero, locals, offset(counter_slot)?);
    b.ins().jump(exit, &[]);
    b.switch_to_block(ordinary);
    Ok(())
}

fn get(b: &mut FunctionBuilder<'_>, base: Value, at: usize) -> Value {
    b.ins().load(types::I64, MemFlags::new(), base, at as i32)
}
fn put(b: &mut FunctionBuilder<'_>, base: Value, at: usize, value: Value) {
    b.ins().store(MemFlags::new(), value, base, at as i32);
}
fn require(b: &mut FunctionBuilder<'_>, yes: Value, cold: Block) {
    let next = b.create_block();
    b.ins().brif(yes, next, &[], cold, &[]);
    b.switch_to_block(next);
}
fn add(b: &mut FunctionBuilder<'_>, left: Value, right: Value, cold: Block) -> Value {
    let (value, overflow) = b.ins().uadd_overflow(left, right);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, cold);
    value
}
fn mul(b: &mut FunctionBuilder<'_>, left: Value, right: Value, cold: Block) -> Value {
    let high = b.ins().umulhi(left, right);
    let good = b.ins().icmp_imm(IntCC::Equal, high, 0);
    require(b, good, cold);
    b.ins().imul(left, right)
}

// Bounds are established by the caller's remaining-length branch. The load
// is unaligned and explicitly little-endian, preserving byte order on every
// target. It replaces exactly `width` original byte loads and FNV steps.
fn hash_chunk(
    b: &mut FunctionBuilder<'_>,
    address: cranelift_frontend::Variable,
    cursor: cranelift_frontend::Variable,
    hash: cranelift_frontend::Variable,
    width: i64,
) {
    let start = b.use_var(address);
    let at = b.use_var(cursor);
    let pointer = b.ins().iadd(start, at);
    let ty = match width {
        8 => types::I64,
        4 => types::I32,
        2 => types::I16,
        _ => unreachable!(),
    };
    let flags = MemFlags::new().with_endianness(cranelift_codegen::ir::Endianness::Little);
    let word = b.ins().load(ty, flags, pointer, 0);
    let word = if width == 8 {
        word
    } else {
        b.ins().uextend(types::I64, word)
    };
    let mut current = b.use_var(hash);
    for byte in 0..width {
        let part = if byte == 0 {
            word
        } else {
            b.ins().ushr_imm(word, byte * 8)
        };
        let part = b.ins().band_imm(part, 255);
        let mixed = b.ins().bxor(current, part);
        current = b.ins().imul_imm(mixed, HASH_PRIME as i64);
    }
    b.def_var(hash, current);
    let next = b.ins().iadd_imm(at, width);
    b.def_var(cursor, next);
}

/// Every pointer and extent must come from the caller's live checked loans.
/// `prepaid` means the entire physical worst-case allowance was already paid;
/// otherwise the finite physical allowance in frame.fuel/comparison is reduced
/// per query. Neither mode consumes producer, Map, or Collection logical fees.
/// All exits store the exact completed query count and remaining allowance.
pub(in crate::native_artifact) fn emit(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    complete: Block,
    declined: Block,
    missing: Block,
    prepaid: bool,
    vectorized: bool,
    physical_order: bool,
    word_chunks: bool,
) {
    let cold = b.create_block();
    let done = b.create_block();
    let missing_key = b.create_block();
    let header = b.create_block();
    let query = b.create_block();
    let hash_head = b.create_block();
    let hash_byte = b.create_block();
    let hash_done = b.create_block();
    let probe = b.create_block();
    let inspect = b.create_block();
    let compare = b.create_block();
    let compare_byte = b.create_block();
    let scalar_tail = b.create_block();
    let advance = b.create_block();
    let hit = b.create_block();
    let miss = b.create_block();
    let commit = b.create_block();
    let fuel = b.declare_var(types::I64);
    let comparison = b.declare_var(types::I64);
    let cursor = b.declare_var(types::I64);
    let hash_at = b.declare_var(types::I64);
    let hash_word = b.declare_var(types::I64);
    let probe_at = b.declare_var(types::I64);
    let probes = b.declare_var(types::I64);
    let equal_at = b.declare_var(types::I64);
    let key_address = b.declare_var(types::I64);
    let key_bytes = b.declare_var(types::I64);
    let slot_address = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    let initial = get(b, frame, offset_of!(Frame, fuel));
    b.def_var(fuel, initial);
    let initial = get(b, frame, offset_of!(Frame, comparison));
    b.def_var(comparison, initial);
    let initial = get(b, frame, offset_of!(Frame, next));
    b.def_var(cursor, initial);
    let epoch = get(b, frame, offset_of!(Frame, epoch));
    let issued = get(b, frame, offset_of!(Frame, issued));
    let same = b.ins().icmp(IntCC::Equal, epoch, issued);
    let live = b.ins().icmp_imm(IntCC::NotEqual, issued, 0);
    let good = b.ins().band(same, live);
    require(b, good, cold);
    let capacity = get(b, frame, offset_of!(Frame, capacity));
    let mask = b.ins().iadd_imm(capacity, -1);
    let bits = b.ins().band(capacity, mask);
    let power = b.ins().icmp_imm(IntCC::Equal, bits, 0);
    let minimum = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, capacity, 4);
    let good = b.ins().band(power, minimum);
    require(b, good, cold);
    let count = get(b, frame, offset_of!(Frame, count));
    let rows = get(b, frame, offset_of!(Frame, rows));
    let columns = get(b, frame, offset_of!(Frame, columns));
    let product = mul(b, rows, columns, cold);
    let geometry = b.ins().icmp(IntCC::Equal, product, count);
    require(b, geometry, cold);
    if physical_order {
        // The selected source region iterates CollectionElement in physical
        // order. Prepare lends exactly this normalized geometry, including an
        // empty batch. Validate it once before omitting per-key division.
        let one_row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
        require(b, one_row, cold);
    }
    let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, rows, 0);
    let good = b.ins().bor(empty, nonzero);
    require(b, good, cold);
    let start = b.use_var(cursor);
    let valid = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, start, count);
    require(b, valid, cold);
    let cells = get(b, frame, offset_of!(Frame, cells));
    let slots = get(b, frame, offset_of!(Frame, slots));
    let output = get(b, frame, offset_of!(Frame, output));
    let stored = get(b, frame, offset_of!(Frame, stored_bytes));
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at = b.use_var(cursor);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, query, &[], done, &[]);
    b.switch_to_block(query);
    let at = b.use_var(cursor);
    let physical = if physical_order {
        at
    } else {
        let row = b.ins().urem(at, rows);
        let column = b.ins().udiv(at, rows);
        let base = b.ins().imul(row, columns);
        b.ins().iadd(base, column)
    };
    let offset = b.ins().imul_imm(physical, size_of::<Cell>() as i64);
    let cell = b.ins().iadd(cells, offset);
    let address = get(b, cell, offset_of!(Cell, address));
    let length = get(b, cell, offset_of!(Cell, bytes));
    b.def_var(key_address, address);
    b.def_var(key_bytes, length);
    if !prepaid {
        // Conservative native physical bound includes every table probe, all
        // stored-key bytes under full collisions, and the query hash bytes.
        let four = b.ins().iconst(types::I64, 4);
        let cost = mul(b, capacity, four, cold);
        let overhead = b.ins().iconst(types::I64, 32);
        let cost = add(b, cost, overhead, cold);
        let cost = add(b, cost, stored, cold);
        let cost = add(b, cost, length, cold);
        let scale = b.ins().iconst(types::I64, 64);
        let compute = mul(b, cost, scale, cold);
        let available = b.use_var(fuel);
        let a = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, available, compute);
        let compares = b.use_var(comparison);
        let c = b
            .ins()
            .icmp(IntCC::UnsignedGreaterThanOrEqual, compares, cost);
        let enough = b.ins().band(a, c);
        require(b, enough, cold);
        let next = b.ins().isub(available, compute);
        b.def_var(fuel, next);
        let next = b.ins().isub(compares, cost);
        b.def_var(comparison, next);
    }
    b.def_var(hash_at, zero);
    let seed = b.ins().iconst(types::I64, HASH_OFFSET as i64);
    b.def_var(hash_word, seed);
    b.ins().jump(hash_head, &[]);
    b.switch_to_block(hash_head);
    let at = b.use_var(hash_at);
    let length = b.use_var(key_bytes);
    if word_chunks {
        // Keep exactly the original bytewise FNV recurrence. One bounded
        // little-endian load supplies independent byte values; only pointer,
        // load and loop-control work is shared. No payload byte is omitted.
        let remaining = b.ins().isub(length, at);
        let full = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 8);
        let batch = b.create_block();
        let tail = b.create_block();
        b.ins().brif(full, batch, &[], tail, &[]);
        b.switch_to_block(batch);
        hash_chunk(b, key_address, hash_at, hash_word, 8);
        b.ins().jump(hash_head, &[]);
        b.switch_to_block(tail);
        // Once fewer than eight bytes remain, take each exact tail width at
        // most once. The final byte block therefore never re-enters the loop.
        for width in [4, 2] {
            let at = b.use_var(hash_at);
            let remaining = b.ins().isub(length, at);
            let full = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, width);
            let batch = b.create_block();
            let tail = b.create_block();
            b.ins().brif(full, batch, &[], tail, &[]);
            b.switch_to_block(batch);
            hash_chunk(b, key_address, hash_at, hash_word, width);
            b.ins().jump(tail, &[]);
            b.switch_to_block(tail);
        }
    }
    let at = b.use_var(hash_at);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, length);
    b.ins().brif(more, hash_byte, &[], hash_done, &[]);
    b.switch_to_block(hash_byte);
    let address = b.use_var(key_address);
    let at = b.use_var(hash_at);
    let address = b.ins().iadd(address, at);
    let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
    let byte = b.ins().uextend(types::I64, byte);
    let hash = b.use_var(hash_word);
    let hash = b.ins().bxor(hash, byte);
    let hash = b.ins().imul_imm(hash, HASH_PRIME as i64);
    b.def_var(hash_word, hash);
    let at = b.ins().iadd_imm(at, 1);
    b.def_var(hash_at, at);
    b.ins()
        .jump(if word_chunks { hash_done } else { hash_head }, &[]);
    b.switch_to_block(hash_done);
    let hash = b.use_var(hash_word);
    let at = b.ins().band(hash, mask);
    b.def_var(probe_at, at);
    b.def_var(probes, capacity);
    b.ins().jump(probe, &[]);
    b.switch_to_block(probe);
    let left = b.use_var(probes);
    let more = b.ins().icmp_imm(IntCC::NotEqual, left, 0);
    b.ins().brif(more, inspect, &[], cold, &[]);
    b.switch_to_block(inspect);
    let at = b.use_var(probe_at);
    let offset = b.ins().imul_imm(at, size_of::<Slot>() as i64);
    let slot = b.ins().iadd(slots, offset);
    b.def_var(slot_address, slot);
    let occupied = get(b, slot, offset_of!(Slot, occupied));
    let empty = b.ins().icmp_imm(IntCC::Equal, occupied, 0);
    let occupied_block = b.create_block();
    b.ins().brif(empty, miss, &[], occupied_block, &[]);
    b.switch_to_block(occupied_block);
    let prior = get(b, slot, offset_of!(Slot, hash));
    let hash = b.use_var(hash_word);
    let same = b.ins().icmp(IntCC::Equal, prior, hash);
    let prior = get(b, slot, offset_of!(Slot, bytes));
    let length = b.use_var(key_bytes);
    let same_length = b.ins().icmp(IntCC::Equal, prior, length);
    let same = b.ins().band(same, same_length);
    let start_compare = b.create_block();
    b.ins().brif(same, start_compare, &[], advance, &[]);
    b.switch_to_block(start_compare);
    b.def_var(equal_at, zero);
    b.ins().jump(compare, &[]);
    b.switch_to_block(compare);
    let at = b.use_var(equal_at);
    let length = b.use_var(key_bytes);
    if vectorized {
        // `at <= length` is maintained from zero and each checked advance.
        // Read a full vector only when both equal-length strings have 16 bytes.
        let remaining = b.ins().isub(length, at);
        let full = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 16);
        let batch = b.create_block();
        let scalar = b.create_block();
        b.ins().brif(full, batch, &[], scalar, &[]);
        b.switch_to_block(batch);
        let source = b.use_var(key_address);
        let source = b.ins().iadd(source, at);
        let slot = b.use_var(slot_address);
        let target = get(b, slot, offset_of!(Slot, address));
        let target = b.ins().iadd(target, at);
        let left = b.ins().load(types::I8X16, MemFlags::new(), source, 0);
        let right = b.ins().load(types::I8X16, MemFlags::new(), target, 0);
        let different = b.ins().icmp(IntCC::NotEqual, left, right);
        let any = b.ins().vany_true(different);
        let matched = b.create_block();
        b.ins().brif(any, advance, &[], matched, &[]);
        b.switch_to_block(matched);
        let next = b.ins().iadd_imm(at, 16);
        b.def_var(equal_at, next);
        b.ins().jump(compare, &[]);
        b.switch_to_block(scalar);
        // Short UTF8 keys and vector tails still admit bounded whole-word
        // equality. Unaligned loads are legal; neither string is read beyond
        // its proven equal byte extent. A mismatch advances the same probe.
        let widths: &[(i64, cranelift_codegen::ir::Type)] = if word_chunks {
            &[(8, types::I64), (4, types::I32), (2, types::I16)]
        } else {
            &[(8, types::I64), (4, types::I32)]
        };
        for &(width, ty) in widths {
            let at = b.use_var(equal_at);
            let remaining = b.ins().isub(length, at);
            let full = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, width);
            let batch = b.create_block();
            let tail = b.create_block();
            b.ins().brif(full, batch, &[], tail, &[]);
            b.switch_to_block(batch);
            let source = b.use_var(key_address);
            let source = b.ins().iadd(source, at);
            let slot = b.use_var(slot_address);
            let target = get(b, slot, offset_of!(Slot, address));
            let target = b.ins().iadd(target, at);
            let left = b.ins().load(ty, MemFlags::new(), source, 0);
            let right = b.ins().load(ty, MemFlags::new(), target, 0);
            let same = b.ins().icmp(IntCC::Equal, left, right);
            let matched = b.create_block();
            b.ins().brif(same, matched, &[], advance, &[]);
            b.switch_to_block(matched);
            let next = b.ins().iadd_imm(at, width);
            b.def_var(equal_at, next);
            b.ins().jump(tail, &[]);
            b.switch_to_block(tail);
        }
    }
    b.ins().jump(scalar_tail, &[]);
    b.switch_to_block(scalar_tail);
    let at = b.use_var(equal_at);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, length);
    b.ins().brif(more, compare_byte, &[], hit, &[]);
    b.switch_to_block(compare_byte);
    let source = b.use_var(key_address);
    let at = b.use_var(equal_at);
    let source = b.ins().iadd(source, at);
    let slot = b.use_var(slot_address);
    let target = get(b, slot, offset_of!(Slot, address));
    let target = b.ins().iadd(target, at);
    let left = b.ins().load(types::I8, MemFlags::new(), source, 0);
    let right = b.ins().load(types::I8, MemFlags::new(), target, 0);
    let same = b.ins().icmp(IntCC::Equal, left, right);
    let next_compare = b.create_block();
    b.ins().brif(same, next_compare, &[], advance, &[]);
    b.switch_to_block(next_compare);
    let at = b.ins().iadd_imm(at, 1);
    b.def_var(equal_at, at);
    b.ins().jump(scalar_tail, &[]);
    b.switch_to_block(advance);
    let at = b.use_var(probe_at);
    let at = b.ins().iadd_imm(at, 1);
    let at = b.ins().band(at, mask);
    b.def_var(probe_at, at);
    let left = b.use_var(probes);
    let left = b.ins().iadd_imm(left, -1);
    b.def_var(probes, left);
    b.ins().jump(probe, &[]);
    b.switch_to_block(miss);
    b.ins().jump(missing_key, &[]);
    b.switch_to_block(hit);
    let slot = b.use_var(slot_address);
    let value = get(b, slot, offset_of!(Slot, value));
    let at = b.use_var(cursor);
    let offset = b.ins().imul_imm(at, 8);
    let destination = b.ins().iadd(output, offset);
    put(b, destination, 0, value);
    b.ins().jump(commit, &[]);
    b.switch_to_block(commit);
    let at = b.use_var(cursor);
    let at = b.ins().iadd_imm(at, 1);
    b.def_var(cursor, at);
    b.ins().jump(header, &[]);
    for (block, target) in [(done, complete), (cold, declined), (missing_key, missing)] {
        b.switch_to_block(block);
        let f = b.use_var(fuel);
        let c = b.use_var(comparison);
        let at = b.use_var(cursor);
        put(b, frame, offset_of!(Frame, fuel), f);
        put(b, frame, offset_of!(Frame, comparison), c);
        put(b, frame, offset_of!(Frame, next), at);
        b.ins().jump(target, &[]);
    }
}

#[cfg(all(test, feature = "canonical-native-program", feature = "jit"))]
#[path = "utf8_lookup_tests.rs"]
mod tests;
