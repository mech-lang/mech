//! Ordered products over admitted row-major F64 banks. SIMD lanes compute
//! independent columns; each cell keeps the ordinary +0 identity and k order.
use super::numeric_binary::address;
use super::numeric_dot_core::{self, Stream};
use super::numeric_product_epilogues::Epilogue;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
#[path = "numeric_product_rows.rs"]
mod rows;

fn cell(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    index: Value,
    count: Value,
    columns: Value,
    inner: Value,
    vector: bool,
    epilogue: Option<&Epilogue<'_>>,
    transposed_left: bool,
) {
    let row = b.ins().udiv(index, columns);
    let column = b.ins().urem(index, columns);
    let left_base = if transposed_left {
        row
    } else {
        b.ins().imul(row, inner)
    };
    let left_pointer = address(b, left, left_base);
    let right_pointer = address(b, right, column);
    let left_step = if transposed_left {
        // Only reached for a nonempty output, hence columns is nonzero.
        let rows = b.ins().udiv(count, columns);
        b.ins().imul_imm(rows, 8)
    } else {
        b.ins().iconst(types::I64, 8)
    };
    let right_step = b.ins().imul_imm(columns, 8);
    let result = numeric_dot_core::emit(
        b,
        &[Stream {
            pointer: left_pointer,
            step: left_step,
            vector: false,
        }],
        Stream {
            pointer: right_pointer,
            step: right_step,
            vector,
        },
        inner,
        vector,
    )[0];
    let result = if let Some(epilogue) = epilogue {
        epilogue.apply(b, result, row, column, index, vector)
    } else {
        result
    };
    let destination = address(b, output, index);
    b.ins().store(MemFlags::new(), result, destination, 0);
}

pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    count: Value,
    columns: Value,
    inner: Value,
    row_pairs: bool,
    row_quartets: bool,
    done: Block,
) {
    emit_body(
        b,
        left,
        right,
        output,
        count,
        columns,
        inner,
        None,
        row_pairs,
        row_quartets,
        false,
        done,
    );
}
pub(super) fn emit_epilogue(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    count: Value,
    columns: Value,
    inner: Value,
    epilogue: &Epilogue<'_>,
    row_pairs: bool,
    row_quartets: bool,
    done: Block,
) {
    emit_body(
        b,
        left,
        right,
        output,
        count,
        columns,
        inner,
        Some(epilogue),
        row_pairs,
        row_quartets,
        epilogue.transposed_left(),
        done,
    );
}
fn emit_body(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    count: Value,
    columns: Value,
    inner: Value,
    epilogue: Option<&Epilogue<'_>>,
    row_pairs: bool,
    row_quartets: bool,
    transposed_left: bool,
    done: Block,
) {
    if row_pairs {
        rows::emit(
            b,
            left,
            right,
            output,
            count,
            columns,
            inner,
            epilogue,
            row_quartets,
            transposed_left,
            done,
        );
        return;
    }
    let index = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(index, zero);
    let header = b.create_block();
    let body = b.create_block();
    let pair = b.create_block();
    let single = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at = b.use_var(index);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    // This body is reached only for a nonempty result, which certifies
    // columns>0. A vector may never straddle a row, even on an odd width.
    let column = b.ins().urem(at, columns);
    let remaining = b.ins().isub(columns, column);
    let two = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    b.ins().brif(two, pair, &[], single, &[]);
    for (block, vector, width) in [(pair, true, 2), (single, false, 1)] {
        b.switch_to_block(block);
        cell(
            b,
            left,
            right,
            output,
            at,
            count,
            columns,
            inner,
            vector,
            epilogue,
            transposed_left,
        );
        let advanced = b.ins().iadd_imm(at, width);
        b.def_var(index, advanced);
        b.ins().jump(header, &[]);
    }
}

#[cfg(test)]
#[path = "numeric_product_view_tests.rs"]
mod view_tests;
