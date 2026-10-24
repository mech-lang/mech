//! Generated storage operations over an invocation-owned explicit table layout.
//! Source control flow and replacement arithmetic remain in the original CFG.
use super::super::{NativeCompileError, compiler::offset, program_hooks::Hooks,
    program_packed_map::{Frame, Slot, GROUP, HASH_FIRST, HASH_SECOND, narrow, slot_bytes, value_offset}, program_key_domain::Domain};
use cranelift_codegen::ir::{Block, Endianness, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

#[path = "program_packed_frame_codegen.rs"]
mod frame_state;
pub(in crate::native_artifact) use frame_state::{FrameState, effects_enabled};
#[cfg(test)]
pub(in crate::native_artifact) use frame_state::{take_frame_effect_emissions, without_frame_effects};

#[cfg(test)]
#[path = "program_packed_hash_tests.rs"]
mod hash_tests;

/// Compile-time facts rederived from the source CFG for this selected key.
/// Memory-mode callers and unsupported graphs always use Unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub(in crate::native_artifact) enum LookupFact { Unknown, Fresh, Cached, Present, Absent }
impl LookupFact {
    pub(super) fn name(self) -> &'static str { match self {
        Self::Unknown => "Unknown", Self::Fresh => "Fresh", Self::Cached => "Cached",
        Self::Present => "Present", Self::Absent => "Absent",
    }}
}

fn hash_words(b: &mut FunctionBuilder<'_>, low: Value, high: Value) -> Value {
    let left = b.ins().bxor_imm(low, HASH_FIRST as i64);
    let right = b.ins().bxor_imm(high, HASH_SECOND as i64);
    let product_low = b.ins().imul(left, right);
    let product_high = b.ins().umulhi(left, right);
    b.ins().bxor(product_low, product_high)
}

fn get(b: &mut FunctionBuilder<'_>, frame: Value, at: usize) -> Value {
    b.ins().load(types::I64, MemFlags::new(), frame, at as i32)
}
fn put(b: &mut FunctionBuilder<'_>, frame: Value, at: usize, value: Value) {
    b.ins().store(MemFlags::new(), value, frame, at as i32);
}
fn encode(b: &mut FunctionBuilder<'_>, byte: Value, domain: Domain) -> Value {
    let shifted = b.ins().ushr_imm(byte, i64::from(domain.shift().expect("packed bit window")));
    b.ins().band_imm(shifted, (1i64 << domain.bits()) - 1)
}
macro_rules! read {
    ($b:expr, $f:expr, $field:ident) => { $f.read($b, core::mem::offset_of!(Frame, $field)) };
}
macro_rules! write {
    ($b:expr, $f:expr, $field:ident, $v:expr) => { $f.write($b, core::mem::offset_of!(Frame, $field), $v) };
}

fn fail(b: &mut FunctionBuilder<'_>, frame: &FrameState, hooks: &Hooks, function: Value, context: Value,
    site: usize, op: u64, error: Block)
{
    let zero = b.ins().iconst(types::I64, 0);
    frame.call(b, hooks, function, context, site, op, [zero; 3], error);
    // The private failure operation always returns an error. Keep malformed
    // host behavior from resuming an unchecked native memory operation.
    super::super::compiler::guard(b, zero, error, super::super::abi::STATUS_ABI);
}

// A batch prepays root work, but an individual call limit applies only when
// the source reaches that call. The owner refreshes flags on every new key;
// ordinary calls need only this branch, with diagnostics on the failure path.
fn check_call(b: &mut FunctionBuilder<'_>, frame: &FrameState, allowed: Value, hooks: &Hooks,
    function: Value, context: Value, site: usize, op: u64, error: Block)
{
    let accepted = b.create_block();
    let denied = b.create_block();
    b.set_cold_block(denied);
    b.ins().brif(allowed, accepted, &[], denied, &[]);
    b.switch_to_block(denied);
    fail(b, frame, hooks, function, context, site, op, error);
    b.ins().jump(accepted, &[]);
    b.switch_to_block(accepted);
}

pub(super) fn select(b: &mut FunctionBuilder<'_>, frame: Value, domain: Domain, start: Value, end: Value,
    hooks: &Hooks, function: Value, context: Value, site: usize, error: Block)
{
    select_state(b, &FrameState::memory(frame, domain), start, end, hooks, function, context, site, error)
}

