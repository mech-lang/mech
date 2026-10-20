//! Native dense counting for one/two-grapheme windows of a proved ASCII source.
//! Scratch and source loans are admitted before entry; no callbacks in the loop.
use super::*;
use cranelift_codegen::ir::Value;

pub(super) fn emit(b: &mut FunctionBuilder<'_>) {
    let entry = b.create_block();
    let valid = b.create_block();
    let empty = b.create_block();
    let one = b.create_block();
    let pair = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let (source, length, bins, width) = (args[0], args[1], args[2], args[4]);
    let positive = b.ins().icmp_imm(IntCC::NotEqual, width, 0);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, width, length);
    let nonempty = b.ins().band(positive, fits);
    b.ins().brif(nonempty, valid, &[], empty, &[]);
    b.switch_to_block(empty);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[zero]);
    b.switch_to_block(valid);
    let last = b.ins().isub(length, width);
    let accepted = b.ins().iadd_imm(last, 1);
    // Width is invariant across this entire already-admitted region. Select
    // its instruction sequence once, not at every source-window start.
    let single = b.ins().icmp_imm(IntCC::Equal, width, 1);
    b.ins().brif(single, one, &[], pair, &[]);
    b.switch_to_block(one);
    emit_width(b, source, bins, accepted, 1);
    b.switch_to_block(pair);
    emit_width(b, source, bins, accepted, 2);
}

fn emit_width(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    bins: Value,
    accepted: Value,
    width: usize,
) {
    let bulk = b.create_block();
    let tail_check = b.create_block();
    let tail = b.create_block();
    let done = b.create_block();
    for block in [bulk, tail_check, tail] {
        b.append_block_param(block, types::I64);
    }
    let zero = b.ins().iconst(types::I64, 0);
    // This subtraction may wrap on a short input, but its value is only used
    // in bulk, whose entry proves accepted >= 8. No load uses the wrapped bound.
    let bulk_last = b.ins().iadd_imm(accepted, -8);
    let has_bulk = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, accepted, 8);
    b.ins()
        .brif(has_bulk, bulk, &[zero.into()], tail_check, &[zero.into()]);
    b.switch_to_block(bulk);
    let offset = b.block_params(bulk)[0];
    let address = b.ins().iadd(source, offset);
    // Eight legal starts need 8 or 9 source bytes. offset <= accepted-8 proves
    // the LAST load is within the actual source. Reuse overlapping bytes and
    // compute keys before touching disjoint scratch; no endian/alignment claim.
    let bytes = (0..8 + width - 1)
        .map(|i| load_byte(b, address, i as i32))
        .collect::<Vec<_>>();
    let keys = (0..8)
        .map(|i| key(b, &bytes[i..i + width]))
        .collect::<Vec<_>>();
    // Repeated keys MUST see preceding updates. Do not load all old counts
    // first: that would lose increments on homogeneous/overlapping windows.
    for key in keys {
        increment(b, bins, key);
    }
    let next = b.ins().iadd_imm(offset, 8);
    let more = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, next, bulk_last);
    b.ins()
        .brif(more, bulk, &[next.into()], tail_check, &[next.into()]);
    b.switch_to_block(tail_check);
    let offset = b.block_params(tail_check)[0];
    let remains = b.ins().icmp(IntCC::UnsignedLessThan, offset, accepted);
    b.ins().brif(remains, tail, &[offset.into()], done, &[]);
    b.switch_to_block(tail);
    let offset = b.block_params(tail)[0];
    let address = b.ins().iadd(source, offset);
    let bytes = (0..width)
        .map(|i| load_byte(b, address, i as i32))
        .collect::<Vec<_>>();
    let index = key(b, &bytes);
    increment(b, bins, index);
    let next = b.ins().iadd_imm(offset, 1);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, next, accepted);
    b.ins().brif(more, tail, &[next.into()], done, &[]);
    b.switch_to_block(done);
    b.ins().return_(&[accepted]);
}

fn load_byte(b: &mut FunctionBuilder<'_>, address: Value, offset: i32) -> Value {
    let byte = b.ins().load(types::I8, MemFlags::new(), address, offset);
    b.ins().uextend(types::I64, byte)
}

fn key(b: &mut FunctionBuilder<'_>, bytes: &[Value]) -> Value {
    if bytes.len() == 1 {
        bytes[0]
    } else {
        let high = b.ins().ishl_imm(bytes[0], 7);
        b.ins().bor(high, bytes[1])
    }
}

fn increment(b: &mut FunctionBuilder<'_>, bins: Value, key: Value) {
    let byte_offset = b.ins().ishl_imm(key, 3);
    let cell = b.ins().iadd(bins, byte_offset);
    // Width is 1/2, source bytes <= 127, bins has 128^width admitted zeroed cells.
    // Each cell <= accepted <= length, so the increment cannot overflow.
    let old = b.ins().load(types::I64, MemFlags::new(), cell, 0);
    let count = b.ins().iadd_imm(old, 1);
    b.ins().store(MemFlags::new(), count, cell, 0);
}
