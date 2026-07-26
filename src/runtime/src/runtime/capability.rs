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
use crate::{
  CapabilityAlreadyExistsError, CapabilityNotFoundError,
  CapabilityNotRevocableError,
};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub(super) enum RuntimeCapabilityMutation {
  Grant(Arc<dyn Capability>),
  Revoke(CapabilityId),
  Use(CapabilityId),
}

#[derive(Clone, Debug, Default)]
pub(super) struct RuntimeCapabilityOverlay {
  operations: Vec<RuntimeCapabilityMutation>,
  grants: HashMap<CapabilityId, Arc<dyn Capability>>,
  grant_order: Vec<CapabilityId>,
  revocations: HashSet<CapabilityId>,
  uses: HashMap<CapabilityId, u64>,
  usage_order: Vec<CapabilityId>,
}

impl RuntimeCapabilityOverlay {
  pub(super) fn mark(&self) -> usize {
    self.operations.len()
  }

  pub(super) fn is_empty(&self) -> bool {
    self.grants.is_empty()
      && self.revocations.is_empty()
      && self.uses.is_empty()
  }

  pub(super) fn stage_grant(
    &mut self,
    capability: Arc<dyn Capability>,
  ) -> MResult<()> {
    self.stage_operation(RuntimeCapabilityMutation::Grant(capability))
  }

  pub(super) fn stage_revocation(
    &mut self,
    capability: CapabilityId,
  ) -> MResult<()> {
    self.stage_operation(RuntimeCapabilityMutation::Revoke(capability))
  }

  pub(super) fn stage_use(
    &mut self,
    capability: CapabilityId,
  ) -> MResult<()> {
    self.stage_operation(RuntimeCapabilityMutation::Use(capability))
  }

  pub(super) fn rollback_to(&mut self, mark: usize) -> MResult<()> {
    if mark > self.operations.len() {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "rollback_capability_overlay",
          reason: format!(
            "capability savepoint mark {} exceeds overlay length {}",
            mark,
            self.operations.len(),
          ),
        },
        None,
      ));
    }
    self.operations.truncate(mark);
    self.rebuild()
  }

  pub(super) fn provisional(
    &self,
    capability: CapabilityId,
  ) -> Option<Arc<dyn Capability>> {
    self.grants.get(&capability).cloned()
  }

  pub(super) fn check(
    &mut self,
    request: &CapabilityRequest,
  ) -> MResult<Option<CapabilityId>> {
    let selected = self.preview_check(request)?;
    if let Some(capability) = selected {
      self.stage_use(capability)?;
    }
    Ok(selected)
  }

  pub(super) fn preview_check(
    &self,
    request: &CapabilityRequest,
  ) -> MResult<Option<CapabilityId>> {
    for id in &self.grant_order {
      let Some(capability) = self.grants.get(id) else {
        continue;
      };
      if capability.subject_key() != request.subject {
        continue;
      }
      if let Some(max_uses) = capability.max_uses() {
        let actual = self.uses.get(id).copied().unwrap_or(0);
        if actual >= max_uses {
          continue;
        }
      }
      let decision = capability.preview_check(request)?;
      if decision.allowed {
        return Ok(Some(*id));
      }
    }
    Ok(None)
  }

  pub(super) fn grants(
    &self,
  ) -> impl Iterator<Item = (CapabilityId, Arc<dyn Capability>)> + '_ {
    self.grant_order.iter().filter_map(|id| {
      self
        .grants
        .get(id)
        .map(|capability| (*id, capability.clone()))
    })
  }

  pub(super) fn usage_deltas(
    &self,
  ) -> impl Iterator<Item = (CapabilityId, u64)> + '_ {
    self.usage_order.iter().filter_map(|id| {
      let uses = self.uses.get(id).copied().unwrap_or(0);
      (uses != 0).then_some((*id, uses))
    })
  }

  pub(super) fn pending_uses(
    &self,
  ) -> &HashMap<CapabilityId, u64> {
    &self.uses
  }

  pub(super) fn revocations(
    &self,
  ) -> impl Iterator<Item = CapabilityId> + '_ {
    self.revocations.iter().copied()
  }

  pub(super) fn revocation_ids(&self) -> HashSet<CapabilityId> {
    self.revocations.clone()
  }

  fn stage_operation(
    &mut self,
    operation: RuntimeCapabilityMutation,
  ) -> MResult<()> {
    self.operations.push(operation);
    if let Err(error) = self.rebuild() {
      self.operations.pop();
      self.rebuild().expect(
        "previous capability overlay state must remain valid",
      );
      return Err(error);
    }
    Ok(())
  }

  fn rebuild(&mut self) -> MResult<()> {
    self.grants.clear();
    self.grant_order.clear();
    self.revocations.clear();
    self.uses.clear();
    self.usage_order.clear();
    for operation in &self.operations {
      match operation {
        RuntimeCapabilityMutation::Grant(capability) => {
          let id = capability.id();
          self.revocations.remove(&id);
          if !self.grants.contains_key(&id) {
            self.grant_order.push(id);
          }
          self.grants.insert(capability.id(), capability.clone());
        }
        RuntimeCapabilityMutation::Revoke(capability) => {
          if self.grants.remove(capability).is_some() {
            self.grant_order.retain(|id| id != capability);
            self.uses.remove(capability);
            self.usage_order.retain(|id| id != capability);
          } else {
            self.revocations.insert(*capability);
          }
        }
        RuntimeCapabilityMutation::Use(capability) => {
          if self.revocations.contains(capability) {
            return Err(MechError::new(
              RuntimeInvalidOperationError {
                operation: "rebuild_capability_overlay",
                reason: format!(
                  "capability {} was used after transaction-local revocation",
                  capability,
                ),
              },
              None,
            ));
          }
          if !self.uses.contains_key(capability) {
            self.usage_order.push(*capability);
          }
          let current = self.uses.get(capability).copied().unwrap_or(0);
          let next =
            Self::incremented_usage(*capability, current)?;
          self.uses.insert(*capability, next);
        }
      }
    }
    Ok(())
  }

  fn incremented_usage(
    capability: CapabilityId,
    current: u64,
  ) -> MResult<u64> {
    current.checked_add(1).ok_or_else(|| {
      MechError::new(
        RuntimeInvalidOperationError {
          operation: "rebuild_capability_overlay",
          reason: format!(
            "capability {} transaction-local usage count overflowed",
            capability,
          ),
        },
        None,
      )
    })
  }
}