pub(super) fn select_state(b: &mut FunctionBuilder<'_>, frame: &FrameState, start: Value, end: Value,
    hooks: &Hooks, function: Value, context: Value, site: usize, error: Block)
{
    select_state_with_bounds(b, frame, start, end, hooks, function, context, site, error, false)
}

// Only the compiler's source-loop certificate selects `proved`. The original
// range call cap still runs at its source site, before any selected byte read.
pub(super) fn select_state_with_bounds(b: &mut FunctionBuilder<'_>, frame: &FrameState, start: Value, end: Value,
    hooks: &Hooks, function: Value, context: Value, site: usize, error: Block, proved: bool)
{
    select_state_proved(b, frame, start, end, hooks, function, context, site, error,
        SelectionProof { bounds: proved, ..Default::default() })
}

#[derive(Clone, Copy, Default)]
pub(super) struct SelectionProof { pub bounds: bool, pub calls: bool, pub adjacent: bool }

pub(super) fn select_state_proved(b: &mut FunctionBuilder<'_>, frame: &FrameState, start: Value, end: Value,
    hooks: &Hooks, function: Value, context: Value, site: usize, error: Block, proof: SelectionProof)
{
    let bits = i64::from(frame.domain.bits());
    let narrow = narrow(frame.domain);
    if !proof.calls {
        let allowed = read!(b, frame, range_call_allowed);
        check_call(b, frame, allowed, hooks, function, context, site, 10, error);
    }
    let zero = b.ins().iconst(types::I64, 0);
    let width = read!(b, frame, width);
    let source = read!(b, frame, source_ptr);
    let length = read!(b, frame, source_len);
    let begin = b.ins().iadd_imm(start, -1);
    let finish = if proof.bounds { b.ins().iadd(begin, width) } else {
        let (finish, overflow) = b.ins().uadd_overflow(begin, width);
        let positive = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, start, 0);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, finish, length);
        let no_overflow = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        let valid = b.ins().band(positive, fits);
        let valid = b.ins().band(valid, no_overflow);
        let checked = b.create_block();
        let bad = b.create_block();
        b.ins().brif(valid, checked, &[], bad, &[]);
        b.switch_to_block(bad);
        // Delegate only failure formatting to the maintained range/selection,
        // retaining its original origin and precise semantic error.
        frame.call(b, hooks, function, context, site, 6, [start, end, zero], error);
        fail(b, frame, hooks, function, context, site, 9, error);
        b.ins().jump(checked, &[]);
        b.switch_to_block(checked);
        finish
    };
    let rolling = if proof.adjacent {
        b.ins().iconst(types::I8, 1)
    } else {
        let previous = read!(b, frame, previous_start);
        let next = b.ins().iadd_imm(previous, 1);
        let adjacent = b.ins().icmp(IntCC::Equal, begin, next);
        let initialized = b.ins().icmp_imm(IntCC::NotEqual, previous, -1);
        b.ins().band(adjacent, initialized)
    };
    let reuse = b.create_block();
    let fresh = b.create_block();
    let loop_head = b.create_block();
    let loop_body = b.create_block();
    let done = b.create_block();
    for _ in 0..3 { b.append_block_param(loop_head, types::I64); }
    for _ in 0..2 { b.append_block_param(done, types::I64); }
    b.ins().brif(rolling, reuse, &[], fresh, &[]);
    b.switch_to_block(reuse);
    let old_low = read!(b, frame, key_low);
    let last = b.ins().iadd_imm(finish, -1);
    let address = b.ins().iadd(source, last);
    let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
    let byte = b.ins().uextend(types::I64, byte);
    let byte = encode(b, byte, frame.domain);
    let low = b.ins().ishl_imm(old_low, bits);
    let low = b.ins().bor(low, byte);
    let high = if narrow { zero } else {
        let old_high = read!(b, frame, key_high);
        let carry = b.ins().ushr_imm(old_low, 64 - bits);
        let high = b.ins().ishl_imm(old_high, bits);
        let high = b.ins().bor(high, carry);
        let high_mask = read!(b, frame, high_mask);
        b.ins().band(high, high_mask)
    };
    let low_mask = read!(b, frame, low_mask);
    let low = b.ins().band(low, low_mask);
    b.ins().jump(done, &[low.into(), high.into()]);
    b.switch_to_block(fresh);
    b.ins().jump(loop_head, &[zero.into(), zero.into(), zero.into()]);
    b.switch_to_block(loop_head);
    let args = b.block_params(loop_head).to_vec();
    let (index, low, high) = (args[0], args[1], args[2]);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, width);
    b.ins().brif(more, loop_body, &[], done, &[low.into(), high.into()]);
    b.switch_to_block(loop_body);
    let position = b.ins().iadd(begin, index);
    let address = b.ins().iadd(source, position);
    let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
    let byte = b.ins().uextend(types::I64, byte);
    let byte = encode(b, byte, frame.domain);
    let next_low = b.ins().ishl_imm(low, bits);
    let next_low = b.ins().bor(next_low, byte);
    let next_high = if narrow { zero } else {
        let carry = b.ins().ushr_imm(low, 64 - bits);
        let high = b.ins().ishl_imm(high, bits);
        b.ins().bor(high, carry)
    };
    let index = b.ins().iadd_imm(index, 1);
    b.ins().jump(loop_head, &[index.into(), next_low.into(), next_high.into()]);
    b.switch_to_block(done);
    let keys = b.block_params(done).to_vec();
    write!(b, frame, key_low, keys[0]);
    write!(b, frame, key_high, keys[1]);
    write!(b, frame, previous_start, begin);
    write!(b, frame, entry_known, zero);
    write!(b, frame, entry_pointer, zero);
}

