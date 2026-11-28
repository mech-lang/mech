//! Strict ordered reductions shared by products and source-proved dot maps.
//! Streams are compiler SSA recipes, never runtime dispatch or storage. The
//! caller proves each reached load's full bounds and immutable loan lifetime.
use crate::native_artifact::compiler::f64_arithmetic;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeScalarOp;

#[derive(Clone, Copy)]
pub(super) struct Stream {
    pub pointer: Value,
    /// Signed byte step between consecutive original reduction terms.
    pub step: Value,
    /// Adjacent independent output columns are contiguous at this input.
    pub vector: bool,
}
impl Stream {
    fn load(self, b: &mut FunctionBuilder<'_>, at: Value, vector: bool) -> Value {
        let offset = b.ins().imul(at, self.step);
        let pointer = b.ins().iadd(self.pointer, offset);
        let packed = vector && self.vector;
        let kind = if packed {
            types::F64.by(2).unwrap()
        } else {
            types::F64
        };
        let value = b.ins().load(kind, MemFlags::new(), pointer, 0);
        if vector && !packed {
            b.ins().splat(types::F64.by(2).unwrap(), value)
        } else {
            value
        }
    }
}
/// One reached right load serves up to four independent left-row sums. SIMD
/// lanes represent separate outputs. Every output keeps original +0, term
/// order, separate rounded multiply and add; no partial or horizontal sums.
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    left: &[Stream],
    right: Stream,
    count: Value,
    vector: bool,
) -> Vec<Value> {
    assert!((1..=4).contains(&left.len()));
    let kind = if vector {
        types::F64.by(2).unwrap()
    } else {
        types::F64
    };
    let cursor = b.declare_var(types::I64);
    let sums = (0..left.len())
        .map(|_| b.declare_var(kind))
        .collect::<Vec<_>>();
    let zero = b.ins().iconst(types::I64, 0);
    let identity = b.ins().f64const(0.0);
    let identity = if vector {
        b.ins().splat(kind, identity)
    } else {
        identity
    };
    b.def_var(cursor, zero);
    for &sum in &sums {
        b.def_var(sum, identity);
    }
    let head = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[]);
    b.switch_to_block(head);
    let at = b.use_var(cursor);
    let more = b.ins().icmp(IntCC::UnsignedLessThan, at, count);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let rhs = right.load(b, at, vector);
    for (&stream, &sum) in left.iter().zip(&sums) {
        let lhs = stream.load(b, at, vector);
        let term = f64_arithmetic(b, NativeScalarOp::F64Mul, lhs, rhs);
        let prior = b.use_var(sum);
        let next = f64_arithmetic(b, NativeScalarOp::F64Add, prior, term);
        b.def_var(sum, next);
    }
    let next = b.ins().iadd_imm(at, 1);
    b.def_var(cursor, next);
    b.ins().jump(head, &[]);
    b.switch_to_block(done);
    sums.into_iter().map(|sum| b.use_var(sum)).collect()
}

#[cfg(test)]
#[path = "numeric_dot_core_tests.rs"]
mod tests;
