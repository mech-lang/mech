//! Completed selected scratch is copied by physical spans, never re-evaluated.
//! Every interval is clipped by the existing source-derived Selection. These
//! compiler-local recipes introduce no runtime frame, proof or allocation.
use super::{Axis, AxisSelection, Coordinates, Selection};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

#[derive(Clone, Copy)]
struct Interval {
    first: Value,
    count: Value,
    one: bool,
}
fn intervals(b: &mut FunctionBuilder<'_>, axis: &AxisSelection, extent: u64) -> Vec<Interval> {
    let mut out = vec![Interval {
        first: axis.first,
        count: axis.first_count,
        one: axis.singleton || extent == 1,
    }];
    if axis.split {
        let count = b.ins().isub(axis.selected, axis.first_count);
        out.push(Interval {
            first: axis.second,
            count,
            one: extent == 1,
        });
    }
    out
}
fn full(b: &mut FunctionBuilder<'_>, extent: u64) -> Interval {
    Interval {
        first: b.ins().iconst(types::I64, 0),
        count: b.ins().iconst(types::I64, extent as i64),
        one: extent == 1,
    }
}
fn cell_type(width: u8) -> Result<cranelift_codegen::ir::Type, NativeCompileError> {
    match width {
        1 => Ok(types::I8),
        2 => Ok(types::I16),
        4 => Ok(types::I32),
        8 => Ok(types::I64),
        _ => Err(backend("selected publication lost exact cell width")),
    }
}
fn cell(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    width: u8,
) -> Result<(), NativeCompileError> {
    let bits = b.ins().load(cell_type(width)?, MemFlags::new(), input, 0);
    b.ins().store(MemFlags::new(), bits, output, 0);
    Ok(())
}
/// Byte count is a positive whole-cell span inside both admitted banks. A
/// vector is issued only when all16 bytes are selected and initialized. The
/// exact scalar tail never reaches a hole, the next span, or bank padding.
fn copy_span(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    bytes: Value,
    width: u8,
) -> Result<(), NativeCompileError> {
    let packed = b.ins().band_imm(bytes, -16);
    let head = b.create_block();
    let body = b.create_block();
    let tail = b.create_block();
    let last = b.create_block();
    let done = b.create_block();
    b.append_block_param(head, types::I64);
    b.append_block_param(tail, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(head, &[zero.into()]);
    b.switch_to_block(head);
    let offset = b.block_params(head)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, packed);
    b.ins().brif(more, body, &[], tail, &[offset.into()]);
    b.switch_to_block(body);
    let src = b.ins().iadd(input, offset);
    let dst = b.ins().iadd(output, offset);
    let bits = b.ins().load(types::I8X16, MemFlags::new(), src, 0);
    b.ins().store(MemFlags::new(), bits, dst, 0);
    let next = b.ins().iadd_imm(offset, 16);
    b.ins().jump(head, &[next.into()]);
    b.switch_to_block(tail);
    let offset = b.block_params(tail)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, bytes);
    b.ins().brif(more, last, &[], done, &[]);
    b.switch_to_block(last);
    let src = b.ins().iadd(input, offset);
    let dst = b.ins().iadd(output, offset);
    cell(b, src, dst, width)?;
    let next = b.ins().iadd_imm(offset, i64::from(width));
    b.ins().jump(tail, &[next.into()]);
    b.switch_to_block(done);
    Ok(())
}
fn nonempty(b: &mut FunctionBuilder<'_>, row: Interval, column: Interval) -> Block {
    let body = b.create_block();
    let done = b.create_block();
    let rows = b.ins().icmp_imm(IntCC::NotEqual, row.count, 0);
    let columns = b.ins().icmp_imm(IntCC::NotEqual, column.count, 0);
    let both = b.ins().band(rows, columns);
    b.ins().brif(both, body, &[], done, &[]);
    b.switch_to_block(body);
    done
}
fn rectangle(
    b: &mut FunctionBuilder<'_>,
    source: Value,
    destination: Value,
    row: Interval,
    column: Interval,
    columns: u64,
    whole_columns: bool,
    width: u8,
) -> Result<(), NativeCompileError> {
    let done = nonempty(b, row, column);
    // Clipped intervals satisfy first+count<=extent. The original phase has
    // already proved rows*columns*width<=65536 and both complete spans. Thus
    // every accessed offset and address stays in that checked envelope.
    let linear = b.ins().imul_imm(row.first, columns as i64);
    let linear = b.ins().iadd(linear, column.first);
    let offset = b.ins().imul_imm(linear, i64::from(width));
    let input = b.ins().iadd(source, offset);
    let output = b.ins().iadd(destination, offset);
    let bytes = b.ins().imul_imm(column.count, i64::from(width));
    if whole_columns {
        let bytes = b.ins().imul(row.count, bytes);
        copy_span(b, input, output, bytes, width)?;
    } else if row.one {
        if column.one {
            cell(b, input, output, width)?;
        } else {
            copy_span(b, input, output, bytes, width)?;
        }
    } else {
        let rows = b.create_block();
        let ready = b.create_block();
        for _ in 0..3 {
            b.append_block_param(rows, types::I64);
        }
        b.ins()
            .jump(rows, &[input.into(), output.into(), row.count.into()]);
        b.switch_to_block(rows);
        let args = b.block_params(rows).to_vec();
        if column.one {
            cell(b, args[0], args[1], width)?;
        } else {
            copy_span(b, args[0], args[1], bytes, width)?;
        }
        let stride = (columns as i64) * i64::from(width);
        let input = b.ins().iadd_imm(args[0], stride);
        let output = b.ins().iadd_imm(args[1], stride);
        let remaining = b.ins().iadd_imm(args[2], -1);
        let more = b.ins().icmp_imm(IntCC::NotEqual, remaining, 0);
        b.ins().brif(
            more,
            rows,
            &[input.into(), output.into(), remaining.into()],
            ready,
            &[],
        );
        b.switch_to_block(ready);
    }
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    Ok(())
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    selection: &Selection,
    source: Value,
    destination: Value,
    width: u8,
) -> Result<(), NativeCompileError> {
    cell_type(width)?;
    if !selection.rectangular {
        let Coordinates::Single { range, .. } = &selection.coordinates else {
            unreachable!()
        };
        let extent = selection
            .rows
            .checked_mul(selection.columns)
            .ok_or_else(|| backend("selected linear publication extent overflow"))?;
        for column in intervals(b, range, extent) {
            let row = full(b, 1);
            rectangle(b, source, destination, row, column, extent, false, width)?;
        }
        return Ok(());
    }
    let (rows, columns, whole_columns) = match &selection.coordinates {
        Coordinates::Single {
            axis: Axis::Row,
            range,
        } => (
            intervals(b, range, selection.rows),
            vec![full(b, selection.columns)],
            true,
        ),
        Coordinates::Single {
            axis: Axis::Column,
            range,
        } => (
            vec![full(b, selection.rows)],
            intervals(b, range, selection.columns),
            false,
        ),
        Coordinates::Rectangle { row, column } => (
            intervals(b, row, selection.rows),
            intervals(b, column, selection.columns),
            column.full,
        ),
    };
    for row in rows {
        for &column in &columns {
            rectangle(
                b,
                source,
                destination,
                row,
                column,
                selection.columns,
                whole_columns,
                width,
            )?;
        }
    }
    Ok(())
}
