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

  pub(super) fn with_atomic_program_operation<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    execute: impl FnOnce(
      &mut MechRuntime,
      &mut RuntimeContext,
    ) -> MResult<T>,
  ) -> MResult<T> {
    self.ensure_runtime_healthy(operation)?;
    self.validate_context_for_runtime(context)?;
    self.reject_program_operation_reentrancy(operation)?;

    let requested_transaction = context.transaction;
    if let Some(owner) = self.program_transaction_owner {
      if requested_transaction != Some(owner) {
        return Err(MechError::new(
          RuntimeProgramBusy {
            operation,
            owner,
            requester: requested_transaction,
          },
          None,
        ));
      }
    }

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
