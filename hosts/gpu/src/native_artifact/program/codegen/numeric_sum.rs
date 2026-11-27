//! Ordered +0-seeded F64 reductions. SIMD lanes are independent outputs;
//! every lane retains the source term order, including empty inner axes.
use super::numeric_binary::{address, finite_result};
use crate::native_artifact::compiler::f64_arithmetic;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeScalarOp;

fn cell(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    index: Value,
    count: Value,
    inner: Value,
    axis: usize,
    vector: bool,
    cancel: Option<Block>,
) {
    let ty = if vector {
        types::F64.by(2).unwrap()
    } else {
        types::F64
    };
    let k = b.declare_var(types::I64);
    let sum = b.declare_var(ty);
    let zero = b.ins().iconst(types::I64, 0);
    let identity = b.ins().f64const(0.0);
    let identity = if vector {
        b.ins().splat(ty, identity)
    } else {
        identity
    };
    b.def_var(k, zero);
    b.def_var(sum, identity);
    let base = if axis == 1 {
        b.ins().imul(index, inner)
    } else {
        index
    };
    let header = b.create_block();
    let body = b.create_block();
    let finish = b.create_block();
    b.ins().jump(header, &[]);
    b.switch_to_block(header);
    let offset = b.use_var(k);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, offset, inner);
    b.ins().brif(more, body, &[], finish, &[]);
    b.switch_to_block(body);
    let ordinal = if axis == 0 {
        let row = b.ins().imul(offset, count);
        b.ins().iadd(row, base)
    } else {
        b.ins().iadd(base, offset)
    };
    let pointer = address(b, input, ordinal);
    let term = if vector && axis == 1 {
        let first = b.ins().load(types::F64, MemFlags::new(), pointer, 0);
        let next_row = b.ins().iadd(ordinal, inner);
        let pointer = address(b, input, next_row);
        let second = b.ins().load(types::F64, MemFlags::new(), pointer, 0);
        let pair = b.ins().splat(ty, first);
        b.ins().insertlane(pair, second, 1)
    } else {
        b.ins().load(ty, MemFlags::new(), pointer, 0)
    };
    let prior = b.use_var(sum);
    let next = f64_arithmetic(b, NativeScalarOp::F64Add, prior, term);
    b.def_var(sum, next);
    let next = b.ins().iadd_imm(offset, 1);
    b.def_var(k, next);
    b.ins().jump(header, &[]);
    b.switch_to_block(finish);
    let result = b.use_var(sum);
    finite_result(b, result, cancel);
    let destination = address(b, output, index);
    b.ins().store(MemFlags::new(), result, destination, 0);
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    output: Value,
    count: Value,
    inner: Value,
    axis: usize,
    cancel: Option<Block>,
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
    let remaining = b.ins().isub(count, at);
    let two = b
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 2);
    b.ins().brif(two, pair, &[], single, &[]);
    for (block, vector, width) in [(pair, true, 2), (single, false, 1)] {
        b.switch_to_block(block);
        cell(b, input, output, at, count, inner, axis, vector, cancel);
        let next = b.ins().iadd_imm(at, width);
        b.def_var(index, next);
        b.ins().jump(header, &[]);
    }
}
