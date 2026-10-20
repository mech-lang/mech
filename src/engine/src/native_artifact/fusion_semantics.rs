//! Descriptive original semantic reads shared by every private fusion pass.
//! This neither validates execution authority nor evaluates any language node.
use super::iteration_validation::ControlPlan;
use super::{
    NativeCollectionResult as ResultKind, NativeControlLoopRef as Loop,
    NativeHistogramQueryStepKind as Kind, NativeIterationAction as Action,
    NativeIterationTerminator as Terminator, NativeMatchAction as Scalar,
    NativeMatchTerminator as Term, NativeMatchValue as Value,
};
use std::borrow::Cow;
const BAD: &str = "invalid native fusion semantic read inventory";
/// Reconstruct semantic actions only; never construct missing language Values.
/// All result strategies share this use inventory so one pass cannot hide an
/// escaping read from a later pass.
pub(crate) fn semantic_actions(plan: &impl ControlPlan) -> Result<Vec<Action>, &'static str> {
    plan.blocks()
        .iter()
        .map(|block| semantic_action(plan, block.action()).map(Cow::into_owned))
        .collect()
}
/// Immutable readers borrow ordinary actions; rewriting passes still request
/// the owned snapshot above. Virtual actions use the same checked expansion.
fn semantic_action<'a>(
    plan: &impl ControlPlan,
    action: &'a Action,
) -> Result<Cow<'a, Action>, &'static str> {
    let expanded = match action {
        Action::ProjectStringMapEntry { collection } => {
            let p = plan
                .collections()
                .get(*collection as usize)
                .and_then(|c| c.string_element_map_projection())
                .ok_or(BAD)?;
            Action::LibraryCall {
                template: p.selection_template().clone(),
                operands: vec![p.map(), p.element()].into(),
                output: p.output(),
            }
        }
        Action::PrepareStringSpanSelector { collection } => {
            let p = plan
                .collections()
                .get(*collection as usize)
                .and_then(|c| c.string_span_projection())
                .ok_or(BAD)?;
            Action::LibraryCall {
                template: p.range_template().clone(),
                operands: p.operands().as_slice().into(),
                output: p.selector(),
            }
        }
        Action::ProjectStringSpan { collection } => {
            let p = plan
                .collections()
                .get(*collection as usize)
                .and_then(|c| c.string_span_projection())
                .ok_or(BAD)?;
            Action::LibraryCall {
                template: p.selection_template().clone(),
                operands: vec![p.source(), p.selector()].into(),
                output: p.output(),
            }
        }
        Action::TextJoinFinish { collection } => {
            let c = plan.collections().get(*collection as usize).ok_or(BAD)?;
            let ResultKind::TextJoin(s) = c.result() else {
                return Err(BAD);
            };
            Action::LibraryCall {
                template: s.template().clone(),
                operands: vec![c.output(), s.separator()].into(),
                output: s.output(),
            }
        }
        Action::HistogramFinish { collection } => {
            let c = plan.collections().get(*collection as usize).ok_or(BAD)?;
            let ResultKind::Histogram(s) = c.result() else {
                return Err(BAD);
            };
            Action::LibraryCall {
                template: s.template().clone(),
                operands: vec![c.output()].into(),
                output: s.output(),
            }
        }
        Action::HistogramQueryStep { collection, step } => {
            let c = plan.collections().get(*collection as usize).ok_or(BAD)?;
            let ResultKind::GuardedHistogramQuery(s) = c.result() else {
                return Err(BAD);
            };
            let step = s.steps().get(*step as usize).ok_or(BAD)?;
            match step.kind() {
                Kind::Alias { .. } => {
                    let [source] = step.operands() else {
                        return Err(BAD);
                    };
                    if step.template().is_some() {
                        return Err(BAD);
                    }
                    Action::Scalar(Scalar::Copy {
                        source: *source,
                        output: step.output().ok_or(BAD)?,
                    })
                }
                Kind::Cleanup => {
                    if step.template().is_some()
                        || !step.operands().is_empty()
                        || step.output().is_some()
                    {
                        return Err(BAD);
                    }
                    Action::Scalar(Scalar::None)
                }
                _ => Action::LibraryCall {
                    template: step.template().ok_or(BAD)?.clone(),
                    operands: step.operands().into(),
                    output: step.output().ok_or(BAD)?,
                },
            }
        }
        action => return Ok(Cow::Borrowed(action)),
    };
    Ok(Cow::Owned(expanded))
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Read {
    Action(u32),
    Terminator(u32),
    Other,
}
pub(super) fn reads(
    plan: &impl ControlPlan,
    actions: &[Action],
) -> Result<Vec<Vec<Read>>, &'static str> {
    let mut all = vec![Vec::new(); plan.values().len()];
    visit_reads(
        plan,
        actions.iter().map(|action| Ok(Cow::Borrowed(action))),
        |value, usage| {
            all.get_mut(value.get() as usize).ok_or(BAD)?.push(usage);
            Ok(())
        },
    )?;
    Ok(all)
}
fn visit_reads<'a>(
    plan: &impl ControlPlan,
    actions: impl Iterator<Item = Result<Cow<'a, Action>, &'static str>>,
    mut read: impl FnMut(Value, Read) -> Result<(), &'static str>,
) -> Result<(), &'static str> {
    for (site, (block, action)) in plan.blocks().iter().zip(actions).enumerate() {
        let usage = Read::Action(site as u32);
        match action?.as_ref() {
            Action::LibraryCall { operands, .. }
            | Action::Scalar(Scalar::Primitive { operands, .. }) => {
                for &value in operands {
                    read(value, usage)?;
                }
            }
            Action::Scalar(Scalar::Copy { source, .. })
            | Action::CollectionPush { value: source, .. } => read(*source, usage)?,
            Action::Structural {
                source,
                observation,
                ..
            } => {
                read(*source, usage)?;
                if let super::NativeStructuralObservation::StringEqual { peer } = observation {
                    read(*peer, usage)?;
                }
            }
            Action::PrepareU64Range { iteration } => {
                if *iteration as usize >= plan.iteration_count() {
                    return Err(BAD);
                }
                let Loop::Collection(g) = plan.iteration(*iteration as usize) else {
                    return Err(BAD);
                };
                let super::NativeCollectionSource::U64Range(cursor) = g.source() else {
                    return Err(BAD);
                };
                for &value in cursor.operands() {
                    read(value, usage)?;
                }
            }
            Action::Scalar(Scalar::None)
            | Action::CollectionBegin { .. }
            | Action::CollectionElement { .. }
            | Action::CollectionFinish { .. }
            | Action::Initialize { .. }
            | Action::Latch { .. } => {}
            // semantic_actions must have expanded all virtual consumer actions.
            Action::TextJoinFinish { .. }
            | Action::HistogramFinish { .. }
            | Action::HistogramQueryStep { .. }
            | Action::PrepareStringSpanSelector { .. }
            | Action::ProjectStringSpan { .. }
            | Action::ProjectStringMapEntry { .. } => return Err(BAD),
        }
        let usage = Read::Terminator(site as u32);
        match block.terminator() {
            Terminator::Scalar(Term::Branch { condition, .. }) => read(condition, usage)?,
            Terminator::Scalar(Term::LiteralBranch { left, right, .. }) => {
                read(left, usage)?;
                read(right, usage)?;
            }
            _ => {}
        }
    }
    for id in 0..plan.iteration_count() {
        match plan.iteration(id) {
            Loop::Collection(g) => read(g.source().semantic_value(), Read::Other)?,
            Loop::Iteration(g) => {
                read(g.seed(), Read::Other)?;
                read(g.next(), Read::Other)?;
            }
            Loop::Tail(g) => {
                read(g.seed(), Read::Other)?;
                read(g.next(), Read::Other)?;
                read(g.returned(), Read::Other)?;
            }
        }
    }
    for (value, _) in plan.output_values().chain(plan.effect_values()) {
        read(value, Read::Other)?;
    }
    Ok(())
}
pub(crate) fn semantic_use_counts(plan: &impl ControlPlan) -> Result<Vec<usize>, &'static str> {
    let mut counts = vec![0usize; plan.values().len()];
    visit_reads(
        plan,
        plan.blocks()
            .iter()
            .map(|block| semantic_action(plan, block.action())),
        |value, usage| {
            if let Read::Action(site) | Read::Terminator(site) = usage
                && site as usize >= plan.blocks().len()
            {
                return Err(BAD);
            }
            let count = counts.get_mut(value.get() as usize).ok_or(BAD)?;
            *count = count.checked_add(1).ok_or(BAD)?;
            Ok(())
        },
    )?;
    Ok(counts)
}

#[cfg(all(
    test,
    feature = "source",
    feature = "artifact-codec",
    feature = "semantic-compiler",
    feature = "resident-artifact",
    feature = "pure_text",
    feature = "map_ops",
    feature = "convert"
))]
#[path = "fusion_semantics_tests.rs"]
mod tests;
