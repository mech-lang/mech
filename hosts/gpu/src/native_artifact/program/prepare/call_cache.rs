//! Dense storage inventory for every prepared call, including cold fallbacks.
use mech_engine::native_artifact::{NativePreparedLibraryCall, NativeProgramArtifactPlan};
use std::num::NonZeroUsize;

#[derive(Debug, Default)]
pub(in crate::native_artifact) struct Layout {
    slots: Box<[Option<NonZeroUsize>]>,
    repeated: Box<[bool]>,
}
impl Layout {
    /// Recomputed from the same validated plan and factories on image load.
    /// Storage classification grants no permission to visit or invoke a site.
    pub(in crate::native_artifact) fn derive(
        plan: &NativeProgramArtifactPlan,
        calls: &[Option<NativePreparedLibraryCall>],
    ) -> Result<Self, &'static str> {
        if calls.len() != plan.blocks().len() {
            return Err("call cache inventory differs from program blocks");
        }
        let count = calls.iter().filter(|call| call.is_some()).count();
        if count == 0 {
            return Ok(Self::default());
        }
        let mut slots = Vec::with_capacity(calls.len());
        let mut repeated = Vec::with_capacity(count);
        for (call, block) in calls.iter().zip(plan.blocks()) {
            let slot = if call.is_some() {
                let next = repeated
                    .len()
                    .checked_add(1)
                    .ok_or("call cache index overflows")?;
                repeated.push(block.iteration().is_some());
                NonZeroUsize::new(next)
            } else {
                None
            };
            slots.push(slot);
        }
        Ok(Self {
            slots: slots.into_boxed_slice(),
            repeated: repeated.into_boxed_slice(),
        })
    }
    #[inline]
    pub(in crate::native_artifact) fn slot(&self, site: usize) -> Option<usize> {
        self.slots.get(site)?.map(|slot| slot.get() - 1)
    }
    pub(in crate::native_artifact) fn len(&self) -> usize {
        self.repeated.len()
    }
    pub(in crate::native_artifact) fn repeated(&self) -> &[bool] {
        &self.repeated
    }
}
