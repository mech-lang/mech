use super::*;
use mech_engine::native_artifact::NativeScalarOp;
/// Intersect the exact source U64 comparison with the current outer range.
/// Windows contain only the child-dot arm; the invariant arm is filled once.
fn windows(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    recipe: &Batch,
    start: Value,
    count: Value,
) -> Result<Vec<(Value, Value)>, NativeCompileError> {
    use NativeScalarOp::*;
    let zero = b.ins().iconst(types::I64, 0);
    let predicate: &Predicate = match &recipe.predicate {
        Some(predicate) => predicate,
        None => return Ok(vec![(zero, count)]),
    };
    let bound = context.word(b, predicate.bound)?;
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
    let op = if predicate.dot_when_true {
        predicate.op
    } else {
        match predicate.op {
            U64Eq => U64Ne,
            U64Ne => U64Eq,
            U64Lt => U64Ge,
            U64Le => U64Gt,
            U64Gt => U64Le,
            U64Ge => U64Lt,
            _ => unreachable!("proved U64 comparison"),
        }
    };
    let rest_before = b.ins().isub(count, before);
    let rest_through = b.ins().isub(count, through);
    Ok(match op {
        U64Lt => vec![(zero, before)],
        U64Le => vec![(zero, through)],
        U64Gt => vec![(through, rest_through)],
        U64Ge => vec![(before, rest_before)],
        U64Eq => vec![(before, b.ins().isub(through, before))],
        U64Ne => vec![(zero, before), (through, rest_through)],
        _ => unreachable!("proved U64 comparison"),
    })
}
fn fill(b: &mut FunctionBuilder<'_>, output: Value, count: Value, word: Value) {
    let cursor = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(cursor, zero);
    let head = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[]);
    b.switch_to_block(head);
    let at = b.use_var(cursor);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let offset = b.ins().imul_imm(at, 8);
    let pointer = b.ins().iadd(output, offset);
    b.ins().store(MemFlags::new(), word, pointer, 0);
    let next = b.ins().iadd_imm(at, 1);
    b.def_var(cursor, next);
    b.ins().jump(head, &[]);
    b.switch_to_block(done);
}
fn child_count(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    recipe: &Batch,
) -> Result<(Value, Value), NativeCompileError> {
    let first = context.word(b, recipe.first)?;
    let last = context.word(b, recipe.last)?;
    let (mut count, overflow) = b.ins().usub_overflow(last, first);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, context.slow);
    if recipe.inclusive {
        let one = b.ins().iconst(types::I64, 1);
        let (next, overflow) = b.ins().uadd_overflow(count, one);
        count = next;
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        require(b, good, context.slow);
    }
    let positive = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
    require(b, positive, context.slow);
    let valid = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, count, recipe.maximum as i64);
    require(b, valid, context.slow);
    if recipe.constant_count {
        let valid = b.ins().icmp_imm(IntCC::Equal, count, recipe.maximum as i64);
        require(b, valid, context.slow);
        count = b.ins().iconst(types::I64, recipe.maximum as i64);
    }
    Ok((first, count))
}
fn cell(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    streams: &[streams::Prepared],
    inner_count: Value,
    output: Value,
    offset: Value,
    at: Value,
    vector: bool,
) {
    let left = streams[0].stream(b, Some(at), vector);
    let right = streams[1].stream(b, Some(at), vector);
    let result = numeric_dot_core::emit(b, &[left], right, inner_count, vector)[0];
    super::super::numeric_product_epilogues::finite(b, result, vector, context.slow);
    let ordinal = b.ins().iadd(offset, at);
    let bytes = b.ins().imul_imm(ordinal, 8);
    let pointer = b.ins().iadd(output, bytes);
    b.ins().store(MemFlags::new(), result, pointer, 0);
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    recipe: &Batch,
    region: &Region,
    matrices: &[(Value, Value, Value, Value)],
    start: Value,
    count: Value,
    output: Value,
) -> Result<(), NativeCompileError> {
    // Partial fast results may be overwritten by original replay. Prove the
    // entire output is disjoint from every source, including forged views.
    let output_bytes = b.ins().imul_imm(count, 8);
    let output_end = b.ins().iadd(output, output_bytes);
    for (index, &(pointer, cells, _, _)) in matrices.iter().enumerate() {
        let bytes = b
            .ins()
            .imul_imm(cells, i64::from(region.matrices[index].width));
        let end = b.ins().iadd(pointer, bytes);
        let before = b
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, output_end, pointer);
        let after = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, output);
        let disjoint = b.ins().bor(before, after);
        let empty = b.ins().icmp_imm(IntCC::Equal, cells, 0);
        let good = b.ins().bor(empty, disjoint);
        require(b, good, context.slow);
    }
    let windows = windows(b, context, recipe, start, count)?;
    if let Some(alternative) = recipe.alternative {
        let word = context.word(b, alternative)?;
        fill(b, output, count, word);
    }
    for (offset, outer_count) in windows {
        let work = b.create_block();
        let done = b.create_block();
        let positive = b.ins().icmp_imm(IntCC::NotEqual, outer_count, 0);
        b.ins().brif(positive, work, &[], done, &[]);
        b.switch_to_block(work);
        // An inactive source arm must not evaluate its child's endpoints.
        let (inner, inner_count) = child_count(b, context, recipe)?;
        let outer = b.ins().iadd(start, offset);
        let mut streams = Vec::new();
        for operand in &recipe.dot.operands {
            let (pointer, cells, rows, columns) = matrices[recipe.matrices[operand.matrix]];
            streams.push(streams::prepare(
                b,
                context,
                operand,
                [pointer, cells, rows, columns],
                inner,
                inner_count,
                Some((outer, outer_count)),
            )?);
        }
        let left_packed = streams[0].packed(b);
        let right_packed = streams[1].packed(b);
        let packed = b.ins().band(left_packed, right_packed);
        let cursor = b.declare_var(types::I64);
        let zero = b.ins().iconst(types::I64, 0);
        b.def_var(cursor, zero);
        let head = b.create_block();
        let body = b.create_block();
        let pair = b.create_block();
        let single = b.create_block();
        b.ins().jump(head, &[]);
        b.switch_to_block(head);
        let at = b.use_var(cursor);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, at, outer_count);
        b.ins().brif(more, body, &[], done, &[]);
        b.switch_to_block(body);
        let remaining = b.ins().isub(outer_count, at);
        let two = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
        let two = b.ins().band(two, packed);
        b.ins().brif(two, pair, &[], single, &[]);
        b.switch_to_block(pair);
        cell(b, context, &streams, inner_count, output, offset, at, true);
        let next = b.ins().iadd_imm(at, 2);
        b.def_var(cursor, next);
        b.ins().jump(head, &[]);
        b.switch_to_block(single);
        cell(b, context, &streams, inner_count, output, offset, at, false);
        let next = b.ins().iadd_imm(at, 1);
        b.def_var(cursor, next);
        b.ins().jump(head, &[]);
        b.switch_to_block(done);
    }
    Ok(())
}
