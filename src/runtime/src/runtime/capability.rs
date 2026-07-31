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
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;
    capability.validate()?;

    let id = capability.id();

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
    let mut context = self.runtime_context()?;
    self.revoke_capability_with_context(&mut context, capability)
  }

  pub fn revoke_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    capability: CapabilityId,
  ) -> MResult<()> {
    self.validate_context_for_runtime(context)?;
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
    self.validate_context_for_runtime(context)?;
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

#[cfg(test)]
mod tests {
  use super::*;

  use std::collections::VecDeque;
  use std::sync::atomic::{AtomicBool, Ordering};

  use mech_core::{GenericError, MResult, MechError};

  use crate::capability::{
    BasicCapability, BasicCapabilityKernel, CapabilityDerivation, CapabilityGrant,
    CapabilityKernel, Subject,
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
}
