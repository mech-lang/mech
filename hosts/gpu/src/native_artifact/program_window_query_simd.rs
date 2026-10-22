//! Eight exact overlapping starts per batch on little-endian AArch64.
//! These are emitted instructions, not a runtime helper or new authority.
//! The caller has retained/admitted both immutable loans. Every 16-byte load and
//! every full candidate window is bounded; scalar code handles all tails.
use super::*;
use cranelift_codegen::ir::ConstantData;

fn pair(b: &mut FunctionBuilder<'_>, bytes: Value, start: u8) -> Value {
    let mask = (start..start + 8)
        .chain(start + 1..start + 9)
        .collect::<Vec<_>>();
    // Every index is inside this one 16-byte source vector. A one-input
    // lookup is exactly the same gather, without requesting a two-register
    // table containing two copies of the source on AArch64.
    let mask = b.func.dfg.constants.insert(ConstantData::from(mask));
    let mask = b.ins().vconst(types::I8X16, mask);
    let bytes = b.ins().swizzle(bytes, mask);
    b.ins().bitcast(types::I64X2, little(), bytes)
}

fn scan_blocks(b: &mut FunctionBuilder<'_>, vectors: bool) -> (Block, Block, Block) {
    let init = b.create_block();
    let scan = b.create_block();
    let exit = b.create_block();
    b.append_block_param(scan, types::I64);
    if vectors {
        for _ in 0..4 {
            b.append_block_param(scan, types::I64X2);
        }
    } else {
        b.append_block_param(scan, types::I64);
    }
    b.append_block_param(exit, types::I64);
    b.append_block_param(exit, types::I64);
    (init, scan, exit)
}

pub(super) fn short(
    b: &mut FunctionBuilder<'_>,
    window: WindowValues,
    key: Value,
    mask: Value,
    zero: Value,
) -> (Value, Value) {
    let span = b.ins().iconst(types::I64, 16);
    count(b, window, Query::Masked { key, mask }, span, zero)
}

/// For widths8..16 the first and last eight bytes cover the entire query,
/// including overlaps. The caller has already checked width == query length.
pub(super) fn two_words(
    b: &mut FunctionBuilder<'_>,
    window: WindowValues,
    first: Value,
    zero: Value,
) -> (Value, Value) {
    let offset = b.ins().iadd_imm(window.width, -8);
    let address = b.ins().iadd(window.query, offset);
    let last = b.ins().load(types::I64, little(), address, 0);
    let span = b.ins().iadd_imm(window.width, 8);
    count(
        b,
        window,
        Query::TwoWords {
            first,
            last,
            offset,
        },
        span,
        zero,
    )
}

enum Query {
    Masked {
        key: Value,
        mask: Value,
    },
    TwoWords {
        first: Value,
        last: Value,
        offset: Value,
    },
}

fn count(
    b: &mut FunctionBuilder<'_>,
    window: WindowValues,
    query: Query,
    span: Value,
    zero: Value,
) -> (Value, Value) {
    let (init, scan, exit) = scan_blocks(b, true);
    let finish = b.create_block();
    b.append_block_param(finish, types::I64);
    for _ in 0..4 {
        b.append_block_param(finish, types::I64X2);
    }
    let fits = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, window.length, span);
    b.ins()
        .brif(fits, init, &[], exit, &[zero.into(), zero.into()]);
    b.switch_to_block(init);
    let limit = b.ins().isub(window.length, span);
    let (key, mask, second) = match query {
        Query::Masked { key, mask } => (
            b.ins().splat(types::I64X2, key),
            Some(b.ins().splat(types::I64X2, mask)),
            None,
        ),
        Query::TwoWords {
            first,
            last,
            offset,
        } => (
            b.ins().splat(types::I64X2, first),
            None,
            Some((b.ins().splat(types::I64X2, last), offset)),
        ),
    };
    let zeros = b.ins().splat(types::I64X2, zero);
    b.ins().jump(
        scan,
        &[
            zero.into(),
            zeros.into(),
            zeros.into(),
            zeros.into(),
            zeros.into(),
        ],
    );
    b.switch_to_block(scan);
    let params = b.block_params(scan).to_vec();
    let offset = params[0];
    let address = b.ins().iadd(window.source, offset);
    let bytes = b.ins().load(types::I8X16, MemFlags::new(), address, 0);
    let second = second.map(|(key, offset)| {
        let address = b.ins().iadd(address, offset);
        (key, b.ins().load(types::I8X16, MemFlags::new(), address, 0))
    });
    let mut counts = Vec::new();
    for i in 0..4 {
        let word = pair(b, bytes, (i * 2) as u8);
        let word = if let Some(mask) = mask {
            b.ins().band(word, mask)
        } else {
            word
        };
        let mut same = b.ins().icmp(IntCC::Equal, word, key);
        if let Some((key, bytes)) = second {
            let word = pair(b, bytes, (i * 2) as u8);
            let last_same = b.ins().icmp(IntCC::Equal, word, key);
            same = b.ins().band(same, last_same);
        }
        // Equal vector lanes contain all ones (-1). Four independent lane
        // counters avoid a loop-carried scalar increment for every start.
        counts.push(b.ins().isub(params[i + 1], same));
    }
    let next = b.ins().iadd_imm(offset, 8);
    let more = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, limit);
    let args = [
        next.into(),
        counts[0].into(),
        counts[1].into(),
        counts[2].into(),
        counts[3].into(),
    ];
    b.ins().brif(more, scan, &args, finish, &args);
    b.switch_to_block(finish);
    let params = b.block_params(finish).to_vec();
    let mut total = params[1];
    for part in &params[2..] {
        total = b.ins().iadd(total, *part);
    }
    let low = b.ins().extractlane(total, 0);
    let high = b.ins().extractlane(total, 1);
    let count = b.ins().iadd(low, high);
    b.ins().jump(exit, &[params[0].into(), count.into()]);
    b.switch_to_block(exit);
    (b.block_params(exit)[0], b.block_params(exit)[1])
}

