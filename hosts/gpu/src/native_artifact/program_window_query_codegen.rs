//! Generic exact byte-window matching within the caller's admitted String loans.
//! Short keys use a masked word scan plus a bounded byte tail; longer keys use
//! an eight-byte prefix before the ordinary remainder. No alphabet assumption.
use super::*;
use cranelift_codegen::ir::{Block, Endianness, Value};

pub(super) fn emit(b: &mut FunctionBuilder<'_>) {
    let entry = b.create_block();
    let empty = b.create_block();
    let select = b.create_block();
    let wide = b.create_block();
    let short = b.create_block();
    let pack = b.create_block();
    let packed = b.create_block();
    let bulk = b.create_block();
    let outer = b.create_block();
    let compare = b.create_block();
    let equal = b.create_block();
    let matched = b.create_block();
    let advance = b.create_block();
    let done = b.create_block();
    b.append_block_params_for_function_params(entry);
    for block in [pack, bulk, outer] {
        b.append_block_param(block, types::I64);
        b.append_block_param(block, types::I64);
    }
    for block in [packed, compare, advance, done] {
        b.append_block_param(block, types::I64);
    }
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let (source, length, query, query_length, width) =
        (args[0], args[1], args[2], args[3], args[4]);
    let zero = b.ins().iconst(types::I64, 0);
    let positive = b.ins().icmp_imm(IntCC::NotEqual, width, 0);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, width, length);
    let same_width = b.ins().icmp(IntCC::Equal, width, query_length);
    let valid = b.ins().band(positive, fits);
    let valid = b.ins().band(valid, same_width);
    b.ins().brif(valid, select, &[], empty, &[]);
    b.switch_to_block(empty);
    b.ins().return_(&[zero]);
    b.switch_to_block(select);
    let last = b.ins().isub(length, width);
    let large = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, width, 8);
    b.ins().brif(large, wide, &[], short, &[]);

    // width == query_length >= 8, offset <= length - width. Both complete
    // eight-byte reads are inside the actual immutable owners, even for an
    // unaligned final window. No aligned/can_move/notrap flags are asserted.
    b.switch_to_block(wide);
    emit_wide(b, source, query, width, last, zero, done);

    b.switch_to_block(short);
    let fits_word = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, 8);
    b.ins().brif(
        fits_word,
        pack,
        &[zero.into(), zero.into()],
        outer,
        &[zero.into(), zero.into()],
    );
    // Pack only the actual 1..7 query bytes, never a full-word query overread.
    // Explicit little-endian loads make the mask independent of target endian.
    b.switch_to_block(pack);
    let index = b.block_params(pack)[0];
    let key = b.block_params(pack)[1];
    let address = b.ins().iadd(query, index);
    let byte = b.ins().load(types::I8, MemFlags::new(), address, 0);
    let byte = b.ins().uextend(types::I64, byte);
    let shift = b.ins().ishl_imm(index, 3);
    let byte = b.ins().ishl(byte, shift);
    let key = b.ins().bor(key, byte);
    let next = b.ins().iadd_imm(index, 1);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, next, width);
    b.ins().brif(
        more,
        pack,
        &[next.into(), key.into()],
        packed,
        &[key.into()],
    );
    b.switch_to_block(packed);
    let key = b.block_params(packed)[0];
    let bits = b.ins().ishl_imm(width, 3);
    let shift = b.ins().irsub_imm(bits, 64);
    let all = b.ins().iconst(types::I64, -1);
    let mask = b.ins().ushr(all, shift);
    let last_word = b.ins().iadd_imm(length, -8);
    b.ins().jump(bulk, &[zero.into(), zero.into()]);
    b.switch_to_block(bulk);
    let offset = b.block_params(bulk)[0];
    let count = b.block_params(bulk)[1];
    let address = b.ins().iadd(source, offset);
    let word = b.ins().load(types::I64, little(), address, 0);
    let word = b.ins().band(word, mask);
    let same = b.ins().icmp(IntCC::Equal, word, key);
    let increment = b.ins().uextend(types::I64, same);
    let count = b.ins().iadd(count, increment);
    let next = b.ins().iadd_imm(offset, 1);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, last_word);
    b.ins().brif(
        more,
        bulk,
        &[next.into(), count.into()],
        outer,
        &[next.into(), count.into()],
    );

    // At most seven starts remain after the word scan, since width < 8.
    // For sources shorter than eight bytes this is the whole original loop.
    b.switch_to_block(outer);
    let offset = b.block_params(outer)[0];
    let count = b.block_params(outer)[1];
    b.ins().jump(compare, &[zero.into()]);
    emit_bytes(
        b, compare, equal, matched, advance, source, query, offset, count, width,
    );
    b.switch_to_block(advance);
    let count = b.block_params(advance)[0];
    advance_window(b, offset, count, last, outer, done);
    b.switch_to_block(done);
    let answer = b.block_params(done)[0];
    b.ins().return_(&[answer]);
}

