//! Exact-bit native transpose over runtime-admitted disjoint typed banks.
//! Two-by-two integer tiles keep both source loads and output stores contiguous.
use super::numeric_binary::address;
use cranelift_codegen::ir::{
    Block, ConstantData, Endianness, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;

fn pair(b: &mut FunctionBuilder<'_>, first: Value, second: Value) -> Value {
    let value = b.ins().splat(types::I64X2, first);
    b.ins().insertlane(value, second, 1)
}

// Use the same byte-shuffle representation as the maintained packed F64
// kernels. Keeping whole word lanes in vector registers permits native ZIPs
// without scalar extraction/reinsertion through general registers.
fn interleave(b: &mut FunctionBuilder<'_>, a: Value, c: Value, high: bool) -> Value {
    let first = if high { 8u8 } else { 0 };
    let mask = (first..first + 8)
        .chain(first + 16..first + 24)
        .collect::<Vec<_>>();
    let mask = b.func.dfg.immediates.push(ConstantData::from(mask));
    let flags = MemFlags::new().with_endianness(Endianness::Little);
    let a = b.ins().bitcast(types::I8X16, flags, a);
    let c = b.ins().bitcast(types::I8X16, flags, c);
    let value = b.ins().shuffle(a, c, mask);
    b.ins().bitcast(types::I64X2, flags, value)
}

/// One or two output rows. All accesses are inside their exact real-cell
/// rectangle; tails never read padding. The caller has validated the complete
/// geometry and disjoint spans before entering this nonfallible copy.
fn row_band(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    columns: Value,
    input_columns: Value,
    row: Value,
    two_rows: bool,
) {
    let source_base = address(b, input, row);
    let destination_row = b.ins().imul(row, columns);
    let destination_base = address(b, output, destination_row);
    let source_stride = b.ins().imul_imm(input_columns, 8);
    let destination_stride = b.ins().imul_imm(columns, 8);
    let column = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(column, zero);
    let header = b.create_block();
    let tile = b.create_block();
    let tail = b.create_block();
    let single = b.create_block();
    let done = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at = b.use_var(column);
    let remaining = b.ins().isub(columns, at);
    let two_columns = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    b.ins().brif(two_columns, tile, &[], tail, &[]);
    b.switch_to_block(tile);
    let source_offset = b.ins().imul(at, source_stride);
    let first = b.ins().iadd(source_base, source_offset);
    let second = b.ins().iadd(first, source_stride);
    let to = address(b, destination_base, at);
    if two_rows {
        let a = b.ins().load(types::I64X2, MemFlags::new(), first, 0);
        let c = b.ins().load(types::I64X2, MemFlags::new(), second, 0);
        let low = interleave(b, a, c, false);
        let high = interleave(b, a, c, true);
        b.ins().store(MemFlags::new(), low, to, 0);
        let next_row = b.ins().iadd(to, destination_stride);
        b.ins().store(MemFlags::new(), high, next_row, 0);
    } else {
        let a = b.ins().load(types::I64, MemFlags::new(), first, 0);
        let c = b.ins().load(types::I64, MemFlags::new(), second, 0);
        let value = pair(b, a, c);
        b.ins().store(MemFlags::new(), value, to, 0);
    }
    let next = b.ins().iadd_imm(at, 2);
    b.def_var(column, next);
    b.ins().jump(header, &[]);
    b.switch_to_block(tail);
    let at = b.use_var(column);
    let last = b.ins().icmp(IntCC::UnsignedLessThan, at, columns);
    b.ins().brif(last, single, &[], done, &[]);
    b.switch_to_block(single);
    let source_offset = b.ins().imul(at, source_stride);
    let from = b.ins().iadd(source_base, source_offset);
    let to = address(b, destination_base, at);
    if two_rows {
        let words = b.ins().load(types::I64X2, MemFlags::new(), from, 0);
        let first = b.ins().extractlane(words, 0);
        let second = b.ins().extractlane(words, 1);
        b.ins().store(MemFlags::new(), first, to, 0);
        let next_row = b.ins().iadd(to, destination_stride);
        b.ins().store(MemFlags::new(), second, next_row, 0);
    } else {
        let word = b.ins().load(types::I64, MemFlags::new(), from, 0);
        b.ins().store(MemFlags::new(), word, to, 0);
    }
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
}

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    count: Value,
    output_columns: Value,
    input_columns: Value,
    done: Block,
) {
    let start = b.create_block();
    let nonempty = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
    // Empty axes may be arbitrarily large and both pointers may be null.
    // Do not derive strides or enter a row loop until count is nonzero.
    b.ins().brif(nonempty, start, &[], done, &[]);
    b.switch_to_block(start);
    let row = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(row, zero);
    let header = b.create_block();
    let paired = b.create_block();
    let tail = b.create_block();
    let single = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at = b.use_var(row);
    let remaining = b.ins().isub(input_columns, at);
    let two = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    b.ins().brif(two, paired, &[], tail, &[]);
    b.switch_to_block(paired);
    row_band(b, input, output, output_columns, input_columns, at, true);
    let next = b.ins().iadd_imm(at, 2);
    b.def_var(row, next);
    b.ins().jump(header, &[]);
    b.switch_to_block(tail);
    let at = b.use_var(row);
    let last = b.ins().icmp(IntCC::UnsignedLessThan, at, input_columns);
    b.ins().brif(last, single, &[], done, &[]);
    b.switch_to_block(single);
    row_band(b, input, output, output_columns, input_columns, at, false);
    b.ins().jump(done, &[]);
}

/// Private emission ablation: the accepted ordinal transpose remains available
/// to differential tests without changing loan admission or source accounting.
pub(super) fn emit_ordinal(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    count: Value,
    output_columns: Value,
    input_columns: Value,
    done: Block,
) {
    let header = b.create_block();
    let body = b.create_block();
    b.append_block_param(header, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    // A nonempty admitted product guarantees nonzero axes. Empty matrices
    // take the header's done edge before any division, load or store.
    let row = b.ins().udiv(index, output_columns);
    let column = b.ins().urem(index, output_columns);
    let source = b.ins().imul(column, input_columns);
    let source = b.ins().iadd(source, row);
    let from = address(b, input, source);
    let to = address(b, output, index);
    let word = b.ins().load(types::I64, MemFlags::new(), from, 0);
    b.ins().store(MemFlags::new(), word, to, 0);
    let next = b.ins().iadd_imm(index, 1);
    b.ins().jump(header, &[next.into()]);
}

#[cfg(test)]
#[path = "numeric_transpose_tests.rs"]
mod tests;
