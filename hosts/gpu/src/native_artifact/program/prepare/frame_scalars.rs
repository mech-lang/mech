//! Exact structural scalar observations from the existing private frame bank.
use mech_engine::native_artifact::{
    NativeBufferFrameLane as Lane, NativeBufferFrameOperation as Operation, NativeBufferFrames,
    NativeIterationAction as Action, NativeMatchValue, NativeProgramArtifactPlan, NativeScalarKind,
};

pub(in crate::native_artifact) const READ_WORK: u64 = 64;

// Runtime-only callers need eligibility; emission consumes the exact lanes.
pub(in crate::native_artifact) type Read = (usize, NativeScalarKind, NativeMatchValue);

pub(in crate::native_artifact) fn read(
    plan: &NativeProgramArtifactPlan,
    frames: &NativeBufferFrames,
    site: usize,
) -> Option<Read> {
    // Selected record access retains its original factory admission. This
    // composes only the already exact structural observation proof.
    let Action::Structural { source, output, .. } = plan.blocks().get(site)?.action() else {
        return None;
    };
    let action = frames.action(site)?;
    // A last-use projection also retires managed owners after reading. Keep
    // that operation on its existing ordered helper path; a word load alone
    // cannot replace its ownership effect or its finite retirement charges.
    if !action.retire().is_empty() {
        return None;
    }
    let Operation::Project {
        source: frame,
        ordinal,
        output: target,
    } = action.operation()
    else {
        return None;
    };
    if source != frame || output != target {
        return None;
    }
    let field = frames.fields(*source)?.get(*ordinal)?;
    let Lane::Scalar { at, kind } = field.lane() else {
        return None;
    };
    let ty = &plan.values()[output.get() as usize];
    let word = at.checked_mul(2)?;
    (ty.schema() == field.schema() && ty.scalar_kind() == Some(kind))
        .then_some((word, kind, *output))
}
