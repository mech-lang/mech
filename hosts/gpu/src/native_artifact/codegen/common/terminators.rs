//! Ordinary source terminators shared by scalar and packed SSA strategies.
use super::ssa_words::Words;
use crate::native_artifact::iteration_code_plan::CodePlan;
use cranelift_codegen::ir::{
    Block, InstBuilder, MemFlags,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationTerminator as Term, NativeMatchTerminator as Branch, NativeScalarKind,
};
pub(in crate::native_artifact) fn emit_source_terminator(
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    code: CodePlan<'_>,
    terminator: Term,
    fail_at: Block,
    target: impl Fn(u32) -> Block,
) {
    match terminator {
        Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => {
            b.ins().jump(target(body), &[]);
        }
        Term::Scalar(Branch::Branch { condition, yes, no }) => {
            let value = words.read(b, condition.get() as usize);
            b.ins().brif(value, target(yes), &[], target(no), &[]);
        }
        Term::Scalar(Branch::LiteralBranch {
            left,
            right,
            yes,
            no,
        }) => {
            let kind = code.kind(left).unwrap();
            let left = words.read(b, left.get() as usize);
            let right = words.read(b, right.get() as usize);
            let equal = if kind == NativeScalarKind::F64 {
                let left = b.ins().bitcast(types::F64, MemFlags::new(), left);
                let right = b.ins().bitcast(types::F64, MemFlags::new(), right);
                b.ins().fcmp(FloatCC::Equal, left, right)
            } else {
                b.ins().icmp(IntCC::Equal, left, right)
            };
            b.ins().brif(equal, target(yes), &[], target(no), &[]);
        }
        Term::Scalar(Branch::Reject) => {
            // Exhaustive source matches retain their fail-closed default
            // arm. Preserve its origin and unwind the owned frame/fuel.
            let status = b.ins().iconst(
                types::I64,
                crate::native_artifact::control_abi::STATUS_REJECT as i64,
            );
            b.ins().jump(fail_at, &[status.into()]);
        }
        _ => unreachable!("checked packed SSA terminator"),
    }
}
