//! Source-selected F64 matrix loops over one checked synchronous loan.
//! Flat scalar/exact broadcasts use independent SIMD lanes, with scalar tails.
use super::hooks::Hooks;
use crate::native_artifact::program::{
    prepare::numeric_binary::{Kind, Operation},
    protocol::{GlobalHelper, abi::NumericBinaryFrame as Frame},
};
use crate::native_artifact::{
    NativeCompileError,
    compiler::{backend, f64_arithmetic, guard},
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{NativeMatchValue, NativeScalarOp};
use std::mem::offset_of;

fn get(b: &mut FunctionBuilder<'_>, frame: Value, offset: usize) -> Value {
    b.ins()
        .load(types::I64, MemFlags::new(), frame, offset as i32)
}
pub(super) fn address(b: &mut FunctionBuilder<'_>, pointer: Value, index: Value) -> Value {
    let bytes = b.ins().imul_imm(index, 8);
    b.ins().iadd(pointer, bytes)
}
fn scalar_at(
    b: &mut FunctionBuilder<'_>,
    pointer: Value,
    scalar: Value,
    mode: Value,
    index: Value,
    columns: Value,
) -> Value {
    let plain = b.create_block();
    let matrix = b.create_block();
    let done = b.create_block();
    b.append_block_param(done, types::F64);
    let is_scalar = b.ins().icmp_imm(IntCC::Equal, mode, 0);
    b.ins().brif(is_scalar, plain, &[], matrix, &[]);
    b.switch_to_block(plain);
    let value = b.ins().bitcast(types::F64, MemFlags::new(), scalar);
    b.ins().jump(done, &[value.into()]);
    b.switch_to_block(matrix);
    // A nonempty output certifies nonzero columns. Every mode and physical
    // span was checked against current canonical geometry before this loan.
    let row = b.ins().udiv(index, columns);
    let column = b.ins().urem(index, columns);
    let is_column = b.ins().icmp_imm(IntCC::Equal, mode, 2);
    let coordinate = b.ins().select(is_column, row, column);
    let exact = b.ins().icmp_imm(IntCC::Equal, mode, 1);
    let ordinal = b.ins().select(exact, index, coordinate);
    let pointer = address(b, pointer, ordinal);
    let value = b.ins().load(types::F64, MemFlags::new(), pointer, 0);
    b.ins().jump(done, &[value.into()]);
    b.switch_to_block(done);
    b.block_params(done)[0]
}
fn flat(
    b: &mut FunctionBuilder<'_>,
    op: NativeScalarOp,
    left: Value,
    right: Value,
    sleft: Value,
    sright: Value,
    output: Value,
    count: Value,
    left_scalar: bool,
    right_scalar: bool,
    done: Block,
) {
    let vector_type = types::F64.by(2).expect("two F64 lanes");
    let scalar_left = b.ins().bitcast(types::F64, MemFlags::new(), sleft);
    let scalar_right = b.ins().bitcast(types::F64, MemFlags::new(), sright);
    let splat_left = left_scalar.then(|| b.ins().splat(vector_type, scalar_left));
    let splat_right = right_scalar.then(|| b.ins().splat(vector_type, scalar_right));
    let index = b.declare_var(types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.def_var(index, zero);
    let header = b.create_block();
    let body = b.create_block();
    let tail = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let at = b.use_var(index);
    let remaining = b.ins().isub(count, at);
    let pair = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    b.ins().brif(pair, body, &[], tail, &[]);
    b.switch_to_block(body);
    let a = if let Some(value) = splat_left {
        value
    } else {
        let p = address(b, left, at);
        b.ins().load(vector_type, MemFlags::new(), p, 0)
    };
    let c = if let Some(value) = splat_right {
        value
    } else {
        let p = address(b, right, at);
        b.ins().load(vector_type, MemFlags::new(), p, 0)
    };
    let next = f64_arithmetic(b, op, a, c);
    let p = address(b, output, at);
    b.ins().store(MemFlags::new(), next, p, 0);
    let advanced = b.ins().iadd_imm(at, 2);
    b.def_var(index, advanced);
    b.ins().jump(header, &[]);
    b.switch_to_block(tail);
    let at = b.use_var(index);
    let has_tail = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    let single = b.create_block();
    b.ins().brif(has_tail, single, &[], done, &[]);
    b.switch_to_block(single);
    let a = if left_scalar {
        scalar_left
    } else {
        let p = address(b, left, at);
        b.ins().load(types::F64, MemFlags::new(), p, 0)
    };
    let c = if right_scalar {
        scalar_right
    } else {
        let p = address(b, right, at);
        b.ins().load(types::F64, MemFlags::new(), p, 0)
    };
    let next = f64_arithmetic(b, op, a, c);
    let p = address(b, output, at);
    b.ins().store(MemFlags::new(), next, p, 0);
    b.ins().jump(done, &[]);
}
/// Validate both physical spans before any typed unary kernel access. Current
/// owner/shape authority comes from the exact synchronous prepare/finish loan.
fn unary_spans(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    input_count: Value,
    output_count: Value,
    error: Block,
) {
    let mut ends = Vec::new();
    for (pointer, count) in [(input, input_count), (output, output_count)] {
        let good = b.ins().icmp_imm(
            IntCC::UnsignedLessThanOrEqual,
            count,
            (isize::MAX as u64 / 8) as i64,
        );
        guard(
            b,
            good,
            error,
            crate::native_artifact::abi::STATUS_INTEGRITY,
        );
        let empty = b.ins().icmp_imm(IntCC::Equal, count, 0);
        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
        let good = b.ins().bor(empty, nonnull);
        guard(
            b,
            good,
            error,
            crate::native_artifact::abi::STATUS_INTEGRITY,
        );
        let low = b.ins().band_imm(pointer, 7);
        let good = b.ins().icmp_imm(IntCC::Equal, low, 0);
        guard(
            b,
            good,
            error,
            crate::native_artifact::abi::STATUS_INTEGRITY,
        );
        let bytes = b.ins().imul_imm(count, 8);
        let (end, overflow) = b.ins().uadd_overflow(pointer, bytes);
        let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
        guard(
            b,
            good,
            error,
            crate::native_artifact::abi::STATUS_INTEGRITY,
        );
        ends.push(end);
    }
    let before = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, ends[0], output);
    let after = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, ends[1], input);
    let good = b.ins().bor(before, after);
    guard(
        b,
        good,
        error,
        crate::native_artifact::abi::STATUS_INTEGRITY,
    );
}
/// Leaves the builder at the original ordinary call on refusal. The returned
/// join is reached only after that call or successful native publication.
pub(in crate::native_artifact) fn emit_entry(
    b: &mut FunctionBuilder<'_>,
    hooks: &Hooks,
    site: usize,
    op: &Operation,
    operands: &[NativeMatchValue],
    locals: Value,
    is_scalar: impl Fn(NativeMatchValue) -> bool,
    product_row_pairs: bool,
    product_row_quartets: bool,
    transpose_tiles: bool,
    error: Block,
) -> Result<Block, NativeCompileError> {
    if operands.iter().map(|v| v.get()).collect::<Vec<_>>() != op.operands {
        return Err(backend("matrix arithmetic source operands differ"));
    }
    hooks.library_operation(
        b,
        site,
        GlobalHelper::NumericBinaryPrepare.raw(),
        operands,
        locals,
        is_scalar,
        error,
    )?;
    let frame = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let native = b.create_block();
    let ordinary = b.create_block();
    let joined = b.create_block();
    b.ins().brif(frame, native, &[], ordinary, &[]);
    b.switch_to_block(native);
    let left = get(b, frame, offset_of!(Frame, left));
    let right = get(b, frame, offset_of!(Frame, right));
    let output = get(b, frame, offset_of!(Frame, output));
    let count = get(b, frame, offset_of!(Frame, count));
    let columns = get(b, frame, offset_of!(Frame, columns));
    let lmode = get(b, frame, offset_of!(Frame, left_mode));
    let rmode = get(b, frame, offset_of!(Frame, right_mode));
    let sleft = get(b, frame, offset_of!(Frame, left_scalar));
    let sright = get(b, frame, offset_of!(Frame, right_scalar));
    let complete = b.create_block();
    match op.kind {
        Kind::Transpose { input } => {
            let source_columns = get(b, frame, offset_of!(Frame, inner));
            let (expected, overflow) = b.ins().umul_overflow(columns, source_columns);
            let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            guard(
                b,
                good,
                error,
                crate::native_artifact::abi::STATUS_INTEGRITY,
            );
            let good = b.ins().icmp(IntCC::Equal, expected, count);
            guard(
                b,
                good,
                error,
                crate::native_artifact::abi::STATUS_INTEGRITY,
            );
            unary_spans(b, left, output, count, count, error);
            let mut output_columns = columns;
            let mut input_columns = source_columns;
            for (known, actual, target) in [
                (input[0], columns, &mut output_columns),
                (input[1], source_columns, &mut input_columns),
            ] {
                if let Some(known) = known {
                    let good = b.ins().icmp_imm(IntCC::Equal, actual, known as i64);
                    guard(
                        b,
                        good,
                        error,
                        crate::native_artifact::abi::STATUS_INTEGRITY,
                    );
                    *target = b.ins().iconst(types::I64, known as i64);
                }
            }
            let emit = if transpose_tiles {
                super::numeric_transpose::emit
            } else {
                super::numeric_transpose::emit_ordinal
            };
            emit(
                b,
                left,
                output,
                count,
                output_columns,
                input_columns,
                complete,
            );
        }
        Kind::Sum { axis, input } => {
            let mut inner = get(b, frame, offset_of!(Frame, inner));
            let mut outputs = count;
            let expected_columns = if axis == 0 {
                count
            } else {
                b.ins().iconst(types::I64, 1)
            };
            let good = b.ins().icmp(IntCC::Equal, columns, expected_columns);
            guard(
                b,
                good,
                error,
                crate::native_artifact::abi::STATUS_INTEGRITY,
            );
            let (input_count, overflow) = b.ins().umul_overflow(count, inner);
            let good = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            guard(
                b,
                good,
                error,
                crate::native_artifact::abi::STATUS_INTEGRITY,
            );
            unary_spans(b, left, output, input_count, count, error);
            for (known, actual, target) in [
                (input[axis], inner, &mut inner),
                (input[1 - axis], count, &mut outputs),
            ] {
                if let Some(known) = known {
                    let good = b.ins().icmp_imm(IntCC::Equal, actual, known as i64);
                    guard(
                        b,
                        good,
                        error,
                        crate::native_artifact::abi::STATUS_INTEGRITY,
                    );
                    *target = b.ins().iconst(types::I64, known as i64);
                }
            }
            super::numeric_sum::emit(b, left, output, outputs, inner, axis, complete);
        }
        Kind::Product => {
            let inner = get(b, frame, offset_of!(Frame, inner));
            super::numeric_product::emit(
                b,
                left,
                right,
                output,
                count,
                columns,
                inner,
                product_row_pairs,
                product_row_quartets,
                complete,
            );
        }
        Kind::Elementwise(op) => {
            let flat_entry = b.create_block();
            let strided = b.create_block();
            let lf = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, lmode, 1);
            let rf = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, rmode, 1);
            let both = b.ins().band(lf, rf);
            b.ins().brif(both, flat_entry, &[], strided, &[]);
            b.switch_to_block(flat_entry);
            let ls = b.ins().icmp_imm(IntCC::Equal, lmode, 0);
            let rs = b.ins().icmp_imm(IntCC::Equal, rmode, 0);
            let lscalar = b.create_block();
            let lexact = b.create_block();
            b.ins().brif(ls, lscalar, &[], lexact, &[]);
            for (block, scalar) in [(lscalar, true), (lexact, false)] {
                b.switch_to_block(block);
                let rscalar = b.create_block();
                let rexact = b.create_block();
                b.ins().brif(rs, rscalar, &[], rexact, &[]);
                for (block, rscalar) in [(rscalar, true), (rexact, false)] {
                    b.switch_to_block(block);
                    flat(
                        b, op, left, right, sleft, sright, output, count, scalar, rscalar, complete,
                    );
                }
            }
            b.switch_to_block(strided);
            let index = b.declare_var(types::I64);
            let zero = b.ins().iconst(types::I64, 0);
            b.def_var(index, zero);
            let header = b.create_block();
            let body = b.create_block();
            b.ins().jump(header, &[]);
            b.switch_to_block(header);
            let at = b.use_var(index);
            let valid = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
            b.ins().brif(valid, body, &[], complete, &[]);
            b.switch_to_block(body);
            let a = scalar_at(b, left, sleft, lmode, at, columns);
            let c = scalar_at(b, right, sright, rmode, at, columns);
            let next = f64_arithmetic(b, op, a, c);
            let p = address(b, output, at);
            b.ins().store(MemFlags::new(), next, p, 0);
            let next = b.ins().iadd_imm(at, 1);
            b.def_var(index, next);
            b.ins().jump(header, &[]);
        }
    }
    b.switch_to_block(complete);
    let zero = b.ins().iconst(types::I64, 0);
    hooks.call(
        b,
        site,
        GlobalHelper::NumericBinaryFinish,
        [frame, count, zero],
        error,
    );
    let ready = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
    let valid = b.ins().icmp_imm(IntCC::Equal, ready, 1);
    guard(
        b,
        valid,
        error,
        crate::native_artifact::abi::STATUS_INTEGRITY,
    );
    b.ins().jump(joined, &[]);
    b.switch_to_block(ordinary);
    Ok(joined)
}
