//! Runtime transaction abort and rollback protocol.

use crate::runtime::{
  MechRuntime,
  ScopedRuntimeState,
};
use crate::{
  ActiveRuntimeEffectPhase,
  RuntimeContext,
  RuntimeEffectId,
  RuntimeEventKind,
  RuntimeTransactionNotFoundError,
  TransactionId,
};
use mech_core::{
  MResult,
  MechError,
};
use std::collections::HashSet;

impl MechRuntime {
  pub fn abort_runtime_transaction(
    &mut self,
    context: &mut RuntimeContext,
    reason: impl Into<String>,
  ) -> MResult<()> {
    self.reject_effect_reentrancy("abort_runtime_transaction")?;
    self.reject_program_operation_reentrancy("abort_runtime_transaction")?;
    self.abort_runtime_transaction_internal(
      context,
      reason.into(),
      true,
    )
  }

  pub(in crate::runtime) fn abort_runtime_transaction_internal(
    &mut self,
    context: &mut RuntimeContext,
    reason: String,
    restore_program: bool,
  ) -> MResult<()> {
    let (transaction_id, rollback_failures) =
      self.abort_runtime_transaction_cleanup(
        context,
        &reason,
        restore_program,
      )?;

    if rollback_failures.is_empty() {
      return Ok(());
    }

    Err(self.poison_program_operation(
      "abort_runtime_transaction",
      Some(transaction_id),
      reason,
      rollback_failures,
    ))
  }

  pub(in crate::runtime) fn abort_runtime_transaction_cleanup(
    &mut self,
    context: &mut RuntimeContext,
    reason: &str,
    restore_program: bool,
  ) -> MResult<(TransactionId, Vec<String>)> {
    self.validate_context_for_runtime(context)?;

    let transaction_id = Self::context_transaction_id(context)?;
    let owns_program = self.program_transaction_owner == Some(transaction_id);
    let mut rollback_failures = Vec::new();
    let mut envelope = self
      .active_transactions
      .remove(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })?;

    if owns_program && restore_program {
      match &envelope.program {
        Some(baseline) => {
          if let Err(error) = self.program.restore(baseline.program.clone()) {
            rollback_failures.push(format!(
              "program restore failed: {:?}",
              error,
            ));
          }
          self.restore_live_state(baseline.live.clone());
        }
        None => rollback_failures.push(
          "program owner transaction has no retained-program baseline".to_string(),
        ),
      }
    }

    envelope
      .context_baseline
      .restore_preserving_consumption(context);
    if let Err(error) = self.validate_context_for_runtime(context) {
      rollback_failures.push(format!(
        "context baseline restore invariant failed: {:?}",
        error,
      ));
    }

    let abortable_effects = envelope.effects.abortable_ids();
    let phase_guard = ScopedRuntimeState::enter(
      &self.active_effect_phase,
      ActiveRuntimeEffectPhase::Aborting,
    );
    let effect_abort_failures = envelope.effects.abort_all();
    drop(phase_guard);
    let failed_effect_aborts: HashSet<RuntimeEffectId> =
      effect_abort_failures
        .iter()
        .map(|failure| failure.effect_id)
        .collect();
    rollback_failures.extend(
      Self::describe_effect_failures(effect_abort_failures),
    );

    if let Err(error) = envelope.store.abort(reason) {
      rollback_failures.push(format!(
        "staged store discard invariant failed: {:?}",
        error,
      ));
    }

    if owns_program {
      self.program_transaction_owner = None;
    }

    for effect_id in abortable_effects {
      if failed_effect_aborts.contains(&effect_id) {
        continue;
      }
      let _ = self.emit_event_immediate_to_context(
        context,
        RuntimeEventKind::EffectAborted { effect_id },
      );
    }

    let _ = self.emit_event_immediate_to_context(
      context,
      RuntimeEventKind::TransactionAborted {
        transaction_id,
        message: reason.to_string(),
      },
    );

    Ok((transaction_id, rollback_failures))
  }

  fn validate_implicit_cleanup_complete(
    &self,
    context: &RuntimeContext,
    transaction_id: TransactionId,
  ) -> Vec<String> {
    let mut failures = Vec::new();

    if self.active_transactions.contains_key(&transaction_id) {
      failures.push(format!(
        "active implicit transaction envelope {} still exists after cleanup",
        transaction_id,
      ));
    }
    if self.program_transaction_owner == Some(transaction_id) {
      failures.push(format!(
        "program owner still references implicit transaction {} after cleanup",
        transaction_id,
      ));
    }
    if context.transaction == Some(transaction_id) {
      failures.push(format!(
        "runtime context still references implicit transaction {} after cleanup",
        transaction_id,
      ));
    }
    if self
      .active_program_operation
      .get()
      .is_some_and(|active| active.transaction_id == transaction_id)
    {
      failures.push(format!(
        "active program operation still references implicit transaction {} after cleanup",
        transaction_id,
      ));
    }

    failures
  }

  fn finish_implicit_cleanup_best_effort(
    &mut self,
    context: &mut RuntimeContext,
    transaction_id: TransactionId,
    reason: &str,
  ) -> Vec<String> {
    let mut failures = Vec::new();

    if let Some(mut transaction) = self.active_transactions.remove(&transaction_id) {
      let phase_guard = ScopedRuntimeState::enter(
        &self.active_effect_phase,
        ActiveRuntimeEffectPhase::Aborting,
      );
      let effect_abort_failures = transaction.effects.abort_all();
      drop(phase_guard);
      failures.extend(
        Self::describe_effect_failures(effect_abort_failures),
      );
      if let Err(error) = transaction.store.abort(reason) {
        failures.push(format!(
          "best-effort staged store discard failed: {:?}",
          error,
        ));
      }
    }
    if self.program_transaction_owner == Some(transaction_id) {
      self.program_transaction_owner = None;
    }
    if context.transaction == Some(transaction_id) {
      context.transaction = None;
    }
    if self
      .active_program_operation
      .get()
      .is_some_and(|active| active.transaction_id == transaction_id)
    {
      self.active_program_operation.set(None);
    }

    failures
  }

  pub(in crate::runtime) fn cleanup_failed_implicit_operation(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    transaction_id: TransactionId,
    reason: &str,
  ) -> Vec<String> {
    let mut failures = Vec::new();

    #[cfg(test)]
    failures.extend(
      self.apply_program_transaction_test_fault(transaction_id),
    );

    match self.abort_runtime_transaction_cleanup(
      context,
      reason,
      false,
    ) {
      Ok((cleaned_transaction_id, cleanup_failures)) => {
        failures.extend(cleanup_failures);
        if cleaned_transaction_id != transaction_id {
          failures.push(format!(
            "implicit cleanup targeted transaction {}, expected {}",
            cleaned_transaction_id,
            transaction_id,
          ));
        }
      }
      Err(error) => failures.push(format!(
        "implicit transaction cleanup for `{}` transaction {} could not start: {:?}",
        operation,
        transaction_id,
        error,
      )),
    }

    let invariant_failures =
      self.validate_implicit_cleanup_complete(context, transaction_id);
    if !invariant_failures.is_empty() {
      failures.extend(invariant_failures);
      failures.extend(self.finish_implicit_cleanup_best_effort(
        context,
        transaction_id,
        reason,
      ));
      failures.extend(
        self
          .validate_implicit_cleanup_complete(context, transaction_id)
          .into_iter()
          .map(|failure| {
            format!(
              "implicit cleanup invariant remained unsatisfied: {}",
              failure,
            )
          }),
      );
    }

    failures
  }
}
