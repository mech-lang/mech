//! Runtime-owned effect journal and lifecycle mechanics.

use super::*;
use crate::{
  PreparedRuntimeEffect, RuntimeEffectFailure, RuntimeEffectFailurePhase,
  RuntimeEffectId, RuntimeEffectRecord,
};
#[cfg(test)]
use crate::{
  RuntimeAfterCommitEffect, RuntimeCompensatableEffect,
  RuntimeEffectMetadata, RuntimeEffectSource, RuntimeTransactionalEffect,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeEffectState {
  Staged,
  Prepared,
  Applied,
}

#[derive(Debug)]
pub(super) struct RuntimeEffectEntry {
  pub(super) id: RuntimeEffectId,
  pub(super) state: RuntimeEffectState,
  pub(super) effect: PreparedRuntimeEffect,
  resource_write: Option<RuntimeStagedResourceWrite>,
}

#[derive(Debug)]
struct RuntimeStagedResourceWrite {
  base_uri: String,
  path: String,
  value: Value,
}

pub(super) struct RuntimeEffectStepFailure {
  pub(super) failure: RuntimeEffectFailure,
  pub(super) error: MechError,
}

pub(super) struct RuntimeTransactionalCommitReport {
  pub(super) committed: Vec<RuntimeEffectId>,
  pub(super) failures: Vec<RuntimeEffectStepFailure>,
  pub(super) participant_outcomes: Vec<String>,
}

#[derive(Debug, Default)]
pub(super) struct RuntimeEffectJournal {
  entries: Vec<RuntimeEffectEntry>,
  next_sequence: u64,
}

impl RuntimeEffectJournal {
  pub(super) fn new() -> Self {
    Self::default()
  }

  pub(super) fn mark(&self) -> usize {
    self.entries.len()
  }

  pub(super) fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  pub(super) fn records(&self) -> Vec<RuntimeEffectRecord> {
    self.entries.iter().map(|entry| {
      RuntimeEffectRecord::new(
        entry.id,
        entry.effect.metadata(),
        entry.effect.protocol(),
      )
    }).collect()
  }

  pub(super) fn prepared_transactional_ids(
    &self,
  ) -> Vec<RuntimeEffectId> {
    self.entries.iter().filter_map(|entry| {
      if entry.state == RuntimeEffectState::Prepared
        && matches!(entry.effect, PreparedRuntimeEffect::Transactional(_))
      {
        Some(entry.id)
      } else {
        None
      }
    }).collect()
  }

  pub(super) fn applied_compensatable_ids(
    &self,
  ) -> Vec<RuntimeEffectId> {
    self.entries.iter().filter_map(|entry| {
      if entry.state == RuntimeEffectState::Applied
        && matches!(entry.effect, PreparedRuntimeEffect::Compensatable(_))
      {
        Some(entry.id)
      } else {
        None
      }
    }).collect()
  }

  pub(super) fn after_commit_ids(&self) -> Vec<RuntimeEffectId> {
    self.entries.iter().filter_map(|entry| {
      if matches!(entry.effect, PreparedRuntimeEffect::AfterCommit(_)) {
        Some(entry.id)
      } else {
        None
      }
    }).collect()
  }

  pub(super) fn abortable_ids(&self) -> Vec<RuntimeEffectId> {
    self.entries.iter().filter_map(|entry| {
      if matches!(entry.effect, PreparedRuntimeEffect::AfterCommit(_)) {
        None
      } else {
        Some(entry.id)
      }
    }).collect()
  }

  pub(super) fn abortable_ids_after(
    &self,
    mark: usize,
  ) -> Vec<RuntimeEffectId> {
    self.entries
      .get(mark..)
      .unwrap_or_default()
      .iter()
      .filter_map(|entry| {
        if matches!(entry.effect, PreparedRuntimeEffect::AfterCommit(_)) {
          None
        } else {
          Some(entry.id)
        }
      })
      .collect()
  }

  pub(super) fn validate_active(
    &self,
    transaction: TransactionId,
  ) -> Vec<String> {
    let mut failures = Vec::new();
    let mut previous_sequence = None;

    for entry in &self.entries {
      if entry.id.transaction != transaction {
        failures.push(format!(
          "effect {} belongs to transaction {}, expected {}",
          entry.id,
          entry.id.transaction,
          transaction,
        ));
      }
      if entry.state != RuntimeEffectState::Staged {
        failures.push(format!(
          "effect {} entered active commit in state {:?}",
          entry.id,
          entry.state,
        ));
      }
      if previous_sequence.is_some_and(|previous| {
        entry.id.sequence <= previous
      }) {
        failures.push(format!(
          "effect {} is not in strictly increasing sequence order",
          entry.id,
        ));
      }
      previous_sequence = Some(entry.id.sequence);
    }

    if previous_sequence.is_some_and(|sequence| {
      self.next_sequence <= sequence
    }) {
      failures.push(format!(
        "effect next sequence {} does not advance past the journal tail",
        self.next_sequence,
      ));
    }

    failures
  }

  #[cfg(test)]
  pub(super) fn len(&self) -> usize {
    self.entries.len()
  }

  #[cfg(test)]
  pub(super) fn next_sequence(&self) -> u64 {
    self.next_sequence
  }

  pub(super) fn stage(
    &mut self,
    transaction: TransactionId,
    effect: PreparedRuntimeEffect,
  ) -> RuntimeEffectId {
    self.stage_entry(transaction, effect, None)
  }

  pub(super) fn stage_resource_write(
    &mut self,
    transaction: TransactionId,
    effect: PreparedRuntimeEffect,
    base_uri: String,
    path: String,
    value: Value,
  ) -> RuntimeEffectId {
    self.stage_entry(
      transaction,
      effect,
      Some(RuntimeStagedResourceWrite {
        base_uri,
        path,
        value,
      }),
    )
  }

  fn stage_entry(
    &mut self,
    transaction: TransactionId,
    effect: PreparedRuntimeEffect,
    resource_write: Option<RuntimeStagedResourceWrite>,
  ) -> RuntimeEffectId {
    let id = RuntimeEffectId {
      transaction,
      sequence: self.next_sequence,
    };
    self.next_sequence = self.next_sequence.saturating_add(1);
    self.entries.push(RuntimeEffectEntry {
      id,
      state: RuntimeEffectState::Staged,
      effect,
      resource_write,
    });
    id
  }

  pub(super) fn staged_resource_value(
    &self,
    base_uri: &str,
    path: &str,
  ) -> Option<Value> {
    self.entries.iter().rev().find_map(|entry| {
      let write = entry.resource_write.as_ref()?;
      if write.base_uri == base_uri && write.path == path {
        Some(write.value.clone())
      } else {
        None
      }
    })
  }

  pub(super) fn rollback_to(
    &mut self,
    mark: usize,
  ) -> Vec<RuntimeEffectFailure> {
    if mark > self.entries.len() {
      return vec![RuntimeEffectFailure {
        effect_id: RuntimeEffectId {
          transaction: self
            .entries
            .first()
            .map(|entry| entry.id.transaction)
            .unwrap_or(TransactionId(0)),
          sequence: self.next_sequence,
        },
        phase: RuntimeEffectFailurePhase::Abort,
        message: format!(
          "effect savepoint mark {} exceeds journal length {}",
          mark,
          self.entries.len(),
        ),
      }];
    }

    let mut failures = Vec::new();
    let mut tail = self.entries.split_off(mark);
    let mut failed = Vec::new();
    while let Some(mut entry) = tail.pop() {
      match abort_effect_entry(&mut entry) {
        Ok(()) => {}
        Err(failure) => {
          failures.push(failure);
          failed.push(entry);
        }
      }
    }
    failed.reverse();
    self.entries.extend(failed);
    failures
  }

  pub(super) fn abort_all(&mut self) -> Vec<RuntimeEffectFailure> {
    self.rollback_to(0)
  }

  pub(super) fn prepare_transactional(
    &mut self,
  ) -> Result<(), RuntimeEffectStepFailure> {
    for entry in &mut self.entries {
      if entry.state != RuntimeEffectState::Staged {
        continue;
      }
      let PreparedRuntimeEffect::Transactional(effect) = &mut entry.effect else {
        continue;
      };
      if let Err(error) = effect.prepare() {
        return Err(effect_step_failure(
          entry.id,
          RuntimeEffectFailurePhase::Prepare,
          error,
        ));
      }
      entry.state = RuntimeEffectState::Prepared;
    }
    Ok(())
  }

  pub(super) fn abort_prepared_reverse(
    &mut self,
  ) -> Vec<RuntimeEffectFailure> {
    let mut failures = Vec::new();
    for entry in self.entries.iter_mut().rev() {
      if entry.state != RuntimeEffectState::Prepared {
        continue;
      }
      let PreparedRuntimeEffect::Transactional(effect) = &mut entry.effect else {
        continue;
      };
      match effect.abort() {
        Ok(()) => entry.state = RuntimeEffectState::Staged,
        Err(error) => failures.push(RuntimeEffectFailure {
          effect_id: entry.id,
          phase: RuntimeEffectFailurePhase::Abort,
          message: format!("{:?}", error),
        }),
      }
    }
    failures
  }

  pub(super) fn apply_compensatable(
    &mut self,
  ) -> Result<(), RuntimeEffectStepFailure> {
    for entry in &mut self.entries {
      if entry.state != RuntimeEffectState::Staged {
        continue;
      }
      let PreparedRuntimeEffect::Compensatable(effect) = &mut entry.effect else {
        continue;
      };
      if let Err(error) = effect.apply() {
        return Err(effect_step_failure(
          entry.id,
          RuntimeEffectFailurePhase::Apply,
          error,
        ));
      }
      entry.state = RuntimeEffectState::Applied;
    }
    Ok(())
  }

  pub(super) fn compensate_applied_reverse(
    &mut self,
  ) -> Vec<RuntimeEffectFailure> {
    let mut failures = Vec::new();
    for entry in self.entries.iter_mut().rev() {
      if entry.state != RuntimeEffectState::Applied {
        continue;
      }
      let PreparedRuntimeEffect::Compensatable(effect) = &mut entry.effect else {
        continue;
      };
      match effect.compensate() {
        Ok(()) => entry.state = RuntimeEffectState::Staged,
        Err(error) => failures.push(RuntimeEffectFailure {
          effect_id: entry.id,
          phase: RuntimeEffectFailurePhase::Compensate,
          message: format!("{:?}", error),
        }),
      }
    }
    failures
  }

  pub(super) fn commit_transactional(
    &mut self,
  ) -> RuntimeTransactionalCommitReport {
    let mut outcomes = Vec::new();
    let mut committed = Vec::new();
    let mut failures = Vec::new();
    for entry in &mut self.entries {
      if entry.state != RuntimeEffectState::Prepared {
        continue;
      }
      let PreparedRuntimeEffect::Transactional(effect) = &mut entry.effect else {
        continue;
      };
      match effect.commit() {
        Ok(()) => {
          outcomes.push(format!(
            "transactional effect {} committed",
            entry.id,
          ));
          committed.push(entry.id);
        }
        Err(error) => {
          let step = effect_step_failure(
            entry.id,
            RuntimeEffectFailurePhase::Commit,
            error,
          );
          outcomes.push(format!(
            "transactional effect {} commit failed: {}",
            entry.id,
            step.failure.message,
          ));
          failures.push(step);
        }
      }
    }
    RuntimeTransactionalCommitReport {
      committed,
      failures,
      participant_outcomes: outcomes,
    }
  }

  pub(super) fn deliver_after_commit(
    &mut self,
  ) -> Vec<RuntimeEffectFailure> {
    let mut failures = Vec::new();
    for entry in &mut self.entries {
      let PreparedRuntimeEffect::AfterCommit(effect) = &mut entry.effect else {
        continue;
      };
      if let Err(error) = effect.deliver() {
        failures.push(RuntimeEffectFailure {
          effect_id: entry.id,
          phase: RuntimeEffectFailurePhase::Deliver,
          message: format!("{:?}", error),
        });
      }
    }
    failures
  }
}

fn effect_step_failure(
  effect_id: RuntimeEffectId,
  phase: RuntimeEffectFailurePhase,
  error: MechError,
) -> RuntimeEffectStepFailure {
  RuntimeEffectStepFailure {
    failure: RuntimeEffectFailure {
      effect_id,
      phase,
      message: format!("{:?}", error),
    },
    error,
  }
}

fn abort_effect_entry(
  entry: &mut RuntimeEffectEntry,
) -> Result<(), RuntimeEffectFailure> {
  let result = match (&mut entry.effect, entry.state) {
    (
      PreparedRuntimeEffect::Transactional(effect),
      RuntimeEffectState::Staged
        | RuntimeEffectState::Prepared
        | RuntimeEffectState::Applied,
    ) => effect.abort().map_err(|error| {
      (RuntimeEffectFailurePhase::Abort, error)
    }),
    (
      PreparedRuntimeEffect::Compensatable(effect),
      RuntimeEffectState::Applied,
    ) => effect.compensate().map_err(|error| {
      (RuntimeEffectFailurePhase::Compensate, error)
    }),
    (
      PreparedRuntimeEffect::Compensatable(effect),
      RuntimeEffectState::Staged | RuntimeEffectState::Prepared,
    ) => effect.abort().map_err(|error| {
      (RuntimeEffectFailurePhase::Abort, error)
    }),
    (PreparedRuntimeEffect::AfterCommit(_), _) => Ok(()),
  };

  result.map_err(|(phase, error)| RuntimeEffectFailure {
    effect_id: entry.id,
    phase,
    message: format!("{:?}", error),
  })
}

impl MechRuntime {
  pub(super) fn describe_effect_failures(
    failures: impl IntoIterator<Item = RuntimeEffectFailure>,
  ) -> Vec<String> {
    failures
      .into_iter()
      .map(|failure| {
        format!(
          "effect {} {:?} failed: {}",
          failure.effect_id,
          failure.phase,
          failure.message,
        )
      })
      .collect()
  }

  pub(super) fn poison_effect_cleanup(
    &mut self,
    operation: &'static str,
    transaction_id: TransactionId,
    original_error: String,
    cleanup_failures: Vec<String>,
  ) -> MechError {
    self.health = RuntimeHealth::Poisoned(RuntimePoisonRecord {
      operation: operation.to_string(),
      transaction_id: Some(transaction_id),
      original_error: original_error.clone(),
      rollback_failures: cleanup_failures.clone(),
    });
    MechError::new(
      RuntimeEffectCleanupFailed {
        operation,
        transaction_id,
        original_error,
        cleanup_failures,
      },
      None,
    )
  }

  pub(super) fn poison_external_commit_indeterminate(
    &mut self,
    transaction_id: TransactionId,
    failures: Vec<RuntimeEffectFailure>,
    participant_outcomes: Vec<String>,
  ) -> MechError {
    let failed_effects = failures
      .iter()
      .map(|failure| failure.effect_id.to_string())
      .collect::<Vec<_>>()
      .join(", ");
    let original_error = format!(
      "external effects [{}] failed to commit after runtime store transaction {} committed",
      failed_effects,
      transaction_id,
    );
    self.health = RuntimeHealth::Poisoned(RuntimePoisonRecord {
      operation: "commit_runtime_transaction".to_string(),
      transaction_id: Some(transaction_id),
      original_error,
      rollback_failures: participant_outcomes.clone(),
    });
    MechError::new(
      RuntimeExternalCommitIndeterminate {
        transaction_id,
        failures,
        participant_outcomes,
      },
      None,
    )
  }

  pub fn stage_runtime_effect_with_context(
    &mut self,
    context: &mut RuntimeContext,
    effect: PreparedRuntimeEffect,
  ) -> MResult<RuntimeEffectId> {
    self.ensure_runtime_mutation_allowed(
      "stage_runtime_effect_with_context",
    )?;
    self.validate_context_for_runtime(context)?;

    let transaction_id = Self::context_transaction_id(context)?;
    if self.active_execution_transaction(transaction_id)?.state
      != RuntimeExecutionTransactionState::Active
    {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "stage_runtime_effect_with_context",
          reason: format!(
            "transaction {} is not accepting new effects",
            transaction_id,
          ),
        },
        None,
      ));
    }
    let cost = effect.cost();
    let metadata = effect.metadata();
    let protocol = effect.protocol();
    context.charge_bytes(cost.bytes)?;
    context.charge_items(cost.items)?;
    let store_before = self
      .active_execution_transaction(transaction_id)?
      .store
      .clone();
    let effect_mark = self
      .active_execution_transaction(transaction_id)?
      .effects
      .mark();
    let context_events_before = context.events.clone();
    let effect_id = self
      .active_execution_transaction_mut(transaction_id)?
      .effects
      .stage(transaction_id, effect);

    if let Err(error) = self.emit_event_to_context(
      context,
      RuntimeEventKind::EffectStaged {
        effect_id,
        source: metadata.source,
        operation: metadata.operation,
        resource: metadata.resource,
        protocol,
      },
    ) {
      let phase_guard = ScopedRuntimeState::enter(
        &self.active_effect_phase,
        ActiveRuntimeEffectPhase::Aborting,
      );
      let cleanup = {
        let transaction =
          self.active_execution_transaction_mut(transaction_id)?;
        transaction.store = store_before;
        transaction.effects.rollback_to(effect_mark)
      };
      drop(phase_guard);
      context.events = context_events_before;
      if cleanup.is_empty() {
        return Err(error);
      }
      return Err(self.poison_effect_cleanup(
        "stage_runtime_effect_with_context",
        transaction_id,
        format!("{:?}", error),
        Self::describe_effect_failures(cleanup),
      ));
    }

    Ok(effect_id)
  }

  pub(super) fn stage_runtime_resource_effect_with_context(
    &mut self,
    context: &mut RuntimeContext,
    effect: PreparedRuntimeEffect,
    base_uri: String,
    path: String,
    value: Value,
  ) -> MResult<RuntimeEffectId> {
    self.ensure_runtime_mutation_allowed(
      "stage_runtime_resource_effect_with_context",
    )?;
    self.validate_context_for_runtime(context)?;

    let transaction_id = Self::context_transaction_id(context)?;
    if self.active_execution_transaction(transaction_id)?.state
      != RuntimeExecutionTransactionState::Active
    {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "stage_runtime_resource_effect_with_context",
          reason: format!(
            "transaction {} is not accepting new effects",
            transaction_id,
          ),
        },
        None,
      ));
    }
    let cost = effect.cost();
    let metadata = effect.metadata();
    let protocol = effect.protocol();
    context.charge_bytes(cost.bytes)?;
    context.charge_items(cost.items)?;
    let store_before = self
      .active_execution_transaction(transaction_id)?
      .store
      .clone();
    let effect_mark = self
      .active_execution_transaction(transaction_id)?
      .effects
      .mark();
    let context_events_before = context.events.clone();
    let effect_id = self
      .active_execution_transaction_mut(transaction_id)?
      .effects
      .stage_resource_write(
        transaction_id,
        effect,
        base_uri,
        path,
        value,
      );

    if let Err(error) = self.emit_event_to_context(
      context,
      RuntimeEventKind::EffectStaged {
        effect_id,
        source: metadata.source,
        operation: metadata.operation,
        resource: metadata.resource,
        protocol,
      },
    ) {
      let phase_guard = ScopedRuntimeState::enter(
        &self.active_effect_phase,
        ActiveRuntimeEffectPhase::Aborting,
      );
      let cleanup = {
        let transaction =
          self.active_execution_transaction_mut(transaction_id)?;
        transaction.store = store_before;
        transaction.effects.rollback_to(effect_mark)
      };
      drop(phase_guard);
      context.events = context_events_before;
      if cleanup.is_empty() {
        return Err(error);
      }
      return Err(self.poison_effect_cleanup(
        "stage_runtime_resource_effect_with_context",
        transaction_id,
        format!("{:?}", error),
        Self::describe_effect_failures(cleanup),
      ));
    }

    Ok(effect_id)
  }

  pub(super) fn execute_runtime_effect_immediately(
    &mut self,
    mut effect: PreparedRuntimeEffect,
  ) -> MResult<RuntimeEffectId> {
    self.ensure_runtime_mutation_allowed(
      "execute_runtime_effect_immediately",
    )?;

    let effect_id = RuntimeEffectId {
      transaction: self.next_transaction_id(),
      sequence: 0,
    };
    match &mut effect {
      PreparedRuntimeEffect::Transactional(effect) => {
        let phase_guard = ScopedRuntimeState::enter(
          &self.active_effect_phase,
          ActiveRuntimeEffectPhase::Preparing,
        );
        let prepare_result = effect.prepare();
        drop(phase_guard);
        prepare_result?;

        let phase_guard = ScopedRuntimeState::enter(
          &self.active_effect_phase,
          ActiveRuntimeEffectPhase::Committing,
        );
        let commit_result = effect.commit();
        drop(phase_guard);
        if let Err(error) = commit_result {
          return Err(self.poison_external_commit_indeterminate(
            effect_id.transaction,
            vec![RuntimeEffectFailure {
              effect_id,
              phase: RuntimeEffectFailurePhase::Commit,
              message: format!("{:?}", error),
            }],
            vec![format!(
              "immediate transactional effect {} commit failed: {:?}",
              effect_id,
              error,
            )],
          ));
        }
      }
      PreparedRuntimeEffect::Compensatable(effect) => {
        let phase_guard = ScopedRuntimeState::enter(
          &self.active_effect_phase,
          ActiveRuntimeEffectPhase::Applying,
        );
        let result = effect.apply();
        drop(phase_guard);
        result?;
      }
      PreparedRuntimeEffect::AfterCommit(effect) => {
        let phase_guard = ScopedRuntimeState::enter(
          &self.active_effect_phase,
          ActiveRuntimeEffectPhase::Delivering,
        );
        let result = effect.deliver();
        drop(phase_guard);
        result?;
      }
    }
    Ok(effect_id)
  }

}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    BasicCapability, ClosureHostFunction, InMemoryDocsProvider,
    InMemorySourceResolver, NodeId, SharedCapabilityKernel,
  };
  use std::collections::HashSet;
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::{Arc, Mutex};

  #[derive(Debug)]
  struct FailingEventIdGenerator {
    next: u128,
    event_call: usize,
    fail_calls: HashSet<usize>,
  }

  impl FailingEventIdGenerator {
    fn new(fail_calls: impl IntoIterator<Item = usize>) -> Self {
      Self {
        next: 1,
        event_call: 0,
        fail_calls: fail_calls.into_iter().collect(),
      }
    }

    fn next_id(&mut self) -> u128 {
      let id = self.next;
      self.next = self.next.saturating_add(1);
      id
    }
  }

  impl IdGenerator for FailingEventIdGenerator {
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
      self.event_call = self.event_call.saturating_add(1);
      if self.fail_calls.contains(&self.event_call) {
        EventId(0)
      } else {
        EventId(1_000 + self.event_call as u128)
      }
    }

    fn node_id(&mut self) -> NodeId {
      NodeId(self.next_id())
    }

    fn message_id(&mut self) -> MessageId {
      MessageId(self.next_id())
    }
  }

  #[derive(Debug, Clone)]
  struct SyntheticEffectError {
    message: String,
  }

  impl MechErrorKind for SyntheticEffectError {
    fn name(&self) -> &str {
      "SyntheticEffectError"
    }

    fn message(&self) -> String {
      self.message.clone()
    }
  }

  fn synthetic_error(message: impl Into<String>) -> MechError {
    MechError::new(
      SyntheticEffectError {
        message: message.into(),
      },
      None,
    )
  }

  fn record(log: &Arc<Mutex<Vec<String>>>, entry: impl Into<String>) {
    log.lock().unwrap().push(entry.into());
  }

  fn synthetic_metadata(name: &str) -> RuntimeEffectMetadata {
    RuntimeEffectMetadata::new(
      RuntimeEffectSource::Custom {
        name: name.to_string(),
      },
      "synthetic",
    )
  }

  #[derive(Debug)]
  struct SyntheticTransactionalEffect {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
    fail_prepare: bool,
    fail_commit: bool,
    fail_abort: bool,
  }

  impl RuntimeTransactionalEffect for SyntheticTransactionalEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
      synthetic_metadata(self.name)
    }

    fn prepare(&mut self) -> MResult<()> {
      record(&self.log, format!("{}:prepare", self.name));
      if self.fail_prepare {
        return Err(synthetic_error(format!(
          "{} prepare failed",
          self.name,
        )));
      }
      Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
      record(&self.log, format!("{}:commit", self.name));
      if self.fail_commit {
        return Err(synthetic_error(format!(
          "{} commit failed",
          self.name,
        )));
      }
      Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
      record(&self.log, format!("{}:abort", self.name));
      if self.fail_abort {
        return Err(synthetic_error(format!(
          "{} abort failed",
          self.name,
        )));
      }
      Ok(())
    }
  }

  #[derive(Debug)]
  struct SyntheticCompensatableEffect {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
    fail_apply: bool,
    fail_compensate: bool,
    fail_abort: bool,
  }

  impl RuntimeCompensatableEffect for SyntheticCompensatableEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
      synthetic_metadata(self.name)
    }

    fn apply(&mut self) -> MResult<()> {
      record(&self.log, format!("{}:apply", self.name));
      if self.fail_apply {
        return Err(synthetic_error(format!(
          "{} apply failed",
          self.name,
        )));
      }
      Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
      record(&self.log, format!("{}:compensate", self.name));
      if self.fail_compensate {
        return Err(synthetic_error(format!(
          "{} compensate failed",
          self.name,
        )));
      }
      Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
      record(&self.log, format!("{}:abort", self.name));
      if self.fail_abort {
        return Err(synthetic_error(format!(
          "{} abort failed",
          self.name,
        )));
      }
      Ok(())
    }
  }

  #[derive(Debug)]
  struct SyntheticAfterCommitEffect {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
    fail_deliver: bool,
  }

  #[derive(Debug)]
  struct FailOnceAbortEffect {
    attempts: Arc<AtomicUsize>,
  }

  impl RuntimeTransactionalEffect for FailOnceAbortEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
      synthetic_metadata("fail-once-abort")
    }

    fn prepare(&mut self) -> MResult<()> {
      Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
      Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
      if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
        return Err(synthetic_error("deliberate first abort failure"));
      }
      Ok(())
    }
  }

  impl RuntimeAfterCommitEffect for SyntheticAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
      synthetic_metadata(self.name)
    }

    fn deliver(&mut self) -> MResult<()> {
      record(&self.log, format!("{}:deliver", self.name));
      if self.fail_deliver {
        return Err(synthetic_error(format!(
          "{} delivery failed",
          self.name,
        )));
      }
      Ok(())
    }
  }

  fn transactional(
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
  ) -> SyntheticTransactionalEffect {
    SyntheticTransactionalEffect {
      name,
      log,
      fail_prepare: false,
      fail_commit: false,
      fail_abort: false,
    }
  }

  fn compensatable(
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
  ) -> SyntheticCompensatableEffect {
    SyntheticCompensatableEffect {
      name,
      log,
      fail_apply: false,
      fail_compensate: false,
      fail_abort: false,
    }
  }

  fn after_commit(
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
  ) -> SyntheticAfterCommitEffect {
    SyntheticAfterCommitEffect {
      name,
      log,
      fail_deliver: false,
    }
  }

  #[derive(Debug)]
  struct NoopAfterCommit {
    name: &'static str,
  }

  #[derive(Debug)]
  struct SensitiveAfterCommit {
    secret_payload: String,
  }

  impl RuntimeAfterCommitEffect for SensitiveAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::Custom {
          name: "sensitive-test".to_string(),
        },
        "deliver",
      )
      .with_resource("test://metadata-only")
    }

    fn deliver(&mut self) -> MResult<()> {
      assert!(!self.secret_payload.is_empty());
      Ok(())
    }
  }

  #[derive(Debug)]
  struct CostedAfterCommit {
    cost: crate::RuntimeEffectCost,
  }

  impl RuntimeAfterCommitEffect for CostedAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
      synthetic_metadata("costed").with_cost(self.cost)
    }

    fn deliver(&mut self) -> MResult<()> {
      Ok(())
    }
  }

  impl RuntimeAfterCommitEffect for NoopAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::Custom {
          name: self.name.to_string(),
        },
        "deliver",
      )
    }

    fn deliver(&mut self) -> MResult<()> {
      Ok(())
    }
  }

  fn effect(name: &'static str) -> PreparedRuntimeEffect {
    PreparedRuntimeEffect::AfterCommit(Box::new(NoopAfterCommit { name }))
  }

  #[test]
  fn journal_rollback_does_not_reuse_effect_sequences() {
    let transaction = TransactionId(7);
    let mut journal = RuntimeEffectJournal::new();

    assert_eq!(journal.stage(transaction, effect("a")).sequence, 0);
    let mark = journal.mark();
    assert_eq!(journal.stage(transaction, effect("b")).sequence, 1);
    assert!(journal.rollback_to(mark).is_empty());
    assert_eq!(journal.stage(transaction, effect("c")).sequence, 2);

    assert_eq!(journal.len(), 2);
    assert_eq!(journal.next_sequence(), 3);
  }

  #[test]
  fn failed_savepoint_cleanup_preserves_effect_for_outer_abort_retry() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let result: MResult<()> = runtime.with_atomic_program_operation(
      &mut context,
      "fail_once_effect_cleanup",
      |runtime, context| {
        let effect_id = runtime.stage_runtime_effect_with_context(
          context,
          PreparedRuntimeEffect::Transactional(Box::new(
            FailOnceAbortEffect {
              attempts: attempts.clone(),
            },
          )),
        )?;
        assert_eq!(effect_id.sequence, 0);
        Err(synthetic_error("deliberate retained operation failure"))
      },
    );

    assert_eq!(
      result.unwrap_err().kind_name(),
      "RuntimeProgramRollbackFailed",
    );
    assert!(runtime.is_poisoned());
    let transaction = runtime
      .active_execution_transaction(transaction_id)
      .unwrap();
    assert_eq!(transaction.effects.len(), 1);
    assert_eq!(transaction.effects.next_sequence(), 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    runtime
      .abort_runtime_transaction(&mut context, "retry retained effect abort")
      .unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn transaction_history_persists_effect_metadata_without_payload() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let secret = "raw-secret-payload-must-not-be-durable";
    let effect_id = runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::AfterCommit(Box::new(
          SensitiveAfterCommit {
            secret_payload: secret.to_string(),
          },
        )),
      )
      .unwrap();

    runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap();

    let transaction = runtime
      .get_transaction(transaction_id)
      .unwrap()
      .unwrap();
    assert_eq!(transaction.effects.len(), 1);
    assert_eq!(transaction.effects[0].id, effect_id);
    assert_eq!(
      transaction.effects[0].protocol,
      crate::RuntimeEffectProtocol::AfterCommit,
    );
    assert_eq!(
      transaction.effects[0].resource.as_deref(),
      Some("test://metadata-only"),
    );
    assert!(!format!("{:?}", transaction).contains(secret));
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::EffectDelivered { effect_id: delivered }
          if delivered == effect_id
      )
    }));
  }

  #[test]
  fn savepoint_rollback_discards_effect_and_staging_event() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let result: MResult<RuntimeEffectId> =
      runtime.with_atomic_program_operation(
        &mut context,
        "effect_staging_event_rollback",
        |runtime, context| {
          let effect_id = runtime.stage_runtime_effect_with_context(
            context,
            effect("rolled-back"),
          )?;
          Err(synthetic_error(format!(
            "deliberate rollback for {}",
            effect_id,
          )))
        },
      );

    assert_eq!(result.unwrap_err().kind_name(), "SyntheticEffectError");
    assert!(runtime
      .active_execution_transaction(transaction_id)
      .unwrap()
      .effects
      .is_empty());
    assert!(!context.events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::EffectStaged { .. })
    }));
    runtime
      .abort_runtime_transaction(&mut context, "test cleanup")
      .unwrap();
  }

  #[test]
  fn rolled_back_effect_cost_is_not_refunded() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let bytes_before = context.budget.used_bytes;
    let items_before = context.budget.used_items;

    let result: MResult<()> = runtime.with_atomic_program_operation(
      &mut context,
      "costed_effect_failure",
      |runtime, context| {
        runtime.stage_runtime_effect_with_context(
          context,
          PreparedRuntimeEffect::AfterCommit(Box::new(
            CostedAfterCommit {
              cost: crate::RuntimeEffectCost {
                bytes: 17,
                items: 3,
              },
            },
          )),
        )?;
        Err(synthetic_error("deliberate costed operation failure"))
      },
    );

    assert_eq!(result.unwrap_err().kind_name(), "SyntheticEffectError");
    assert_eq!(context.budget.used_bytes, bytes_before + 17);
    assert_eq!(context.budget.used_items, items_before + 3);
    let transaction =
      runtime.active_execution_transaction(transaction_id).unwrap();
    assert_eq!(transaction.effects.len(), 0);
    assert_eq!(transaction.effects.next_sequence(), 1);

    runtime
      .abort_runtime_transaction(&mut context, "cost test cleanup")
      .unwrap();
  }

  #[test]
  fn prepare_failure_aborts_prepared_participants_and_stays_retryable() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(transactional(
          "first",
          log.clone(),
        ))),
      )
      .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_prepare = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(second)),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "SyntheticEffectError");
    assert_eq!(
      *log.lock().unwrap(),
      vec!["first:prepare", "second:prepare", "first:abort"],
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(
      runtime
        .active_execution_transaction(transaction_id)
        .unwrap()
        .state,
      RuntimeExecutionTransactionState::Active,
    );
    assert!(!runtime.is_poisoned());
    assert!(context.events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::EffectPreparationFailed { .. }
      )
    }));
    assert!(context.events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::EffectAborted { .. })
    }));

    runtime
      .abort_runtime_transaction(&mut context, "prepare test cleanup")
      .unwrap();
  }

  #[test]
  fn prepared_effect_abort_failure_poisons_runtime() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let mut first = transactional("first", log.clone());
    first.fail_abort = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(first)),
      )
      .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_prepare = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(second)),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, Some(transaction_id));
    let poison = match runtime.health() {
      RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
      RuntimeHealth::Poisoned(poison) => poison,
    };
    assert!(poison.original_error.contains("second prepare failed"));
    assert!(poison
      .rollback_failures
      .iter()
      .any(|failure| failure.contains("first abort failed")));
    assert_eq!(
      *log.lock().unwrap(),
      vec!["first:prepare", "second:prepare", "first:abort"],
    );

    assert!(runtime
      .abort_runtime_transaction(&mut context, "abort failure cleanup")
      .is_err());
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn apply_failure_compensates_and_aborts_in_reverse_phase_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(transactional(
          "transactional",
          log.clone(),
        ))),
      )
      .unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Compensatable(Box::new(compensatable(
          "first",
          log.clone(),
        ))),
      )
      .unwrap();
    let mut second = compensatable("second", log.clone());
    second.fail_apply = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Compensatable(Box::new(second)),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "SyntheticEffectError");
    assert_eq!(
      *log.lock().unwrap(),
      vec![
        "transactional:prepare",
        "first:apply",
        "second:apply",
        "first:compensate",
        "transactional:abort",
      ],
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(!runtime.is_poisoned());
    assert!(context.events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::EffectCompensated { .. })
    }));
    assert!(context.events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::EffectAborted { .. })
    }));

    runtime
      .abort_runtime_transaction(&mut context, "apply test cleanup")
      .unwrap();
  }

  #[test]
  fn store_failure_compensates_effect_and_keeps_transaction_active() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Compensatable(Box::new(compensatable(
          "reversible",
          log.clone(),
        ))),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(900), "missing", "update"),
      )
      .unwrap();

    assert!(runtime
      .commit_runtime_transaction_detailed(&mut context)
      .is_err());

    assert_eq!(
      *log.lock().unwrap(),
      vec!["reversible:apply", "reversible:compensate"],
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(!runtime.is_poisoned());

    runtime
      .abort_runtime_transaction(&mut context, "store test cleanup")
      .unwrap();
  }

  #[test]
  fn compensation_failure_poisons_runtime_with_complete_diagnostic() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let mut first = compensatable("first", log.clone());
    first.fail_compensate = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Compensatable(Box::new(first)),
      )
      .unwrap();
    let mut second = compensatable("second", log.clone());
    second.fail_apply = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Compensatable(Box::new(second)),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, Some(transaction_id));
    let poison = match runtime.health() {
      RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
      RuntimeHealth::Poisoned(poison) => poison,
    };
    assert!(poison.original_error.contains("second apply failed"));
    assert!(poison
      .rollback_failures
      .iter()
      .any(|failure| failure.contains("first compensate failed")));
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::EffectCompensationFailed { .. }
      )
    }));

    assert!(runtime
      .abort_runtime_transaction(&mut context, "poison test cleanup")
      .is_err());
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn provider_commit_failure_after_store_commit_is_indeterminate() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(transactional(
          "first",
          log.clone(),
        ))),
      )
      .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_commit = true;
    let failing_effect_id = runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(second)),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate");
    let indeterminate = error
      .kind_as::<RuntimeExternalCommitIndeterminate>()
      .unwrap();
    assert_eq!(indeterminate.transaction_id, transaction_id);
    assert_eq!(indeterminate.failures.len(), 1);
    assert_eq!(
      indeterminate.failures[0].effect_id,
      failing_effect_id,
    );
    assert_eq!(
      *log.lock().unwrap(),
      vec![
        "first:prepare",
        "second:prepare",
        "first:commit",
        "second:commit",
      ],
    );
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime
      .get_transaction(transaction_id)
      .unwrap()
      .is_some());
    let events = runtime.list_events(None).unwrap();
    assert!(events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::TransactionalEffectCommitted { .. }
      )
    }));
    assert!(events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::ExternalCommitIndeterminate { .. }
      )
    }));
  }

  #[test]
  fn every_prepared_participant_receives_commit_and_all_failures_are_reported() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(transactional(
          "first",
          log.clone(),
        ))),
      )
      .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_commit = true;
    let second_id = runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(second)),
      )
      .unwrap();
    let mut third = transactional("third", log.clone());
    third.fail_commit = true;
    let third_id = runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(third)),
      )
      .unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::AfterCommit(Box::new(after_commit(
          "suppressed",
          delivery_log.clone(),
        ))),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();
    let indeterminate = error
      .kind_as::<RuntimeExternalCommitIndeterminate>()
      .unwrap();

    assert_eq!(
      *log.lock().unwrap(),
      vec![
        "first:prepare",
        "second:prepare",
        "third:prepare",
        "first:commit",
        "second:commit",
        "third:commit",
      ],
    );
    assert_eq!(
      indeterminate
        .failures
        .iter()
        .map(|failure| failure.effect_id)
        .collect::<Vec<_>>(),
      vec![second_id, third_id],
    );
    assert!(delivery_log.lock().unwrap().is_empty());
    let poison = match runtime.health() {
      RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
      RuntimeHealth::Poisoned(poison) => poison,
    };
    assert!(poison
      .rollback_failures
      .iter()
      .any(|outcome| outcome.contains("second commit failed")));
    assert!(poison
      .rollback_failures
      .iter()
      .any(|outcome| outcome.contains("third commit failed")));
    assert_eq!(context.transaction, None);
    assert!(runtime
      .get_transaction(transaction_id)
      .unwrap()
      .is_some());
    assert_eq!(
      runtime
        .list_events(None)
        .unwrap()
        .iter()
        .filter(|event| {
          matches!(
            event.kind,
            RuntimeEventKind::ExternalCommitIndeterminate { .. }
          )
        })
        .count(),
      2,
    );
  }

  #[test]
  fn poisoned_runtime_owned_mutation_is_fail_closed() {
    let callback_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = callback_calls.clone();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .resource_provider(Box::new(InMemoryDocsProvider::new()))
      .build()
      .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
      .grant_capability(Arc::new(BasicCapability::from_keys(
        CapabilityId(900),
        &subject,
        "host:demo/poison-gate",
        ["call"],
      )))
      .unwrap();
    runtime
      .grant_capability(Arc::new(BasicCapability::from_keys(
        CapabilityId(901),
        "task:1",
        "db://users",
        [":read"],
      )))
      .unwrap();
    runtime
      .register_mech_host_function(ClosureHostFunction::new_pure(
        "demo/poison-gate",
        move |_services, _context, _args| {
          observed_calls.fetch_add(1, Ordering::SeqCst);
          Ok(Value::Empty)
        },
      ))
      .unwrap();
    let object_id = ObjectId(902);
    let actor_id = ActorId(903);
    let task_id = TaskId(904);
    runtime
      .put_object(ObjectRecord::text(object_id, "note", "before"))
      .unwrap();
    runtime
      .put_actor(ActorRecord::new(actor_id, "actor:poison"))
      .unwrap();
    runtime
      .put_task(TaskRecord::new(task_id, "task:poison"))
      .unwrap();

    let mut cleanup_context = runtime.runtime_context().unwrap();
    let cleanup_transaction = runtime
      .begin_transaction(&mut cleanup_context)
      .unwrap();
    let mut poison_context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut poison_context).unwrap();
    let mut failing = transactional(
      "poison-runtime",
      Arc::new(Mutex::new(Vec::new())),
    );
    failing.fail_commit = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut poison_context,
        PreparedRuntimeEffect::Transactional(Box::new(failing)),
      )
      .unwrap();
    assert_eq!(
      runtime
        .commit_runtime_transaction(&mut poison_context)
        .unwrap_err()
        .kind_name(),
      "RuntimeExternalCommitIndeterminate",
    );
    assert!(runtime.is_poisoned());

    let mut poison_kinds = Vec::new();
    poison_kinds.push(
      runtime
        .call_host(HostCall::new("demo/poison-gate", Vec::new()))
        .unwrap_err()
        .kind_name(),
    );
    let used_steps_before = cleanup_context.budget.used_steps;
    let capability_uses_before =
      observed_kernel.successful_uses_for_test(CapabilityId(901));
    let overlay_uses_before = runtime
      .active_execution_transaction(cleanup_transaction)
      .unwrap()
      .capabilities
      .usage_deltas()
      .collect::<Vec<_>>();
    poison_kinds.push(
      runtime
        .check_capability_with_context(
          &mut cleanup_context,
          &CapabilityRequest::from_keys(
            "task:1",
            ":read",
            "db://users",
          ),
        )
        .unwrap_err()
        .kind_name(),
    );
    assert_eq!(cleanup_context.budget.used_steps, used_steps_before);
    assert_eq!(
      observed_kernel.successful_uses_for_test(CapabilityId(901)),
      capability_uses_before,
    );
    assert_eq!(
      runtime
        .active_execution_transaction(cleanup_transaction)
        .unwrap()
        .capabilities
        .usage_deltas()
        .collect::<Vec<_>>(),
      overlay_uses_before,
    );
    let resolver_before =
      runtime.source_resolver() as *const dyn SourceResolver;
    poison_kinds.push(
      runtime
        .set_source_resolver(InMemorySourceResolver::new())
        .unwrap_err()
        .kind_name(),
    );
    let resolver_after =
      runtime.source_resolver() as *const dyn SourceResolver;
    assert!(std::ptr::eq(resolver_before, resolver_after));
    poison_kinds.push(
      runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
          CapabilityId(905),
          "task:1",
          "db://other",
          [":read"],
        )))
        .unwrap_err()
        .kind_name(),
    );
    poison_kinds.push(
      runtime
        .revoke_capability(CapabilityId(901))
        .unwrap_err()
        .kind_name(),
    );
    poison_kinds.push(
      runtime
        .check_capability(&CapabilityRequest::from_keys(
          "task:1",
          ":read",
          "db://users",
        ))
        .unwrap_err()
        .kind_name(),
    );
    poison_kinds.push(
      runtime
        .write_resource(RuntimeResourceWriteRequest {
          base_uri: "docs://manual".to_string(),
          path: "poisoned".to_string(),
          context_name: "manual".to_string(),
          operation: RuntimeCapabilityOperation::Write,
          value: Value::String(mech_core::Ref::new(
            "must-not-write".to_string(),
          )),
          intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap_err()
        .kind_name(),
    );
    poison_kinds.push(
      runtime
        .update_object(ObjectRecord::text(
          object_id,
          "note",
          "after",
        ))
        .unwrap_err()
        .kind_name(),
    );
    poison_kinds.push(
      runtime
        .update_actor(ActorRecord::new(actor_id, "actor:changed"))
        .unwrap_err()
        .kind_name(),
    );
    poison_kinds.push(
      runtime
        .update_task(TaskRecord::new(task_id, "task:changed"))
        .unwrap_err()
        .kind_name(),
    );
    poison_kinds.push(
      runtime
        .stage_runtime_effect_with_context(
          &mut cleanup_context,
          effect("must-not-stage"),
        )
        .unwrap_err()
        .kind_name(),
    );

    assert!(poison_kinds
      .iter()
      .all(|kind| *kind == "RuntimePoisoned"));
    assert_eq!(callback_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
      runtime.get_object(object_id).unwrap().unwrap().data,
      b"before",
    );
    assert_eq!(
      runtime.get_actor(actor_id).unwrap().unwrap().subject,
      "actor:poison",
    );
    assert_eq!(
      runtime.get_task(task_id).unwrap().unwrap().subject,
      "task:poison",
    );
    assert!(!runtime
      .capability_kernel()
      .is_revoked(CapabilityId(901))
      .unwrap());

    runtime
      .abort_runtime_transaction(
        &mut cleanup_context,
        "poisoned runtime cleanup remains allowed",
      )
      .unwrap();
    assert_eq!(cleanup_context.transaction, None);
    assert!(!runtime
      .active_transactions
      .contains_key(&cleanup_transaction));
  }

  #[test]
  fn prepare_failure_audit_failure_is_nonfatal() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder()
      .id_generator(FailingEventIdGenerator::new([5]))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(transactional(
          "first",
          log.clone(),
        ))),
      )
      .unwrap();
    let mut second = transactional("second", log);
    second.fail_prepare = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(second)),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "SyntheticEffectError");
    assert!(error.full_chain_message().contains("second prepare failed"));
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, Some(transaction_id));
    runtime
      .abort_runtime_transaction(&mut context, "prepare audit cleanup")
      .unwrap();
  }

  #[test]
  fn compensation_audit_failure_is_nonfatal() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder()
      .id_generator(FailingEventIdGenerator::new([5]))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Compensatable(Box::new(compensatable(
          "first",
          log.clone(),
        ))),
      )
      .unwrap();
    let mut second = compensatable("second", log);
    second.fail_apply = true;
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Compensatable(Box::new(second)),
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "SyntheticEffectError");
    assert!(error.full_chain_message().contains("second apply failed"));
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, Some(transaction_id));
    runtime
      .abort_runtime_transaction(&mut context, "compensation audit cleanup")
      .unwrap();
  }

  #[test]
  fn explicit_abort_audit_failure_does_not_poison() {
    let mut runtime = MechRuntime::builder()
      .id_generator(FailingEventIdGenerator::new([5]))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(transactional(
          "abortable",
          Arc::new(Mutex::new(Vec::new())),
        ))),
      )
      .unwrap();

    runtime
      .abort_runtime_transaction(&mut context, "audit failure abort")
      .unwrap();

    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, None);
  }

  #[test]
  fn committed_effect_audit_failure_still_delivers_after_commit() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder()
      .id_generator(FailingEventIdGenerator::new([6]))
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::Transactional(Box::new(transactional(
          "transactional",
          log.clone(),
        ))),
      )
      .unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::AfterCommit(Box::new(after_commit(
          "after",
          log.clone(),
        ))),
      )
      .unwrap();

    let outcome = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap();

    assert_eq!(outcome.delivery_failures, Vec::new());
    assert_eq!(outcome.audit_failures.len(), 1);
    assert_eq!(
      outcome.audit_failures[0].phase,
      RuntimeEffectFailurePhase::Audit,
    );
    assert_eq!(
      *log.lock().unwrap(),
      vec![
        "transactional:prepare",
        "transactional:commit",
        "after:deliver",
      ],
    );
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, None);
  }

  #[test]
  fn after_commit_delivery_failure_keeps_committed_runtime_healthy() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::AfterCommit(Box::new(after_commit(
          "first",
          log.clone(),
        ))),
      )
      .unwrap();
    let mut second = after_commit("second", log.clone());
    second.fail_deliver = true;
    let failing_effect_id = runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::AfterCommit(Box::new(second)),
      )
      .unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::AfterCommit(Box::new(after_commit(
          "third",
          log.clone(),
        ))),
      )
      .unwrap();

    let outcome = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap();

    assert_eq!(outcome.transaction_id, transaction_id);
    assert_eq!(outcome.delivery_failures.len(), 1);
    assert_eq!(
      outcome.delivery_failures[0].effect_id,
      failing_effect_id,
    );
    assert_eq!(
      *log.lock().unwrap(),
      vec!["first:deliver", "second:deliver", "third:deliver"],
    );
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert!(runtime
      .get_transaction(transaction_id)
      .unwrap()
      .is_some());
    let events = runtime.list_events(None).unwrap();
    assert!(events.iter().any(|event| {
      matches!(event.kind, RuntimeEventKind::EffectDelivered { .. })
    }));
    assert!(events.iter().any(|event| {
      matches!(
        event.kind,
        RuntimeEventKind::EffectDeliveryFailed {
          effect_id,
          ..
        } if effect_id == failing_effect_id
      )
    }));
  }

  #[test]
  fn outer_abort_discards_effects_in_reverse_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    for name in ["first", "second", "third"] {
      runtime
        .stage_runtime_effect_with_context(
          &mut context,
          PreparedRuntimeEffect::Transactional(Box::new(transactional(
            name,
            log.clone(),
          ))),
        )
        .unwrap();
    }

    runtime
      .abort_runtime_transaction(&mut context, "discard")
      .unwrap();

    assert_eq!(
      *log.lock().unwrap(),
      vec!["third:abort", "second:abort", "first:abort"],
    );
  }

  #[test]
  fn mutation_is_rejected_while_an_effect_phase_is_active() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
      .active_effect_phase
      .set(Some(ActiveRuntimeEffectPhase::Preparing));

    let error = runtime.begin_transaction(&mut context).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectOperationReentrant");
    assert_eq!(context.transaction, None);
    assert!(runtime.active_transactions.is_empty());
  }

  #[test]
  fn source_resolver_replacement_is_rejected_while_an_effect_phase_is_active() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let resolver_before =
      runtime.source_resolver() as *const dyn SourceResolver;
    runtime
      .active_effect_phase
      .set(Some(ActiveRuntimeEffectPhase::Preparing));

    let error = runtime
      .set_source_resolver(InMemorySourceResolver::new())
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectOperationReentrant");
    let resolver_after =
      runtime.source_resolver() as *const dyn SourceResolver;
    assert!(std::ptr::eq(resolver_before, resolver_after));
  }

  #[test]
  fn broken_effect_identity_poisons_before_external_work() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .stage_runtime_effect_with_context(
        &mut context,
        PreparedRuntimeEffect::AfterCommit(Box::new(after_commit(
          "identity",
          log.clone(),
        ))),
      )
      .unwrap();
    runtime
      .active_execution_transaction_mut(transaction_id)
      .unwrap()
      .effects
      .entries[0]
      .id
      .transaction = TransactionId(transaction_id.0.saturating_add(1));

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    assert!(log.lock().unwrap().is_empty());
    assert_eq!(context.transaction, Some(transaction_id));
  }
}
