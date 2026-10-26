//! Exact immutable lexical retirement sets, derived from the checked plan.
//! This is descriptor metadata, not a scheduler or an alternate lifetime proof.
use mech_engine::native_artifact::{
    NativeControlLoop, NativeIterationValueScope, NativeMatchValue, NativeProgramArtifactPlan,
};

#[derive(Clone, Copy, Debug)]
pub(in crate::native_artifact) enum RetirementKind {
    Collection = 0,
    TailLatch = 1,
    TailExit = 2,
}
const KINDS: [RetirementKind; 3] = [
    RetirementKind::Collection,
    RetirementKind::TailLatch,
    RetirementKind::TailExit,
];

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct RetirementTables {
    // A three-range prefix per loop, followed by ascending slot ordinals.
    // The invocation copies ALL words into its own admitted typed storage.
    words: Box<[u64]>,
    iterations: usize,
}
impl RetirementTables {
    pub(super) fn derive(plan: &NativeProgramArtifactPlan) -> Result<Self, &'static str> {
        if plan.values().len() != plan.value_scopes().len() {
            return Err("retirement scope count differs from values");
        }
        let iterations = plan.iterations().len();
        let aggregates = plan.scalar_aggregates();
        let ranges = iterations
            .checked_mul(KINDS.len())
            .ok_or("retirement range count overflow")?;
        if ranges == 0 {
            return Ok(Self {
                words: Box::new([]),
                iterations,
            });
        }
        let prefix = ranges.checked_add(1).ok_or("retirement prefix overflow")?;
        let mut words = Vec::<u64>::new();
        words
            .try_reserve_exact(prefix)
            .map_err(|_| "retirement prefix allocation")?;
        words.resize(prefix, 0);
        // Two O(values * lexical depth) passes avoid a compile-time
        // values-by-all-loops scan and allocate one exact final table capacity.
        for slot in 0..plan.values().len() {
            Self::each_range(plan, &aggregates, slot, |range| {
                words[range] = words[range]
                    .checked_add(1)
                    .ok_or("retirement slot count overflow")?;
                Ok(())
            })?;
        }
        let mut total = u64::try_from(prefix).map_err(|_| "retirement prefix exceeds u64")?;
        for word in &mut words[..ranges] {
            let count = *word;
            *word = total;
            total = total
                .checked_add(count)
                .ok_or("retirement table size overflow")?;
        }
        words[ranges] = total;
        let total = usize::try_from(total).map_err(|_| "retirement table exceeds host")?;
        words
            .try_reserve_exact(total - prefix)
            .map_err(|_| "retirement slot allocation")?;
        let mut next = Vec::new();
        next.try_reserve_exact(ranges)
            .map_err(|_| "retirement cursor allocation")?;
        next.extend_from_slice(&words[..ranges]);
        words.resize(total, 0);
        for slot in 0..plan.values().len() {
            let ordinal =
                u64::from(u32::try_from(slot).map_err(|_| "retirement slot exceeds u32")?);
            Self::each_range(plan, &aggregates, slot, |range| {
                let offset =
                    usize::try_from(next[range]).map_err(|_| "retirement cursor exceeds host")?;
                let end =
                    usize::try_from(words[range + 1]).map_err(|_| "retirement end exceeds host")?;
                if offset >= end {
                    return Err("retirement cursor exceeds exact range");
                }
                words[offset] = ordinal;
                next[range] += 1;
                Ok(())
            })?;
        }
        if next
            .iter()
            .enumerate()
            .any(|(range, &end)| end != words[range + 1])
        {
            return Err("retirement table did not fill exact ranges");
        }
        Ok(Self {
            words: words.into_boxed_slice(),
            iterations,
        })
    }

    fn each_range(
        plan: &NativeProgramArtifactPlan,
        aggregates: &mech_engine::native_artifact::NativeScalarAggregates,
        slot: usize,
        mut visit: impl FnMut(usize) -> Result<(), &'static str>,
    ) -> Result<(), &'static str> {
        if plan.values()[slot].scalar_kind().is_some() || aggregates.slot_at(slot).is_some() {
            return Ok(());
        }
        let mut owner = match plan.value_scopes()[slot] {
            NativeIterationValueScope::Immutable { iteration } => iteration,
            NativeIterationValueScope::Carrier { iteration } => Some(iteration),
        };
        let mut remaining = plan.iterations().len();
        while let Some(id) = owner {
            remaining = remaining
                .checked_sub(1)
                .ok_or("retirement scope parent cycle")?;
            let control = plan
                .iterations()
                .get(id as usize)
                .ok_or("retirement scope parent outside plan")?;
            let row = id as usize * KINDS.len();
            let differs = |keep: Option<NativeMatchValue>| {
                keep.is_none_or(|value| value.get() as usize != slot)
            };
            match control {
                NativeControlLoop::Collection(data) => {
                    if differs(data.element()) {
                        visit(row + RetirementKind::Collection as usize)?;
                    }
                }
                NativeControlLoop::Tail(data) => {
                    if differs(Some(data.carrier())) {
                        visit(row + RetirementKind::TailLatch as usize)?;
                    }
                    if differs(Some(data.returned())) {
                        visit(row + RetirementKind::TailExit as usize)?;
                    }
                }
                NativeControlLoop::Iteration(_) => {}
            }
            owner = control.as_ref().parent();
        }
        Ok(())
    }

    pub(in crate::native_artifact) fn words(&self) -> &[u64] {
        &self.words
    }

    pub(in crate::native_artifact) fn slots<'a>(
        &self,
        words: &'a [u64],
        iteration: u32,
        kind: RetirementKind,
    ) -> Result<&'a [u64], &'static str> {
        if iteration as usize >= self.iterations || words.len() != self.words.len() {
            return Err("retirement table differs from descriptor");
        }
        let index = iteration as usize * KINDS.len() + kind as usize;
        let start = usize::try_from(words[index]).map_err(|_| "retirement offset exceeds host")?;
        let end =
            usize::try_from(words[index + 1]).map_err(|_| "retirement offset exceeds host")?;
        let prefix = self.iterations * KINDS.len() + 1;
        if start < prefix {
            return Err("retirement range overlaps prefix");
        }
        words
            .get(start..end)
            .ok_or("retirement range outside backing")
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "program_retirement_tests.rs"]
mod tests;
