// Capability methods
// ---------------------------------------------------------------------------

// These methods manage capabilities within the runtime, allowing for granting, revoking, and checking capabilities. A capability represents a permission or access right to perform certain actions or access certain resources. In Mech, they are used to control access to various runtime features and resources, ensuring that actors and tasks can only perform actions they are authorized for, granting fine-grained control over resources and actions in the runtime, etc.
// 
// The methods include:
// - `grant_capability`: Grants a capability to the runtime and emits a CapabilityGranted event.
// - `revoke_capability`: Revokes a capability from the runtime and emits a CapabilityRevoked event.
// - `check_capability`: Checks if a capability request is valid and returns the corresponding CapabilityId if it is.
// - `get_capability`: Retrieves a capability by its ID.

// Like with actors, there is a _with_context version of each method, allowing for transactional operations and proper event emission within the context of an active transaction.

use super::*;

impl MechRuntime {

  pub fn grant_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    capability: Arc<dyn Capability>,
  ) -> MResult<CapabilityId> {
    context.validate()?;
    context.charge_step()?;
    capability.validate()?;

    let id = capability.id();

    self
      .capability_kernel
      .grant(CapabilityGrant::new(capability.clone()))?;

    self.store.grant_capability(id, capability)?;
    context.add_capability(id);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::CapabilityGranted {
        capability_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn revoke_capability(&mut self, capability: CapabilityId) -> MResult<()> {
    let mut context = self.runtime_context()?;
    self.revoke_capability_with_context(&mut context, capability)
  }

  pub fn revoke_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    capability: CapabilityId,
  ) -> MResult<()> {
    context.validate()?;
    context.charge_step()?;

    self
      .capability_kernel
      .revoke(CapabilityRevocation::new(capability))?;

    self.store.revoke_capability(capability)?;
    context.remove_capability(capability);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::CapabilityRevoked {
        capability_id: capability,
      },
    )?;

    Ok(())
  }

  pub fn check_capability(
    &mut self,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    self.capability_kernel.check(request)
  }

  pub fn check_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    context.validate()?;
    context.charge_step()?;
    self.capability_kernel.check(request)
  }

  pub fn get_capability(
    &self,
    id: CapabilityId,
  ) -> MResult<Option<Arc<dyn Capability>>> {
    self.store.get_capability(id)
  }
}