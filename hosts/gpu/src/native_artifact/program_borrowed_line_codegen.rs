//! Read one proved row from ordinary cells or a borrowed compact-offset loan.
//! The owning runtime supplies the discriminator; no source value can do so.
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

pub(super) fn load(b: &mut FunctionBuilder, table: Value, index: Value) -> [Value; 2] {
    let cells = b.create_block();
    let chunks = b.create_block();
    let offsets = b.create_block();
    let single = b.create_block();
    let joined = b.create_block();
    b.append_block_param(joined, types::I64);
    b.append_block_param(joined, types::I64);
    let tag = b.ins().band_imm(table, 1);
    let direct = b.ins().icmp_imm(IntCC::NotEqual, tag, 0);
    b.ins().brif(direct, chunks, &[], cells, &[]);
    b.switch_to_block(cells);
    let byte_offset = b.ins().ishl_imm(index, 4);
    let row = b.ins().iadd(table, byte_offset);
    let pointer = b.ins().load(types::I64, MemFlags::new(), row, 0);
    let length = b.ins().load(types::I64, MemFlags::new(), row, 8);
    b.ins().jump(joined, &[pointer.into(), length.into()]);

    b.switch_to_block(chunks);
    let first = b.ins().band_imm(table, -2);
    let first_count = b.ins().load(types::I64, MemFlags::new(), first, 32);
    let in_first = b.ins().icmp(IntCC::UnsignedLessThan, index, first_count);
    let second = b.ins().iadd_imm(first, 40);
    let segment = b.ins().select(in_first, first, second);
    let second_index = b.ins().isub(index, first_count);
    let row = b.ins().select(in_first, index, second_index);
    let base = b.ins().load(types::I64, MemFlags::new(), segment, 0);
    let length = b.ins().load(types::I64, MemFlags::new(), segment, 8);
    let lf = b.ins().load(types::I64, MemFlags::new(), segment, 16);
    let first_index = b.ins().load(types::I64, MemFlags::new(), segment, 24);
    let packed = b.ins().icmp_imm(IntCC::NotEqual, lf, 0);
    b.ins().brif(packed, offsets, &[], single, &[]);
    b.switch_to_block(single);
    b.ins().jump(joined, &[base.into(), length.into()]);

    b.switch_to_block(offsets);
    let row = b.ins().iadd(row, first_index);
    let row_bytes = b.ins().ishl_imm(row, 2);
    let end_at = b.ins().iadd(lf, row_bytes);
    let end = b.ins().load(types::I32, MemFlags::new(), end_at, 0);
    let end = b.ins().uextend(types::I64, end);
    // Saturate the preceding index at zero: even the unselected load remains
    // inside the real LF array. Row zero then selects its actual byte start 0.
    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, row, 0);
    let previous = b.ins().iadd_imm(row, -1);
    let zero = b.ins().iconst(types::I64, 0);
    let previous = b.ins().select(nonzero, previous, zero);
    let previous_bytes = b.ins().ishl_imm(previous, 2);
    let previous_at = b.ins().iadd(lf, previous_bytes);
    let previous_end = b.ins().load(types::I32, MemFlags::new(), previous_at, 0);
    let previous_end = b.ins().uextend(types::I64, previous_end);
    let next = b.ins().iadd_imm(previous_end, 1);
    let start = b.ins().select(nonzero, next, zero);
    // Empty cells load their own LF, which is in bounds; nonempty cells load
    // the preceding byte. Strip exactly one CR only in the latter case.
    let nonempty = b.ins().icmp(IntCC::UnsignedGreaterThan, end, start);
    let preceding = b.ins().iadd_imm(end, -1);
    let inspected = b.ins().select(nonempty, preceding, end);
    let inspected_at = b.ins().iadd(base, inspected);
    let byte = b.ins().load(types::I8, MemFlags::new(), inspected_at, 0);
    let cr = b.ins().icmp_imm(IntCC::Equal, byte, 13);
    let trim = b.ins().band(nonempty, cr);
    let trim = b.ins().uextend(types::I64, trim);
    let end = b.ins().isub(end, trim);
    let pointer = b.ins().iadd(base, start);
    let length = b.ins().isub(end, start);
    b.ins().jump(joined, &[pointer.into(), length.into()]);
    b.switch_to_block(joined);
    [b.block_params(joined)[0], b.block_params(joined)[1]]
}
