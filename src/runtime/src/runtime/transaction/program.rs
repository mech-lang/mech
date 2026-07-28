//! Atomic retained-program transaction coordination.
//!
//! [`RuntimeTransaction`] remains the staged-store component. The private
//! execution envelope in this module coordinates it with retained-program,
//! live-runtime, and context state.

use super::{
  RuntimeCommitResolution,
  RuntimeContextCheckpoint,
  RuntimeExecutionTransactionMode,
  RuntimeOperationSavepoint,
  RuntimeProgramBaseline,
  RuntimeProgramOperationSavepoint,
};
use crate::runtime::{
  MechRuntime,
  RuntimeLiveStateSnapshot,
  ScopedRuntimeState,
};
use crate::{
  ActiveRuntimeEffectPhase,
  RuntimeContext,
  RuntimeEffectId,
  RuntimeEffectOperationReentrant,
  RuntimeEventKind,
  RuntimeHealth,
  RuntimeInvalidOperationError,
  RuntimePoisonRecord,
  RuntimePoisoned,
  RuntimeProgramBusy,
  RuntimeProgramOperationReentrant,
  RuntimeProgramRollbackFailed,
  TaskId,
  TransactionId,
};
use mech_core::{
  MResult,
  MechError,
};
use mech_program::MechProgramCheckpoint;
use std::collections::HashSet;
#[cfg(feature = "invariant_define")]
use mech_program::{
  IntegrityConstraintFailureReason, IntegrityConstraintViolationSet,
};
#[cfg(feature = "invariant_define")]
use crate::{
  RuntimeIntegrityConstraintFailureReason,
  RuntimeIntegrityConstraintViolation,
};

#[cfg(feature = "invariant_define")]
pub(in crate::runtime) struct IntegrityFailureAudit {
  transaction_id: TransactionId,
  task_id: Option<TaskId>,
  violations: Vec<RuntimeIntegrityConstraintViolation>,
}

#[cfg(feature = "invariant_define")]
pub(in crate::runtime) fn integrity_failure_audit(
  error: &MechError,
  transaction_id: TransactionId,
  task_id: Option<TaskId>,
) -> Option<IntegrityFailureAudit> {
  let failures =
    error.kind_as::<IntegrityConstraintViolationSet>()?;
  let violations = failures
    .violations
    .iter()
    .map(|violation| RuntimeIntegrityConstraintViolation {
      interpreter_id: violation.interpreter_id,
      constraint_id: violation.constraint_id,
      name: violation.name.clone(),
      expression: violation.expression.clone(),
      reason: match violation.reason {
        IntegrityConstraintFailureReason::EvaluatedFalse => {
          RuntimeIntegrityConstraintFailureReason::EvaluatedFalse
        }
        IntegrityConstraintFailureReason::ExpectedBool => {
          RuntimeIntegrityConstraintFailureReason::ExpectedBool
        }
        IntegrityConstraintFailureReason::BorrowConflict => {
          RuntimeIntegrityConstraintFailureReason::BorrowConflict
        }
      },
      evaluated_kind: violation
        .evaluated_kind
        .as_ref()
        .map(ToString::to_string),
      actual: violation.actual.clone(),
      operator: violation.operator.as_ref().map(|operator| {
        format!("{:?}", operator)
      }),
      expected: violation.expected.clone(),
    })
    .collect();
  Some(IntegrityFailureAudit {
    transaction_id,
    task_id,
    violations,
  })
}

pub(in crate::runtime) enum RuntimeProgramOwnershipAcquisition {
  Existing,
  NewlyAcquired {
    program: MechProgramCheckpoint,
    live: RuntimeLiveStateSnapshot,
  },
}

#[derive(Clone, Copy, Debug)]
pub(in crate::runtime) struct ActiveRuntimeProgramOperation {
  pub(in crate::runtime) transaction_id: TransactionId,
  pub(in crate::runtime) operation: &'static str,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProgramTransactionTestFault {
  RemoveImplicitEnvelopeBeforeCleanup,
  FailImplicitStoreAbort,
}

