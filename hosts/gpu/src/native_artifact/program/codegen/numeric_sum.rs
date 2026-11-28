//! Ordered +0-seeded F64 reductions. SIMD lanes are independent outputs;
//! every lane retains the source term order, including empty inner axes.
use super::numeric_binary::address;
use crate::native_artifact::compiler::f64_arithmetic;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeScalarOp;

/// A compile-time term emitter. Implementations emit direct typed arithmetic,
/// never a runtime operation table. Lanes always name independent outputs.
pub(super) trait Terms {
    fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        row: Value,
        column: Value,
        index: Value,
        vector: bool,
    ) -> Value;
}
struct Input(Value);
impl Terms for Input {
    fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        _row: Value,
        _column: Value,
        index: Value,
        vector: bool,
    ) -> Value {
        let pointer = address(b, self.0, index);
        let ty = if vector {
            types::F64.by(2).unwrap()
        } else {
            types::F64
        };
        b.ins().load(ty, MemFlags::new(), pointer, 0)
    }
}
fn cell(
    b: &mut FunctionBuilder<'_>,
    terms: &impl Terms,
    output: Value,
    index: Value,
    count: Value,
    inner: Value,
    axis: usize,
    vector: bool,
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
    let (row, column) = if axis == 0 {
        (offset, index)
    } else {
        (index, offset)
    };
    let term = if vector && axis == 1 {
        let first = terms.emit(b, row, column, ordinal, false);
        let next_ordinal = b.ins().iadd(ordinal, inner);
        let next_row = b.ins().iadd_imm(row, 1);
        let second = terms.emit(b, next_row, column, next_ordinal, false);
        let pair = b.ins().splat(ty, first);
        b.ins().insertlane(pair, second, 1)
    } else {
        terms.emit(b, row, column, ordinal, vector)
    };
    let prior = b.use_var(sum);
    let next = f64_arithmetic(b, NativeScalarOp::F64Add, prior, term);
    b.def_var(sum, next);
    let next = b.ins().iadd_imm(offset, 1);
    b.def_var(k, next);
    b.ins().jump(header, &[]);
    b.switch_to_block(finish);
    let result = b.use_var(sum);
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
    done: Block,
) {
    emit_terms(b, output, count, inner, axis, &Input(input), done);
}
pub(super) fn emit_terms(
    b: &mut FunctionBuilder<'_>,
    output: Value,
    count: Value,
    inner: Value,
    axis: usize,
    terms: &impl Terms,
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
        cell(b, terms, output, at, count, inner, axis, vector);
        let next = b.ins().iadd_imm(at, width);
        b.def_var(index, next);
        b.ins().jump(header, &[]);
    }
}
