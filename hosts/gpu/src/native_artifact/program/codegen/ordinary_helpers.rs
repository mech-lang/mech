//! Compose adjacent source admission and ordinary fixed-lane callbacks.
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::protocol::GlobalHelper;
use mech_engine::native_artifact::{
    NativeControlLoopRef as Loop, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
};

pub(in crate::native_artifact) fn selected(
    code: CodePlan<'_>,
    site: usize,
) -> Option<GlobalHelper> {
    let CodePlan::Program(program) = code else {
        return None;
    };
    let metadata = program.metadata();
    let block = &code.blocks()[site];
    // Keep every native specialization and argument-transport ABI guard at
    // its established admission boundary. Selected actions below have only
    // infallible, in-bounds local word loads before their ordinary callback.
    if !matches!(block.terminator(), Terminator::Scalar(_))
        || metadata.prefix.is_some()
        || metadata.compositions.entry(site).is_some()
        || metadata.numeric_maps.at_reduction_finish(site).is_some()
        || metadata.numeric_maps.at_transpose_finish(site).is_some()
        || metadata.scalar_aggregates.action(site).is_some()
        || metadata
            .borrowed_strings
            .initializers
            .get(site)
            .copied()
            .flatten()
            .is_some()
        || metadata
            .borrowed_strings
            .actions
            .get(site)
            .copied()
            .flatten()
            .is_some()
        || metadata.map_admission.group(site).is_some()
        || metadata
            .closed_tails
            .sites
            .get(site)
            .copied()
            .flatten()
            .is_some()
        || metadata
            .frozen_maps
            .copies
            .get(site)
            .copied()
            .flatten()
            .is_some()
        || metadata
            .frozen_maps
            .reads
            .get(site)
            .copied()
            .flatten()
            .is_some()
        || metadata.map_owner_words.as_ref().is_some_and(|owner| {
            owner.initializer(site).is_some()
                || owner.exit(site).is_some()
                || owner.call_region(site).is_some()
                || owner.action_region(site).is_some()
                || owner.span_site(site).is_some()
        })
    {
        return None;
    }
    match block.action() {
        Action::PrepareU64Range { .. }
        | Action::CollectionBegin { .. }
        | Action::CollectionElement { .. }
        | Action::CollectionPush { .. }
        | Action::CollectionFinish { .. } => Some(GlobalHelper::AdmitCollection),
        Action::Initialize { iteration } => match code.iteration(*iteration as usize) {
            Loop::Collection(_) => Some(GlobalHelper::AdmitCollection),
            Loop::Iteration(loop_) if code.kind(loop_.carrier()).is_none() => {
                Some(GlobalHelper::AdmitMove)
            }
            Loop::Tail(loop_) if code.kind(loop_.carrier()).is_none() => {
                Some(GlobalHelper::AdmitMove)
            }
            _ => None,
        },
        // Counted latches have a native zero-counter ABI guard before their
        // move and retain separate admission. Tail latches have no such guard.
        Action::Latch { iteration } => match code.iteration(*iteration as usize) {
            Loop::Tail(loop_) if code.kind(loop_.carrier()).is_none() => {
                Some(GlobalHelper::AdmitMove)
            }
            _ => None,
        },
        Action::Structural { .. } => Some(GlobalHelper::AdmitStructuralObservation),
        Action::Scalar(Scalar::Copy { output, .. }) if code.kind(*output).is_none() => {
            Some(GlobalHelper::AdmitMove)
        }
        _ => None,
    }
}
