//! Ordered products over admitted row-major F64 banks. SIMD lanes compute
//! independent columns; each cell keeps the ordinary +0 identity and k order.
use super::numeric_binary::address;
use crate::native_artifact::compiler::f64_arithmetic;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeScalarOp;

fn cell(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    index: Value,
    columns: Value,
    inner: Value,
    vector: bool,
    count: Value,
    transposed: [bool; 2],
) {
    let kind = if vector {
        types::F64.by(2).unwrap()
    } else {
        types::F64
    };
    let row = b.ins().udiv(index, columns);
    let column = b.ins().urem(index, columns);
    let left_base = b.ins().imul(row, inner);
    let offset = b.declare_var(types::I64);
    let accumulator = b.declare_var(kind);
    let zero = b.ins().iconst(types::I64, 0);
    let identity = b.ins().f64const(0.0);
    let identity = if vector {
        b.ins().splat(kind, identity)
    } else {
        identity
    };
    b.def_var(offset, zero);
    b.def_var(accumulator, identity);
    let header = b.create_block();
    let body = b.create_block();
    let finish = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let k = b.use_var(offset);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, k, inner);
    b.ins().brif(more, body, &[], finish, &[]);
    b.switch_to_block(body);
    let li = if transposed[0] {
        // Reached only for a nonempty output: columns is positive. The
        // original matrix is [inner, rows], retained in its row-major owner.
        let rows = b.ins().udiv(count, columns);
        let base = b.ins().imul(k, rows);
        b.ins().iadd(base, row)
    } else {
        b.ins().iadd(left_base, k)
    };
    let ri = if transposed[1] {
        let base = b.ins().imul(column, inner);
        b.ins().iadd(base, k)
    } else {
        let base = b.ins().imul(k, columns);
        b.ins().iadd(base, column)
    };
    let lp = address(b, left, li);
    let rp = address(b, right, ri);
    let a = b.ins().load(types::F64, MemFlags::new(), lp, 0);
    let a = if vector { b.ins().splat(kind, a) } else { a };
    let c = if transposed[1] && vector {
        let first = b.ins().load(types::F64, MemFlags::new(), rp, 0);
        let second_index = b.ins().iadd(ri, inner);
        let second_pointer = address(b, right, second_index);
        let second = b.ins().load(types::F64, MemFlags::new(), second_pointer, 0);
        let pair = b.ins().splat(kind, first);
        b.ins().insertlane(pair, second, 1)
    } else {
        b.ins().load(kind, MemFlags::new(), rp, 0)
    };
    let term = f64_arithmetic(b, NativeScalarOp::F64Mul, a, c);
    let prior = b.use_var(accumulator);
    let next = f64_arithmetic(b, NativeScalarOp::F64Add, prior, term);
    b.def_var(accumulator, next);
    let advanced = b.ins().iadd_imm(k, 1);
    b.def_var(offset, advanced);
    b.ins().jump(header, &[]);
    b.switch_to_block(finish);
    let result = b.use_var(accumulator);
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
    done: Block,
) {
    emit_with_views(
        b,
        left,
        right,
        output,
        count,
        columns,
        inner,
        [false, false],
        done,
    );
}

pub(super) fn emit_with_views(
    b: &mut FunctionBuilder<'_>,
    left: Value,
    right: Value,
    output: Value,
    count: Value,
    columns: Value,
    inner: Value,
    transposed: [bool; 2],
    done: Block,
) {
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
            b, left, right, output, at, columns, inner, vector, count, transposed,
        );
        let advanced = b.ins().iadd_imm(at, width);
        b.def_var(index, advanced);
        b.ins().jump(header, &[]);
    }
}
