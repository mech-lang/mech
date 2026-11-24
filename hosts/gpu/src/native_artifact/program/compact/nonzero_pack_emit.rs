//! Copy active packed words in order. Unvisited destination words remain untouched.
use super::nonzero_pack::Pack;
use cranelift_codegen::{
    cursor::{Cursor, FuncCursor},
    ir::{
        Block, ConstantData, Endianness, Function, InstBuilder, MemFlags, Value, condcodes::IntCC,
        types,
    },
};

fn block(f: &mut Function) -> Block {
    let block = f.dfg.make_block();
    f.layout.append_block(block);
    block
}

fn address(b: &mut FuncCursor<'_>, data: Value, index: Value) -> Value {
    let offset = b.ins().ishl_imm(index, 2);
    b.ins().iadd(data, offset)
}

pub(crate) fn lower(f: &mut Function, pack: Pack) {
    use IntCC::{Equal, UnsignedGreaterThanOrEqual, UnsignedLessThanOrEqual};
    let group = block(f);
    let present = block(f);
    let dense = block(f);
    let mixed = block(f);
    let advance = block(f);
    let [index, used] = core::array::from_fn(|_| f.dfg.append_block_param(group, types::I64));
    let next_used = f.dfg.append_block_param(advance, types::I64);
    let checked = pack.original.block(&f.dfg.value_lists);
    let args = pack.original.args(&f.dfg.value_lists).collect::<Vec<_>>();
    let mask_data = [0u32, 1, 2, 3]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    let lane_constant = f.dfg.constants.insert(ConstantData::from(mask_data));
    let mut b = FuncCursor::new(f).at_inst(pack.entry);
    // With both cursors at zero, at most n words are produced and neither
    // increment can wrap. Check both physical byte spans before coalescing.
    let cursors = b.ins().bor(pack.index, pack.used);
    let initial = b.ins().icmp_imm(Equal, cursors, 0);
    let bounded = b
        .ins()
        .icmp_imm(UnsignedLessThanOrEqual, pack.limit, i64::from(u32::MAX));
    let enough = b.ins().icmp_imm(UnsignedGreaterThanOrEqual, pack.limit, 4);
    let start_fits = b
        .ins()
        .icmp_imm(UnsignedLessThanOrEqual, pack.start, (u64::MAX >> 2) as i64);
    let readable = b
        .ins()
        .icmp(UnsignedLessThanOrEqual, pack.start, pack.input_len);
    let available = b.ins().isub(pack.input_len, pack.start);
    let window = b.ins().icmp(UnsignedLessThanOrEqual, pack.limit, available);
    let writable = b
        .ins()
        .icmp(UnsignedLessThanOrEqual, pack.limit, pack.output_len);
    let source = address(&mut b, pack.input, pack.start);
    let input_end = address(&mut b, source, pack.limit);
    let output_end = address(&mut b, pack.output, pack.limit);
    let start_valid = b.ins().icmp(UnsignedGreaterThanOrEqual, source, pack.input);
    let source_valid = b.ins().icmp(UnsignedGreaterThanOrEqual, input_end, source);
    let output_valid = b
        .ins()
        .icmp(UnsignedGreaterThanOrEqual, output_end, pack.output);
    let separate_before = b
        .ins()
        .icmp(UnsignedLessThanOrEqual, input_end, pack.output);
    let separate_after = b.ins().icmp(UnsignedLessThanOrEqual, output_end, source);
    let disjoint = b.ins().bor(separate_before, separate_after);
    let admitted = [
        bounded,
        enough,
        start_fits,
        readable,
        window,
        writable,
        start_valid,
        source_valid,
        output_valid,
        disjoint,
    ]
    .into_iter()
    .fold(initial, |ok, guard| b.ins().band(ok, guard));
    let last = b.ins().iadd_imm(pack.limit, -4);
    let lanes = b.ins().vconst(types::I32X4, lane_constant);
    b.func.dfg.replace(pack.entry).brif(
        admitted,
        group,
        &[pack.index.into(), pack.used.into()],
        checked,
        &args,
    );

    let mem = MemFlags::new()
        .with_notrap()
        .with_endianness(Endianness::Little);
    b.goto_bottom(group);
    let input_at = address(&mut b, source, index);
    let values = b.ins().load(types::I32X4, mem, input_at, 0);
    let any = b.ins().vany_true(values);
    b.ins().brif(any, present, &[], advance, &[used.into()]);

    b.goto_bottom(present);
    let all = b.ins().vall_true(values);
    b.ins().brif(all, dense, &[], mixed, &[]);

    b.goto_bottom(dense);
    let ix = b.ins().ireduce(types::I32, index);
    let ix = b.ins().splat(types::I32X4, ix);
    let ids = b.ins().iadd(ix, lanes);
    let weights = b.ins().ishl_imm(values, 10);
    let keys = b.ins().bor(weights, ids);
    let destination = address(&mut b, pack.output, used);
    b.ins().store(mem, keys, destination, 0);
    let after = b.ins().iadd_imm(used, 4);
    b.ins().jump(advance, &[after.into()]);

    // Mixed groups run four scalar predicates on the one vector already read.
    // Skip each inactive store. The order and the unmodified suffix match the
    // scalar recurrence even when the destination initially contains data.
    b.goto_bottom(mixed);
    let mut count = used;
    for lane in 0..4u8 {
        let write = block(b.func);
        let join = block(b.func);
        let updated = b.func.dfg.append_block_param(join, types::I64);
        let value = b.ins().extractlane(values, lane);
        b.ins().brif(value, write, &[], join, &[count.into()]);
        b.goto_bottom(write);
        let at = b.ins().ireduce(types::I32, index);
        let symbol = b.ins().iadd_imm(at, i64::from(lane));
        let high = b.ins().ishl_imm(value, 10);
        let key = b.ins().bor(high, symbol);
        let cell = address(&mut b, pack.output, count);
        b.ins().store(mem, key, cell, 0);
        let increment = b.ins().iadd_imm(count, 1);
        b.ins().jump(join, &[increment.into()]);
        b.goto_bottom(join);
        count = updated;
    }
    b.ins().jump(advance, &[count.into()]);

    b.goto_bottom(advance);
    let next_index = b.ins().iadd_imm(index, 4);
    let more = b.ins().icmp(UnsignedLessThanOrEqual, next_index, last);
    let args = [next_index.into(), next_used.into()];
    b.ins().brif(more, group, &args, checked, &args);
}