/// Match all eight controls before touching payload slots. Return one low bit
/// per matching byte and the high bits of empty bytes; trailing-zero count / 8
/// selects the same lane order as the runtime's linear group probe.
fn control_group(b: &mut FunctionBuilder<'_>, address: Value, tag: Value) -> (Value, Value) {
    let flags = MemFlags::new().with_endianness(Endianness::Little);
    let byte = b.ins().ireduce(types::I8, tag);
    let (raw, matches) = if cfg!(target_arch = "aarch64") {
        // Read exactly the admitted eight-byte group into its vector class.
        // The old integer load required a GPR-to-SIMD transfer before cmeq.
        let group = b.ins().load(types::I8X8, flags, address, 0);
        let raw = b.ins().bitcast(types::I64, flags, group);
        let tags = b.ins().splat(types::I8X8, byte);
        let equal = b.ins().icmp(IntCC::Equal, group, tags);
        (raw, b.ins().bitcast(types::I64, flags, equal))
    } else {
        // x86 has 128-bit SIMD registers. Duplicate the already bounded
        // eight-byte load; no extra memory beyond the admitted group is read.
        let raw = b.ins().load(types::I64, flags, address, 0);
        let group = b.ins().splat(types::I64X2, raw);
        let group = b.ins().bitcast(types::I8X16, flags, group);
        let tags = b.ins().splat(types::I8X16, byte);
        let equal = b.ins().icmp(IntCC::Equal, group, tags);
        let words = b.ins().bitcast(types::I64X2, flags, equal);
        (raw, b.ins().extractlane(words, 0))
    };
    let matches = b.ins().band_imm(matches, 0x0101010101010101);
    // Occupied controls are seven-bit hashes; EMPTY is exactly 0x80.
    let empty = b.ins().band_imm(raw, 0x8080808080808080u64 as i64);
    (matches, empty)
}

fn probe(b: &mut FunctionBuilder<'_>, frame: &FrameState, hooks: &Hooks,
    function: Value, context: Value, site: usize, error: Block, fact: LookupFact, nonempty_proved: bool) -> Value
{
    match fact {
        LookupFact::Cached | LookupFact::Present => read!(b, frame, entry_pointer),
        LookupFact::Absent => b.ins().iconst(types::I64, 0),
        LookupFact::Fresh => probe_fresh(b, frame, hooks, function, context, site, error, nonempty_proved),
        LookupFact::Unknown => {
            let cached = b.create_block(); let initial = b.create_block(); let done = b.create_block();
            b.append_block_param(done, types::I64);
            let known = read!(b, frame, entry_known);
            b.ins().brif(known, cached, &[], initial, &[]);
            b.switch_to_block(cached);
            let entry = read!(b, frame, entry_pointer);
            b.ins().jump(done, &[entry.into()]);
            b.switch_to_block(initial);
            let entry = probe_fresh(b, frame, hooks, function, context, site, error, nonempty_proved);
            b.ins().jump(done, &[entry.into()]);
            b.switch_to_block(done);
            b.block_params(done)[0]
        }
    }
}