fn little() -> MemFlags {
    MemFlags::new().with_endianness(Endianness::Little)
}

fn emit_wide(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    query: Value,
    width: Value,
    last: Value,
    zero: Value,
    done: Block,
) {
    let outer = b.create_block();
    let prefix = b.create_block();
    let compare = b.create_block();
    let equal = b.create_block();
    let matched = b.create_block();
    let advance = b.create_block();
    b.append_block_param(outer, types::I64);
    b.append_block_param(outer, types::I64);
    b.append_block_param(compare, types::I64);
    b.append_block_param(advance, types::I64);
    let key = b.ins().load(types::I64, little(), query, 0);
    let eight = b.ins().iconst(types::I64, 8);
    b.ins().jump(outer, &[zero.into(), zero.into()]);
    b.switch_to_block(outer);
    let offset = b.block_params(outer)[0];
    let count = b.block_params(outer)[1];
    let address = b.ins().iadd(source, offset);
    let word = b.ins().load(types::I64, little(), address, 0);
    let same = b.ins().icmp(IntCC::Equal, word, key);
    b.ins().brif(same, prefix, &[], advance, &[count.into()]);
    b.switch_to_block(prefix);
    let more = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, width, 8);
    b.ins().brif(more, compare, &[eight.into()], matched, &[]);
    emit_bytes(
        b, compare, equal, matched, advance, source, query, offset, count, width,
    );
    b.switch_to_block(advance);
    let count = b.block_params(advance)[0];
    advance_window(b, offset, count, last, outer, done);
}

// Every caller establishes 0 <= index < width and offset <= length - width.
// Thus offset + index < length and index < query_length, without overflow.
#[allow(clippy::too_many_arguments)]
fn emit_bytes(
    b: &mut FunctionBuilder<'_>,
    compare: Block,
    equal: Block,
    matched: Block,
    advance: Block,
    source: Value,
    query: Value,
    offset: Value,
    count: Value,
    width: Value,
) {
    b.switch_to_block(compare);
    let index = b.block_params(compare)[0];
    let position = b.ins().iadd(offset, index);
    let source_address = b.ins().iadd(source, position);
    let query_address = b.ins().iadd(query, index);
    let left = b.ins().load(types::I8, MemFlags::new(), source_address, 0);
    let right = b.ins().load(types::I8, MemFlags::new(), query_address, 0);
    let same = b.ins().icmp(IntCC::Equal, left, right);
    b.ins().brif(same, equal, &[], advance, &[count.into()]);
    b.switch_to_block(equal);
    let next = b.ins().iadd_imm(index, 1);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, next, width);
    b.ins().brif(more, compare, &[next.into()], matched, &[]);
    b.switch_to_block(matched);
    let incremented = b.ins().iadd_imm(count, 1);
    b.ins().jump(advance, &[incremented.into()]);
}

fn advance_window(
    b: &mut FunctionBuilder<'_>,
    offset: Value,
    count: Value,
    last: Value,
    outer: Block,
    done: Block,
) {
    let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, last);
    let next = b.ins().iadd_imm(offset, 1);
    b.ins().brif(
        more,
        outer,
        &[next.into(), count.into()],
        done,
        &[count.into()],
    );
}
