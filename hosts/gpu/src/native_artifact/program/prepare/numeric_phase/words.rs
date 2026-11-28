//! Resolve scalar observations to their current carrier coordinate. Copies
//! defined outside this traversal are not evidence about its current state.
use mech_engine::native_artifact::{
    NativeBufferFrameOperation as Frame, NativeBufferFrames, NativeIterationAction as Action,
    NativeMatchAction as Scalar, NativeProgramArtifactPlan as Plan, NativeProgramFacts,
    NativeScalarAggregateAction as Aggregate, NativeScalarAggregates, NativeTailLoop,
};
use std::collections::BTreeSet;

pub(super) struct Words<'a, 'p> {
    pub plan: &'p Plan,
    pub facts: &'a NativeProgramFacts<'p>,
    pub frames: &'a NativeBufferFrames,
    pub aggregates: &'a NativeScalarAggregates,
    pub tail: &'a NativeTailLoop,
    pub iteration: u32,
}

impl Words<'_, '_> {
    pub(super) fn carrier(&self, value: u32) -> Option<usize> {
        self.scalar(value, &mut BTreeSet::new())
    }

    pub(super) fn coordinate(&self, value: u32, field: usize) -> Option<usize> {
        self.field(value, field, &mut BTreeSet::new())
    }

    /// Exact root copies retain the current enum identity. A payload projection
    /// at offset zero does not, even when its first scalar has the same bits.
    pub(super) fn same_carrier(&self, mut value: u32) -> bool {
        let mut seen = BTreeSet::new();
        while value != self.tail.carrier().get() {
            if !seen.insert(value) {
                return false;
            }
            let Some(site) = self.definition(value) else {
                return false;
            };
            let source = match self.frames.action(site).map(|a| a.operation()) {
                Some(Frame::Copy { source, output }) if output.get() == value => Some(*source),
                _ => match self.plan.blocks()[site].action() {
                    Action::Scalar(Scalar::Copy { source, output }) if output.get() == value => {
                        Some(*source)
                    }
                    _ => None,
                },
            };
            let Some(source) = source else { return false };
            value = source.get();
        }
        true
    }

    /// Original structural coordinates, independent of normalized prefix homes.
    pub(super) fn framed_carrier(&self, mut value: u32, tag: Option<u32>) -> Option<usize> {
        use mech_engine::native_artifact::{
            NativeStructuralObservation as Observation, NativeStructuralStep as Step,
        };
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(value) {
                return None;
            }
            let site = self.definition(value)?;
            if let Some(action) = self.frames.action(site) {
                match action.operation() {
                    Frame::Observe {
                        source,
                        path,
                        observation,
                        output,
                    } if output.get() == value
                        && self.same_carrier(source.get())
                        && matches!(
                            observation,
                            Observation::U64 | Observation::CanonicalValue
                        ) =>
                    {
                        return match (tag, &path[..]) {
                            (
                                Some(tag),
                                [
                                    Step::EnumPayload { ordinal },
                                    Step::TupleItem { ordinal: field },
                                ],
                            ) if tag == *ordinal => Some(*field as usize),
                            (None, [Step::TupleItem { ordinal }]) => Some(*ordinal as usize),
                            _ => None,
                        };
                    }
                    Frame::Project {
                        source,
                        ordinal,
                        output,
                    } if tag.is_none()
                        && output.get() == value
                        && self.same_carrier(source.get()) =>
                    {
                        return Some(*ordinal);
                    }
                    _ => {}
                }
            }
            if let Action::Scalar(Scalar::Copy { source, output }) =
                self.plan.blocks()[site].action()
                && output.get() == value
            {
                value = source.get();
                continue;
            }
            return None;
        }
    }

    fn definition(&self, value: u32) -> Option<usize> {
        let [definition] = self.facts.definition_sites(value).ok()? else {
            return None;
        };
        let site = definition.site()? as usize;
        (self.plan.blocks()[site].iteration() == Some(self.iteration)).then_some(site)
    }

    fn scalar(&self, value: u32, active: &mut BTreeSet<(u32, Option<usize>)>) -> Option<usize> {
        if !active.insert((value, None)) {
            return None;
        }
        let answer = (|| {
            let site = self.definition(value)?;
            if let Some(Aggregate::Observe {
                source,
                offset,
                ordinal: None,
                output,
                ..
            }) = self.aggregates.action(site)
                && output.get() == value
            {
                return self.field(source.get(), *offset, active);
            }
            if let Some(frame) = self.frames.action(site)
                && let Frame::Project {
                    source,
                    ordinal,
                    output,
                } = frame.operation()
                && output.get() == value
            {
                return self.field(source.get(), *ordinal, active);
            }
            if let Action::Scalar(Scalar::Copy { source, output }) =
                self.plan.blocks()[site].action()
                && output.get() == value
            {
                return self.scalar(source.get(), active);
            }
            None
        })();
        active.remove(&(value, None));
        answer
    }

    fn field(
        &self,
        value: u32,
        ordinal: usize,
        active: &mut BTreeSet<(u32, Option<usize>)>,
    ) -> Option<usize> {
        if value == self.tail.carrier().get() {
            return Some(ordinal);
        }
        if !active.insert((value, Some(ordinal))) {
            return None;
        }
        let answer = (|| {
            let site = self.definition(value)?;
            match self.aggregates.action(site) {
                Some(Aggregate::Copy { source, output }) if output.get() == value => {
                    return self.field(source.get(), ordinal, active);
                }
                Some(Aggregate::Project {
                    source,
                    offset,
                    output,
                    ..
                }) if output.get() == value => {
                    return self.field(source.get(), offset.checked_add(ordinal)?, active);
                }
                _ => {}
            }
            if let Some(frame) = self.frames.action(site)
                && let Frame::Copy { source, output } = frame.operation()
                && output.get() == value
            {
                return self.field(source.get(), ordinal, active);
            }
            None
        })();
        active.remove(&(value, Some(ordinal)));
        answer
    }
}
