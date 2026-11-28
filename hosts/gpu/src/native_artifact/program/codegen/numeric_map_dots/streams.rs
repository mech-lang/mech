use super::*;
/// Base and signed strides have no owner authority. The enclosing map has
/// already checked the entire current bank, geometry, width and loan epoch.
pub(super) struct Prepared {
    pointer: Value,
    inner_step: Value,
    outer_step: Value,
    outer_dependent: bool,
}
impl Prepared {
    pub fn stream(&self, b: &mut FunctionBuilder<'_>, at: Option<Value>, vector: bool) -> Stream {
        let pointer = match at {
            None => self.pointer,
            Some(at) => {
                let offset = b.ins().imul(at, self.outer_step);
                b.ins().iadd(self.pointer, offset)
            }
        };
        Stream {
            pointer,
            step: self.inner_step,
            vector: vector && self.outer_dependent,
        }
    }
    pub fn packed(&self, b: &mut FunctionBuilder<'_>) -> Value {
        if self.outer_dependent {
            b.ins().icmp_imm(IntCC::Equal, self.outer_step, 8)
        } else {
            b.ins().iconst(types::I8, 1)
        }
    }
}
fn expression(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    index: &Index,
    coordinates: [Value; 2],
) -> Result<Value, NativeCompileError> {
    Ok(match &index.expr {
        Expr::Coordinate(axis) => coordinates[*axis as usize],
        Expr::Constant(n) => b.ins().iconst(types::I64, *n as i64),
        Expr::Capture(id) => context.word(b, *id)?,
        Expr::Binary { op, left, right } => {
            let left = expression(b, context, left, coordinates)?;
            let right = expression(b, context, right, coordinates)?;
            primitive(b, *op, &[left, right], context.arithmetic)
        }
    })
}
/// Returns physical cell offset and (for linear source access) logical column.
/// Matrix banks are row-major; Mech's one-index logical traversal is column-major.
fn position(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    operand: &Operand,
    view: [Value; 4],
    coordinates: [Value; 2],
) -> Result<(Value, Option<Value>), NativeCompileError> {
    let [_, cells, rows, columns] = view;
    let selectors = operand
        .indices
        .iter()
        .map(|i| expression(b, context, i, coordinates))
        .collect::<Result<Vec<_>, _>>()?;
    match &selectors[..] {
        [index] => {
            let ordinal = b.ins().iadd_imm(*index, -1);
            let valid = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, cells);
            require(b, valid, context.slow);
            // A valid cell plus checked rows*columns==cells proves rows>0.
            let column = b.ins().udiv(ordinal, rows);
            let consumed = b.ins().imul(column, rows);
            let row = b.ins().isub(ordinal, consumed);
            let base = b.ins().imul(row, columns);
            let physical = b.ins().iadd(base, column);
            let one_row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
            let one_column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
            let vector = b.ins().bor(one_row, one_column);
            // For a vector, logical and physical ordinals coincide. Keep
            // that exact fact visible so fixed vector schemas eliminate
            // unnecessary quotient/remainder work in the preflight itself.
            let physical = b.ins().select(vector, ordinal, physical);
            Ok((physical, Some(column)))
        }
        [row, column] => {
            let row = b.ins().iadd_imm(*row, -1);
            let column = b.ins().iadd_imm(*column, -1);
            for (index, bound) in [(row, rows), (column, columns)] {
                let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, bound);
                require(b, valid, context.slow);
            }
            let base = b.ins().imul(row, columns);
            Ok((b.ins().iadd(base, column), None))
        }
        _ => Err(backend("ordered source dot selector rank")),
    }
}
fn step(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    operand: &Operand,
    view: [Value; 4],
    first: [Value; 2],
    count: Value,
    axis: usize,
    base: Value,
) -> Result<Value, NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    if !operand.indices.iter().any(|i| i.dependencies[axis]) {
        return Ok(zero);
    }
    let many = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, count, 1);
    let work = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::I64);
    b.ins().brif(many, work, &[], done, &[zero.into()]);
    b.switch_to_block(work);
    let mut next = first;
    next[axis] = b.ins().iadd_imm(first[axis], 1);
    let (next, _) = position(b, context, operand, view, next)?;
    let difference = b.ins().isub(next, base);
    let bytes = b.ins().imul_imm(difference, 8);
    b.ins().jump(done, &[bytes.into()]);
    b.switch_to_block(done);
    Ok(b.block_params(done)[0])
}
pub(super) fn prepare(
    b: &mut FunctionBuilder<'_>,
    context: &Context<'_>,
    operand: &Operand,
    view: [Value; 4],
    inner: Value,
    inner_count: Value,
    outer: Option<(Value, Value)>,
) -> Result<Prepared, NativeCompileError> {
    let zero = b.ins().iconst(types::I64, 0);
    let outer_first = outer.map_or(zero, |p| p.0);
    let inner_tail = b.ins().iadd_imm(inner_count, -1);
    let inner_last = b.ins().iadd(inner, inner_tail);
    let mut corners = vec![[inner, outer_first], [inner_last, outer_first]];
    if let Some((first, count)) = outer {
        let tail = b.ins().iadd_imm(count, -1);
        let last = b.ins().iadd(first, tail);
        corners.extend([[inner, last], [inner_last, last]]);
    }
    let mut positions = Vec::new();
    for corner in corners {
        positions.push(position(b, context, operand, view, corner)?);
    }
    if let Some(column) = positions[0].1 {
        let one_row = b.ins().icmp_imm(IntCC::Equal, view[2], 1);
        let one_column = b.ins().icmp_imm(IntCC::Equal, view[3], 1);
        let vector = b.ins().bor(one_row, one_column);
        // Each index expression and every checked subexpression is jointly
        // affine. Corner validity proves the whole rectangle, including all
        // arithmetic intermediates. In a non-vector, equal corner columns
        // additionally prove a single affine physical segment.
        for &(_, other) in positions.iter().skip(1) {
            let same = b
                .ins()
                .icmp(IntCC::Equal, column, other.expect("same selector rank"));
            let valid = b.ins().bor(vector, same);
            require(b, valid, context.slow);
        }
    }
    let base = positions[0].0;
    let first = [inner, outer_first];
    let inner_step = step(b, context, operand, view, first, inner_count, 0, base)?;
    let outer_step = match outer {
        Some((_, count)) => step(b, context, operand, view, first, count, 1, base)?,
        None => zero,
    };
    let offset = b.ins().imul_imm(base, 8);
    let pointer = b.ins().iadd(view[0], offset);
    Ok(Prepared {
        pointer,
        inner_step,
        outer_step,
        outer_dependent: operand.indices.iter().any(|i| i.dependencies[1]),
    })
}
