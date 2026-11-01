//! Generic 128-entry ASCII map, in bounded 16-byte batches on AArch64.
//! The same entry guard supplies the actual initialized u16 table and source/
//! output owners. Source and output may coincide: the full batch is loaded
//! before any store. A missing entry leaves the batch to the scalar fallback.
use super::*;
use cranelift_codegen::ir::Value;

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    length: Value,
    table: Value,
    output: Value,
) -> Value {
    let init = b.create_block();
    let scan = b.create_block();
    let write = b.create_block();
    let tail = b.create_block();
    b.append_block_param(scan, types::I64);
    b.append_block_param(tail, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    let fits = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, 16);
    b.ins().brif(fits, init, &[], tail, &[zero.into()]);
    b.switch_to_block(init);
    let last = b.ins().iadd_imm(length, -16);
    let mut tables = Vec::new();
    let mut offsets = Vec::new();
    for index in 0..8 {
        // Each pair covers exactly 16 of the existing 128 u16 cells.
        let low = b
            .ins()
            .load(types::I16X8, MemFlags::new(), table, index * 32);
        let high = b
            .ins()
            .load(types::I16X8, MemFlags::new(), table, index * 32 + 16);
        // Unsigned saturation preserves 0..127 exactly, and maps EVERY
        // missing marker (128..65535) to a byte whose high bit is set.
        tables.push(b.ins().uunarrow(low, high));
        let offset = b.ins().iconst(types::I8, (index * 16) as i64);
        offsets.push(b.ins().splat(types::I8X16, offset));
    }
    let high_bit = b.ins().iconst(types::I8, 128);
    let high_bit = b.ins().splat(types::I8X16, high_bit);
    b.ins().jump(scan, &[zero.into()]);
    b.switch_to_block(scan);
    let offset = b.block_params(scan)[0];
    let address = b.ins().iadd(source, offset);
    let keys = b.ins().load(types::I8X16, MemFlags::new(), address, 0);
    let mut mapped = b.ins().swizzle(tables[0], keys);
    for index in 1..8 {
        // For an admitted ASCII key, exactly one block index is in 0..15.
        // Swizzle zeroes every other block, including wrapping subtraction.
        let indices = b.ins().isub(keys, offsets[index]);
        let part = b.ins().swizzle(tables[index], indices);
        mapped = b.ins().bor(mapped, part);
    }
    let invalid = b.ins().band(mapped, high_bit);
    let missing = b.ins().vany_true(invalid);
    b.ins().brif(missing, tail, &[offset.into()], write, &[]);
    b.switch_to_block(write);
    let address = b.ins().iadd(output, offset);
    b.ins().store(MemFlags::new(), mapped, address, 0);
    let next = b.ins().iadd_imm(offset, 16);
    let more = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, last);
    b.ins()
        .brif(more, scan, &[next.into()], tail, &[next.into()]);
    b.switch_to_block(tail);
    b.block_params(tail)[0]
}
