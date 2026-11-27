//! Two independent bounded-key accumulators over each eight-source prefix.
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

fn positions(b: &mut FunctionBuilder<'_>, k: Value, lanes: Value) -> Value {
    let index = b.ins().ireduce(types::I16, k);
    let index = b.ins().splat(types::I16X8, index);
    b.ins().iadd(index, lanes)
}

/// The caller admits at least eight immutable source elements, a four-aligned
/// prefix length, and best/absolute end <= 65535. Cost has no index expression;
/// its callback reads only the already admitted four-element window and uses
/// total U32 arithmetic. It neither changes source state nor settles charges.
pub(super) fn emit<F>(
    b: &mut FunctionBuilder<'_>,
    start: Value,
    end: Value,
    best: Value,
    carry: bool,
    latch: bool,
    head_group: bool,
    mut cost: F,
) -> Value
where
    F: FnMut(&mut FunctionBuilder<'_>, Value, Value) -> Value,
{
    let lanes = super::packed_key::halfwords(b, [1, 2, 3, 4, 0, 0, 0, 0]);
    let four = super::packed_key::halfwords(b, [4; 8]);
    let eight = super::packed_key::halfwords(b, [8; 8]);
    let initial = super::packed_key::initial(b, best);
    let count = b.ins().isub(end, start);
    let (start, paired_end, seed) = if head_group {
        // Remove the optional four-element group at the beginning, leaving
        // the existing end value as the paired-loop bound. When this group
        // exists count >= 12, so at least eight elements remain. Otherwise
        // the original admitted count is already a multiple of eight.
        let extra = b.ins().band_imm(count, 4);
        let has_head = b.ins().icmp_imm(IntCC::NotEqual, extra, 0);
        let head = b.create_block();
        let setup = b.create_block();
        b.append_block_param(setup, types::I64);
        b.append_block_param(setup, types::I32X4);
        b.ins()
            .brif(has_head, head, &[], setup, &[start.into(), initial.into()]);
        b.switch_to_block(head);
        let index = positions(b, start, lanes);
        let prices = cost(b, start, index);
        let key = super::packed_key::key(b, prices, index);
        let seed = b.ins().umin(key, initial);
        let next = b.ins().iadd_imm(start, 4);
        b.ins().jump(setup, &[next.into(), seed.into()]);
        b.switch_to_block(setup);
        let values = b.block_params(setup);
        (values[0], end, values[1])
    } else {
        // Preserve the original IR construction order when this option is off.
        // The optional tail's pure admitted reads can precede the prefix;
        // packed ordinals retain earliest original source order.
        let prefix = b.ins().band_imm(count, -8);
        let paired_end = b.ins().iadd(start, prefix);
        let tail = b.create_block();
        let setup = b.create_block();
        b.append_block_param(setup, types::I32X4);
        let has_tail = b.ins().icmp(IntCC::UnsignedLessThan, paired_end, end);
        b.ins().brif(has_tail, tail, &[], setup, &[initial.into()]);
        b.switch_to_block(tail);
        let index = positions(b, paired_end, lanes);
        let prices = cost(b, paired_end, index);
        let key = super::packed_key::key(b, prices, index);
        let seed = b.ins().umin(key, initial);
        b.ins().jump(setup, &[seed.into()]);
        b.switch_to_block(setup);
        (start, paired_end, b.block_params(setup)[0])
    };

    let header = b.create_block();
    let body = (!latch).then(|| b.create_block());
    let finish = b.create_block();
    for ty in [types::I64, types::I32X4, types::I32X4] {
        b.append_block_param(header, ty);
    }
    if latch {
        for _ in 0..2 {
            b.append_block_param(finish, types::I32X4);
        }
    }
    let mut entry = vec![start.into(), seed.into(), initial.into()];
    if carry {
        b.append_block_param(header, types::I16X8);
        entry.push(positions(b, start, lanes).into());
    }
    b.ins().jump(header, &entry);
    b.switch_to_block(header);
    let state = b.block_params(header).to_vec();
    let k = state[0];
    if let Some(body) = body {
        let more = b.ins().icmp(IntCC::UnsignedLessThan, k, paired_end);
        b.ins().brif(more, body, &[], finish, &[]);
        b.switch_to_block(body);
    }
    let first = if carry {
        state[3]
    } else {
        positions(b, k, lanes)
    };
    let second = b.ins().iadd(first, four);
    let next_four = b.ins().iadd_imm(k, 4);
    let first_cost = cost(b, k, first);
    let second_cost = cost(b, next_four, second);
    let first_key = super::packed_key::key(b, first_cost, first);
    let second_key = super::packed_key::key(b, second_cost, second);
    let a = b.ins().umin(first_key, state[1]);
    let c = b.ins().umin(second_key, state[2]);
    let next = b.ins().iadd_imm(k, 8);
    let mut args = vec![next.into(), a.into(), c.into()];
    if carry {
        // The terminal value may wrap in unused lanes; it is never read or
        // dereferenced after k reaches paired_end. Every consumed ordinal
        // is in 1..=65535 by the caller's absolute-end admission.
        args.push(b.ins().iadd(first, eight).into());
    }
    if latch {
        // The admitted prefix has at least one group of eight. No iteration
        // or speculative second load passes the rounded paired_end.
        let more = b.ins().icmp(IntCC::UnsignedLessThan, next, paired_end);
        b.ins()
            .brif(more, header, &args, finish, &[a.into(), c.into()]);
    } else {
        b.ins().jump(header, &args);
    }
    b.switch_to_block(finish);
    let (a, c) = if latch {
        let state = b.block_params(finish);
        (state[0], state[1])
    } else {
        (state[1], state[2])
    };
    b.ins().umin(a, c)
}
