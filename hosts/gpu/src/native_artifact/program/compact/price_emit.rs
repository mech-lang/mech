//! Vectorize bounded blending on disjoint spans without changing checked arithmetic.
use super::price::{Blend, ENDS, MAX_BASE, START};
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{Block, Endianness, Function, InstBuilder, MemFlags, Value, condcodes::IntCC, types},
};

const END: u64 = ENDS[2];

fn block(f: &mut Function) -> Block {
    let block = f.dfg.make_block();
    f.layout.append_block(block);
    block
}

fn splat(b: &mut FuncCursor<'_>, n: i64) -> Value {
    let word = b.ins().iconst(types::I32, n);
    b.ins().splat(types::I32X4, word)
}

fn operands(
    b: &mut FuncCursor<'_>,
    prices: Value,
    entries: Value,
    at: Value,
    mem: MemFlags,
) -> (Value, Value) {
    let p_offset = b.ins().ishl_imm(at, 2);
    let e_offset = b.ins().ishl_imm(at, 1);
    let out = b.ins().iadd(prices, p_offset);
    let entry = b.ins().iadd(entries, e_offset);
    let cell = b.ins().load(types::I64, mem, entry, 0);
    let cell = b.ins().splat(types::I64X2, cell);
    let order = MemFlags::new().with_endianness(Endianness::Little);
    let cell = b.ins().bitcast(types::I16X8, order, cell);
    let extras = b.ins().ushr_imm(cell, 9);
    let extras = b.ins().uwiden_low(extras);
    (out, extras)
}

pub(crate) fn lower(f: &mut Function, blend: Blend) {
    let setup = block(f);
    let weighted = block(f);
    let defaults = block(f);
    let default_chunk = block(f);
    let chunk = block(f);
    let continue_at = block(f);
    let next_segment = block(f);
    let done = block(f);
    let [first, limit, price] =
        core::array::from_fn(|_| f.dfg.append_block_param(setup, types::I64));
    let at = f.dfg.append_block_param(chunk, types::I64);
    let default_at = f.dfg.append_block_param(default_chunk, types::I64);
    let original = blend.original.block(&f.dfg.value_lists);
    let args = blend.original.args(&f.dfg.value_lists).collect::<Vec<_>>();
    let mut b = FuncCursor::new(f).at_inst(blend.entry);
    let initial = b.ins().icmp_imm(IntCC::Equal, blend.at, START as i64);
    let prices_fit = b.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        blend.price_len,
        END as i64,
    );
    let entries_fit = b.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        blend.entry_len,
        END as i64,
    );
    let output_end = b.ins().iadd_imm(blend.prices, (END * 4) as i64);
    let entry_end = b.ins().iadd_imm(blend.entries, (END * 2) as i64);
    let no_wrap0 = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, blend.prices, output_end);
    let no_wrap1 = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, blend.entries, entry_end);
    let separate0 = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, output_end, blend.entries);
    let separate1 = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, entry_end, blend.prices);
    let separate = b.ins().bor(separate0, separate1);
    let weight_fits = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, blend.weight, 8);
    let literal_fits = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        blend.literal,
        MAX_BASE as i64,
    );
    let length_fits = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        blend.length,
        MAX_BASE as i64,
    );
    let accepted = [
        prices_fit,
        entries_fit,
        no_wrap0,
        no_wrap1,
        separate,
        weight_fits,
        literal_fits,
        length_fits,
    ]
    .into_iter()
    .fold(initial, |yes, test| b.ins().band(yes, test));
    let end = b.ins().iconst(types::I64, ENDS[0] as i64);
    b.func.dfg.replace(blend.entry).brif(
        accepted,
        setup,
        &[blend.at.into(), end.into(), blend.length.into()],
        original,
        &args,
    );

    b.goto_bottom(setup);
    let use_defaults = b.ins().icmp_imm(IntCC::Equal, blend.weight, 8);
    b.ins().brif(use_defaults, defaults, &[], weighted, &[]);

    b.goto_bottom(defaults);
    let base = b.ins().ireduce(types::I32, price);
    let base = b.ins().splat(types::I32X4, base);
    b.ins().jump(default_chunk, &[first.into()]);

    b.goto_bottom(default_chunk);
    let mem = MemFlags::new()
        .with_notrap()
        .with_endianness(Endianness::Little);
    let (out, extras) = operands(&mut b, blend.prices, blend.entries, default_at, mem);
    let extra_cost = b.ins().ishl_imm(extras, 4);
    let result = b.ins().iadd(base, extra_cost);
    b.ins().store(mem, result, out, 0);
    let next = b.ins().iadd_imm(default_at, 4);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, next, limit);
    b.ins()
        .brif(more, default_chunk, &[next.into()], continue_at, &[]);

    b.goto_bottom(weighted);
    let weight = b.ins().ireduce(types::I32, blend.weight);
    let price = b.ins().ireduce(types::I32, price);
    let eight = b.ins().iconst(types::I32, 8);
    let current_weight = b.ins().isub(eight, weight);
    let current_weight = b.ins().splat(types::I32X4, current_weight);
    let doubled_weight = b.ins().ishl_imm(weight, 1);
    let doubled_weight = b.ins().splat(types::I32X4, doubled_weight);
    let quotient = b.ins().ushr_imm(price, 3);
    let quotient = b.ins().imul(quotient, weight);
    let quotient = b.ins().splat(types::I32X4, quotient);
    let remainder = b.ins().band_imm(price, 7);
    let remainder = b.ins().imul(remainder, weight);
    let remainder = b.ins().splat(types::I32X4, remainder);
    let low_bits = splat(&mut b, 7);
    b.ins().jump(chunk, &[first.into()]);

    b.goto_bottom(chunk);
    // Every segment is a multiple of four and both whole spans are disjoint.
    // Addresses are naturally aligned for the source elements, even when a
    // full vector starts between two vector-aligned positions.
    let mem = MemFlags::new()
        .with_notrap()
        .with_endianness(Endianness::Little);
    let (out, extras) = operands(&mut b, blend.prices, blend.entries, at, mem);
    let prior = b.ins().load(types::I32X4, mem, out, 0);
    // Split the division by eight before multiplying. The quotients sum
    // to at most U32::MAX because the unsigned weights are convex. The
    // low-bit term restores the exact rounding of the source U64 sum.
    let upper = b.ins().ushr_imm(prior, 3);
    let upper = b.ins().imul(upper, current_weight);
    let extras = b.ins().imul(extras, doubled_weight);
    let default = b.ins().iadd(quotient, extras);
    let lower = b.ins().band(prior, low_bits);
    let lower = b.ins().imul(lower, current_weight);
    let lower = b.ins().iadd(lower, remainder);
    let lower = b.ins().ushr_imm(lower, 3);
    let result = b.ins().iadd(upper, default);
    let result = b.ins().iadd(result, lower);
    b.ins().store(mem, result, out, 0);
    let next = b.ins().iadd_imm(at, 4);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, next, limit);
    b.ins().brif(more, chunk, &[next.into()], continue_at, &[]);

    b.goto_bottom(continue_at);
    let final_segment = b.ins().icmp_imm(IntCC::Equal, limit, END as i64);
    b.ins().brif(final_segment, done, &[], next_segment, &[]);

    b.goto_bottom(next_segment);
    let first_segment = b.ins().icmp_imm(IntCC::Equal, limit, ENDS[0] as i64);
    let literal_end = b.ins().iconst(types::I64, ENDS[1] as i64);
    let offset_end = b.ins().iconst(types::I64, END as i64);
    let offset_price = b.ins().iconst(types::I64, 78);
    let end = b.ins().select(first_segment, literal_end, offset_end);
    let base = b.ins().select(first_segment, blend.literal, offset_price);
    b.ins()
        .jump(setup, &[limit.into(), end.into(), base.into()]);

    b.goto_bottom(done);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[zero, zero]);
}

