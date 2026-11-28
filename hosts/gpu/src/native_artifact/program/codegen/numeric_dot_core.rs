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
        let pointer = at;
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
    let remaining = b.declare_var(types::I64);
    let sums = (0..left.len())
        .map(|_| b.declare_var(kind))
        .collect::<Vec<_>>();
    let streams = left.iter().copied().chain([right]).collect::<Vec<_>>();
    let positions = streams
        .iter()
        .map(|stream| {
            let position = b.declare_var(types::I64);
            b.def_var(position, stream.pointer);
            position
        })
        .collect::<Vec<_>>();
    let identity = b.ins().f64const(0.0);
    let identity = if vector {
        b.ins().splat(kind, identity)
    } else {
        identity
    };
    b.def_var(remaining, count);
    for &sum in &sums {
        b.def_var(sum, identity);
    }
    // All reached source loads are admitted immutable streams. Carry addresses
    // rather than rebuilding index*stride for every term. Integer address
    // additions are modular; an unused one-past address is never dereferenced.
    let term = |b: &mut FunctionBuilder<'_>, advance: bool| {
        let rp = b.use_var(positions[left.len()]);
        let rhs = right.load(b, rp, vector);
        for (at, (&stream, &sum)) in left.iter().zip(&sums).enumerate() {
            let pointer = b.use_var(positions[at]);
            let lhs = stream.load(b, pointer, vector);
            let product = f64_arithmetic(b, NativeScalarOp::F64Mul, lhs, rhs);
            let prior = b.use_var(sum);
            let next = f64_arithmetic(b, NativeScalarOp::F64Add, prior, product);
            b.def_var(sum, next);
        }
        if advance {
            for (&position, stream) in positions.iter().zip(&streams) {
                let pointer = b.use_var(position);
                let next = b.ins().iadd(pointer, stream.step);
                b.def_var(position, next);
            }
        }
    };
    let head = b.create_block();
    let body = b.create_block();
    let tail = b.create_block();
    let last = b.create_block();
    let done = b.create_block();
    b.ins().jump(head, &[]);
    b.switch_to_block(head);
    let rest = b.use_var(remaining);
    let pair = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, rest, 2);
    b.ins().brif(pair, body, &[], tail, &[]);
    b.switch_to_block(body);
    // Each accumulator observes k, then k+1, with separate rounded operations.
    // The first complete term precedes the second; there are no partial sums.
    term(b, true);
    term(b, true);
    let next = b.ins().iadd_imm(rest, -2);
    b.def_var(remaining, next);
    b.ins().jump(head, &[]);
    b.switch_to_block(tail);
    let rest = b.use_var(remaining);
    let nonempty = b.ins().icmp_imm(IntCC::NotEqual, rest, 0);
    b.ins().brif(nonempty, last, &[], done, &[]);
    b.switch_to_block(last);
    term(b, false);
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    sums.into_iter().map(|sum| b.use_var(sum)).collect()
}

#[cfg(test)]
#[path = "numeric_dot_core_tests.rs"]
mod tests;