pub(super) fn wide_prefix(
    b: &mut FunctionBuilder<'_>,
    window: WindowValues,
    key: Value,
    zero: Value,
) -> (Value, Value) {
    let (init, scan, exit) = scan_blocks(b, false);
    let hits = b.create_block();
    let compare = b.create_block();
    let equal = b.create_block();
    let matched = b.create_block();
    let advance_hit = b.create_block();
    let advance_batch = b.create_block();
    b.append_block_param(hits, types::I64);
    b.append_block_param(hits, types::I64);
    for block in [compare, advance_hit, advance_batch] {
        b.append_block_param(block, types::I64);
    }
    let fits_load = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, window.length, 16);
    let fits_starts = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, window.last, 7);
    let fits = b.ins().band(fits_load, fits_starts);
    b.ins()
        .brif(fits, init, &[], exit, &[zero.into(), zero.into()]);
    b.switch_to_block(init);
    let load_limit = b.ins().iadd_imm(window.length, -16);
    let start_limit = b.ins().iadd_imm(window.last, -7);
    let limit = b.ins().umin(load_limit, start_limit);
    let key = b.ins().splat(types::I64X2, key);
    let eight = b.ins().iconst(types::I64, 8);
    let mut weights = Vec::new();
    for i in 0..4 {
        let bytes = (1u64 << (2 * i))
            .to_le_bytes()
            .into_iter()
            .chain((1u64 << (2 * i + 1)).to_le_bytes())
            .collect::<Vec<_>>();
        let data = b.func.dfg.constants.insert(ConstantData::from(bytes));
        weights.push(b.ins().vconst(types::I64X2, data));
    }
    b.ins().jump(scan, &[zero.into(), zero.into()]);
    b.switch_to_block(scan);
    let offset = b.block_params(scan)[0];
    let count = b.block_params(scan)[1];
    let address = b.ins().iadd(window.source, offset);
    let bytes = b.ins().load(types::I8X16, MemFlags::new(), address, 0);
    let mut mask = None;
    for (i, weight) in weights.iter().enumerate() {
        let word = pair(b, bytes, (i * 2) as u8);
        let same = b.ins().icmp(IntCC::Equal, word, key);
        let bits = b.ins().band(same, *weight);
        mask = Some(match mask {
            Some(old) => b.ins().bor(old, bits),
            None => bits,
        });
    }
    let mask = mask.expect("four prefix pairs");
    let low = b.ins().extractlane(mask, 0);
    let high = b.ins().extractlane(mask, 1);
    let mask = b.ins().bor(low, high);
    let any = b.ins().icmp_imm(IntCC::NotEqual, mask, 0);
    b.ins().brif(
        any,
        hits,
        &[mask.into(), count.into()],
        advance_batch,
        &[count.into()],
    );
    b.switch_to_block(hits);
    let mask = b.block_params(hits)[0];
    let count = b.block_params(hits)[1];
    // This block is reached only with a nonzero low-eight-bit mask. Each
    // bit names exactly one distinct admitted start, including dense hits.
    let lane = b.ins().ctz(mask);
    let start = b.ins().iadd(offset, lane);
    let tail = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThan, window.width, 8);
    b.ins().brif(tail, compare, &[eight.into()], matched, &[]);
    ByteLoop {
        compare,
        equal,
        matched,
        advance: advance_hit,
    }
    .emit(b, window.source, window.query, start, count, window.width);
    b.switch_to_block(advance_hit);
    let count = b.block_params(advance_hit)[0];
    let previous = b.ins().iadd_imm(mask, -1);
    let remaining = b.ins().band(mask, previous);
    let any = b.ins().icmp_imm(IntCC::NotEqual, remaining, 0);
    b.ins().brif(
        any,
        hits,
        &[remaining.into(), count.into()],
        advance_batch,
        &[count.into()],
    );
    b.switch_to_block(advance_batch);
    let count = b.block_params(advance_batch)[0];
    let next = b.ins().iadd_imm(offset, 8);
    let more = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, limit);
    b.ins().brif(
        more,
        scan,
        &[next.into(), count.into()],
        exit,
        &[next.into(), count.into()],
    );
    b.switch_to_block(exit);
    (b.block_params(exit)[0], b.block_params(exit)[1])
}
