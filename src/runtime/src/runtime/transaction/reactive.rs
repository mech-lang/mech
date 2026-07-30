//! Runtime coordination for compact reactive program turns.

use super::{
  ActiveRuntimeProgramOperation,
  RuntimeCommitResolution,
  RuntimeExecutionTransactionMode,
  RuntimeOperationSavepoint,
  RuntimeProgramOwnershipAcquisition,
};
use crate::runtime::state::ScopedRuntimeState;
use crate::runtime::MechRuntime;
use crate::{
  RuntimeContext,
  RuntimeInvalidOperationError,
  TransactionId,
};
use mech_core::{
  MResult,
  MechError,
  MechExecutionServices,
  Value,
};
use std::cell::RefCell;
use std::collections::HashSet;
use mech_program::{
  ExecutionServicesBorrowConflict, ProgramInputUpdate,
  MechProgram, ProgramTurnFinalization,
};

#[cfg(test)]
thread_local! {
  static COORDINATED_SERVICE_REENTRY:
    RefCell<Option<&'static str>> = const { RefCell::new(None) };
}

#[cfg(test)]
fn arm_coordinated_service_reentry(name: &'static str) {
  COORDINATED_SERVICE_REENTRY.with(|armed| {
    assert!(
      armed.replace(Some(name)).is_none(),
      "coordinated service reentry was already armed",
    );
  });
}

#[cfg(test)]
fn take_coordinated_service_reentry(name: &str) -> bool {
  COORDINATED_SERVICE_REENTRY.with(|armed| {
    if *armed.borrow() != Some(name) {
      return false;
    }
    armed.replace(None);
    true
  })
}

fn execution_services_borrow_conflict(
  operation: &'static str,
) -> MechError {
  MechError::new(
    ExecutionServicesBorrowConflict { operation },
    None,
  )
  .with_compiler_loc()
}

fn reactive_panic_message(
  panic: &(dyn std::any::Any + Send),
) -> String {
  if let Some(message) = panic.downcast_ref::<&'static str>() {
    return (*message).to_string();
  }

  if let Some(message) = panic.downcast_ref::<String>() {
    return message.clone();
  }

  "non-string reactive panic payload".to_string()
}

struct RuntimeCoordinatedTurn<'a> {
  runtime: &'a mut MechRuntime,
  context: &'a mut RuntimeContext,
  finalization: RuntimeReactiveFinalization,
}

struct RuntimeCoordinatedExecutionServices<'a, 'turn> {
  turn: &'a RefCell<RuntimeCoordinatedTurn<'turn>>,
}

