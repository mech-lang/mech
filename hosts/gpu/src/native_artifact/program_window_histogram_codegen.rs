//! Native dense counting for one/two-grapheme windows of a proved ASCII source.
//! Scratch and source loans are admitted before entry; no callbacks in the loop.
use super::*;
use crate::native_artifact::program_window_histogram_layout::{
    BANKED_CELLS, BANKED_PAIR_CELLS, BANKS, SINGLE_CELLS,
};
use cranelift_codegen::ir::{Endianness, Value};

pub(super) fn emit(b: &mut FunctionBuilder<'_>) {
    let entry = b.create_block();
    let valid = b.create_block();
    let empty = b.create_block();
    let one = b.create_block();
    let serial = b.create_block();
    let banked = b.create_block();
    let pair = b.create_block();
    let pair_serial = b.create_block();
    let pair_banked = b.create_block();
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
    // The fourth word is the actual admitted extent, not a guessed capacity.
    // Exact physical extents select four disjoint banks. Ordinary one-bank
    // callers retain the serial body. The five-word ABI is unchanged.
    let expanded = b.ins().icmp_imm(IntCC::Equal, args[3], BANKED_CELLS as i64);
    b.ins().brif(expanded, banked, &[], serial, &[]);
    b.switch_to_block(serial);
    emit_width(b, source, bins, accepted, 1, false);
    b.switch_to_block(banked);
    emit_width(b, source, bins, accepted, 1, true);
    b.switch_to_block(pair);
    let expanded = b
        .ins()
        .icmp_imm(IntCC::Equal, args[3], BANKED_PAIR_CELLS as i64);
    b.ins().brif(expanded, pair_banked, &[], pair_serial, &[]);
    b.switch_to_block(pair_serial);
    emit_width(b, source, bins, accepted, 2, false);
    b.switch_to_block(pair_banked);
    emit_width(b, source, bins, accepted, 2, true);
}

fn emit_width(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    bins: Value,
    accepted: Value,
    width: usize,
    banked: bool,
) {
    let logical_cells = SINGLE_CELLS.pow(width as u32);
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
    // the complete word (and the final ninth byte for pairs) is in the owner.
    // Explicit little endian preserves byte order without an alignment claim.
    let word = b.ins().load(
        types::I64,
        MemFlags::new().with_endianness(Endianness::Little),
        address,
        0,
    );
    let mut bytes = (0..8)
        .map(|i| {
            let shifted = b.ins().ushr_imm(word, i * 8);
            b.ins().band_imm(shifted, 255)
        })
        .collect::<Vec<_>>();
    if width == 2 {
        bytes.push(load_byte(b, address, 8));
    }
    let keys = (0..8)
        .map(|i| key(b, &bytes[i..i + width]))
        .collect::<Vec<_>>();
    if banked {
        for group in keys.chunks_exact(BANKS) {
            // Each group touches one cell in each disjoint bank, even when
            // every key repeats. Finish these stores before the NEXT group
            // reads the same banks: group 0 and group 1 can alias.
            let cells = group
                .iter()
                .enumerate()
                .map(|(bank, &key)| {
                    let offset = b.ins().ishl_imm(key, 3);
                    let base = b.ins().iadd_imm(bins, (bank * logical_cells * 8) as i64);
                    b.ins().iadd(base, offset)
                })
                .collect::<Vec<_>>();
            let counts = cells
                .iter()
                .map(|&cell| {
                    let old = b.ins().load(types::I64, MemFlags::new(), cell, 0);
                    b.ins().iadd_imm(old, 1)
                })
                .collect::<Vec<_>>();
            for (cell, count) in cells.into_iter().zip(counts) {
                b.ins().store(MemFlags::new(), count, cell, 0);
            }
        }
    } else {
        // Repeated keys MUST see preceding updates within the same bank.
        for key in keys {
            increment(b, bins, key);
        }
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
    if banked {
        reduce_banks(b, bins, logical_cells);
    }
    b.ins().return_(&[accepted]);
}

fn reduce_banks(b: &mut FunctionBuilder<'_>, bins: Value, logical_cells: usize) {
    let reduce = b.create_block();
    let done = b.create_block();
    b.append_block_param(reduce, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(reduce, &[zero.into()]);
    b.switch_to_block(reduce);
    let offset = b.block_params(reduce)[0];
    let cell = b.ins().iadd(bins, offset);
    let mut count = b.ins().load(types::I64, MemFlags::new(), cell, 0);
    for bank in 1..BANKS {
        let part = b.ins().load(
            types::I64,
            MemFlags::new(),
            cell,
            (bank * logical_cells * 8) as i32,
        );
        count = b.ins().iadd(count, part);
    }
    // Every source byte contributed exactly once, so even the sum <= length.
    // Only this logical prefix is materialized; the private banks are dropped.
    b.ins().store(MemFlags::new(), count, cell, 0);
    let next = b.ins().iadd_imm(offset, 8);
    let more = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, next, (logical_cells * 8) as i64);
    b.ins().brif(more, reduce, &[next.into()], done, &[]);
    b.switch_to_block(done);
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
