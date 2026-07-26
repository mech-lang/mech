//! Runtime-owned effect journal and lifecycle mechanics.

use super::*;
use crate::{
  PreparedRuntimeEffect, RuntimeEffectFailure, RuntimeEffectFailurePhase,
  RuntimeEffectId,
};
#[cfg(test)]
use crate::{
  RuntimeAfterCommitEffect, RuntimeEffectMetadata, RuntimeEffectSource,
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
}

pub(super) struct RuntimeEffectStepFailure {
  pub(super) failure: RuntimeEffectFailure,
  pub(super) error: MechError,
}

pub(super) struct RuntimeEffectCommitFailure {
  pub(super) step: RuntimeEffectStepFailure,
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
    let id = RuntimeEffectId {
      transaction,
      sequence: self.next_sequence,
    };
    self.next_sequence = self.next_sequence.saturating_add(1);
    self.entries.push(RuntimeEffectEntry {
      id,
      state: RuntimeEffectState::Staged,
      effect,
    });
    id
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
    for entry in self.entries[mark..].iter_mut().rev() {
      if let Err(failure) = abort_effect_entry(entry) {
        failures.push(failure);
      }
    }
    self.entries.truncate(mark);
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
  ) -> Result<Vec<String>, RuntimeEffectCommitFailure> {
    let mut outcomes = Vec::new();
    for entry in &mut self.entries {
      if entry.state != RuntimeEffectState::Prepared {
        continue;
      }
      let PreparedRuntimeEffect::Transactional(effect) = &mut entry.effect else {
        continue;
      };
      match effect.commit() {
        Ok(()) => outcomes.push(format!(
          "transactional effect {} committed",
          entry.id,
        )),
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
          return Err(RuntimeEffectCommitFailure {
            step,
            participant_outcomes: outcomes,
          });
        }
      }
    }
    Ok(outcomes)
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
    effect_id: RuntimeEffectId,
    participant_outcomes: Vec<String>,
  ) -> MechError {
    let original_error = format!(
      "external effect {} commit failed after runtime store transaction {} committed",
      effect_id,
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
        effect_id,
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
    self.ensure_runtime_healthy("stage_runtime_effect_with_context")?;
    self.reject_effect_reentrancy("stage_runtime_effect_with_context")?;
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
    context.charge_bytes(cost.bytes)?;
    context.charge_items(cost.items)?;

    Ok(
      self
        .active_execution_transaction_mut(transaction_id)?
        .effects
        .stage(transaction_id, effect),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct NoopAfterCommit {
    name: &'static str,
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
}
