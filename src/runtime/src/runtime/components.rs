//! Runtime component access and controlled replacement.

use super::*;

impl MechRuntime {
  pub(crate) fn store(&self) -> &dyn MechStore {
    self.store.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn store_mut(&mut self) -> &mut dyn MechStore {
    self.store.as_mut()
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

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn source_resolver_mut(&mut self) -> &mut dyn SourceResolver {
    self.source_resolver.as_mut()
  }

  pub(crate) fn set_source_resolver(
    &mut self,
    source_resolver: impl SourceResolver + 'static,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed("set_source_resolver")?;
    self.source_resolver = Box::new(source_resolver);
    Ok(())
  }

  pub(crate) fn host_registry(&self) -> &dyn HostRegistry {
    self.host_registry.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn host_registry_mut(&mut self) -> &mut dyn HostRegistry {
    self.host_registry.as_mut()
  }

  pub(crate) fn host_policy(&self) -> &dyn HostCallPolicy {
    self.host_policy.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn host_policy_mut(&mut self) -> &mut dyn HostCallPolicy {
    self.host_policy.as_mut()
  }

  pub(crate) fn scheduler(&self) -> &dyn Scheduler {
    self.scheduler.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn scheduler_mut(&mut self) -> &mut dyn Scheduler {
    self.scheduler.as_mut()
  }

  pub fn scheduler_policy(&self) -> &SchedulerPolicy {
    &self.scheduler_policy
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn scheduler_policy_mut(&mut self) -> &mut SchedulerPolicy {
    &mut self.scheduler_policy
  }

  pub(crate) fn actor_behavior_driver(&self) -> &dyn ActorBehaviorDriver {
    self.actor_behavior_driver.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn actor_behavior_driver_mut(&mut self) -> &mut dyn ActorBehaviorDriver {
    self.actor_behavior_driver.as_mut()
  }

  pub(crate) fn module_builder(&self) -> &ModuleBuilder {
    &self.module_builder
  }

  pub(crate) fn set_scheduler_policy(&mut self, scheduler_policy: SchedulerPolicy) -> MResult<()> {
    self.ensure_runtime_mutation_allowed("set_scheduler_policy")?;
    scheduler_policy.validate()?;
    self.scheduler_policy = scheduler_policy;
    Ok(())
  }

  // ---------------------------------------------------------------------------
  // Context helpers
  // ---------------------------------------------------------------------------
}
