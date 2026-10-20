//! Constructor-private guarded-query pass and opaque-plan rederivation.
//! Every selected semantic read is inventoried once in fusion_semantics.
#[cfg(not(all(feature = "map_ops", feature = "convert")))]
use super::NativeGuardedHistogramQuery as Sink;
use super::fusion_semantics::reads;
#[cfg(not(all(feature = "map_ops", feature = "convert")))]
use super::fusion_semantics::{Read, SemanticActions};
pub(crate) use super::fusion_semantics::{semantic_actions, semantic_use_counts};
use super::iteration_validation::ControlPlan;
use super::{
    NativeCollectionResult as ResultKind, NativeIterationAction as Action,
    NativeProgramArtifactPlan as Plan,
};
const BAD: &str = "invalid native guarded histogram query demand";
#[cfg(all(feature = "map_ops", feature = "convert"))]
#[path = "guarded_histogram_query_selected.rs"]
mod selected;
#[cfg(all(feature = "map_ops", feature = "convert"))]
use selected::candidate;
#[cfg(not(all(feature = "map_ops", feature = "convert")))]
fn candidate(
    _: &impl ControlPlan,
    _: &SemanticActions<'_>,
    _: &[Vec<Read>],
    _: usize,
) -> Option<Sink> {
    None
}

pub(super) fn validate(plan: &impl ControlPlan) -> Result<(), &'static str> {
    let actions = semantic_actions(plan)?;
    let uses = reads(plan, &actions)?;
    let mut sites = vec![false; actions.len()];
    for (id, data) in plan.collections().iter().enumerate() {
        let ResultKind::GuardedHistogramQuery(sink) = data.result() else {
            continue;
        };
        if candidate(plan, &actions, &uses, id).as_ref() != Some(sink) {
            return Err(BAD);
        }
        for (ordinal, step) in sink.steps().iter().enumerate() {
            let site = step.site() as usize;
            if core::mem::replace(sites.get_mut(site).ok_or(BAD)?, true)
                || !matches!(plan.blocks()[site].action(),Action::HistogramQueryStep {collection,step} if *collection as usize==id && *step as usize==ordinal)
            {
                return Err(BAD);
            }
        }
    }
    for (site, block) in plan.blocks().iter().enumerate() {
        if matches!(block.action(), Action::HistogramQueryStep { .. }) != sites[site] {
            return Err(BAD);
        }
    }
    Ok(())
}
pub(crate) fn lower(plan: &mut Plan) -> Result<(), &'static str> {
    let actions = semantic_actions(plan)?.into_owned();
    let uses = reads(plan, &actions)?;
    for id in 0..plan.collections.len() {
        if !matches!(plan.collections[id].result(), ResultKind::Materialized) {
            continue;
        }
        let Some(sink) = candidate(plan, &actions, &uses, id) else {
            continue;
        };
        let originals = sink
            .steps()
            .iter()
            .map(|step| {
                (
                    step.site() as usize,
                    plan.blocks[step.site() as usize].action.clone(),
                )
            })
            .collect::<Vec<_>>();
        for (ordinal, step) in sink.steps().iter().enumerate() {
            plan.blocks[step.site() as usize].action = Action::HistogramQueryStep {
                collection: id as u32,
                step: ordinal as u32,
            };
        }
        plan.collections[id].result = ResultKind::GuardedHistogramQuery(sink);
        // This ordinary proof adds a real query read BEFORE begin. No late
        // query construction, shape failure or lazy computation is hoisted.
        if plan.validate_current_visits().is_err() {
            plan.collections[id].result = ResultKind::Materialized;
            for (site, action) in originals {
                plan.blocks[site].action = action;
            }
        }
    }
    Ok(())
}

#[cfg(all(
    test,
    feature = "map_ops",
    feature = "convert",
    feature = "pure_text",
    feature = "source",
    feature = "artifact-codec",
    feature = "semantic-compiler",
    feature = "resident-artifact"
))]
#[path = "guarded_histogram_query_tests.rs"]
mod tests;
