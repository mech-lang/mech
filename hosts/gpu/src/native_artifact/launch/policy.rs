//! Explicit runtime policy shared by managed and resource-hosted launchers.
//! This module is runtime-only; compiler allowances cannot enter launch policy.
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LibraryLimits {
    pub(super) output_elements: u64,
    pub(super) output_bytes: u64,
    pub(super) temporary_bytes: u64,
    pub(super) cloned_bytes: u64,
    pub(super) retained_nodes: u64,
    pub(super) compute_work: u64,
    pub(super) comparison_work: u64,
}
impl LibraryLimits {
    pub(super) fn limits(
        &self,
    ) -> Result<mech_core::MemoryBudgetLimits, mech_core::MemoryPlanError> {
        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()?.limits;
        limits.max_output_elements = Some(self.output_elements);
        limits.max_output_bytes = Some(self.output_bytes);
        limits.max_temporary_bytes = Some(self.temporary_bytes);
        limits.max_cloned_bytes = Some(self.cloned_bytes);
        limits.max_retained_nodes = Some(self.retained_nodes);
        limits.max_compute_work = Some(self.compute_work);
        limits.max_comparison_work = Some(self.comparison_work);
        Ok(limits)
    }
}
