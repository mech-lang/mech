//! Runtime-owned execution transaction coordination.
//!
//! [`RuntimeTransaction`] remains the staged-store component. The private
//! execution envelope in this module coordinates it with retained-program,
//! live-runtime, and context state.

use super::*;
use crate::AccessSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeExecutionTransactionMode {
  Explicit,
  ImplicitProgramOperation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RuntimeTransactionContextIdentity {
  runtime: RuntimeId,
  subject: String,
  task: Option<TaskId>,
  actor: Option<ActorId>,
  actor_message: Option<MessageRecord>,
  actor_state: Option<ObjectId>,
}

impl RuntimeTransactionContextIdentity {
  pub(super) fn capture(context: &RuntimeContext) -> Self {
    Self {
      runtime: context.runtime,
      subject: context.subject.clone(),
      task: context.task,
      actor: context.actor,
      actor_message: context.actor_message.clone(),
      actor_state: context.actor_state,
    }
  }

  pub(super) fn mismatch_reason(&self, context: &RuntimeContext) -> Option<String> {
    if self.runtime != context.runtime {
      return Some(format!(
        "runtime changed from {} to {}",
        self.runtime,
        context.runtime,
      ));
    }
    if self.subject != context.subject {
      return Some(format!(
        "subject changed from `{}` to `{}`",
        self.subject,
        context.subject,
      ));
    }
    if self.task != context.task {
      return Some(format!(
        "task changed from {:?} to {:?}",
        self.task,
        context.task,
      ));
    }
    if self.actor != context.actor {
      return Some(format!(
        "actor changed from {:?} to {:?}",
        self.actor,
        context.actor,
      ));
    }
    if self.actor_message != context.actor_message {
      return Some("actor message changed".to_string());
    }
    if self.actor_state != context.actor_state {
      return Some(format!(
        "actor state changed from {:?} to {:?}",
        self.actor_state,
        context.actor_state,
      ));
    }
    None
  }
}

#[derive(Clone)]
pub(super) struct RuntimeContextCheckpoint {
  runtime: RuntimeId,
  subject: String,
  task: Option<TaskId>,
  actor: Option<ActorId>,
  access: AccessSet,
  module_version: Option<ModuleVersionId>,
  transaction: Option<TransactionId>,
  capabilities: Vec<CapabilityId>,
  budget: ResourceBudget,
  events: Vec<RuntimeEvent>,
  actor_message: Option<MessageRecord>,
  actor_state: Option<ObjectId>,
}

impl RuntimeContextCheckpoint {
  pub(super) fn capture(context: &RuntimeContext) -> Self {
    Self {
      runtime: context.runtime,
      subject: context.subject.clone(),
      task: context.task,
      actor: context.actor,
      access: context.access.clone(),
      module_version: context.module_version,
      transaction: context.transaction,
      capabilities: context.capabilities.clone(),
      budget: context.budget.clone(),
      events: context.events.clone(),
      actor_message: context.actor_message.clone(),
      actor_state: context.actor_state,
    }
  }

  pub(super) fn restore_preserving_consumption(&self, context: &mut RuntimeContext) {
    let used_steps = context.budget.used_steps.max(self.budget.used_steps);
    let used_bytes = context.budget.used_bytes.max(self.budget.used_bytes);
    let used_items = context.budget.used_items.max(self.budget.used_items);
    let used_messages = context.budget.used_messages.max(self.budget.used_messages);

    context.runtime = self.runtime;
    context.subject = self.subject.clone();
    context.task = self.task;
    context.actor = self.actor;
    context.access = self.access.clone();
    context.module_version = self.module_version;
    context.transaction = self.transaction;
    context.capabilities = self.capabilities.clone();
    context.budget = ResourceBudget {
      max_steps: self.budget.max_steps,
      used_steps,
      max_bytes: self.budget.max_bytes,
      used_bytes,
      max_items: self.budget.max_items,
      used_items,
      max_messages: self.budget.max_messages,
      used_messages,
    };
    context.events = self.events.clone();
    context.actor_message = self.actor_message.clone();
    context.actor_state = self.actor_state;
  }

  pub(super) fn access_delta(&self, context: &RuntimeContext) -> AccessSet {
    AccessSet {
      reads: context
        .access
        .reads
        .iter()
        .copied()
        .filter(|object| !self.access.reads.contains(object))
        .collect(),
      writes: context
        .access
        .writes
        .iter()
        .copied()
        .filter(|object| !self.access.writes.contains(object))
        .collect(),
    }
  }
}

#[derive(Clone)]
pub(super) struct RuntimeProgramBaseline {
  pub(super) program: MechProgramCheckpoint,
  pub(super) live: RuntimeLiveStateSnapshot,
}

#[derive(Clone)]
pub(super) struct RuntimeExecutionTransaction {
  pub(super) store: RuntimeTransaction,
  pub(super) mode: RuntimeExecutionTransactionMode,
  pub(super) context_identity: RuntimeTransactionContextIdentity,
  pub(super) context_baseline: RuntimeContextCheckpoint,
  pub(super) program: Option<RuntimeProgramBaseline>,
}

impl RuntimeExecutionTransaction {
  pub(super) fn new(
    store: RuntimeTransaction,
    mode: RuntimeExecutionTransactionMode,
    context_identity: RuntimeTransactionContextIdentity,
    context_baseline: RuntimeContextCheckpoint,
  ) -> Self {
    Self {
      store,
      mode,
      context_identity,
      context_baseline,
      program: None,
    }
  }
}

#[derive(Clone)]
pub(super) struct RuntimeProgramOperationSavepoint {
  pub(super) program: MechProgramCheckpoint,
  pub(super) live: RuntimeLiveStateSnapshot,
  pub(super) transaction: RuntimeExecutionTransaction,
  pub(super) context: RuntimeContextCheckpoint,
}

#[derive(Clone, Debug)]
pub(super) struct ActiveRuntimeProgramOperation {
  pub(super) transaction_id: TransactionId,
  pub(super) operation: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeHealth {
  Healthy,
  Poisoned(RuntimePoisonRecord),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePoisonRecord {
  pub operation: String,
  pub transaction_id: Option<TransactionId>,
  pub original_error: String,
  pub rollback_failures: Vec<String>,
}

impl MechRuntime {
  pub(super) fn ensure_runtime_healthy(
    &self,
    operation: &'static str,
  ) -> MResult<()> {
    match &self.health {
      RuntimeHealth::Healthy => Ok(()),
      RuntimeHealth::Poisoned(poison) => Err(MechError::new(
        RuntimePoisoned {
          operation,
          poison: poison.clone(),
        },
        None,
      )),
    }
  }

  pub(super) fn reject_program_operation_reentrancy(
    &self,
    requested_operation: &'static str,
  ) -> MResult<()> {
    let Some(active) = &self.active_program_operation else {
      return Ok(());
    };
    Err(MechError::new(
      RuntimeProgramOperationReentrant {
        active_operation: active.operation,
        requested_operation,
        transaction_id: active.transaction_id,
      },
      None,
    ))
  }

  pub(super) fn reject_transactional_reactive_turn(
    &self,
    context: &RuntimeContext,
    operation: &'static str,
  ) -> MResult<()> {
    if context.transaction.is_none() && self.program_transaction_owner.is_none() {
      return Ok(());
    }
    Err(MechError::new(
      RuntimeTransactionalReactiveTurnUnsupported {
        operation,
        transaction_id: context.transaction,
        owner: self.program_transaction_owner,
      },
      None,
    ))
  }

  pub(super) fn poison_program_operation(
    &mut self,
    operation: &'static str,
    transaction_id: Option<TransactionId>,
    original_error: String,
    rollback_failures: Vec<String>,
  ) -> MechError {
    self.health = RuntimeHealth::Poisoned(RuntimePoisonRecord {
      operation: operation.to_string(),
      transaction_id,
      original_error: original_error.clone(),
      rollback_failures: rollback_failures.clone(),
    });
    MechError::new(
      RuntimeProgramRollbackFailed {
        operation,
        transaction_id,
        original_error,
        rollback_failures,
      },
      None,
    )
  }

  fn coordinator_invariant_failure<T>(
    &mut self,
    operation: &'static str,
    transaction_id: Option<TransactionId>,
    reason: impl Into<String>,
  ) -> MResult<T> {
    let reason = reason.into();
    Err(self.poison_program_operation(
      operation,
      transaction_id,
      "runtime program coordinator preflight failed".to_string(),
      vec![reason],
    ))
  }

  fn rollback_program_operation(
    &mut self,
    context: &mut RuntimeContext,
    transaction_id: TransactionId,
    savepoint: &RuntimeProgramOperationSavepoint,
  ) -> Vec<String> {
    let mut failures = Vec::new();

    if let Err(error) = self.program.restore(savepoint.program.clone()) {
      failures.push(format!("program restore failed: {:?}", error));
    }

    self.restore_live_state(savepoint.live.clone());

    if !self.active_transactions.contains_key(&transaction_id) {
      failures.push(format!(
        "active execution transaction {} disappeared before operation rollback",
        transaction_id,
      ));
    }
    self.active_transactions.insert(
      transaction_id,
      savepoint.transaction.clone(),
    );

    savepoint
      .context
      .restore_preserving_consumption(context);

    if let Err(error) = self.validate_context_for_runtime(context) {
      failures.push(format!("context restore invariant failed: {:?}", error));
    }

    failures
  }

  pub(super) fn preflight_atomic_program_operation(
    &self,
    context: &RuntimeContext,
    operation: &'static str,
  ) -> MResult<()> {
    self.ensure_runtime_healthy(operation)?;
    self.validate_context_for_runtime(context)?;
    self.reject_program_operation_reentrancy(operation)?;

    if let Some(owner) = self.program_transaction_owner {
      if context.transaction != Some(owner) {
        return Err(MechError::new(
          RuntimeProgramBusy {
            operation,
            owner,
            requester: context.transaction,
          },
          None,
        ));
      }
    }

    Ok(())
  }

  pub(super) fn with_atomic_program_operation<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    execute: impl FnOnce(
      &mut MechRuntime,
      &mut RuntimeContext,
    ) -> MResult<T>,
  ) -> MResult<T> {
    self.preflight_atomic_program_operation(context, operation)?;

    let requested_transaction = context.transaction;
    let implicit = requested_transaction.is_none();
    let mut first_explicit_operation = false;

    let (transaction_id, program_checkpoint, live_checkpoint) = if implicit {
      let program_checkpoint = self.program.checkpoint()?;
      let live_checkpoint = self.live_state_snapshot();
      let transaction_id = self.begin_runtime_transaction_internal(
        context,
        RuntimeExecutionTransactionMode::ImplicitProgramOperation,
      )?;
      self.active_execution_transaction_mut(transaction_id)?.program =
        Some(RuntimeProgramBaseline {
          program: program_checkpoint.clone(),
          live: live_checkpoint.clone(),
        });
      self.program_transaction_owner = Some(transaction_id);
      (transaction_id, program_checkpoint, live_checkpoint)
    } else {
      let transaction_id = requested_transaction.unwrap();
      let mode = self.active_execution_transaction(transaction_id)?.mode;
      if mode != RuntimeExecutionTransactionMode::Explicit {
        return self.coordinator_invariant_failure(
          operation,
          Some(transaction_id),
          "a public retained operation entered an implicit execution transaction without reentrancy detection",
        );
      }

      let program_checkpoint = self.program.checkpoint()?;
      let live_checkpoint = self.live_state_snapshot();
      if self.program_transaction_owner.is_none() {
        self.active_execution_transaction_mut(transaction_id)?.program =
          Some(RuntimeProgramBaseline {
            program: program_checkpoint.clone(),
            live: live_checkpoint.clone(),
          });
        self.program_transaction_owner = Some(transaction_id);
        first_explicit_operation = true;
      } else if self.active_execution_transaction(transaction_id)?.program.is_none() {
        return self.coordinator_invariant_failure(
          operation,
          Some(transaction_id),
          "program ownership exists without a transaction program baseline",
        );
      }
      (transaction_id, program_checkpoint, live_checkpoint)
    };

    let savepoint = RuntimeProgramOperationSavepoint {
      program: program_checkpoint,
      live: live_checkpoint,
      transaction: self
        .active_execution_transaction(transaction_id)?
        .clone(),
      context: RuntimeContextCheckpoint::capture(context),
    };

    self.active_program_operation = Some(ActiveRuntimeProgramOperation {
      transaction_id,
      operation,
    });
    let execution_result = execute(self, context);
    self.active_program_operation = None;

    let original_error = match execution_result {
      Ok(value) if implicit => match self.commit_runtime_transaction_internal(context) {
        Ok(_) => return Ok(value),
        Err(error) => error,
      },
      Ok(value) => return Ok(value),
      Err(error) => error,
    };

    let original_error_text = format!("{:?}", original_error);
    let rollback_failures = self.rollback_program_operation(
      context,
      transaction_id,
      &savepoint,
    );

    if rollback_failures.is_empty() {
      if implicit {
        let _ = self.abort_runtime_transaction_internal(
          context,
          format!("retained program operation `{}` failed", operation),
          false,
        );
      } else if first_explicit_operation {
        self.program_transaction_owner = None;
        if let Ok(transaction) =
          self.active_execution_transaction_mut(transaction_id)
        {
          transaction.program = None;
        }
      }
      return Err(original_error);
    }

    if implicit {
      let _ = self.abort_runtime_transaction_internal(
        context,
        format!("retained program operation `{}` rollback failed", operation),
        false,
      );
      self.program_transaction_owner = None;
    }

    Err(self.poison_program_operation(
      operation,
      Some(transaction_id),
      original_error_text,
      rollback_failures,
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::VecDeque;
  use std::sync::Mutex;

  use crate::capability::{
    BasicCapability, BasicOperation, BasicResource, BasicSubject,
  };
  use crate::{
    ClosureHostFunction, NodeId,
  };

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

  #[test]
  fn program_transaction_outer_abort_restores_program_baseline() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let root_interpreter_id = runtime.program.interpreter().id;
    let plan_len_before = runtime.program.interpreter().plan_len();

    runtime
      .with_atomic_program_operation(
        &mut context,
        "program_transaction_test",
        |runtime, _context| {
          runtime.program.run_source(&MechSourceCode::String(
            "round3-owned := 42".to_string(),
          ))
        },
      )
      .unwrap();

    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(
      runtime
        .program
        .root_symbol_value("round3-owned")
        .is_ok(),
    );

    runtime
      .abort_runtime_transaction(&mut context, "round3 test abort")
      .unwrap();

    assert_eq!(context.transaction, None);
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(runtime.program.interpreter().id, root_interpreter_id);
    assert_eq!(runtime.program.interpreter().plan_len(), plan_len_before);
    assert!(runtime.program.root_symbol_value("round3-owned").is_err());
  }

  #[test]
  fn program_transaction_implicit_success_commits_program_store_and_events() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();

    runtime
      .run_string_with_context(
        &mut context,
        "round3-implicit-success := 7",
      )
      .unwrap();

    assert!(
      runtime
        .program
        .root_symbol_value("round3-implicit-success")
        .is_ok(),
    );
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert_eq!(runtime.list_transactions(None).unwrap().len(), 1);
    let events = runtime.list_events(None).unwrap();
    assert!(events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::ProgramCompleted { .. }
      )
    }));
    assert!(events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::TransactionCommitted { .. }
      )
    }));
  }

  #[test]
  fn program_transaction_implicit_partial_failure_restores_everything() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime
      .run_string("round3-anchor := 1")
      .unwrap();
    let anchor = runtime
      .program
      .interpreter()
      .symbols()
      .borrow()
      .get(hash_str("round3-anchor"))
      .unwrap()
      .clone();
    let anchor_address = anchor.addr();
    let plan_len_before = runtime.program.interpreter().plan_len();
    let live_before = runtime.live_state_snapshot();
    let transactions_before = runtime.list_transactions(None).unwrap().len();
    let events_before = runtime.list_events(None).unwrap().len();
    let mut context = runtime.runtime_context().unwrap();
    let source = MechSourceCode::Program(vec![
      MechSourceCode::String(
        "round3-partial := round3-anchor + 1".to_string(),
      ),
      MechSourceCode::String(
        "round3-failure := missing-round3-value + 1".to_string(),
      ),
    ]);

    let error = runtime.run_source_with_context(&mut context, &source);

    assert!(error.is_err());
    assert!(runtime.program.root_symbol_value("round3-partial").is_err());
    assert_eq!(runtime.program.interpreter().plan_len(), plan_len_before);
    assert_eq!(
      runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("round3-anchor"))
        .unwrap()
        .addr(),
      anchor_address,
    );
    assert_eq!(
      runtime.live_state_snapshot().context_template.is_some(),
      live_before.context_template.is_some(),
    );
    assert_eq!(
      runtime.live_state_snapshot().input_bindings,
      live_before.input_bindings,
    );
    assert_eq!(
      runtime.live_state_snapshot().persistent_sends.len(),
      live_before.persistent_sends.len(),
    );
    assert_eq!(
      runtime.live_state_snapshot().registration_mode,
      live_before.registration_mode,
    );
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert_eq!(
      runtime.list_transactions(None).unwrap().len(),
      transactions_before,
    );
    let events = runtime.list_events(None).unwrap();
    let new_events = &events[events_before..];
    assert!(new_events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::TransactionAborted { .. }
      )
    }));
    assert!(new_events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::ProgramFailed { .. }
      )
    }));
    assert!(!new_events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::ProgramCompleted { .. }
      )
    }));
  }

  #[test]
  fn explicit_program_operations_use_savepoints_before_outer_abort() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .run_string_with_context(&mut context, "round3-a := 1")
      .unwrap();
    let plan_len_after_a = runtime.program.interpreter().plan_len();
    let events_after_a = context.events.clone();
    let access_after_a = context.access.clone();
    let staged_events_after_a = runtime
      .active_transaction_mut(transaction_id)
      .unwrap()
      .staged_event_ids();

    let failure: MResult<()> = runtime.with_atomic_program_operation(
      &mut context,
      "explicit_b_test",
      |runtime, context| {
        runtime.program.run_source(&MechSourceCode::String(
          "round3-b := round3-a + 1".to_string(),
        ))?;
        runtime
          .active_transaction_mut(transaction_id)?
          .stage_put_object(ObjectRecord::text(
            ObjectId(350),
            "note",
            "B provisional",
          ))?;
        context.record_write(ObjectId(350));
        runtime.emit_event_to_context(
          context,
          RuntimeEventKind::ObjectCreated {
            object_id: ObjectId(350),
          },
        )?;
        Err(MechError::new(
          RuntimeInvalidOperationError {
            operation: "explicit_b_test",
            reason: "deliberate B failure".to_string(),
          },
          None,
        ))
      },
    );

    assert!(failure.is_err());
    assert!(runtime.program.root_symbol_value("round3-a").is_ok());
    assert!(runtime.program.root_symbol_value("round3-b").is_err());
    assert_eq!(runtime.program.interpreter().plan_len(), plan_len_after_a);
    assert_eq!(context.events, events_after_a);
    assert_eq!(context.access, access_after_a);
    let transaction = runtime.active_transaction_mut(transaction_id).unwrap();
    assert_eq!(transaction.staged_puts().count(), 0);
    assert_eq!(transaction.staged_event_ids(), staged_events_after_a);
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));

    runtime
      .run_string_with_context(
        &mut context,
        "round3-c := round3-a + 2",
      )
      .unwrap();
    assert!(runtime.program.root_symbol_value("round3-c").is_ok());

    runtime
      .abort_runtime_transaction(&mut context, "discard A and C")
      .unwrap();

    assert!(runtime.program.root_symbol_value("round3-a").is_err());
    assert!(runtime.program.root_symbol_value("round3-b").is_err());
    assert!(runtime.program.root_symbol_value("round3-c").is_err());
    assert!(runtime.get_object(ObjectId(350)).unwrap().is_none());
    assert_eq!(runtime.program_transaction_owner, None);
  }

  #[test]
  fn explicit_program_commit_keeps_program_and_commits_access_delta() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.record_read(ObjectId(70));
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .run_string_with_context(
        &mut context,
        "round3-committed := 41 + 1",
      )
      .unwrap();
    context.record_read(ObjectId(71));
    context.record_write(ObjectId(72));

    runtime
      .commit_runtime_transaction(&mut context)
      .unwrap();

    assert!(runtime
      .program
      .root_symbol_value("round3-committed")
      .is_ok());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    assert!(!record.read_set.contains(&ObjectId(70)));
    assert!(record.read_set.contains(&ObjectId(71)));
    assert!(record.write_set.contains(&ObjectId(72)));
  }

  #[test]
  fn explicit_commit_failure_keeps_program_provisional_until_abort() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .run_string_with_context(
        &mut context,
        "round3-provisional := 42",
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(200), "note", "missing"),
      )
      .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert!(runtime
      .program
      .root_symbol_value("round3-provisional")
      .is_ok());
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
      .abort_runtime_transaction(&mut context, "failed commit")
      .unwrap();

    assert!(runtime
      .program
      .root_symbol_value("round3-provisional")
      .is_err());
    assert_eq!(runtime.program_transaction_owner, None);
  }

  #[test]
  fn one_transaction_owns_program_while_other_store_work_remains_allowed() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context_a = runtime.runtime_context().unwrap();
    let transaction_a = runtime.begin_transaction(&mut context_a).unwrap();
    runtime
      .run_string_with_context(&mut context_a, "round3-owner-a := 1")
      .unwrap();

    let mut context_b = runtime.runtime_context().unwrap();
    let transaction_b = runtime.begin_transaction(&mut context_b).unwrap();
    runtime
      .put_object_with_context(
        &mut context_b,
        ObjectRecord::text(ObjectId(300), "note", "B store-only"),
      )
      .unwrap();

    let b_error = runtime
      .run_string_with_context(&mut context_b, "round3-owner-b := 2")
      .unwrap_err();
    assert_eq!(b_error.kind_name(), "RuntimeProgramBusy");

    let mut unowned_context = runtime.runtime_context().unwrap();
    let implicit_error = runtime
      .run_string_with_context(
        &mut unowned_context,
        "round3-unowned := 3",
      )
      .unwrap_err();
    assert_eq!(implicit_error.kind_name(), "RuntimeProgramBusy");
    assert_eq!(runtime.program_transaction_owner, Some(transaction_a));

    runtime
      .abort_runtime_transaction(&mut context_a, "release A")
      .unwrap();
    runtime
      .run_string_with_context(&mut context_b, "round3-owner-b := 2")
      .unwrap();

    assert_eq!(runtime.program_transaction_owner, Some(transaction_b));
    assert!(runtime.program.root_symbol_value("round3-owner-b").is_ok());
    assert!(runtime.get_object(ObjectId(300)).unwrap().is_none());

    runtime
      .abort_runtime_transaction(&mut context_b, "release B")
      .unwrap();
    assert!(runtime.program.root_symbol_value("round3-owner-b").is_err());
    assert!(runtime.get_object(ObjectId(300)).unwrap().is_none());
  }

  #[test]
  fn failed_first_explicit_operation_releases_program_ownership() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context_a = runtime.runtime_context().unwrap();
    let transaction_a = runtime.begin_transaction(&mut context_a).unwrap();

    assert!(runtime
      .run_string_with_context(
        &mut context_a,
        "round3-first-fails := missing-round3-first + 1",
      )
      .is_err());
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(runtime.active_transactions.contains_key(&transaction_a));
    assert!(runtime
      .active_execution_transaction(transaction_a)
      .unwrap()
      .program
      .is_none());

    let mut context_b = runtime.runtime_context().unwrap();
    let transaction_b = runtime.begin_transaction(&mut context_b).unwrap();
    runtime
      .run_string_with_context(&mut context_b, "round3-after-failure := 2")
      .unwrap();
    assert_eq!(runtime.program_transaction_owner, Some(transaction_b));

    runtime
      .abort_runtime_transaction(&mut context_b, "release B")
      .unwrap();
    runtime
      .abort_runtime_transaction(&mut context_a, "release A")
      .unwrap();
  }

  #[test]
  fn failed_operation_restores_context_and_staging_but_keeps_budget_usage() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.capabilities = vec![CapabilityId(10)];
    context.budget = ResourceBudget::default()
      .with_max_steps(100)
      .with_max_bytes(100)
      .with_max_items(100)
      .with_max_messages(100);
    context.charge_step().unwrap();
    context.record_read(ObjectId(10));
    let baseline = context.clone();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let operation_events = context.events.clone();
    let staged_events = runtime
      .active_transaction_mut(transaction_id)
      .unwrap()
      .staged_event_ids();

    let result: MResult<()> = runtime.with_atomic_program_operation(
      &mut context,
      "context_rollback_test",
      |runtime, context| {
        context.charge_steps(3)?;
        context.charge_bytes(4)?;
        context.charge_items(5)?;
        context.charge_messages(6)?;
        context.record_read(ObjectId(11));
        context.record_write(ObjectId(12));
        runtime
          .active_transaction_mut(transaction_id)?
          .stage_put_object(ObjectRecord::text(
            ObjectId(400),
            "note",
            "provisional",
          ))?;
        runtime.emit_event_to_context(
          context,
          RuntimeEventKind::ObjectCreated {
            object_id: ObjectId(400),
          },
        )?;

        context.runtime = RuntimeId(999);
        context.subject = "mutated-subject".to_string();
        context.task = Some(TaskId(20));
        context.actor = Some(ActorId(21));
        context.module_version = Some(ModuleVersionId(22));
        context.transaction = None;
        context.capabilities = vec![CapabilityId(23)];
        context.budget.max_steps = Some(4);
        context.budget.max_bytes = Some(5);
        context.budget.max_items = Some(6);
        context.budget.max_messages = Some(7);
        context.actor_message = Some(MessageRecord::new(
          MessageId(24),
          ActorId(21),
          "mutated",
          Vec::new(),
        ));
        context.actor_state = Some(ObjectId(25));

        Err(MechError::new(
          RuntimeInvalidOperationError {
            operation: "context_rollback_test",
            reason: "deliberate failure".to_string(),
          },
          None,
        ))
      },
    );

    assert!(result.is_err());
    assert_eq!(context.runtime, baseline.runtime);
    assert_eq!(context.subject, baseline.subject);
    assert_eq!(context.task, baseline.task);
    assert_eq!(context.actor, baseline.actor);
    assert_eq!(context.module_version, baseline.module_version);
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(context.capabilities, baseline.capabilities);
    assert_eq!(context.access, baseline.access);
    assert_eq!(context.events, operation_events);
    assert_eq!(context.actor_message, baseline.actor_message);
    assert_eq!(context.actor_state, baseline.actor_state);
    assert_eq!(context.budget.max_steps, Some(100));
    assert_eq!(context.budget.max_bytes, Some(100));
    assert_eq!(context.budget.max_items, Some(100));
    assert_eq!(context.budget.max_messages, Some(100));
    assert_eq!(context.budget.used_steps, baseline.budget.used_steps + 3);
    assert_eq!(context.budget.used_bytes, baseline.budget.used_bytes + 4);
    assert_eq!(context.budget.used_items, baseline.budget.used_items + 5);
    assert_eq!(
      context.budget.used_messages,
      baseline.budget.used_messages + 6,
    );
    let transaction = runtime.active_transaction_mut(transaction_id).unwrap();
    assert_eq!(transaction.staged_puts().count(), 0);
    assert_eq!(transaction.staged_event_ids(), staged_events);

    runtime
      .abort_runtime_transaction(&mut context, "context rollback complete")
      .unwrap();
    assert_eq!(context.transaction, None);
    assert_eq!(context.budget.used_steps, baseline.budget.used_steps + 3);
  }

  #[test]
  fn transaction_context_identity_includes_task_actor_message_and_state() {
    fn assert_mismatch(mutate: impl FnOnce(&mut RuntimeContext)) {
      let mut runtime = MechRuntime::builder().build().unwrap();
      let mut context = runtime.runtime_context().unwrap();
      let baseline = context.clone();
      let transaction_id = runtime.begin_transaction(&mut context).unwrap();
      mutate(&mut context);

      let error = runtime
        .run_string_with_context(&mut context, "identity-test := 1")
        .unwrap_err();
      assert_eq!(error.kind_name(), "RuntimeTransactionContextMismatch");
      assert!(runtime.program.root_symbol_value("identity-test").is_err());

      context = baseline;
      context.transaction = Some(transaction_id);
      runtime
        .abort_runtime_transaction(&mut context, "identity mismatch test")
        .unwrap();
    }

    assert_mismatch(|context| context.task = Some(TaskId(1)));
    assert_mismatch(|context| context.actor = Some(ActorId(2)));
    assert_mismatch(|context| {
      context.actor_message = Some(MessageRecord::new(
        MessageId(3),
        ActorId(2),
        "identity",
        Vec::new(),
      ));
    });
    assert_mismatch(|context| context.actor_state = Some(ObjectId(4)));
  }

  #[test]
  fn host_callback_cannot_reenter_program_or_transaction_lifecycle() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
      .grant_capability(Arc::new(BasicCapability::new(
        CapabilityId(500),
        &BasicSubject::new(&subject),
        &BasicResource::new("host:demo/reenter"),
        [BasicOperation::new("call")],
      )))
      .unwrap();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observed_for_host = observed.clone();
    runtime
      .register_mech_host_function(ClosureHostFunction::new(
        "demo/reenter",
        move |_services, _context, _args| {
          ACTIVE_RUNTIME_PROGRAM_HOST.with(|slot| {
            let target = slot
              .borrow()
              .expect("runtime program host target should be active");
            let runtime = unsafe { &mut *target.runtime };
            let context = unsafe { &mut *target.context };
            let errors = [
              runtime
                .run_string_with_context(context, "nested-run := 1")
                .unwrap_err(),
              runtime.begin_transaction(context).unwrap_err(),
              runtime.commit_runtime_transaction(context).unwrap_err(),
              runtime
                .abort_runtime_transaction(context, "nested abort")
                .unwrap_err(),
            ];
            observed_for_host
              .lock()
              .unwrap()
              .extend(errors.into_iter().map(|error| error.kind_name()));
          });
          Err(MechError::new(
            RuntimeInvalidOperationError {
              operation: "demo/reenter",
              reason: "reject outer operation after reentrancy probes".to_string(),
            },
            None,
          ))
        },
      ))
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let outer_error = runtime
      .run_string_with_context(
        &mut context,
        "reentrant-result := demo/reenter()",
      )
      .unwrap_err();

    assert_eq!(outer_error.kind_name(), "RuntimeInvalidOperation");
    let observed = observed.lock().unwrap();
    assert!(!observed.is_empty());
    assert_eq!(observed.len() % 4, 0);
    assert!(observed
      .iter()
      .all(|kind| kind == "RuntimeProgramOperationReentrant"));
    assert!(runtime.program.root_symbol_value("reentrant-result").is_err());
    assert!(runtime.program.root_symbol_value("nested-run").is_err());
    assert!(runtime.active_transactions.is_empty());
  }

  #[test]
  fn completion_event_staging_failure_rolls_back_implicit_program() {
    let mut runtime = MechRuntime::builder()
      .id_generator(ScriptedEventIdGenerator::new(
        1,
        [
          EventId(100),
          EventId(101),
          EventId(102),
          EventId(102),
          EventId(103),
          EventId(104),
        ],
      ))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
      .run_string_with_context(
        &mut context,
        "round3-completion-event-failure := 1",
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "InvalidRuntimeTransaction");
    assert!(runtime
      .program
      .root_symbol_value("round3-completion-event-failure")
      .is_err());
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.list_transactions(None).unwrap().is_empty());
    let events = runtime.list_events(None).unwrap();
    assert!(events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::TransactionAborted { .. })
    }));
    assert!(events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::ProgramFailed { .. })
    }));
    assert!(!events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::ProgramCompleted { .. })
    }));
    assert!(!runtime.is_poisoned());
  }

  #[test]
  fn incomplete_program_restore_poisons_retained_execution_until_abort() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime.run_string("round3-poison-anchor := 1").unwrap();
    assert!(runtime.program.interpreter().plan_len() > 0);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let result: MResult<()> = runtime.with_atomic_program_operation(
      &mut context,
      "poison_test",
      |runtime, _context| {
        runtime.program.interpreter().plan().0.borrow_mut().clear();
        Err(MechError::new(
          RuntimeInvalidOperationError {
            operation: "poison_test",
            reason: "deliberate original failure".to_string(),
          },
          None,
        ))
      },
    );

    let error = result.unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert!(runtime.is_poisoned());
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    let poison = match runtime.health() {
      RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
      RuntimeHealth::Poisoned(poison) => poison,
    };
    assert_eq!(poison.operation, "poison_test");
    assert!(poison.original_error.contains("deliberate original failure"));
    assert!(!poison.rollback_failures.is_empty());

    assert_eq!(
      runtime.run_string("round3-poison-rejected := 1").unwrap_err().kind_name(),
      "RuntimePoisoned",
    );
    let mut fresh_context = runtime.runtime_context().unwrap();
    assert_eq!(
      runtime.begin_transaction(&mut fresh_context).unwrap_err().kind_name(),
      "RuntimePoisoned",
    );
    assert_eq!(
      runtime
        .commit_runtime_transaction(&mut context)
        .unwrap_err()
        .kind_name(),
      "RuntimePoisoned",
    );
    assert!(runtime.list_events(None).is_ok());
    assert!(runtime.program().root_symbol_value("round3-poison-anchor").is_ok());

    let abort_error = runtime
      .abort_runtime_transaction(&mut context, "release poisoned owner")
      .unwrap_err();
    assert_eq!(abort_error.kind_name(), "RuntimeProgramRollbackFailed");
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.shutdown().is_ok());
  }
}