#[cfg(test)]
thread_local! {
  static PROGRAM_TRANSACTION_TEST_FAULT:
    std::cell::RefCell<Option<ProgramTransactionTestFault>> =
      const { std::cell::RefCell::new(None) };
  static RUNTIME_PROGRAM_CHECKPOINT_COUNT:
    std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn set_program_transaction_test_fault(fault: ProgramTransactionTestFault) {
  PROGRAM_TRANSACTION_TEST_FAULT.with(|slot| {
    assert!(
      slot.replace(Some(fault)).is_none(),
      "program transaction test fault was already armed",
    );
  });
}

#[cfg(test)]
fn take_program_transaction_test_fault() -> Option<ProgramTransactionTestFault> {
  PROGRAM_TRANSACTION_TEST_FAULT.with(|slot| slot.replace(None))
}

#[cfg(test)]
pub(in crate::runtime) fn reset_runtime_program_checkpoint_count() {
  RUNTIME_PROGRAM_CHECKPOINT_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(in crate::runtime) fn runtime_program_checkpoint_count() -> usize {
  RUNTIME_PROGRAM_CHECKPOINT_COUNT.with(std::cell::Cell::get)
}

impl MechRuntime {
  #[cfg(feature = "invariant_define")]
  pub(in crate::runtime) fn emit_integrity_failure_audit(
    &mut self,
    context: &mut RuntimeContext,
    audit: Option<IntegrityFailureAudit>,
  ) {
    let Some(audit) = audit else {
      return;
    };
    let _ = self.emit_event_immediate_to_context(
      context,
      RuntimeEventKind::IntegrityConstraintViolated {
        transaction_id: audit.transaction_id,
        task_id: audit.task_id,
        violations: audit.violations,
      },
    );
  }

  fn capture_runtime_program_checkpoint(
    &self,
  ) -> MResult<MechProgramCheckpoint> {
    #[cfg(test)]
    RUNTIME_PROGRAM_CHECKPOINT_COUNT.with(|count| {
      count.set(count.get().saturating_add(1));
    });
    self.program.checkpoint()
  }

  pub(in crate::runtime) fn acquire_program_transaction_ownership(
    &mut self,
    transaction_id: TransactionId,
    operation: &'static str,
  ) -> MResult<RuntimeProgramOwnershipAcquisition> {
    if let Some(owner) = self.program_transaction_owner {
      if owner != transaction_id {
        return Err(MechError::new(
          RuntimeProgramBusy {
            operation,
            owner,
            requester: Some(transaction_id),
          },
          None,
        ));
      }
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      if transaction.mode != RuntimeExecutionTransactionMode::Explicit {
        return self.coordinator_invariant_failure(
          operation,
          Some(transaction_id),
          "program owner transaction is not explicit",
        );
      }
      if transaction.program.is_none() {
        return self.coordinator_invariant_failure(
          operation,
          Some(transaction_id),
          "program ownership exists without a transaction program baseline",
        );
      }
      return Ok(RuntimeProgramOwnershipAcquisition::Existing);
    }

    {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      if transaction.mode != RuntimeExecutionTransactionMode::Explicit {
        return self.coordinator_invariant_failure(
          operation,
          Some(transaction_id),
          "program ownership can only be acquired by an explicit transaction",
        );
      }
      if transaction.program.is_some() {
        return self.coordinator_invariant_failure(
          operation,
          Some(transaction_id),
          "transaction has a program baseline without program ownership",
        );
      }
    }

    let program = self.capture_runtime_program_checkpoint()?;
    let live = self.live_state_snapshot();
    self.active_execution_transaction_mut(transaction_id)?.program =
      Some(RuntimeProgramBaseline {
        program: program.clone(),
        live: live.clone(),
      });
    self.program_transaction_owner = Some(transaction_id);
    Ok(RuntimeProgramOwnershipAcquisition::NewlyAcquired {
      program,
      live,
    })
  }

  pub(in crate::runtime) fn release_new_program_transaction_ownership(
    &mut self,
    transaction_id: TransactionId,
  ) -> MResult<()> {
    if self.program_transaction_owner != Some(transaction_id) {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "release_new_program_transaction_ownership",
          reason: format!(
            "transaction {} does not own the retained program",
            transaction_id,
          ),
        },
        None,
      ));
    }
    if self
      .active_execution_transaction(transaction_id)?
      .program
      .is_none()
    {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "release_new_program_transaction_ownership",
          reason: format!(
            "transaction {} has no retained program baseline",
            transaction_id,
          ),
        },
        None,
      ));
    }
    self.active_execution_transaction_mut(transaction_id)?.program = None;
    self.program_transaction_owner = None;
    Ok(())
  }

  pub(in crate::runtime) fn ensure_runtime_healthy(
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

  pub(in crate::runtime) fn reject_program_operation_reentrancy(
    &self,
    requested_operation: &'static str,
  ) -> MResult<()> {
    let Some(active) = self.active_program_operation.get() else {
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

  pub(in crate::runtime) fn reject_effect_reentrancy(
    &self,
    requested_operation: &'static str,
  ) -> MResult<()> {
    let Some(active_phase) = self.active_effect_phase.get() else {
      return Ok(());
    };
    Err(MechError::new(
      RuntimeEffectOperationReentrant {
        active_phase,
        requested_operation,
      },
      None,
    ))
  }

  pub(in crate::runtime) fn ensure_runtime_mutation_allowed(
    &self,
    operation: &'static str,
  ) -> MResult<()> {
    self.ensure_runtime_healthy(operation)?;
    self.reject_effect_reentrancy(operation)
  }

  pub(in crate::runtime) fn poison_program_operation(
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

  pub(in crate::runtime) fn coordinator_invariant_failure<T>(
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

  pub(in crate::runtime) fn capture_runtime_operation_savepoint(
    &self,
    context: &RuntimeContext,
    transaction_id: TransactionId,
  ) -> MResult<RuntimeOperationSavepoint> {
    let transaction =
      self.active_execution_transaction(transaction_id)?;
    Ok(RuntimeOperationSavepoint {
      store: transaction.store.clone(),
      module_mark: transaction.modules.mark(),
      effect_mark: transaction.effects.mark(),
      capability_mark: transaction.capabilities.mark(),
      context: RuntimeContextCheckpoint::capture(context),
    })
  }

  pub(in crate::runtime) fn rollback_runtime_operation(
    &mut self,
    context: &mut RuntimeContext,
    transaction_id: TransactionId,
    savepoint: &RuntimeOperationSavepoint,
  ) -> Vec<String> {
    let mut failures = Vec::new();

    let phase_guard = ScopedRuntimeState::enter(
      &self.active_effect_phase,
      ActiveRuntimeEffectPhase::Aborting,
    );
    let effect_rollback = match self.active_transactions.get_mut(&transaction_id) {
      Some(transaction) => {
        let abortable_ids = transaction
          .effects
          .abortable_ids_after(savepoint.effect_mark);
        let effect_failures =
          transaction.effects.rollback_to(savepoint.effect_mark);
        let capability_result = transaction
          .capabilities
          .rollback_to(savepoint.capability_mark);
        let module_result =
          transaction.modules.rollback_to(savepoint.module_mark);
        transaction.store = savepoint.store.clone();
        Some((
          effect_failures,
          capability_result,
          module_result,
          abortable_ids,
        ))
      }
      None => None,
    };
    drop(phase_guard);
    match effect_rollback {
      Some((
        effect_failures,
        capability_result,
        module_result,
        abortable_ids,
      )) => {
        let failed_effects: HashSet<RuntimeEffectId> = effect_failures
          .iter()
          .map(|failure| failure.effect_id)
          .collect();
        failures.extend(Self::describe_effect_failures(effect_failures));
        if let Err(error) = capability_result {
          failures.push(format!(
            "capability overlay rollback failed: {:?}",
            error,
          ));
        }
        if let Err(error) = module_result {
          failures.push(format!(
            "module journal rollback failed: {:?}",
            error,
          ));
        }
        for effect_id in abortable_ids {
          if failed_effects.contains(&effect_id) {
            continue;
          }
          let _ = self.emit_effect_event_outside_transaction(
            context,
            RuntimeEventKind::EffectAborted { effect_id },
          );
        }
      }
      None => failures.push(format!(
        "active execution transaction {} disappeared before operation rollback",
        transaction_id,
      )),
    }

    savepoint
      .context
      .restore_preserving_consumption(context);

    if let Err(error) = self.validate_context_for_runtime(context) {
      failures.push(format!("context restore invariant failed: {:?}", error));
    }

    failures
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
    failures.extend(self.rollback_runtime_operation(
      context,
      transaction_id,
      &savepoint.runtime,
    ));
    failures
  }

  #[cfg(test)]
  pub(in crate::runtime) fn apply_program_transaction_test_fault(
    &mut self,
    transaction_id: TransactionId,
  ) -> Vec<String> {
    let mut failures = Vec::new();

    match take_program_transaction_test_fault() {
      Some(ProgramTransactionTestFault::RemoveImplicitEnvelopeBeforeCleanup) => {
        self.active_transactions.remove(&transaction_id);
      }
      Some(ProgramTransactionTestFault::FailImplicitStoreAbort) => {
        match self.active_transactions.get_mut(&transaction_id) {
          Some(transaction) => {
            transaction.store.status =
              crate::transaction::TransactionStatus::Committed;
          }
          None => failures.push(format!(
            "could not arm staged-store abort failure for missing implicit transaction {}",
            transaction_id,
          )),
        }
      }
      None => {}
    }

    failures
  }

  pub(in crate::runtime) fn preflight_atomic_program_operation(
    &self,
    context: &RuntimeContext,
    operation: &'static str,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed(operation)?;
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

  pub(in crate::runtime) fn with_atomic_program_operation<T>(
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
      let program_checkpoint =
        self.capture_runtime_program_checkpoint()?;
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
      match self.acquire_program_transaction_ownership(
        transaction_id,
        operation,
      )? {
        RuntimeProgramOwnershipAcquisition::NewlyAcquired {
          program,
          live,
        } => {
          first_explicit_operation = true;
          (transaction_id, program, live)
        }
        RuntimeProgramOwnershipAcquisition::Existing => (
          transaction_id,
          self.capture_runtime_program_checkpoint()?,
          self.live_state_snapshot(),
        ),
      }
    };

    let savepoint = RuntimeProgramOperationSavepoint {
      program: program_checkpoint,
      live: live_checkpoint,
      runtime: self.capture_runtime_operation_savepoint(
        context,
        transaction_id,
      )?,
    };

    let _operation_guard = ScopedRuntimeState::enter(
      &self.active_program_operation,
      ActiveRuntimeProgramOperation {
        transaction_id,
        operation,
      },
    );
    let execution_result = execute(self, context).and_then(|value| {
      #[cfg(feature = "invariant_define")]
      self.program.validate_integrity_constraints()?;
      Ok(value)
    });
    drop(_operation_guard);

    let original_error = match execution_result {
      Ok(value) if implicit => {
        match self.commit_runtime_transaction_internal(context) {
          Ok(RuntimeCommitResolution::Committed(_)) => {
            return Ok(value);
          }
          Ok(
            RuntimeCommitResolution::CommittedWithError {
              error,
              ..
            },
          ) => {
            return Err(error);
          }
          Err(error) => error,
        }
      }
      Ok(value) => return Ok(value),
      Err(error) => error,
    };

    let original_error_text = format!("{:?}", original_error);
    #[cfg(feature = "invariant_define")]
    let integrity_audit = integrity_failure_audit(
      &original_error,
      transaction_id,
      context.task,
    );
    let rollback_failures = self.rollback_program_operation(
      context,
      transaction_id,
      &savepoint,
    );

    if rollback_failures.is_empty() {
      if implicit {
        let cleanup_failures = self.cleanup_failed_implicit_operation(
          context,
          operation,
          transaction_id,
          &format!("retained program operation `{}` failed", operation),
        );
        if cleanup_failures.is_empty() {
          #[cfg(feature = "invariant_define")]
          self.emit_integrity_failure_audit(
            context,
            integrity_audit,
          );
          return Err(original_error);
        }
        return Err(self.poison_program_operation(
          operation,
          Some(transaction_id),
          original_error_text,
          cleanup_failures,
        ));
      } else if first_explicit_operation {
        if let Err(error) =
          self.release_new_program_transaction_ownership(transaction_id)
        {
          return Err(self.poison_program_operation(
            operation,
            Some(transaction_id),
            original_error_text,
            vec![format!(
              "program ownership release failed: {:?}",
              error,
            )],
          ));
        }
      }
      #[cfg(feature = "invariant_define")]
      self.emit_integrity_failure_audit(
        context,
        integrity_audit,
      );
      return Err(original_error);
    }

    let mut rollback_failures = rollback_failures;
    if implicit {
      rollback_failures.extend(self.cleanup_failed_implicit_operation(
        context,
        operation,
        transaction_id,
        &format!(
          "retained program operation `{}` rollback failed",
          operation,
        ),
      ));
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
#[path = "../program_transaction/tests/mod.rs"]
mod tests;
