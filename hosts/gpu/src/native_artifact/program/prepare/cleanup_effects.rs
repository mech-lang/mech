//! Original prices and surviving effects at a checked ordinary Tail boundary.
//! Successful strict-child exits have already cleared nonreturned child locals.
use super::frame_retirement::{FrameRetirements, Operation};
use super::retirement::{RetirementKind, RetirementTables};
use mech_engine::native_artifact::{
    NativeBufferFrames, NativeControlLoop, NativeIterationValueScope, NativeProgramArtifactPlan,
};

#[derive(Debug)]
pub(in crate::native_artifact) struct Boundary {
    pub(in crate::native_artifact) total: u64,
    pub(in crate::native_artifact) charges: usize,
    pub(in crate::native_artifact) omitted_work: u64,
    pub(in crate::native_artifact) has_frames: bool,
    pub(in crate::native_artifact) effects: Box<[u64]>,
}

/// Pure descriptor data, rederived for owned images. Original inventories remain
/// authoritative for scan charges, ordered failures and partial cleanup.
#[derive(Debug, Default)]
pub(in crate::native_artifact) struct Layout {
    boundaries: Box<[Option<Boundary>]>,
}

impl Layout {
    pub(in crate::native_artifact) fn derive(
        plan: &NativeProgramArtifactPlan,
        tables: &RetirementTables,
        frames: &NativeBufferFrames,
        recipes: &FrameRetirements,
    ) -> Result<Self, &'static str> {
        let independent = independent_slots(plan.values().len(), frames, recipes)?;
        let mut boundaries = Vec::new();
        for (iteration, control) in plan.iterations().iter().enumerate() {
            for kind in [
                RetirementKind::Collection,
                RetirementKind::TailLatch,
                RetirementKind::TailExit,
            ] {
                if !matches!(control, NativeControlLoop::Tail(_))
                    || matches!(kind, RetirementKind::Collection)
                {
                    boundaries.push(None);
                    continue;
                }
                let slots = tables.slots(tables.words(), iteration as u32, kind)?;
                let omit = |slot: usize| {
                    independent.get(slot) == Some(&true)
                        && cleared_child(plan, slot, iteration as u32)
                };
                let omitted_slots = slots.iter().filter(|&&slot| omit(slot as usize)).count();
                if omitted_slots == 0 {
                    boundaries.push(None);
                    continue;
                }
                let mut summary = Boundary {
                    total: 0,
                    charges: slots.len(),
                    omitted_work: 0,
                    has_frames: false,
                    effects: Box::new([]),
                };
                let mut effects = Vec::new();
                effects
                    .try_reserve_exact(slots.len() - omitted_slots)
                    .map_err(|_| "physical retirement allocation")?;
                for &slot in slots {
                    let slot = usize::try_from(slot).map_err(|_| "retirement slot exceeds host")?;
                    if slot >= plan.values().len() {
                        return Err("physical retirement slot outside plan");
                    }
                    let mut price = 32u64;
                    if let Some((work, _)) = recipes.get(slot) {
                        price = price.checked_add(work).ok_or("retirement price overflow")?;
                        summary.charges = summary
                            .charges
                            .checked_add(1)
                            .ok_or("retirement charge overflow")?;
                        summary.has_frames = true;
                    }
                    summary.total = summary
                        .total
                        .checked_add(price)
                        .ok_or("retirement total overflow")?;
                    if omit(slot) {
                        summary.omitted_work = summary
                            .omitted_work
                            .checked_add(price)
                            .ok_or("omitted retirement price overflow")?;
                    } else {
                        effects.push(slot as u64);
                    }
                }
                summary.effects = effects.into_boxed_slice();
                boundaries.push(Some(summary));
            }
        }
        if boundaries.iter().all(Option::is_none) {
            return Ok(Self::default());
        }
        Ok(Self {
            boundaries: boundaries.into_boxed_slice(),
        })
    }

    pub(in crate::native_artifact) fn get(
        &self,
        iteration: u32,
        kind: RetirementKind,
    ) -> Option<&Boundary> {
        self.boundaries
            .get(iteration as usize * 3 + kind as usize)?
            .as_ref()
    }
}

fn cleared_child(plan: &NativeProgramArtifactPlan, slot: usize, boundary: u32) -> bool {
    let owner = match plan.value_scopes().get(slot) {
        Some(NativeIterationValueScope::Immutable { iteration }) => *iteration,
        Some(NativeIterationValueScope::Carrier { iteration }) => Some(*iteration),
        None => return false,
    };
    let Some(owner) = owner else {
        return false;
    };
    let Some(NativeControlLoop::Tail(tail)) = plan.iterations().get(owner as usize) else {
        return false;
    };
    if owner == boundary || tail.returned().get() as usize == slot {
        return false;
    }
    // Current-visit validation closes every definition within its lexical owner
    // and every child edge through initialize/exit. Parent continuation requires
    // successful exit cleanup; an error aborts the invocation. Never-entered
    // banks start empty. Keep collection/iteration exceptions conservative.
    let mut ancestor = tail.parent();
    while let Some(id) = ancestor {
        let Some(NativeControlLoop::Tail(tail)) = plan.iterations().get(id as usize) else {
            return false;
        };
        if id == boundary {
            return true;
        }
        ancestor = tail.parent();
    }
    false
}

fn independent_slots(
    values: usize,
    frames: &NativeBufferFrames,
    recipes: &FrameRetirements,
) -> Result<Vec<bool>, &'static str> {
    let words = frames
        .scalar_fields()
        .checked_mul(2)
        .ok_or("physical frame word overflow")?;
    let mut word_owners = vec![None; words];
    let mut managed_owners = vec![None; frames.managed_fields()];
    let mut independent = vec![true; values];
    // Current frame layout allocates disjoint lanes per logical value. Explicitly
    // verify it: a future lane-reuse pass must carry allocation epochs before it
    // may reuse this successful-child proof. Canonical slots are not compacted.
    for value in 0..values {
        let Some((_, operations)) = recipes.get(value) else {
            continue;
        };
        for operation in operations {
            let lanes: &mut [Option<usize>] = match operation {
                Operation::Scalars(range) => word_owners
                    .get_mut(range.clone())
                    .ok_or("physical frame words outside bank")?,
                Operation::Managed(at) => managed_owners
                    .get_mut(*at..at + 1)
                    .ok_or("physical frame owner outside bank")?,
            };
            for owner in lanes {
                if let Some(previous) = *owner
                    && previous != value
                {
                    independent[previous] = false;
                    independent[value] = false;
                }
                *owner = Some(value);
            }
        }
    }
    Ok(independent)
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests;
