//! Shared-address, native F64 work for an already private generator bank.
use super::*;
use crate::native_artifact::{compiler::f64_arithmetic, program::prepare::numeric_binary};
use cranelift_codegen::ir::condcodes::FloatCC;
use mech_engine::native_artifact::NativePreparedLibraryCall;

pub(super) fn get(b: &mut FunctionBuilder<'_>, frame: Value, at: usize) -> Value {
    b.ins().load(types::I64, MemFlags::new(), frame, at as i32)
}
pub(super) fn require(b: &mut FunctionBuilder<'_>, good: Value, bad: Block) {
    let next = b.create_block();
    b.ins().brif(good, next, &[], bad, &[]);
    b.switch_to_block(next);
}
pub(super) fn address(b: &mut FunctionBuilder<'_>, pointer: Value, index: Value) -> Value {
    super::super::numeric_binary::address(b, pointer, index)
}
pub(super) fn span(b: &mut FunctionBuilder<'_>, pointer: Value, count: u64, bad: Block) -> Value {
    let count_value = b.ins().iconst(types::I64, count as i64);
    let valid = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        count_value,
        (isize::MAX as u64 / 8) as i64,
    );
    require(b, valid, bad);
    let nonnull = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
    if count != 0 {
        require(b, nonnull, bad);
    }
    let low = b.ins().band_imm(pointer, 7);
    let aligned = b.ins().icmp_imm(IntCC::Equal, low, 0);
    require(b, aligned, bad);
    let bytes = b.ins().iconst(types::I64, (count * 8) as i64);
    let (end, overflow) = b.ins().uadd_overflow(pointer, bytes);
    let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
    require(b, good, bad);
    end
}
pub(super) fn disjoint(
    b: &mut FunctionBuilder<'_>,
    a: Value,
    end_a: Value,
    c: Value,
    end_c: Value,
    bad: Block,
) {
    let left = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end_a, c);
    let right = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end_c, a);
    let good = b.ins().bor(left, right);
    require(b, good, bad);
}
pub(super) fn description(
    b: &mut FunctionBuilder<'_>,
    pointer: Value,
    extents: [u64; 2],
) -> [Value; 6] {
    let count = b.ins().iconst(types::I64, (extents[0] * extents[1]) as i64);
    let rows = b.ins().iconst(types::I64, extents[0] as i64);
    let columns = b.ins().iconst(types::I64, extents[1] as i64);
    let zero = b.ins().iconst(types::I64, 0);
    [pointer, count, count, rows, columns, zero]
}
pub(super) fn describe(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &TypedLayout,
    value: u32,
    pointer: Value,
    extents: [u64; 2],
) {
    let data = description(b, pointer, extents);
    write_view(b, words, layout.slot(value), &data);
}
pub(super) fn view(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &TypedLayout,
    value: u32,
) -> Vec<Value> {
    (0..6)
        .map(|n| words.read(b, layout.slot(value) + n))
        .collect()
}
pub(super) fn write_view(b: &mut FunctionBuilder<'_>, words: &Words, at: usize, values: &[Value]) {
    for (n, &value) in values.iter().enumerate() {
        words.write(b, at + n, value);
    }
}
pub(super) fn copy(b: &mut FunctionBuilder<'_>, source: Value, output: Value, count: u64) {
    let header = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(header, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let more = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, index, count as i64);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let from = address(b, source, index);
    let to = address(b, output, index);
    let value = b.ins().load(types::I64, MemFlags::new(), from, 0);
    b.ins().store(MemFlags::new(), value, to, 0);
    let next = b.ins().iadd_imm(index, 1);
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(done);
}
pub(super) fn linear(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    data: &[Value],
    index: u32,
    schema: &mech_core::SchemaBody,
    bad: Block,
) -> Result<Value, NativeCompileError> {
    let index = words.read(b, index as usize);
    let ordinal = b.ins().iadd_imm(index, -1);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, data[1]);
    require(b, valid, bad);
    Ok(super::super::numeric_words::linear_address(
        b,
        ordinal,
        data[3],
        data[4],
        8,
        schema,
        |_| data[0],
    )?
    .0)
}
pub(super) fn cell(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    data: &[Value],
    row: u32,
    column: u32,
    bad: Block,
) -> Result<Value, NativeCompileError> {
    let row = words.read(b, row as usize);
    let column = words.read(b, column as usize);
    Ok(super::super::numeric_words::cell_address(
        b, row, column, data[3], data[4], 8, data[0], bad,
    )?
    .0)
}
pub(super) fn push_block(
    b: &mut FunctionBuilder<'_>,
    output: Value,
    extents: [u64; 2],
    block: [u64; 2],
    column: Value,
    data: &[Value],
    bad: Block,
) {
    for (actual, expected) in [
        (data[1], block[0] * block[1]),
        (data[3], block[0]),
        (data[4], block[1]),
    ] {
        let good = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
        require(b, good, bad);
    }
    let header = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(header, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let more = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThan, index, (block[0] * block[1]) as i64);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let width = b.ins().iconst(types::I64, block[1] as i64);
    let row = b.ins().udiv(index, width);
    let inner = b.ins().urem(index, width);
    let start = b.ins().imul_imm(row, extents[1] as i64);
    let at = b.ins().imul_imm(column, block[1] as i64);
    let at = b.ins().iadd(start, at);
    let at = b.ins().iadd(at, inner);
    let from = address(b, data[0], index);
    let to = address(b, output, at);
    let value = b.ins().load(types::I64, MemFlags::new(), from, 0);
    b.ins().store(MemFlags::new(), value, to, 0);
    let next = b.ins().iadd_imm(index, 1);
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(done);
}
pub(super) fn transpose(
    b: &mut FunctionBuilder<'_>,
    output: Value,
    extents: [u64; 2],
    data: &[Value],
) {
    let done = b.create_block();
    let count = b.ins().iconst(types::I64, (extents[0] * extents[1]) as i64);
    let output_columns = b.ins().iconst(types::I64, extents[1] as i64);
    let input_columns = b.ins().iconst(types::I64, extents[0] as i64);
    super::super::numeric_transpose::emit(
        b,
        data[0],
        output,
        count,
        output_columns,
        input_columns,
        done,
    );
    b.switch_to_block(done);
}
pub(super) fn binary(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    layout: &TypedLayout,
    call: &NativePreparedLibraryCall,
    operation: &numeric_binary::Operation,
    output: Value,
    extents: [u64; 2],
    bad: Block,
) -> Result<(), NativeCompileError> {
    let numeric_binary::Kind::Elementwise(op) = operation.kind else {
        return Err(backend(
            "generator product is not an elementwise source operation",
        ));
    };
    let mut inputs = Vec::new();
    let mut dimensions = Vec::new();
    for &value in &operation.operands {
        if matches!(layout.ty(value).kind, LayoutKind::Scalar { .. }) {
            let scalar = words.read(b, value as usize);
            inputs.push((None, Some(scalar)));
            dimensions.push([1, 1]);
        } else {
            let data = view(b, words, layout, value);
            inputs.push((Some(data), None));
            let shape = call
                .template()
                .inputs()
                .get(dimensions.len())
                .unwrap()
                .body();
            let extents = super::super::super::prepare::numeric_generators::matrix(shape)
                .ok_or_else(|| backend("generator binary input shape is not fixed F64"))?;
            dimensions.push(extents.map(|n| n as usize));
        }
    }
    let geometry = call
        .f64_matrix_binary_geometry(dimensions[0], dimensions[1])
        .ok_or_else(|| backend("generator binary lacks selected broadcast geometry"))?;
    if geometry[..2] != extents {
        return Err(backend("generator binary output geometry differs"));
    }
    let header = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(header, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let more = b.ins().icmp_imm(
        IntCC::UnsignedLessThan,
        index,
        (extents[0] * extents[1]) as i64,
    );
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let mut values = Vec::new();
    for ((data, scalar), mode) in inputs.iter().zip(&geometry[2..]) {
        let value = if let Some(scalar) = scalar {
            b.ins().bitcast(types::F64, MemFlags::new(), *scalar)
        } else {
            let at = match *mode {
                1 => index,
                2 => {
                    let n = b.ins().iconst(types::I64, extents[1] as i64);
                    b.ins().udiv(index, n)
                }
                3 => {
                    let n = b.ins().iconst(types::I64, extents[1] as i64);
                    b.ins().urem(index, n)
                }
                0 => zero,
                _ => return Err(backend("generator binary broadcast mode is invalid")),
            };
            let pointer = address(b, data.as_ref().unwrap()[0], at);
            b.ins().load(types::F64, MemFlags::new(), pointer, 0)
        };
        let ordered = b.ins().fcmp(FloatCC::Ordered, value, value);
        require(b, ordered, bad);
        values.push(value);
    }
    let value = f64_arithmetic(b, op, values[0], values[1]);
    let pointer = address(b, output, index);
    b.ins().store(MemFlags::new(), value, pointer, 0);
    let next = b.ins().iadd_imm(index, 1);
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(done);
    describe(b, words, layout, operation.output, output, extents);
    Ok(())
}
