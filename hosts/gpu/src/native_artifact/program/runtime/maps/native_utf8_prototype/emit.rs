//! Explicit emitted UTF8 hash/probe/equality loop. There are no call sites.
use super::*;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;

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

pub(super) fn emit(b: &mut FunctionBuilder<'_>, frame: Value) {
    let cold = b.create_block();
    let done = b.create_block();
    let header = b.create_block();
    let query = b.create_block();
    let hash_head = b.create_block();
    let hash_byte = b.create_block();
    let hash_done = b.create_block();
    let probe = b.create_block();
    let inspect = b.create_block();
    let compare = b.create_block();
    let compare_byte = b.create_block();
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
    let one = b.ins().iconst(types::I64, 1);
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
    let row = b.ins().urem(at, rows);
    let column = b.ins().udiv(at, rows);
    let base = b.ins().imul(row, columns);
    let physical = b.ins().iadd(base, column);
    let offset = b.ins().imul_imm(physical, size_of::<Cell>() as i64);
    let cell = b.ins().iadd(cells, offset);
    let address = get(b, cell, offset_of!(Cell, address));
    let length = get(b, cell, offset_of!(Cell, bytes));
    b.def_var(key_address, address);
    b.def_var(key_bytes, length);
    // Exact retained-map conservative price: bounds hashing and complete
    // collisions, never relying on probabilistic hash quality for admission.
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
    b.def_var(hash_at, zero);
    let seed = b.ins().iconst(types::I64, OFFSET as i64);
    b.def_var(hash_word, seed);
    b.ins().jump(hash_head, &[]);
    b.switch_to_block(hash_head);
    let at = b.use_var(hash_at);
    let length = b.use_var(key_bytes);
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
    let hash = b.ins().imul_imm(hash, PRIME as i64);
    b.def_var(hash_word, hash);
    let at = b.ins().iadd_imm(at, 1);
    b.def_var(hash_at, at);
    b.ins().jump(hash_head, &[]);
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
    b.ins().jump(compare, &[]);
    b.switch_to_block(advance);
    let at = b.use_var(probe_at);
    let at = b.ins().iadd_imm(at, 1);
    let at = b.ins().band(at, mask);
    b.def_var(probe_at, at);
    let left = b.use_var(probes);
    let left = b.ins().iadd_imm(left, -1);
    b.def_var(probes, left);
    b.ins().jump(probe, &[]);
    for (block, found) in [(hit, true), (miss, false)] {
        b.switch_to_block(block);
        let value = if found {
            let slot = b.use_var(slot_address);
            get(b, slot, offset_of!(Slot, value))
        } else {
            zero
        };
        let at = b.use_var(cursor);
        let offset = b.ins().imul_imm(at, size_of::<ResultCell>() as i64);
        let destination = b.ins().iadd(output, offset);
        put(b, destination, offset_of!(ResultCell, value), value);
        put(
            b,
            destination,
            offset_of!(ResultCell, found),
            if found { one } else { zero },
        );
        b.ins().jump(commit, &[]);
    }
    b.switch_to_block(commit);
    let at = b.use_var(cursor);
    let at = b.ins().iadd_imm(at, 1);
    b.def_var(cursor, at);
    b.ins().jump(header, &[]);
    for (block, status) in [(done, OK), (cold, COLD)] {
        b.switch_to_block(block);
        let f = b.use_var(fuel);
        let c = b.use_var(comparison);
        let at = b.use_var(cursor);
        put(b, frame, offset_of!(Frame, fuel), f);
        put(b, frame, offset_of!(Frame, comparison), c);
        put(b, frame, offset_of!(Frame, next), at);
        let status = b.ins().iconst(types::I64, status as i64);
        b.ins().return_(&[status]);
    }
}