fn disjoint(b: &mut FuncCursor<'_>, a: Value, a_end: Value, z: Value, z_end: Value) -> Value {
    let earlier = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, a_end, z);
    let later = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, z_end, a);
    b.ins().bor(earlier, later)
}

fn add_scaled(b: &mut FuncCursor<'_>, base: Value, index: Value, shift: i64) -> Value {
    let offset = b.ins().ishl_imm(index, shift);
    b.ins().iadd(base, offset)
}

pub(crate) fn lower_codes(f: &mut Function, codes: super::price::Codes) {
    let head = block(f);
    let read_four = block(f);
    let read_tail = block(f);
    let check_dense = block(f);
    let dense = block(f);
    let lookup = block(f);
    let load_code = block(f);
    let choose = block(f);
    let one_store = block(f);
    let four_stores = block(f);
    let step = block(f);
    let done = block(f);
    let at = f.dfg.append_block_param(head, types::I64);
    let [packed, count] = core::array::from_fn(|_| f.dfg.append_block_param(lookup, types::I64));
    let width = f.dfg.append_block_param(choose, types::I32);
    let count_for_step = f.dfg.append_block_param(step, types::I64);
    let original = codes.original.block(&f.dfg.value_lists);
    let old_args = codes.original.args(&f.dfg.value_lists).collect::<Vec<_>>();
    let mut b = FuncCursor::new(f).at_inst(codes.entry);
    // The same checked loop may resume after prior cells have already been
    // priced. Wide groups and the short tail are admitted at each cursor.
    let initial = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, codes.at, END as i64);
    let prices_fit = b.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        codes.price_len,
        END as i64,
    );
    let entries_fit = b.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        codes.entry_len,
        END as i64,
    );
    let out_end = b.ins().iadd_imm(codes.prices, (END * 4) as i64);
    let entry_end = b.ins().iadd_imm(codes.entries, (END * 2) as i64);
    let symbol_limit = b.ins().iconst(types::I64, 512);
    let symbol_count = b.ins().umin(codes.code_len, symbol_limit);
    let code_end = add_scaled(&mut b, codes.codes, symbol_count, 2);
    let out_order = b.ins().icmp(IntCC::UnsignedLessThan, codes.prices, out_end);
    let entry_order = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, codes.entries, entry_end);
    let code_order = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, codes.codes, code_end);
    let not_entries = disjoint(&mut b, codes.prices, out_end, codes.entries, entry_end);
    let not_codes = disjoint(&mut b, codes.prices, out_end, codes.codes, code_end);
    let accept = [
        prices_fit,
        entries_fit,
        out_order,
        entry_order,
        code_order,
        not_entries,
        not_codes,
    ]
    .into_iter()
    .fold(initial, |yes, test| b.ins().band(yes, test));
    b.func
        .dfg
        .replace(codes.entry)
        .brif(accept, head, &[codes.at.into()], original, &old_args);

    b.goto_bottom(head);
    let e = add_scaled(&mut b, codes.entries, at, 1);
    let dst = add_scaled(&mut b, codes.prices, at, 2);
    let one = b.ins().iconst(types::I64, 1);
    let four = b.ins().iconst(types::I64, 4);
    let enough = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, at, END as i64 - 4);
    b.ins().brif(enough, read_four, &[], read_tail, &[]);

    let mem = MemFlags::new()
        .with_notrap()
        .with_endianness(Endianness::Little);
    b.goto_bottom(read_tail);
    let first = b.ins().uload16(types::I64, mem, e, 0);
    b.ins().jump(lookup, &[first.into(), one.into()]);

    b.goto_bottom(read_four);
    let cells = b.ins().load(types::I64, mem, e, 0);
    let first = b.ins().band_imm(cells, 65535);
    let copies = b.ins().imul_imm(first, 0x0001_0001_0001_0001);
    let repeated = b.ins().icmp(IntCC::Equal, cells, copies);
    b.ins().brif(
        repeated,
        lookup,
        &[first.into(), four.into()],
        check_dense,
        &[],
    );

    b.goto_bottom(check_dense);
    // All four packed cells encode the next four symbols with zero extra
    // cost. Staying below 288 gives the same fallback in all four lanes.
    let ordinals = b.ins().iadd_imm(copies, 0x0003_0002_0001_0000);
    let ordered = b.ins().icmp(IntCC::Equal, cells, ordinals);
    let small = b.ins().icmp_imm(IntCC::UnsignedLessThan, first, 285);
    let last = b.ins().iadd_imm(first, 4);
    let readable = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, last, codes.code_len);
    let valid = b.ins().band(ordered, small);
    let valid = b.ins().band(valid, readable);
    b.ins()
        .brif(valid, dense, &[], lookup, &[first.into(), one.into()]);

    b.goto_bottom(dense);
    let address = add_scaled(&mut b, codes.codes, first, 2);
    let words = b.ins().load(types::I32X4, mem, address, 0);
    let widths = b.ins().ushr_imm(words, 16);
    let zero = splat(&mut b, 0);
    let missing = b.ins().icmp(IntCC::Equal, widths, zero);
    let fallback = splat(&mut b, 13);
    let widths = b.ins().bitselect(missing, fallback, widths);
    let price = b.ins().ishl_imm(widths, 4);
    b.ins().store(mem, price, dst, 0);
    b.ins().jump(step, &[four.into()]);

    b.goto_bottom(lookup);
    let symbol = b.ins().band_imm(packed, 511);
    let extra = b.ins().ushr_imm(packed, 9);
    let known = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, symbol, codes.code_len);
    let zero = b.ins().iconst(types::I32, 0);
    b.ins().brif(known, load_code, &[], choose, &[zero.into()]);

    b.goto_bottom(load_code);
    let address = add_scaled(&mut b, codes.codes, symbol, 2);
    let value = b.ins().load(types::I32, mem, address, 0);
    let value = b.ins().ushr_imm(value, 16);
    b.ins().jump(choose, &[value.into()]);

    b.goto_bottom(choose);
    let shorter = b.ins().icmp_imm(IntCC::UnsignedLessThan, symbol, 288);
    let short = b.ins().iconst(types::I32, 13);
    let long = b.ins().iconst(types::I32, 10);
    let fallback = b.ins().select(shorter, short, long);
    let has_width = b.ins().icmp_imm(IntCC::NotEqual, width, 0);
    let width = b.ins().select(has_width, width, fallback);
    let extra = b.ins().ireduce(types::I32, extra);
    let price = b.ins().iadd(width, extra);
    let price = b.ins().ishl_imm(price, 4);
    let single = b.ins().icmp_imm(IntCC::Equal, count, 1);
    b.ins().brif(single, one_store, &[], four_stores, &[]);

    b.goto_bottom(one_store);
    b.ins().store(mem, price, dst, 0);
    b.ins().jump(step, &[one.into()]);

    b.goto_bottom(four_stores);
    let price = b.ins().splat(types::I32X4, price);
    b.ins().store(mem, price, dst, 0);
    b.ins().jump(step, &[four.into()]);

    b.goto_bottom(step);
    let next = b.ins().iadd(at, count_for_step);
    let more = b.ins().icmp_imm(IntCC::UnsignedLessThan, next, END as i64);
    b.ins().brif(more, head, &[next.into()], done, &[]);

    b.goto_bottom(done);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[zero, zero]);
}