fn finish_failed_capability_grant(
  capability: CapabilityId,
  original: MechError,
  rollback_failures: Vec<(&'static str, MechError)>,
) -> MechError {
  if rollback_failures.is_empty() {
    return original;
  }

  let rollback_failures = rollback_failures
    .into_iter()
    .map(|(component, error)| {
      format!("{component}: {}", error.full_chain_message())
    })
    .collect();

  MechError::new(
    RuntimeCapabilityGrantRollbackFailed {
      capability,
      rollback_failures,
    },
    None,
  )
  .with_source(original)
}

impl MechRuntime {

  pub fn grant_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    capability: Arc<dyn Capability>,
  ) -> MResult<CapabilityId> {
    self.ensure_runtime_mutation_allowed(
      "grant_capability_with_context",
    )?;
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;
    capability.validate()?;

    let id = capability.id();

    if let Some(transaction_id) = context.transaction {
      if self
        .active_execution_transaction(transaction_id)?
        .capabilities
        .provisional(id)
        .is_some()
        || self.capability_kernel.get(id)?.is_some()
      {
        return Err(MechError::new(
          CapabilityAlreadyExistsError { capability: id },
          None,
        ));
      }

      let store_before = self
        .active_execution_transaction(transaction_id)?
        .store
        .clone();
      let overlay_mark = self
        .active_execution_transaction(transaction_id)?
        .capabilities
        .mark();
      let context_events_before = context.events.clone();
      let context_capabilities_before = context.capabilities.clone();

      self
        .active_execution_transaction_mut(transaction_id)?
        .capabilities
        .stage_grant(capability)?;
      if let Err(error) = self.emit_event_to_context(
        context,
        RuntimeEventKind::CapabilityGranted {
          capability_id: id,
        },
      ) {
        let transaction =
          self.active_execution_transaction_mut(transaction_id)?;
        transaction.store = store_before;
        let rollback_result =
          transaction.capabilities.rollback_to(overlay_mark);
        context.events = context_events_before;
        context.capabilities = context_capabilities_before;
        rollback_result?;
        return Err(error);
      }
      context.add_capability(id);
      return Ok(id);
    }

    self.store.grant_capability(id, capability.clone())?;

    if let Err(error) = self
      .capability_kernel
      .grant(CapabilityGrant::new(capability))
    {
      let rollback_failures = match self.store.rollback_capability_grant(id) {
        Ok(()) => Vec::new(),
        Err(rollback_error) => vec![("capability store", rollback_error)],
      };

      return Err(finish_failed_capability_grant(
        id,
        error,
        rollback_failures,
      ));
    }

    let context_events_before = context.events.clone();

    if let Err(error) = self.emit_event_to_context(
      context,
      RuntimeEventKind::CapabilityGranted {
        capability_id: id,
      },
    ) {
      context.events = context_events_before;

      let mut rollback_failures = Vec::new();
      if let Err(rollback_error) = self.capability_kernel.rollback_grant(id) {
        rollback_failures.push(("capability kernel", rollback_error));
      }
      if let Err(rollback_error) = self.store.rollback_capability_grant(id) {
        rollback_failures.push(("capability store", rollback_error));
      }

      return Err(finish_failed_capability_grant(
        id,
        error,
        rollback_failures,
      ));
    }

    context.add_capability(id);
    Ok(id)
  }

  pub fn revoke_capability(&mut self, capability: CapabilityId) -> MResult<()> {
    self.ensure_runtime_mutation_allowed("revoke_capability")?;
    let mut context = self.runtime_context()?;
    self.revoke_capability_with_context(&mut context, capability)
  }

  pub fn revoke_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    capability: CapabilityId,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed(
      "revoke_capability_with_context",
    )?;
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;

    if let Some(transaction_id) = context.transaction {
      let staged = self
        .active_execution_transaction(transaction_id)?
        .capabilities
        .provisional(capability);
      let live = if staged.is_none() {
        self.capability_kernel.get(capability)?
      } else {
        None
      };
      let Some(existing) = staged.or(live) else {
        return Err(MechError::new(
          CapabilityNotFoundError { capability },
          None,
        ));
      };
      if !existing.is_revocable() {
        return Err(MechError::new(
          CapabilityNotRevocableError { capability },
          None,
        ));
      }

      let store_before = self
        .active_execution_transaction(transaction_id)?
        .store
        .clone();
      let overlay_mark = self
        .active_execution_transaction(transaction_id)?
        .capabilities
        .mark();
      let context_events_before = context.events.clone();
      let context_capabilities_before = context.capabilities.clone();

      self
        .active_execution_transaction_mut(transaction_id)?
        .capabilities
        .stage_revocation(capability)?;
      context.remove_capability(capability);
      if let Err(error) = self.emit_event_to_context(
        context,
        RuntimeEventKind::CapabilityRevoked {
          capability_id: capability,
        },
      ) {
        let transaction =
          self.active_execution_transaction_mut(transaction_id)?;
        transaction.store = store_before;
        let rollback_result =
          transaction.capabilities.rollback_to(overlay_mark);
        context.events = context_events_before;
        context.capabilities = context_capabilities_before;
        rollback_result?;
        return Err(error);
      }
      return Ok(());
    }

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
    self.ensure_runtime_mutation_allowed("check_capability")?;
    self.capability_kernel.check(request)
  }

  pub fn check_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    self.ensure_runtime_mutation_allowed(
      "check_capability_with_context",
    )?;
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;
    if let Some(transaction_id) = context.transaction {
      let provisional = self
        .active_execution_transaction_mut(transaction_id)?
        .capabilities
        .check(request)?;
      if let Some(capability) = provisional {
        return Ok(capability);
      }
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      let revocations = transaction.capabilities.revocation_ids();
      let pending_uses = transaction.capabilities.pending_uses().clone();
      let capability = self
        .capability_kernel
        .preview_check_excluding_with_pending_uses(
          request,
          &revocations,
          &pending_uses,
        )?;
      self
        .active_execution_transaction_mut(transaction_id)?
        .capabilities
        .stage_use(capability)?;
      return Ok(capability);
    }
    self.capability_kernel.check(request)
  }

  pub(super) fn preview_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    self.ensure_runtime_mutation_allowed(
      "preview_capability_with_context",
    )?;
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;
    if let Some(transaction_id) = context.transaction {
      let provisional = self
        .active_execution_transaction(transaction_id)?
        .capabilities
        .preview_check(request)?;
      if let Some(capability) = provisional {
        return Ok(capability);
      }
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      let revocations = transaction.capabilities.revocation_ids();
      let pending_uses = transaction.capabilities.pending_uses().clone();
      return self
        .capability_kernel
        .preview_check_excluding_with_pending_uses(
          request,
          &revocations,
          &pending_uses,
        );
    }
    self.capability_kernel.preview_check(request)
  }

  pub fn get_capability(
    &self,
    id: CapabilityId,
  ) -> MResult<Option<Arc<dyn Capability>>> {
    self.store.get_capability(id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::collections::VecDeque;
  use std::sync::atomic::{AtomicBool, Ordering};

  use mech_core::{GenericError, MResult, MechError};

  use crate::capability::{
    BasicCapability, BasicCapabilityKernel, BasicConstraints,
    CapabilityDerivation,
    CapabilityGrant, CapabilityKernel, CapabilityKernelCheckpoint, Subject,
    SharedCapabilityKernel,
  };
  use crate::id::{
    ActorId, EventId, IdGenerator, MessageId, NodeId, ObjectId, RuntimeId,
    SequentialIdGenerator, TaskId, TransactionId,
  };
  use crate::store::{InMemoryStore, MechStore};

  fn capability(
    id: CapabilityId,
    subject: &str,
    revocable: bool,
  ) -> Arc<dyn Capability> {
    Arc::new(
      BasicCapability::from_keys(id, subject, "db://users", [":read"])
        .revocable(revocable),
    )
  }

  fn request(subject: &str) -> CapabilityRequest {
    CapabilityRequest::from_keys(subject, ":read", "db://users")
  }

  fn limited_capability(
    id: CapabilityId,
    max_uses: u64,
  ) -> Arc<dyn Capability> {
    Arc::new(
      BasicCapability::from_keys(
        id,
        "task:1",
        "db://users",
        [":read"],
      )
      .with_constraints(
        BasicConstraints::default().with_max_uses(max_uses),
      ),
    )
  }

  #[derive(Debug)]
  struct ScriptedEventIdGenerator {
    next: u128,
    event_ids: VecDeque<EventId>,
  }

  impl ScriptedEventIdGenerator {
    fn new(next: u128, event_ids: impl IntoIterator<Item = EventId>) -> Self {
      Self {
        next,
        event_ids: event_ids.into_iter().collect(),
      }
    }

    fn next_id(&mut self) -> u128 {
      let id = self.next;
      self.next = self.next.saturating_add(1);
      id
    }
  }

  impl IdGenerator for ScriptedEventIdGenerator {
    fn runtime_id(&mut self) -> RuntimeId {
      RuntimeId(self.next_id())
    }

    fn object_id(&mut self) -> ObjectId {
      ObjectId(self.next_id())
    }

    fn actor_id(&mut self) -> ActorId {
      ActorId(self.next_id())
    }

    fn task_id(&mut self) -> TaskId {
      TaskId(self.next_id())
    }

    fn capability_id(&mut self) -> CapabilityId {
      CapabilityId(self.next_id())
    }

    fn transaction_id(&mut self) -> TransactionId {
      TransactionId(self.next_id())
    }

    fn event_id(&mut self) -> EventId {
      self
        .event_ids
        .pop_front()
        .unwrap_or_else(|| EventId(self.next_id()))
    }

    fn node_id(&mut self) -> NodeId {
      NodeId(self.next_id())
    }

    fn message_id(&mut self) -> MessageId {
      MessageId(self.next_id())
    }
  }

  #[derive(Debug)]
  struct FailingRollbackKernel {
    inner: BasicCapabilityKernel,
    rollback_attempted: Arc<AtomicBool>,
  }

  impl CapabilityKernel for FailingRollbackKernel {
    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
      self.inner.grant(grant)
    }

    fn rollback_grant(&mut self, _capability: CapabilityId) -> MResult<()> {
      self.rollback_attempted.store(true, Ordering::SeqCst);
      Err(MechError::new(
        GenericError {
          msg: "test kernel rollback failed".to_string(),
        },
        None,
      ))
    }

    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
      self.inner.revoke(revocation)
    }

    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
      self.inner.check(request)
    }

    fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
      self.inner.get(id)
    }

    fn list_for_subject(
      &self,
      subject: &dyn Subject,
    ) -> MResult<Vec<CapabilityId>> {
      self.inner.list_for_subject(subject)
    }

    fn derive_capability(
      &mut self,
      derivation: CapabilityDerivation,
    ) -> MResult<CapabilityId> {
      self.inner.derive_capability(derivation)
    }

    fn is_revoked(&self, id: CapabilityId) -> MResult<bool> {
      self.inner.is_revoked(id)
    }
  }

  #[derive(Debug, Default)]
  struct FailingCheckpointRestoreKernel {
    inner: BasicCapabilityKernel,
  }

  impl CapabilityKernel for FailingCheckpointRestoreKernel {
    fn checkpoint(
      &self,
    ) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
      self.inner.checkpoint()
    }

    fn restore(
      &mut self,
      _checkpoint: Box<dyn CapabilityKernelCheckpoint>,
    ) -> MResult<()> {
      Err(MechError::new(
        GenericError {
          msg: "deliberate capability checkpoint restore failure"
            .to_string(),
        },
        None,
      ))
    }

    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
      self.inner.grant(grant)
    }

    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()> {
      self.inner.rollback_grant(capability)
    }

    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
      self.inner.revoke(revocation)
    }

    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
      self.inner.check(request)
    }

    fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
      self.inner.get(id)
    }

    fn list_for_subject(
      &self,
      subject: &dyn Subject,
    ) -> MResult<Vec<CapabilityId>> {
      self.inner.list_for_subject(subject)
    }

    fn derive_capability(
      &mut self,
      derivation: CapabilityDerivation,
    ) -> MResult<CapabilityId> {
      self.inner.derive_capability(derivation)
    }

    fn is_revoked(&self, id: CapabilityId) -> MResult<bool> {
      self.inner.is_revoked(id)
    }
  }

  #[test]
  fn capability_grant_store_failure_never_grants_kernel_authority() {
    let mut store = InMemoryStore::new();
    store
      .grant_capability(CapabilityId(100), capability(CapabilityId(100), "task:1", true))
      .unwrap();

    let mut runtime = MechRuntime::builder().store(store).build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let durable_events_before = runtime.list_events(None).unwrap();

    let error = runtime
      .grant_capability_with_context(
        &mut context,
        capability(CapabilityId(100), "task:1", true),
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "StoreRecordAlreadyExists");
    assert!(runtime
      .capability_kernel()
      .get(CapabilityId(100))
      .unwrap()
      .is_none());
    assert!(!context.has_capability(CapabilityId(100)));
    assert!(!context.events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::CapabilityGranted {
          capability_id: CapabilityId(100),
        }
      )
    }));
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_some());
    assert_eq!(
      runtime
        .store()
        .list_capabilities_for_subject("task:1")
        .unwrap(),
      vec![CapabilityId(100)],
    );
    assert_eq!(runtime.list_events(None).unwrap(), durable_events_before);
  }

  #[test]
  fn capability_grant_kernel_failure_rolls_back_store() {
    let mut kernel = BasicCapabilityKernel::new();
    kernel
      .grant(CapabilityGrant::new(capability(CapabilityId(100), "task:1", true)))
      .unwrap();

    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let context_events_before = context.events.clone();
    let context_capabilities_before = context.capabilities.clone();

    let error = runtime
      .grant_capability_with_context(
        &mut context,
        capability(CapabilityId(100), "task:1", true),
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "CapabilityAlreadyExists");
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert!(runtime
      .capability_kernel()
      .get(CapabilityId(100))
      .unwrap()
      .is_some());
    assert_eq!(context.events, context_events_before);
    assert_eq!(context.capabilities, context_capabilities_before);
  }

  #[test]
  fn capability_grant_event_failure_removes_non_revocable_live_authority() {
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(1);
    let mut runtime = MechRuntime::builder()
      .config(config)
      .id_generator(ScriptedEventIdGenerator::new(
        1,
        [EventId(100), EventId(100)],
      ))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.events.push(RuntimeEvent::new(
      EventId(999),
      999,
      RuntimeEventKind::RuntimeTickStarted,
    ));
    let context_events_before = context.events.clone();

    let error = runtime
      .grant_capability_with_context(
        &mut context,
        capability(CapabilityId(100), "task:1", false),
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "StoreRecordAlreadyExists");
    assert_eq!(context.events, context_events_before);
    assert!(!context.has_capability(CapabilityId(100)));
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert!(runtime
      .capability_kernel()
      .get(CapabilityId(100))
      .unwrap()
      .is_none());
    assert!(runtime.check_capability(&request("task:1")).is_err());
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::RuntimeCreated { .. })
    }));
    assert!(!runtime.list_events(None).unwrap().iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::CapabilityGranted { .. })
    }));
  }

  #[test]
  fn capability_grant_failed_transactional_event_staging_is_compensated() {
    let mut runtime = MechRuntime::builder()
      .id_generator(ScriptedEventIdGenerator::new(
        1,
        [EventId(100), EventId(101), EventId(0)],
      ))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let context_events_before = context.events.clone();

    let error = runtime
      .grant_capability_with_context(
        &mut context,
        capability(CapabilityId(100), "task:1", true),
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "InvalidRuntimeEvent");
    assert_eq!(context.events, context_events_before);
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(!runtime
      .active_transactions
      .get(&transaction_id)
      .unwrap()
      .store
      .staged_events()
      .any(|event| matches!(event.kind, RuntimeEventKind::CapabilityGranted { .. })));
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert!(runtime
      .capability_kernel()
      .get(CapabilityId(100))
      .unwrap()
      .is_none());
    assert!(!context.has_capability(CapabilityId(100)));

    runtime
      .abort_runtime_transaction(&mut context, "test cleanup")
      .unwrap();
  }

  #[test]
  fn capability_grant_incomplete_rollback_is_reported_and_continues() {
    let rollback_attempted = Arc::new(AtomicBool::new(false));
    let kernel = FailingRollbackKernel {
      inner: BasicCapabilityKernel::new(),
      rollback_attempted: rollback_attempted.clone(),
    };
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(1);
    let mut runtime = MechRuntime::builder()
      .config(config)
      .id_generator(ScriptedEventIdGenerator::new(
        1,
        [EventId(100), EventId(100)],
      ))
      .capability_kernel(kernel)
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.events.push(RuntimeEvent::new(
      EventId(999),
      999,
      RuntimeEventKind::RuntimeTickStarted,
    ));
    let context_events_before = context.events.clone();

    let error = runtime
      .grant_capability_with_context(
        &mut context,
        capability(CapabilityId(100), "task:1", false),
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeCapabilityGrantRollbackFailed");
    assert_eq!(
      error.source.as_ref().unwrap().kind_name(),
      "StoreRecordAlreadyExists",
    );
    let rollback = error
      .kind_as::<RuntimeCapabilityGrantRollbackFailed>()
      .unwrap();
    assert!(rollback
      .rollback_failures
      .iter()
      .any(|failure| failure.contains("capability kernel: GenericError")));
    assert!(rollback_attempted.load(Ordering::SeqCst));
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert_eq!(context.events, context_events_before);
    assert!(!context.has_capability(CapabilityId(100)));
    assert!(runtime
      .capability_kernel()
      .get(CapabilityId(100))
      .unwrap()
      .is_some());
  }

  #[test]
  fn capability_grant_success_updates_every_component_once() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    assert_eq!(
      runtime
        .grant_capability_with_context(&mut context, capability(id, "task:1", true))
        .unwrap(),
      id,
    );
    assert!(runtime.get_capability(id).unwrap().is_some());
    assert!(runtime.capability_kernel().get(id).unwrap().is_some());
    assert_eq!(runtime.check_capability(&request("task:1")).unwrap(), id);
    assert_eq!(context.capabilities.iter().filter(|candidate| **candidate == id).count(), 1);
    assert_eq!(
      context
        .events
        .iter()
        .filter(|event| {
          matches!(
            event.kind,
            RuntimeEventKind::CapabilityGranted { capability_id } if capability_id == id
          )
        })
        .count(),
      1,
    );
    assert_eq!(
      runtime
        .list_events(None)
        .unwrap()
        .iter()
        .filter(|event| {
          matches!(
            event.kind,
            RuntimeEventKind::CapabilityGranted { capability_id } if capability_id == id
          )
        })
        .count(),
      1,
    );
  }

  #[test]
  fn provisional_capability_grant_is_visible_only_to_its_transaction() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .build()
      .unwrap();
    let mut owner = runtime.runtime_context().unwrap();
    let mut observer = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    runtime.begin_transaction(&mut owner).unwrap();
    runtime
      .grant_capability_with_context(
        &mut owner,
        capability(id, "task:1", true),
      )
      .unwrap();

    assert_eq!(
      runtime
        .check_capability_with_context(&mut owner, &request("task:1"))
        .unwrap(),
      id,
    );
    assert!(runtime
      .check_capability_with_context(&mut observer, &request("task:1"))
      .is_err());
    assert!(runtime.get_capability(id).unwrap().is_none());
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());

    runtime.abort_runtime_transaction(&mut owner, "test abort").unwrap();
    assert!(runtime.get_capability(id).unwrap().is_none());
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());
  }

  #[test]
  fn provisional_capability_selection_follows_stable_grant_order() {
    let first = CapabilityId(100);
    let second = CapabilityId(101);
    let mut overlay = RuntimeCapabilityOverlay::default();
    overlay
      .stage_grant(capability(first, "task:1", true))
      .unwrap();
    overlay
      .stage_grant(capability(second, "task:1", true))
      .unwrap();

    for _ in 0..32 {
      assert_eq!(
        overlay.preview_check(&request("task:1")).unwrap(),
        Some(first),
      );
    }
    assert_eq!(
      overlay.check(&request("task:1")).unwrap(),
      Some(first),
    );
    assert_eq!(
      overlay
        .grants()
        .map(|(id, _)| id)
        .collect::<Vec<_>>(),
      vec![first, second],
    );
    assert_eq!(
      overlay.usage_deltas().collect::<Vec<_>>(),
      vec![(first, 1)],
    );
  }

  #[test]
  fn capability_use_journal_savepoints_restore_only_later_uses() {
    let id = CapabilityId(100);
    let mut overlay = RuntimeCapabilityOverlay::default();
    let empty_mark = overlay.mark();

    overlay.stage_use(id).unwrap();
    assert!(!overlay.is_empty());
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(id, 1)]);

    let later_mark = overlay.mark();
    overlay.stage_use(id).unwrap();
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(id, 2)]);

    overlay.rollback_to(later_mark).unwrap();
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(id, 1)]);

    overlay.rollback_to(empty_mark).unwrap();
    assert!(overlay.is_empty());
    assert!(overlay.usage_deltas().next().is_none());
  }

  #[test]
  fn capability_use_journal_preserves_live_usage_order() {
    let first = CapabilityId(100);
    let second = CapabilityId(101);
    let mut overlay = RuntimeCapabilityOverlay::default();

    overlay.stage_use(second).unwrap();
    overlay.stage_use(first).unwrap();
    overlay.stage_use(second).unwrap();

    assert_eq!(
      overlay.usage_deltas().collect::<Vec<_>>(),
      vec![(second, 2), (first, 1)],
    );
    assert_eq!(
      overlay.pending_uses(),
      &HashMap::from([(first, 1), (second, 2)]),
    );
  }

  #[test]
  fn provisional_grant_use_revoke_cancels_all_authority_work() {
    let id = CapabilityId(100);
    let mut overlay = RuntimeCapabilityOverlay::default();

    overlay
      .stage_grant(limited_capability(id, 2))
      .unwrap();
    overlay.stage_use(id).unwrap();
    overlay.stage_revocation(id).unwrap();

    assert!(overlay.is_empty());
    assert!(overlay.grants().next().is_none());
    assert!(overlay.revocations().next().is_none());
    assert!(overlay.usage_deltas().next().is_none());
  }

  #[test]
  fn live_use_revoke_and_later_provisional_regrant_are_ordered() {
    let id = CapabilityId(100);
    let mut live_overlay = RuntimeCapabilityOverlay::default();
    live_overlay.stage_use(id).unwrap();
    live_overlay.stage_revocation(id).unwrap();
    assert_eq!(
      live_overlay.usage_deltas().collect::<Vec<_>>(),
      vec![(id, 1)],
    );
    assert_eq!(
      live_overlay.revocations().collect::<Vec<_>>(),
      vec![id],
    );

    let mut provisional_overlay = RuntimeCapabilityOverlay::default();
    provisional_overlay
      .stage_grant(limited_capability(id, 2))
      .unwrap();
    provisional_overlay.stage_use(id).unwrap();
    provisional_overlay.stage_revocation(id).unwrap();
    provisional_overlay
      .stage_grant(limited_capability(id, 2))
      .unwrap();
    provisional_overlay.stage_use(id).unwrap();
    assert_eq!(
      provisional_overlay.usage_deltas().collect::<Vec<_>>(),
      vec![(id, 1)],
    );
  }

  #[test]
  fn failed_capability_rebuild_restores_previous_valid_overlay() {
    let id = CapabilityId(100);
    let mut overlay = RuntimeCapabilityOverlay::default();
    overlay.stage_use(id).unwrap();
    overlay.stage_revocation(id).unwrap();
    let mark = overlay.mark();

    let error = overlay.stage_use(id).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeInvalidOperation");
    assert_eq!(overlay.mark(), mark);
    assert_eq!(
      overlay.usage_deltas().collect::<Vec<_>>(),
      vec![(id, 1)],
    );
    assert_eq!(overlay.revocations().collect::<Vec<_>>(), vec![id]);

    let overflow =
      RuntimeCapabilityOverlay::incremented_usage(id, u64::MAX)
        .unwrap_err();
    assert_eq!(overflow.kind_name(), "RuntimeInvalidOperation");
  }

  #[test]
  fn provisional_revocation_does_not_leak_to_other_transactions() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .build()
      .unwrap();
    let mut administrative = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    runtime
      .grant_capability_with_context(
        &mut administrative,
        capability(id, "task:1", true),
      )
      .unwrap();

    let mut owner = runtime.runtime_context().unwrap();
    let mut observer = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut owner).unwrap();
    runtime
      .revoke_capability_with_context(&mut owner, id)
      .unwrap();

    assert!(runtime
      .check_capability_with_context(&mut owner, &request("task:1"))
      .is_err());
    assert_eq!(
      runtime
        .check_capability_with_context(&mut observer, &request("task:1"))
        .unwrap(),
      id,
    );
    assert!(!runtime.capability_kernel().is_revoked(id).unwrap());

    runtime.abort_runtime_transaction(&mut owner, "test abort").unwrap();
    assert_eq!(
      runtime
        .check_capability_with_context(&mut owner, &request("task:1"))
        .unwrap(),
      id,
    );
  }

  #[test]
  fn failed_retained_operation_truncates_capability_overlay() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let id = CapabilityId(100);

    let result: MResult<()> = runtime.with_atomic_program_operation(
      &mut context,
      "test_capability_overlay",
      |runtime, context| {
        runtime.grant_capability_with_context(
          context,
          capability(id, "task:1", true),
        )?;
        Err(MechError::new(
          GenericError {
            msg: "deliberate operation failure".to_string(),
          },
          None,
        ))
      },
    );

    assert_eq!(result.unwrap_err().kind_name(), "GenericError");
    assert!(runtime
      .active_execution_transaction(transaction_id)
      .unwrap()
      .capabilities
      .is_empty());
    assert!(runtime
      .check_capability_with_context(&mut context, &request("task:1"))
      .is_err());
    assert!(runtime.get_capability(id).unwrap().is_none());
    runtime
      .abort_runtime_transaction(&mut context, "test cleanup")
      .unwrap();
  }

  #[test]
  fn capability_overlay_commits_kernel_and_store_together() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        capability(id, "task:1", true),
      )
      .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.get_capability(id).unwrap().is_some());
    assert!(runtime.capability_kernel().get(id).unwrap().is_some());
    assert_eq!(runtime.check_capability(&request("task:1")).unwrap(), id);
  }

  #[test]
  fn provisional_capability_usage_is_committed_to_live_kernel() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        limited_capability(id, 1),
      )
      .unwrap();
    assert_eq!(
      runtime
        .check_capability_with_context(&mut context, &request("task:1"))
        .unwrap(),
      id,
    );
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.check_capability(&request("task:1")).is_err());
  }

  #[test]
  fn store_failure_restores_provisional_usage_delta() {
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        limited_capability(id, 2),
      )
      .unwrap();
    runtime
      .check_capability_with_context(&mut context, &request("task:1"))
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(500), "note", "missing"),
      )
      .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(observed_kernel.get(id).unwrap().is_none());
    assert_eq!(observed_kernel.successful_uses_for_test(id), 0);
    assert_eq!(
      runtime
        .active_execution_transaction(transaction_id)
        .unwrap()
        .capabilities
        .usage_deltas()
        .collect::<Vec<_>>(),
      vec![(id, 1)],
    );

    runtime
      .abort_runtime_transaction(&mut context, "usage restore cleanup")
      .unwrap();
  }

  #[test]
  fn store_commit_failure_restores_capability_kernel_checkpoint() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        capability(id, "task:1", true),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(500), "note", "missing"),
      )
      .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());
    assert!(runtime.get_capability(id).unwrap().is_none());

    runtime
      .abort_runtime_transaction(&mut context, "test cleanup")
      .unwrap();
  }

  #[test]
  fn provisional_grant_then_revoke_cancels_commit_work() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        capability(id, "task:1", true),
      )
      .unwrap();
    runtime
      .revoke_capability_with_context(&mut context, id)
      .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.get_capability(id).unwrap().is_none());
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());
  }

  #[test]
  fn regrant_after_cancellation_commits_latest_capability() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        capability(id, "task:1", true),
      )
      .unwrap();
    runtime
      .revoke_capability_with_context(&mut context, id)
      .unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        Arc::new(BasicCapability::from_keys(
          id,
          "task:1",
          "db://projects",
          [":read"],
        )),
      )
      .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.check_capability(&request("task:1")).is_err());
    assert_eq!(
      runtime
        .check_capability(&CapabilityRequest::from_keys(
          "task:1",
          ":read",
          "db://projects",
        ))
        .unwrap(),
      id,
    );
  }

  #[test]
  fn capability_checkpoint_restore_failure_poisons_runtime() {
    let mut runtime = MechRuntime::builder()
      .id_generator(SequentialIdGenerator::starting_at(1))
      .capability_kernel(FailingCheckpointRestoreKernel::default())
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(
        &mut context,
        capability(id, "task:1", true),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(500), "note", "missing"),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    let RuntimeHealth::Poisoned(poison) = &runtime.health else {
      panic!("runtime must retain capability cleanup failure");
    };
    assert!(poison.rollback_failures.iter().any(|failure| {
      failure.contains("deliberate capability checkpoint restore failure")
    }));
  }

  #[test]
  fn provisional_capability_enforces_use_limit() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let limited: Arc<dyn Capability> = Arc::new(
      BasicCapability::from_keys(
        id,
        "task:1",
        "db://users",
        [":read"],
      )
      .with_constraints(BasicConstraints {
        max_uses: Some(1),
        ..BasicConstraints::default()
      }),
    );

    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .grant_capability_with_context(&mut context, limited)
      .unwrap();
    assert_eq!(
      runtime
        .check_capability_with_context(&mut context, &request("task:1"))
        .unwrap(),
      id,
    );
    assert!(runtime
      .check_capability_with_context(&mut context, &request("task:1"))
      .is_err());
  }

  #[test]
  fn provisional_revocation_does_not_consume_live_use_limit() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut administrative = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let limited: Arc<dyn Capability> = Arc::new(
      BasicCapability::from_keys(
        id,
        "task:1",
        "db://users",
        [":read"],
      )
      .with_constraints(BasicConstraints {
        max_uses: Some(1),
        ..BasicConstraints::default()
      }),
    );
    runtime
      .grant_capability_with_context(&mut administrative, limited)
      .unwrap();

    let mut owner = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut owner).unwrap();
    runtime
      .revoke_capability_with_context(&mut owner, id)
      .unwrap();
    assert!(runtime
      .check_capability_with_context(&mut owner, &request("task:1"))
      .is_err());
    runtime
      .abort_runtime_transaction(&mut owner, "test abort")
      .unwrap();

    assert_eq!(
      runtime.check_capability(&request("task:1")).unwrap(),
      id,
    );
  }
}
