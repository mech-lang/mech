//! Compose adjacent admission and zero-word callbacks without moving a check.
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::protocol::GlobalHelper;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeMatchAction as Scalar,
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
    // Header/initializer/latch work, argument transport, and every selected
    // native specialization retain their original admission boundary. Only
    // the ordinary action below can run between these two callbacks.
    if !matches!(block.terminator(), Terminator::Scalar(_))
        || metadata.prefix.is_some()
        || metadata.compositions.entry(site).is_some()
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
        Action::Structural { .. } => Some(GlobalHelper::AdmitStructuralObservation),
        Action::Scalar(Scalar::Copy { output, .. }) if code.kind(*output).is_none() => {
            Some(GlobalHelper::AdmitMove)
        }
        _ => None,
    }
}
