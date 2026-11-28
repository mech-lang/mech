//! Pure clipping of a source unsigned comparison against one current interval.
//! Returned offsets are relative to `start`; zero intervals stay empty even
//! when the unused wrapping predecessor/successor is not a source value.
use super::*;
use mech_engine::native_artifact::NativeScalarOp;
pub(super) fn comparison(
    b: &mut FunctionBuilder<'_>,
    op: NativeScalarOp,
    bound: Value,
    start: Value,
    count: Value,
    nonempty: bool,
) -> Vec<(Value, Value)> {
    use NativeScalarOp::*;
    let zero = b.ins().iconst(types::I64, 0);
    let tail = b.ins().iadd_imm(count, -1);
    let last = b.ins().iadd(start, tail);
    let below = b.ins().icmp(IntCC::UnsignedLessThan, bound, start);
    let above = b.ins().icmp(IntCC::UnsignedGreaterThan, bound, last);
    let at_end = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, bound, last);
    let difference = b.ins().isub(bound, start);
    let before = b.ins().select(above, count, difference);
    let before = b.ins().select(below, zero, before);
    let successor = b.ins().iadd_imm(difference, 1);
    let through = b.ins().select(at_end, count, successor);
    let through = b.ins().select(below, zero, through);
    let (before, through) = if nonempty {
        (before, through)
    } else {
        let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
        (
            b.ins().select(empty, zero, before),
            b.ins().select(empty, zero, through),
        )
    };
    let rest_before = b.ins().isub(count, before);
    let rest_through = b.ins().isub(count, through);
    match op {
        U64Lt => vec![(zero, before)],
        U64Le => vec![(zero, through)],
        U64Gt => vec![(through, rest_through)],
        U64Ge => vec![(before, rest_before)],
        U64Eq => vec![(before, b.ins().isub(through, before))],
        U64Ne => vec![(zero, before), (through, rest_through)],
        _ => unreachable!("proved source axis comparison"),
    }
}
