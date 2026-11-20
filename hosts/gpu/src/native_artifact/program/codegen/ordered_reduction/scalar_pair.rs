//! Two already-admitted scalar visits, followed by the original odd tail.
use cranelift_codegen::ir::{Block, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

/// Entry carries biased index, best U32, full item, previous reverse pointer,
/// and a nonzero remaining count. The callbacks retain the caller's immutable
/// read witnesses and pure total item expression; no accounting is changed.
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    entry: Block,
    scalar: Block,
    finish: Block,
    costs: impl FnOnce(&mut FunctionBuilder<'_>, Value, Value) -> (Value, Value, Value),
    item: impl FnOnce(&mut FunctionBuilder<'_>, Value) -> Value,
) {
    let group = b.create_block();
    let tail = b.create_block();
    for ty in [types::I64, types::I32, types::I64, types::I64, types::I64] {
        b.append_block_param(group, ty);
    }
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let has_pair = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, args[4], 2);
    let args: Vec<_> = args.into_iter().map(Into::into).collect();
    b.ins().brif(has_pair, group, &args, scalar, &args);
    b.switch_to_block(group);
    let args = b.block_params(group).to_vec();
    let index = args[0];
    let next = b.ins().iadd_imm(index, 1);
    let (first, second, previous) = costs(b, index, args[3]);
    // Strict comparisons retain the first member of a tie, and a pair tied
    // with the incoming best retains the complete incoming item.
    let later = b.ins().icmp(IntCC::UnsignedLessThan, second, first);
    let pair_minimum = b.ins().select(later, second, first);
    let winner = b.ins().select(later, next, index);
    let better = b.ins().icmp(IntCC::UnsignedLessThan, pair_minimum, args[1]);
    let minimum = b.ins().select(better, pair_minimum, args[1]);
    let selected = item(b, winner);
    let choice = b.ins().select(better, selected, args[2]);
    let next = b.ins().iadd_imm(index, 2);
    let left = b.ins().iadd_imm(args[4], -2);
    let more = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, left, 2);
    let args = [
        next.into(),
        minimum.into(),
        choice.into(),
        previous.into(),
        left.into(),
    ];
    b.ins().brif(more, group, &args, tail, &[]);
    b.switch_to_block(tail);
    let odd = b.ins().icmp_imm(IntCC::NotEqual, left, 0);
    b.ins()
        .brif(odd, scalar, &args, finish, &[minimum.into(), choice.into()]);
}
