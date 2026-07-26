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
  pub fn stage_runtime_effect_with_context(
    &mut self,
    context: &mut RuntimeContext,
    effect: PreparedRuntimeEffect,
  ) -> MResult<RuntimeEffectId> {
    self.ensure_runtime_healthy("stage_runtime_effect_with_context")?;
    self.validate_context_for_runtime(context)?;

    let transaction_id = Self::context_transaction_id(context)?;
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
