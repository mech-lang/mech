//! Runtime-owned effect journal and lifecycle mechanics.

use super::*;
use crate::{
  PreparedRuntimeEffect, RuntimeEffectFailure, RuntimeEffectFailurePhase,
  RuntimeEffectId,
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

  pub(super) fn execute_runtime_effect_immediately(
    &mut self,
    mut effect: PreparedRuntimeEffect,
  ) -> MResult<RuntimeEffectId> {
    self.ensure_runtime_healthy("execute_runtime_effect_immediately")?;
    self.reject_effect_reentrancy("execute_runtime_effect_immediately")?;

    let effect_id = RuntimeEffectId {
      transaction: self.next_transaction_id(),
      sequence: 0,
    };
    match &mut effect {
      PreparedRuntimeEffect::Transactional(effect) => {
        self.active_effect_phase =
          Some(ActiveRuntimeEffectPhase::Preparing);
        let prepare_result = effect.prepare();
        self.active_effect_phase = None;
        prepare_result?;

        self.active_effect_phase =
          Some(ActiveRuntimeEffectPhase::Committing);
        let commit_result = effect.commit();
        self.active_effect_phase = None;
        if let Err(error) = commit_result {
          return Err(self.poison_external_commit_indeterminate(
            effect_id.transaction,
            effect_id,
            vec![format!(
              "immediate transactional effect {} commit failed: {:?}",
              effect_id,
              error,
            )],
          ));
        }
      }
      PreparedRuntimeEffect::Compensatable(effect) => {
        self.active_effect_phase =
          Some(ActiveRuntimeEffectPhase::Applying);
        let result = effect.apply();
        self.active_effect_phase = None;
        result?;
      }
      PreparedRuntimeEffect::AfterCommit(effect) => {
        self.active_effect_phase =
          Some(ActiveRuntimeEffectPhase::Delivering);
        let result = effect.deliver();
        self.active_effect_phase = None;
        result?;
      }
    }
    Ok(effect_id)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};

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
    assert_eq!(indeterminate.effect_id, failing_effect_id);
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
    runtime.active_effect_phase =
      Some(ActiveRuntimeEffectPhase::Preparing);

    let error = runtime.begin_transaction(&mut context).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectOperationReentrant");
    assert_eq!(context.transaction, None);
    assert!(runtime.active_transactions.is_empty());
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
