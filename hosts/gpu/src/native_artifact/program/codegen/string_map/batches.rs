//! Eight independent byte lookups within the existing admitted owner loans.
//! A missing entry leaves the entire batch unwritten; the original scalar
//! loop then writes its valid prefix and identifies the exact first failure.
use crate::native_artifact::program::codegen::string_map::*;
use cranelift_codegen::ir::{Endianness, Value};

pub(in crate::native_artifact::program::codegen::string_map) fn emit(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    length: Value,
    table: &ScalarTable,
    output: Value,
    start: Value,
) -> Value {
    let init = b.create_block();
    let batch = b.create_block();
    let write = b.create_block();
    let tail = b.create_block();
    for block in [batch, tail] {
        b.append_block_param(block, types::I64);
    }
    let zero = b.ins().iconst(types::I64, 0);
    let remaining = b.ins().isub(length, start);
    let fits = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 8);
    b.ins().brif(fits, init, &[], tail, &[start.into()]);
    b.switch_to_block(init);
    let last = b.ins().iadd_imm(length, -8);
    b.ins().jump(batch, &[start.into()]);
    b.switch_to_block(batch);
    let offset = b.block_params(batch)[0];
    let address = b.ins().iadd(source, offset);
    // offset <= length-8 proves this read and the eventual output store.
    // Explicit byte order is portable; no alignment/notrap/can_move promise.
    let little = MemFlags::new().with_endianness(Endianness::Little);
    let bytes = b.ins().load(types::I64, little, address, 0);
    let mut values = Vec::new();
    for i in 0..8 {
        let byte = b.ins().ushr_imm(bytes, i * 8);
        let byte = b.ins().band_imm(byte, 255);
        // The entry guard proved ASCII. Both prepared representations use
        // the same bounded lookup and the same missing/valid-zero contract.
        values.push(table.lookup(b, byte));
    }
    let mut combined = values[0];
    for &value in &values[1..] {
        combined = b.ins().bor(combined, value);
    }
    // Valid replacements are 0..127; ANY high bit marks a missing entry.
    // Inspecting later table cells is pure and cannot change failure order.
    let present = b.ins().icmp_imm(IntCC::UnsignedLessThan, combined, 128);
    b.ins().brif(present, write, &[], tail, &[offset.into()]);
    b.switch_to_block(write);
    let mut packed = zero;
    for (i, value) in values.into_iter().enumerate() {
        let value = b.ins().uextend(types::I64, value);
        let value = b.ins().ishl_imm(value, (i * 8) as i64);
        packed = b.ins().bor(packed, value);
    }
    let destination = b.ins().iadd(output, offset);
    b.ins().store(little, packed, destination, 0);
    let next = b.ins().iadd_imm(offset, 8);
    let more = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, last);
    b.ins()
        .brif(more, batch, &[next.into()], tail, &[next.into()]);
    b.switch_to_block(tail);
    b.block_params(tail)[0]
}
