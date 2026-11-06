//! Current traversal reachability. Nested children are single synchronous edges;
//! the only omitted cycle is the source loop's declared backedge.
use super::*;
use mech_engine::native_artifact::{NativeMatchValue, NativeScalarAggregateAction as Aggregate};

pub(super) struct Flow<'a> {
    pub plan: &'a Plan,
    pub frames: &'a NativeBufferFrames,
    pub aggregates: &'a NativeScalarAggregates,
    pub forest: &'a Forest,
    pub loop_: &'a Loop,
}
impl Flow<'_> {
    pub fn tail(&self) -> &NativeTailLoop {
        let NativeControlLoop::Tail(tail) = &self.plan.iterations()[self.loop_.iteration as usize]
        else {
            unreachable!()
        };
        tail
    }
    pub fn reaches(&self, target: usize, avoid: Option<(usize, usize)>, tag: Option<u64>) -> bool {
        self.reaches_with(target, avoid, tag, None)
    }
    fn reaches_with(
        &self,
        target: usize,
        avoid: Option<(usize, usize)>,
        tag: Option<u64>,
        predicate: Option<(u32, bool)>,
    ) -> bool {
        let mut pending = vec![self.loop_.header];
        let mut seen = BTreeSet::new();
        while let Some(site) = pending.pop() {
            if site == target {
                return true;
            }
            if avoid == Some((site, usize::MAX)) {
                continue;
            }
            if !seen.insert(site) || site == self.loop_.latch || site == self.loop_.exit {
                continue;
            }
            if let Some(child) = self
                .forest
                .loops
                .iter()
                .find(|l| l.parent == Some(self.loop_.iteration) && l.initialize == site)
            {
                pending.push(child.exit);
                continue;
            }
            let term = self.plan.blocks()[site].terminator();
            let known = match term {
                Term::Scalar(Branch::Branch { condition, yes, no }) => self
                    .boolean(condition, tag, predicate)
                    .map(|b| if b { yes } else { no }),
                Term::Scalar(Branch::LiteralBranch {
                    left,
                    right,
                    yes,
                    no,
                }) => self
                    .boolean(left, tag, predicate)
                    .zip(self.boolean(right, tag, predicate))
                    .map(|(a, b)| if a == b { yes } else { no }),
                _ => None,
            };
            let successors = known.map_or_else(
                || {
                    NativeProgramFacts::control_successors(term)
                        .into_iter()
                        .flatten()
                        .collect()
                },
                |s| vec![s],
            );
            pending.extend(
                successors
                    .into_iter()
                    .map(|s| s as usize)
                    .filter(|&next| avoid != Some((site, next))),
            );
        }
        false
    }
    fn boolean(
        &self,
        mut value: NativeMatchValue,
        tag: Option<u64>,
        predicate: Option<(u32, bool)>,
    ) -> Option<bool> {
        let facts = self.plan.facts();
        let mut seen = BTreeSet::new();
        loop {
            if let Some((expected, truth)) = predicate
                && value.get() == expected
            {
                return Some(truth);
            }
            if !seen.insert(value.get()) {
                return None;
            }
            if let Some(c) = self.plan.constants().iter().find(|c| c.value() == value)
                && let mech_core::ValueData::Bool(b) = c.constant().data()
            {
                return Some(*b);
            }
            if let Some(source) = facts.unique_identity_copy(value) {
                value = source;
                continue;
            }
            let [d] = facts.definition_sites(value.get()).ok()? else {
                return None;
            };
            let site = d.site()? as usize;
            if self.plan.blocks()[site].iteration() != Some(self.loop_.iteration) {
                return None;
            }
            if let Some(Aggregate::Observe {
                source,
                offset,
                ordinal: Some(ordinal),
                output,
                guards,
            }) = self.aggregates.action(site)
                && *output == value
                && guards.is_empty()
                && *offset == 0
            {
                let resolver = words::Words {
                    plan: self.plan,
                    facts: &facts,
                    frames: self.frames,
                    aggregates: self.aggregates,
                    tail: self.tail(),
                    iteration: self.loop_.iteration,
                };
                if resolver.coordinate(source.get(), 0) == Some(0) {
                    return tag.map(|tag| tag == u64::from(*ordinal));
                }
            }
            return None;
        }
    }
    pub fn edge_dominates(&self, guard: &Guard, target: usize, tag: Option<u64>) -> bool {
        let facts = self.plan.facts();
        let Ok([definition]) = facts.definition_sites(guard.predicate) else {
            return false;
        };
        let Some(definition) = definition.site().map(|s| s as usize) else {
            return false;
        };
        self.reaches(target, None, tag)
            && !self.reaches(target, Some((definition, usize::MAX)), tag)
            && !self.reaches_with(target, None, tag, Some((guard.predicate, false)))
    }
}
