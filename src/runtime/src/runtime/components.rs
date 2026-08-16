//! Runtime component access and controlled replacement.

use super::MechRuntime;
use crate::{CapabilityKernel, MechStore, SchedulerPolicy, SourceResolver};
use mech_core::MResult;

impl MechRuntime {
    pub(crate) fn store(&self) -> &dyn MechStore {
        self.store.as_ref()
    }

    pub(crate) fn capability_kernel(&self) -> &dyn CapabilityKernel {
        self.capability_kernel.as_ref()
    }

    /// Unchecked administrative escape hatch outside runtime-owned poison
    /// enforcement. Runtime internals must not use it to bypass mutation
    /// preflight.
    pub(crate) fn capability_kernel_mut(&mut self) -> &mut dyn CapabilityKernel {
        self.capability_kernel.as_mut()
    }

    pub(crate) fn source_resolver(&self) -> &(dyn SourceResolver + 'static) {
        self.source_resolver.as_ref()
    }

    pub(crate) fn set_source_resolver(
        &mut self,
        source_resolver: impl SourceResolver + 'static,
    ) -> MResult<()> {
        self.ensure_runtime_mutation_allowed("set_source_resolver")?;
        self.source_resolver = Box::new(source_resolver);
        Ok(())
    }

    pub fn scheduler_policy(&self) -> &SchedulerPolicy {
        &self.scheduler_policy
    }

    // ---------------------------------------------------------------------------
    // Context helpers
    // ---------------------------------------------------------------------------
}