fn probe_fresh(b: &mut FunctionBuilder<'_>, frame: &FrameState, hooks: &Hooks,
    function: Value, context: Value, site: usize, error: Block, nonempty_proved: bool) -> Value
{
    let narrow = narrow(frame.domain);
    let zero = b.ins().iconst(types::I64, 0);
    let one = b.ins().iconst(types::I64, 1);
    let setup = b.create_block();
    let head = b.create_block();
    let candidates = b.create_block();
    let mismatch = b.create_block();
    let empties = b.create_block();
    let advance = b.create_block();
    let found = b.create_block();
    let absent = b.create_block();
    let bad = b.create_block();
    let done = b.create_block();
    b.append_block_param(head, types::I64);
    b.append_block_param(head, types::I64);
    b.append_block_param(candidates, types::I64);
    b.append_block_param(found, types::I64);
    b.append_block_param(done, types::I64);
    let capacity = read!(b, frame, capacity);
    if nonempty_proved {
        // Only a source-certified completed insertion recurrence establishes
        // this fact. Initial, checked and unknown callers keep the branch.
        b.ins().jump(setup, &[]);
    } else {
        let nonempty = b.ins().icmp_imm(IntCC::NotEqual, capacity, 0);
        b.ins().brif(nonempty, setup, &[], absent, &[]);
    }
    b.switch_to_block(setup);
    let low = read!(b, frame, key_low);
    let high = if narrow { zero } else { read!(b, frame, key_high) };
    let hash = if let Some(first) = frame.hash_constant(b, 0) {
        let second = frame.hash_constant(b, 1).expect("same immutable private packet");
        let left = b.ins().bxor(low, first);
        let right = b.ins().bxor(high, second);
        let product_low = b.ins().imul(left, right);
        let product_high = b.ins().umulhi(left, right);
        b.ins().bxor(product_low, product_high)
    } else { hash_words(b, low, high) };
    let mask = b.ins().iadd_imm(capacity, -1);
    let index = b.ins().band(hash, mask);
    let tag = b.ins().ushr_imm(hash, 57);
    let slots = read!(b, frame, slots);
    let controls = read!(b, frame, controls);
    b.ins().jump(head, &[index.into(), zero.into()]);
    b.switch_to_block(head);
    let args = b.block_params(head).to_vec();
    let (index, inspected) = (args[0], args[1]);
    let group_address = b.ins().iadd(controls, index);
    let (matching, empty) = control_group(b, group_address, tag);
    b.ins().brif(matching, candidates, &[matching.into()], empties, &[]);
    b.switch_to_block(candidates);
    let remaining = b.block_params(candidates)[0];
    let bit = b.ins().ctz(remaining);
    let lane = b.ins().ushr_imm(bit, 3);
    let bucket = b.ins().iadd(index, lane);
    let bucket = b.ins().band(bucket, mask);
    let address = b.ins().imul_imm(bucket, slot_bytes(frame.domain) as i64);
    let address = b.ins().iadd(slots, address);
    let stored_low = get(b, address, core::mem::offset_of!(Slot, low));
    let same_low = b.ins().icmp(IntCC::Equal, low, stored_low);
    let same = if narrow { same_low } else {
        let stored_high = get(b, address, core::mem::offset_of!(Slot, high));
        let same_high = b.ins().icmp(IntCC::Equal, high, stored_high);
        b.ins().band(same_low, same_high)
    };
    b.ins().brif(same, found, &[address.into()], mismatch, &[]);
    b.switch_to_block(mismatch);
    let cleared = b.ins().iadd_imm(remaining, -1);
    let remaining = b.ins().band(remaining, cleared);
    b.ins().brif(remaining, candidates, &[remaining.into()], empties, &[]);
    b.switch_to_block(empties);
    b.ins().brif(empty, absent, &[], advance, &[]);
    b.switch_to_block(advance);
    let inspected = b.ins().iadd_imm(inspected, GROUP as i64);
    let bounded = b.ins().icmp(IntCC::UnsignedLessThan, inspected, capacity);
    let index = b.ins().iadd_imm(index, GROUP as i64);
    let index = b.ins().band(index, mask);
    b.ins().brif(bounded, head, &[index.into(), inspected.into()], bad, &[]);
    b.switch_to_block(bad);
    fail(b, frame, hooks, function, context, site, 9, error);
    b.ins().jump(absent, &[]);
    b.switch_to_block(absent);
    write!(b, frame, entry_pointer, zero);
    write!(b, frame, entry_known, one);
    b.ins().jump(done, &[zero.into()]);
    b.switch_to_block(found);
    let entry = b.block_params(found)[0];
    write!(b, frame, entry_pointer, entry);
    write!(b, frame, entry_known, one);
    b.ins().jump(done, &[entry.into()]);
    b.switch_to_block(done);
    b.block_params(done)[0]
}

