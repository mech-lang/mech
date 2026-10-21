//! Immutable storage/dispatch metadata derived from the same canonical plan.
#[cfg(feature = "canonical-native-program")]
use mech_engine::native_artifact::NativeMatchValue;
use mech_engine::native_artifact::{
    NativeBorrowedStringAction as Action, NativeBorrowedStringRegion as Region,
    NativeIterationAction, NativeProgramArtifactPlan,
};

#[derive(Debug)]
pub(super) struct RegionLayout {
    pub proof: Region,
    // The code generator writes these lanes; the owning runtime independently
    // verifies their clearance before accepting a continuation checkpoint.
    pub ready: usize,
    #[cfg(feature = "canonical-native-program")]
    pub captures: usize,
    pub end: usize,
    #[cfg(feature = "canonical-native-program")]
    slots: Box<[Option<usize>]>,
}
impl RegionLayout {
    #[cfg(feature = "canonical-native-program")]
    pub fn slot(&self, value: NativeMatchValue) -> usize {
        self.slots[value.get() as usize].expect("proved borrowed String slot")
    }
    pub fn capture_words(&self) -> usize {
        (self.proof.matrices().len() + self.proof.literals().len()) * 2
    }
}
#[derive(Debug, Default)]
pub(super) struct Layout {
    pub regions: Vec<RegionLayout>,
    pub initializers: Vec<Option<usize>>,
    pub actions: Vec<Option<(usize, usize)>>,
    pub words: usize,
}
impl Layout {
    pub fn derive(plan: &NativeProgramArtifactPlan) -> Result<Self, &'static str> {
        let mut layout = Self {
            initializers: vec![None; plan.blocks().len()],
            actions: vec![None; plan.blocks().len()],
            ..Self::default()
        };
        let base = plan
            .values()
            .len()
            .checked_add(plan.iterations().len())
            .and_then(|n| n.checked_add(plan.scalar_aggregates().words()))
            .ok_or("borrowed String local count overflow")?;
        let mut next = base;
        for proof in plan.borrowed_string_regions() {
            let ordinal = layout.regions.len();
            let ready = next;
            next = next
                .checked_add(1)
                .ok_or("borrowed String ready slot overflow")?;
            #[cfg(feature = "canonical-native-program")]
            let captures = next;
            let mut slots = vec![None; plan.values().len()];
            for &value in proof
                .matrices()
                .iter()
                .chain(proof.literals())
                .chain(proof.values())
            {
                if slots[value.get() as usize].is_some() {
                    return Err("overlapping borrowed String storage");
                }
                slots[value.get() as usize] = Some(next);
                next = next.checked_add(2).ok_or("borrowed String pair overflow")?;
            }
            for (action, (site, _)) in proof.actions().iter().enumerate() {
                layout.actions[*site as usize] = Some((ordinal, action));
            }
            for (site, block) in plan.blocks().iter().enumerate() {
                if matches!(block.action(), NativeIterationAction::Initialize { iteration }
                    if *iteration == proof.iteration())
                {
                    layout.initializers[site] = Some(ordinal);
                }
            }
            layout.regions.push(RegionLayout {
                proof,
                ready,
                #[cfg(feature = "canonical-native-program")]
                captures,
                end: next,
                #[cfg(feature = "canonical-native-program")]
                slots: slots.into(),
            });
        }
        layout.words = next - base;
        Ok(layout)
    }
    pub fn action(&self, site: usize) -> Option<(usize, &Action)> {
        let (region, action) = self.actions.get(site).copied().flatten()?;
        Some((region, &self.regions[region].proof.actions()[action].1))
    }
    pub fn extra_work(&self, site: usize) -> Result<u64, &'static str> {
        if let Some(Some(region)) = self.initializers.get(site) {
            (self.regions[*region].capture_words() as u64)
                .checked_mul(8)
                .and_then(|n| n.checked_add(128))
                .ok_or("borrowed String setup work overflow")
        } else {
            // Equality's byte loop is bounded by the actual <=32-byte literal.
            // Cover its dynamic instructions, not just the emitted loop body.
            Ok(match self.action(site) {
                Some((_, Action::Equal { .. })) => 1024,
                Some(_) => 256,
                None => 0,
            })
        }
    }
}