impl MechExecutionServices
  for RuntimeCoordinatedExecutionServices<'_, '_>
{
  fn invoke_native(
    &mut self,
    name: &str,
    arguments: &[Value],
  ) -> MResult<Value> {
    let mut turn = self
      .turn
      .try_borrow_mut()
      .map_err(|_| {
        execution_services_borrow_conflict(
          "runtime_invoke_native",
        )
      })?;
    #[cfg(test)]
    if take_coordinated_service_reentry(name) {
      let mut nested = RuntimeCoordinatedExecutionServices {
        turn: self.turn,
      };
      return nested.invoke_native(name, arguments);
    }
    let RuntimeCoordinatedTurn {
      runtime,
      context,
      ..
    } = &mut *turn;
    runtime.with_runtime_execution_session(
      context,
      |session| session.invoke_native(name, arguments),
    )
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeReactiveFinalization {
  Pending,
  ExplicitCommit,
  ImplicitCommitted,
  ImplicitCommittedWithError,
  RollbackRequired,
}

pub(in crate::runtime) struct PreparedRuntimeHostInput {
  pub(in crate::runtime) update_count: usize,
  pub(in crate::runtime) ignored_update_count: usize,
  pub(in crate::runtime) binding_count: usize,
  pub(in crate::runtime) updates: Vec<ProgramInputUpdate>,
}

impl MechRuntime {
  pub(in crate::runtime) fn prepare_runtime_host_input(
    &self,
    input: &crate::RuntimeHostInput,
  ) -> MResult<PreparedRuntimeHostInput> {
    input.validate()?;
    let mut updates = Vec::new();
    let mut seen_targets = HashSet::new();
    let mut ignored_update_count = 0;

    for update in &input.updates {
      let Some(bindings) =
        self.live_input_bindings.get(&update.source)
      else {
        ignored_update_count += 1;
        continue;
      };
      if bindings.is_empty() {
        ignored_update_count += 1;
        continue;
      }
      let value = update.value.clone().into_mech_value()?;
      for program_input in bindings {
        if !seen_targets.insert(*program_input) {
          return Err(MechError::new(
            mech_program::ProgramInputDuplicateTarget {
              input: *program_input,
            },
            None,
          ));
        }
        updates.push(ProgramInputUpdate {
          input: *program_input,
          value: value.clone(),
        });
      }
    }

    Ok(PreparedRuntimeHostInput {
      update_count: input.updates.len(),
      ignored_update_count,
      binding_count: updates.len(),
      updates,
    })
  }

  pub(in crate::runtime) fn validate_live_turn_context(
    &self,
    context: &RuntimeContext,
  ) -> MResult<()> {
    let template = self.live_context_template.as_ref().ok_or_else(|| {
      MechError::new(
        RuntimeInvalidOperationError {
          operation: "RuntimeLiveContextMissing",
          reason:
            "host input turn requires a stored live program context"
              .to_string(),
        },
        None,
      )
    })?;

    let identity_matches =
      template.runtime == context.runtime
        && template.subject == context.subject
        && template.task == context.task
        && template.actor == context.actor
        && template.module_version == context.module_version
        && template.actor_message == context.actor_message
        && template.actor_state == context.actor_state
        && template.budget_limits.max_steps
          == context.budget.max_steps
        && template.budget_limits.max_bytes
          == context.budget.max_bytes
        && template.budget_limits.max_items
          == context.budget.max_items
        && template.budget_limits.max_messages
          == context.budget.max_messages;
    if !identity_matches {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "RuntimeLiveContextMismatch",
          reason:
            "host input attempted to change the live program execution identity or budget maxima"
              .to_string(),
        },
        None,
      ));
    }

    let mut expected_authority = template.authority.clone();
    if let Some(transaction_id) = context.transaction {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      for (capability, _) in transaction.capabilities.grants() {
        expected_authority.add(capability);
      }
      let revocations = transaction.capabilities.revocation_ids();
      for capability in revocations {
        expected_authority.remove(capability);
      }
    }
    if context.authority != expected_authority {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "RuntimeLiveContextMismatch",
          reason:
            "host input context authority does not match the live program and active transaction"
              .to_string(),
        },
        None,
      ));
    }

    Ok(())
  }

  pub(in crate::runtime) fn with_atomic_reactive_turn<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    execute: impl FnOnce(
      &mut MechProgram,
      &mut dyn MechExecutionServices,
      &mut dyn FnMut(&T) -> ProgramTurnFinalization,
    ) -> MResult<T>,
    after_program: impl FnOnce(
      &mut MechRuntime,
      &mut RuntimeContext,
      &T,
    ) -> MResult<()>,
  ) -> MResult<T> {
    self.preflight_atomic_program_operation(context, operation)?;

    let implicit = context.transaction.is_none();
    let (transaction_id, newly_acquired_ownership) = if implicit {
      let transaction_id = self.begin_runtime_transaction_internal(
        context,
        RuntimeExecutionTransactionMode::ImplicitReactiveTurn,
      )?;
      if self
        .active_execution_transaction(transaction_id)?
        .program
        .is_some()
      {
        return self.coordinator_invariant_failure(
          operation,
          Some(transaction_id),
          "implicit reactive turn unexpectedly retained a full program baseline",
        );
      }
      self.program_transaction_owner = Some(transaction_id);
      (transaction_id, false)
    } else {
      let transaction_id = context.transaction.unwrap();
      let newly_acquired = matches!(
        self.acquire_program_transaction_ownership(
          transaction_id,
          operation,
        )?,
        RuntimeProgramOwnershipAcquisition::NewlyAcquired { .. },
      );
      (transaction_id, newly_acquired)
    };

    let runtime_savepoint =
      match self.capture_runtime_operation_savepoint(
        context,
        transaction_id,
      ) {
        Ok(savepoint) => savepoint,
        Err(error) => {
          let original_error = format!("{:?}", error);
          let mut failures = if implicit {
            self.cleanup_failed_implicit_operation(
              context,
              operation,
              transaction_id,
              "reactive turn savepoint capture failed",
            )
          } else {
            Vec::new()
          };
          if newly_acquired_ownership {
            if let Err(release_error) =
              self.release_new_program_transaction_ownership(transaction_id)
            {
              failures.push(format!(
                "program ownership release failed: {:?}",
                release_error,
              ));
            }
          }
          if failures.is_empty() {
            return Err(error);
          }
          return Err(self.poison_program_operation(
            operation,
            Some(transaction_id),
            original_error,
            failures,
          ));
        }
      };

    if let Err(error) = context.charge_step() {
      return self.finish_failed_reactive_runtime_turn(
        context,
        operation,
        transaction_id,
        &runtime_savepoint,
        error,
        implicit,
        newly_acquired_ownership,
      );
    }

    let _operation_guard = ScopedRuntimeState::enter(
      &self.active_program_operation,
      ActiveRuntimeProgramOperation {
        transaction_id,
        operation,
      },
    );

    let replacement =
      MechProgram::new(self.program.config.clone());
    let mut program =
      std::mem::replace(&mut self.program, replacement);
    let turn = RefCell::new(RuntimeCoordinatedTurn {
      runtime: self,
      context,
      finalization: RuntimeReactiveFinalization::Pending,
    });
    let mut services =
      RuntimeCoordinatedExecutionServices { turn: &turn };
    let mut after_program = Some(after_program);
    let execution_result = std::panic::catch_unwind(
      std::panic::AssertUnwindSafe(|| {
        let mut finalize = |value: &T| {
          let mut turn = match turn.try_borrow_mut() {
            Ok(turn) => turn,
            Err(_) => {
              return ProgramTurnFinalization::Rollback(
                execution_services_borrow_conflict(
                  "runtime_finalize_reactive_turn",
                ),
              );
            }
          };
          let RuntimeCoordinatedTurn {
            runtime,
            context,
            finalization,
          } = &mut *turn;
          let after_result = after_program
            .take()
            .expect("program finalizer runs exactly once")(
              runtime,
              context,
              value,
            );
          if let Err(error) = after_result {
            *finalization =
              RuntimeReactiveFinalization::RollbackRequired;
            return ProgramTurnFinalization::Rollback(error);
          }
          if !implicit {
            *finalization =
              RuntimeReactiveFinalization::ExplicitCommit;
            return ProgramTurnFinalization::Commit;
          }
          match runtime
            .commit_runtime_transaction_internal(context)
          {
            Ok(
              RuntimeCommitResolution::Committed(
                _,
              ),
            ) => {
              *finalization =
                RuntimeReactiveFinalization::ImplicitCommitted;
              ProgramTurnFinalization::Commit
            }
            Ok(
              RuntimeCommitResolution::CommittedWithError {
                error,
                ..
              },
            ) => {
              *finalization =
                RuntimeReactiveFinalization::ImplicitCommittedWithError;
              ProgramTurnFinalization::CommitWithError(error)
            }
            Err(error) => {
              *finalization =
                RuntimeReactiveFinalization::RollbackRequired;
              ProgramTurnFinalization::Rollback(error)
            }
          }
        };
        execute(
          &mut program,
          &mut services,
          &mut finalize,
        )
      }),
    );
    {
      // The execution callback has returned, so every adapter and finalizer
      // borrow is out of scope. This local RefCell never escapes this method.
      let mut turn = turn.borrow_mut();
      turn.runtime.program = program;
    }
    drop(services);
    drop(_operation_guard);

    let execution_result = match execution_result {
      Ok(result) => result,
      Err(panic) => {
        let message = reactive_panic_message(&*panic);
        {
          let mut turn = turn.borrow_mut();
          let RuntimeCoordinatedTurn {
            runtime,
            context,
            ..
          } = &mut *turn;
          runtime.finish_panicked_reactive_runtime_turn(
            context,
            operation,
            transaction_id,
            &runtime_savepoint,
            implicit,
            message,
          );
        }
        drop(turn);
        std::panic::resume_unwind(panic);
      }
    };
    let finalization = {
      turn.borrow().finalization
    };
    drop(turn);

    match finalization {
      RuntimeReactiveFinalization::ExplicitCommit
      | RuntimeReactiveFinalization::ImplicitCommitted
      | RuntimeReactiveFinalization::ImplicitCommittedWithError => {
        execution_result
      }
      RuntimeReactiveFinalization::Pending
      | RuntimeReactiveFinalization::RollbackRequired => {
        let error = match execution_result {
          Ok(_) => {
            return self.coordinator_invariant_failure(
              operation,
              Some(transaction_id),
              "program returned success without finalizing its reactive turn",
            );
          }
          Err(error) => error,
        };
        // Integrity validation runs inside the program journal before this
        // runtime finalizer. A rejected candidate therefore intentionally
        // leaves finalization pending so the runtime savepoint can discard
        // provisional effects and context changes.
        self.finish_failed_reactive_runtime_turn(
          context,
          operation,
          transaction_id,
          &runtime_savepoint,
          error,
          implicit,
          newly_acquired_ownership,
        )
      }
    }
  }

  fn finish_failed_reactive_runtime_turn<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    transaction_id: TransactionId,
    runtime_savepoint: &RuntimeOperationSavepoint,
    original_error: MechError,
    implicit: bool,
    newly_acquired_ownership: bool,
  ) -> MResult<T> {
    let original_error_text = format!("{:?}", original_error);
    #[cfg(feature = "invariant_define")]
    let integrity_audit =
      super::program::integrity_failure_audit(
        &original_error,
        transaction_id,
        context.task,
      );
    let mut rollback_failures =
      self.rollback_runtime_operation(
        context,
        transaction_id,
        runtime_savepoint,
      );

    if implicit {
      rollback_failures.extend(self.cleanup_failed_implicit_operation(
        context,
        operation,
        transaction_id,
        &format!("reactive operation `{}` failed", operation),
      ));
    } else if newly_acquired_ownership
      && rollback_failures.is_empty()
    {
      if let Err(error) =
        self.release_new_program_transaction_ownership(transaction_id)
      {
        rollback_failures.push(format!(
          "program ownership release failed: {:?}",
          error,
        ));
      }
    }

    if rollback_failures.is_empty() {
      #[cfg(feature = "invariant_define")]
      self.emit_integrity_failure_audit(
        context,
        integrity_audit,
      );
      return Err(original_error);
    }

    Err(self.poison_program_operation(
      operation,
      Some(transaction_id),
      original_error_text,
      rollback_failures,
    ))
  }

  fn finish_panicked_reactive_runtime_turn(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    transaction_id: TransactionId,
    runtime_savepoint: &RuntimeOperationSavepoint,
    implicit: bool,
    panic_message: String,
  ) {
    let reason = format!(
      "reactive operation `{}` panicked: {}",
      operation,
      panic_message,
    );
    let mut cleanup_failures = if implicit {
      let mut failures = self.rollback_runtime_operation(
        context,
        transaction_id,
        runtime_savepoint,
      );
      failures.extend(self.cleanup_failed_implicit_operation(
        context,
        operation,
        transaction_id,
        &reason,
      ));
      failures
    } else {
      let mut failures = Vec::new();
      match self.abort_runtime_transaction_cleanup(
        context,
        &reason,
        true,
      ) {
        Ok((cleaned_transaction_id, abort_failures)) => {
          if cleaned_transaction_id != transaction_id {
            failures.push(format!(
              "panic cleanup targeted transaction {}, expected {}",
              cleaned_transaction_id,
              transaction_id,
            ));
          }
          failures.extend(abort_failures);
        }
        Err(error) => failures.push(format!(
          "transaction cleanup for panicked reactive operation `{}` transaction {} could not start: {:?}",
          operation,
          transaction_id,
          error,
        )),
      }
      failures
    };

    cleanup_failures.extend(
      self.finish_transaction_cleanup_best_effort(
        context,
        transaction_id,
        &reason,
      ),
    );
    cleanup_failures.extend(
      self
        .validate_transaction_cleanup_complete(
          context,
          transaction_id,
        )
        .into_iter()
        .map(|failure| {
          format!(
            "panic cleanup invariant remained unsatisfied: {}",
            failure,
          )
        }),
    );
    cleanup_failures.push(
      "retained program state is not trusted after panic unwound through the compact reactive journal"
        .to_string(),
    );

    let _ = self.poison_program_operation(
      operation,
      Some(transaction_id),
      reason,
      cleanup_failures,
    );
  }

  #[cfg(test)]
  fn with_atomic_reactive_turn_for_test(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    execute: impl FnOnce(
      &mut MechRuntime,
      &mut RuntimeContext,
    ) -> MResult<()>,
  ) -> MResult<()> {
    self.with_atomic_reactive_turn(
      context,
      operation,
      |program, services, finalize| {
        program.step_coordinated(
          0,
          services,
          || finalize(&()),
        )
      },
      |runtime, context, _| execute(runtime, context),
    )
  }
}

#[cfg(test)]
#[path = "tests/reactive/mod.rs"]
mod tests;
