//! Admission expressions are separate from actual traversal debits. Arithmetic
//! here is a guard: overflow selects the original outer header, never an error.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::compiler::backend;
use crate::native_artifact::program::prepare::{
    numeric_phase::Bound,
    numeric_plan::{Operation, Plan},
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::NativeIterationAction as Action;
pub(super) fn envelope(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    code: CodePlan<'_>,
    proof: &Plan,
    shadow: usize,
    root: u32,
    bound: &Bound,
) -> Result<(Value, Value), NativeCompileError> {
    let one = b.ins().iconst(types::I8, 1);
    let value = match bound {
        Bound::Constant(n) => b.ins().iconst(types::I64, *n as i64),
        Bound::Root(field) => {
            let word = proof
                .fields(root)
                .and_then(|f| f.get(*field))
                .copied()
                .flatten()
                .ok_or_else(|| backend("phase admission lacks its root scalar home"))?;
            words.read(b, shadow + word)
        }
        Bound::Length(source) => {
            let array = proof
                .actions
                .iter()
                .find_map(|(site, action)| {
                    let Operation::Length { array, .. } = action else {
                        return None;
                    };
                    let Action::LibraryCall { operands, .. } = code.blocks()[*site].action() else {
                        return None;
                    };
                    (operands.first()?.get() == *source).then_some(*array)
                })
                .ok_or_else(|| backend("phase admission lacks its proved array binding"))?;
            words.read(b, shadow + proof.view(array) + 1)
        }
        Bound::Add(a, c) | Bound::Multiply(a, c) | Bound::Maximum(a, c) => {
            let (a, a_valid) = envelope(b, words, code, proof, shadow, root, a)?;
            let (c, c_valid) = envelope(b, words, code, proof, shadow, root, c)?;
            let valid = b.ins().band(a_valid, c_valid);
            let (value, overflow) = match bound {
                Bound::Add(..) => b.ins().uadd_overflow(a, c),
                Bound::Multiply(..) => b.ins().umul_overflow(a, c),
                Bound::Maximum(..) => {
                    let greater = b.ins().icmp(IntCC::UnsignedGreaterThan, a, c);
                    return Ok((b.ins().select(greater, a, c), valid));
                }
                _ => unreachable!(),
            };
            let clear = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            return Ok((value, b.ins().band(valid, clear)));
        }
    };
    Ok((value, one))
}
