//! Runtime component access and controlled replacement.

use super::MechRuntime;
#[cfg(any(test, all(feature = "watcher", feature = "source")))]
use crate::CapabilityKernel;
#[cfg(test)]
use crate::MechStore;
use crate::SchedulerPolicy;
#[cfg(all(test, feature = "source"))]
use crate::SourceResolver;
#[cfg(all(test, feature = "source"))]
use mech_core::MResult;

impl MechRuntime {
    #[cfg(test)]
    pub(crate) fn store(&self) -> &dyn MechStore {
        self.store.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn capability_kernel(&self) -> &dyn CapabilityKernel {
        self.capability_kernel.as_ref()
    }

    /// Unchecked administrative escape hatch outside runtime-owned poison
    /// enforcement. Runtime internals must not use it to bypass mutation
    /// preflight.
    #[cfg(all(feature = "watcher", feature = "source"))]
    pub(crate) fn capability_kernel_mut(&mut self) -> &mut dyn CapabilityKernel {
        self.capability_kernel.as_mut()
    }

    #[cfg(all(test, feature = "source"))]
    pub(crate) fn source_resolver(&self) -> &(dyn SourceResolver + 'static) {
        self.source_resolver.as_ref()
    }

    #[cfg(all(test, feature = "source"))]
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