pub(super) fn operation(b: &mut FunctionBuilder<'_>, frame: Value, domain: Domain, op: u64, replacement: Value,
    locals: Value, output: usize, token: Value, hooks: &Hooks, function: Value,
    context: Value, site: usize, error: Block) -> Result<(), NativeCompileError>
{
    let value = operation_state(b, &FrameState::memory(frame, domain), op, replacement,
        token, hooks, function, context, site, error);
    b.ins().store(MemFlags::new(), value, locals, offset(output)?);
    Ok(())
}

pub(super) fn operation_state(b: &mut FunctionBuilder<'_>, frame: &FrameState, op: u64, replacement: Value,
    token: Value, hooks: &Hooks, function: Value, context: Value, site: usize, error: Block) -> Value
{
    operation_state_with_lookup(b, frame, op, replacement, token, hooks, function, context, site, error, LookupFact::Unknown)
}

pub(super) fn operation_state_with_lookup(b: &mut FunctionBuilder<'_>, frame: &FrameState, op: u64, replacement: Value,
    token: Value, hooks: &Hooks, function: Value, context: Value, site: usize, error: Block, fact: LookupFact) -> Value
{
    operation_state_proved(b, frame, op, replacement, token, hooks, function, context, site, error, fact, false, false, false, None)
}

pub(super) fn operation_state_proved(b: &mut FunctionBuilder<'_>, frame: &FrameState, op: u64, replacement: Value,
    token: Value, hooks: &Hooks, function: Value, context: Value, site: usize, error: Block, fact: LookupFact,
    calls_proved: bool, progress_proved: bool, nonempty_proved: bool, refresh: Option<Block>) -> Value
{
    if !calls_proved {
        let allowed = if op == 5 { read!(b, frame, write_call_allowed) }
            else { read!(b, frame, read_call_allowed) };
        check_call(b, frame, allowed, hooks, function, context, site, if op == 5 { 12 } else { 11 }, error);
    }
    let entry = probe(b, frame, hooks, function, context, site, error, fact, nonempty_proved);
    let present = match fact {
        LookupFact::Present => b.ins().iconst(types::I8, 1),
        LookupFact::Absent => b.ins().iconst(types::I8, 0),
        _ => b.ins().icmp_imm(IntCC::NotEqual, entry, 0),
    };
    let value = if op == 3 {
        b.ins().uextend(types::I64, present)
    } else {
        let hit = b.create_block();
        let miss = b.create_block();
        let done = b.create_block();
        b.append_block_param(done, types::I64);
        b.ins().brif(present, hit, &[], miss, &[]);
        b.switch_to_block(miss);
        if op == 4 {
            fail(b, frame, hooks, function, context, site, 8, error);
            b.ins().jump(done, &[token.into()]);
        } else {
            let zero = b.ins().iconst(types::I64, 0);
            frame.call(b, hooks, function, context, site, 7, [zero, zero, replacement], error);
            // Re-enter the exact source successor after EVERY insertion that
            // changes storage, including a non-growing insertion. Its refreshed
            // call flags may have changed even when its backing pointers did not.
            b.ins().jump(refresh.unwrap_or(done), &[token.into()]);
        }
        b.switch_to_block(hit);
        let value = if op == 4 {
            get(b, entry, value_offset(frame.domain))
        } else {
            let updates = read!(b, frame, updates);
            if !progress_proved {
                let fits = b.ins().icmp_imm(IntCC::NotEqual, updates, -1);
                let accepted = b.create_block();
                let bad = b.create_block();
                b.ins().brif(fits, accepted, &[], bad, &[]);
                b.switch_to_block(bad);
                fail(b, frame, hooks, function, context, site, 9, error);
                b.ins().jump(accepted, &[]);
                b.switch_to_block(accepted);
            }
            put(b, entry, value_offset(frame.domain), replacement);
            let updates = b.ins().iadd_imm(updates, 1);
            write!(b, frame, updates, updates);
            token
        };
        b.ins().jump(done, &[value.into()]);
        b.switch_to_block(done);
        b.block_params(done)[0]
    };
    value
}
