//! Runtime coordination for compact reactive program turns.

use super::*;
use mech_program::{
  ProgramInputUpdate, ProgramReactiveTurnJournal,
};

pub(super) struct PreparedRuntimeHostInput {
  pub(super) update_count: usize,
  pub(super) ignored_update_count: usize,
  pub(super) binding_count: usize,
  pub(super) updates: Vec<ProgramInputUpdate>,
}

impl MechRuntime {
  pub(super) fn prepare_runtime_host_input(
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

  pub(super) fn validate_live_turn_context(
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

    let mut expected_capabilities = template.capabilities.clone();
    if let Some(transaction_id) = context.transaction {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      expected_capabilities.extend(
        transaction
          .capabilities
          .grants()
          .map(|(capability, _)| capability),
      );
      let revocations = transaction.capabilities.revocation_ids();
      expected_capabilities
        .retain(|capability| !revocations.contains(capability));
    }
    expected_capabilities.sort_unstable();
    expected_capabilities.dedup();

    let mut supplied_capabilities = context.capabilities.clone();
    supplied_capabilities.sort_unstable();
    supplied_capabilities.dedup();
    if supplied_capabilities != expected_capabilities {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "RuntimeLiveContextMismatch",
          reason:
            "host input context capabilities do not match the live program and active transaction"
              .to_string(),
        },
        None,
      ));
    }

    Ok(())
  }

  pub(super) fn with_atomic_reactive_turn<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    execute: impl FnOnce(
      &mut MechRuntime,
      &mut RuntimeContext,
      &mut ProgramReactiveTurnJournal,
    ) -> MResult<T>,
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

    let mut program_journal = ProgramReactiveTurnJournal::new();
    let _operation_guard = ScopedRuntimeState::enter(
      &self.active_program_operation,
      ActiveRuntimeProgramOperation {
        transaction_id,
        operation,
      },
    );
    let execution_result =
      execute(self, context, &mut program_journal);
    drop(_operation_guard);

    match execution_result {
      Ok(value) if implicit => {
        match self.commit_runtime_transaction_internal(context) {
          Ok(super::transaction::RuntimeCommitResolution::Committed(_)) => {
            Ok(value)
          }
          Ok(
            super::transaction::RuntimeCommitResolution::CommittedWithError {
              error,
              ..
            },
          ) => Err(error),
          Err(error) => self.finish_failed_reactive_turn(
            context,
            operation,
            transaction_id,
            program_journal,
            &runtime_savepoint,
            error,
            true,
            false,
          ),
        }
      }
      Ok(value) => Ok(value),
      Err(error) => self.finish_failed_reactive_turn(
        context,
        operation,
        transaction_id,
        program_journal,
        &runtime_savepoint,
        error,
        implicit,
        newly_acquired_ownership,
      ),
    }
  }

  fn rollback_reactive_turn_operation(
    &mut self,
    context: &mut RuntimeContext,
    transaction_id: TransactionId,
    program_journal: ProgramReactiveTurnJournal,
    runtime_savepoint: &RuntimeOperationSavepoint,
  ) -> Vec<String> {
    let mut failures = Vec::new();
    if let Err(error) =
      self.program.rollback_reactive_turn(program_journal)
    {
      failures.push(format!(
        "compact program reactive-turn rollback failed: {:?}",
        error,
      ));
    }
    failures.extend(self.rollback_runtime_operation(
      context,
      transaction_id,
      runtime_savepoint,
    ));
    failures
  }

  fn finish_failed_reactive_turn<T>(
    &mut self,
    context: &mut RuntimeContext,
    operation: &'static str,
    transaction_id: TransactionId,
    program_journal: ProgramReactiveTurnJournal,
    runtime_savepoint: &RuntimeOperationSavepoint,
    original_error: MechError,
    implicit: bool,
    newly_acquired_ownership: bool,
  ) -> MResult<T> {
    let original_error_text = format!("{:?}", original_error);
    let mut rollback_failures = self.rollback_reactive_turn_operation(
      context,
      transaction_id,
      program_journal,
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
      return Err(original_error);
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
  use std::cell::RefCell;
  use std::rc::Rc;
  use std::sync::{Arc, Mutex};

  use crate::capability::{
    BasicCapability, BasicConstraints, BasicOperation, BasicResource,
    BasicSubject, SharedCapabilityKernel,
  };
  use crate::ClosureHostFunction;
  use crate::{
    PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeTransactionalEffect,
  };
  use super::super::program_transaction::{
    reset_runtime_program_checkpoint_count,
    runtime_program_checkpoint_count,
  };
  use mech_core::{
    GenericError, ReactiveSolveStatus, Ref,
  };

  struct ReactiveTransactionTestFunction {
    output: Ref<usize>,
    calls: Rc<RefCell<usize>>,
    fail_on_call: Option<usize>,
  }

  #[derive(Debug)]
  struct ReactiveTransactionalProbe {
    log: Arc<Mutex<Vec<&'static str>>>,
    fail_prepare: bool,
    fail_commit: bool,
    fail_abort: bool,
  }

  impl RuntimeTransactionalEffect for ReactiveTransactionalProbe {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::Custom {
          name: "reactive-transaction-probe".to_string(),
        },
        "reactive-transaction-probe",
      )
    }

    fn prepare(&mut self) -> MResult<()> {
      self.log.lock().unwrap().push("prepare");
      if self.fail_prepare {
        return Err(MechError::new(
          GenericError {
            msg: "deliberate reactive prepare failure".to_string(),
          },
          None,
        ));
      }
      Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
      self.log.lock().unwrap().push("commit");
      if self.fail_commit {
        return Err(MechError::new(
          GenericError {
            msg: "deliberate reactive commit failure".to_string(),
          },
          None,
        ));
      }
      Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
      self.log.lock().unwrap().push("abort");
      if self.fail_abort {
        return Err(MechError::new(
          GenericError {
            msg: "deliberate reactive abort failure".to_string(),
          },
          None,
        ));
      }
      Ok(())
    }
  }

  #[derive(Debug)]
  struct ReactiveAfterCommitFailure;

  impl RuntimeAfterCommitEffect for ReactiveAfterCommitFailure {
    fn metadata(&self) -> RuntimeEffectMetadata {
      RuntimeEffectMetadata::new(
        RuntimeEffectSource::Custom {
          name: "reactive-after-commit-failure".to_string(),
        },
        "reactive-after-commit-failure",
      )
    }

    fn deliver(&mut self) -> MResult<()> {
      Err(MechError::new(
        GenericError {
          msg: "deliberate reactive delivery failure".to_string(),
        },
        None,
      ))
    }
  }

  impl MechFunctionImpl for ReactiveTransactionTestFunction {
    fn solve(&self) {}

    fn solve_result(&self) -> MResult<()> {
      self.solve_reactive().map(|_| ())
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
      let call = {
        let mut calls = self.calls.borrow_mut();
        *calls += 1;
        *calls
      };
      *self.output.borrow_mut() += 1;
      if self.fail_on_call == Some(call) {
        return Err(MechError::new(
          GenericError {
            msg: "deliberate reactive transaction failure".to_string(),
          },
          None,
        ));
      }
      Ok(ReactiveSolveStatus::Changed)
    }

    fn out(&self) -> Value {
      Value::Index(self.output.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
      Ok(vec![Value::Index(self.output.clone())])
    }

    fn to_string(&self) -> String {
      "ReactiveTransactionTestFunction".to_string()
    }
  }

  #[cfg(feature = "compiler")]
  impl MechFunctionCompiler for ReactiveTransactionTestFunction {
    fn compile(&self, _context: &mut CompileCtx) -> MResult<Register> {
      Ok(0)
    }
  }

  fn add_test_function(
    runtime: &mut MechRuntime,
    fail_on_call: Option<usize>,
  ) -> (Ref<usize>, Rc<RefCell<usize>>) {
    let output = Ref::new(0usize);
    let calls = Rc::new(RefCell::new(0usize));
    runtime
      .program
      .interpreter()
      .plan()
      .add_function(Box::new(ReactiveTransactionTestFunction {
        output: output.clone(),
        calls: calls.clone(),
        fail_on_call,
      }));
    (output, calls)
  }

  fn initialize_program_journal(
    runtime: &mut MechRuntime,
    journal: &mut ProgramReactiveTurnJournal,
  ) -> MResult<()> {
    runtime
      .program
      .step_with_reactive_turn_journal(0, journal)
  }

  #[test]
  fn implicit_reactive_turns_use_no_full_program_checkpoints() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    add_test_function(&mut runtime, None);
    reset_runtime_program_checkpoint_count();

    for _ in 0..100 {
      runtime.step(0).unwrap();
    }

    assert_eq!(runtime_program_checkpoint_count(), 0);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
  }

  #[test]
  fn reactive_step_budget_failure_cleans_up_without_poisoning() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, calls) = add_test_function(&mut runtime, None);
    let mut context = runtime
      .runtime_context()
      .unwrap()
      .with_budget(ResourceBudget::default().with_max_steps(0));
    reset_runtime_program_checkpoint_count();

    let error = runtime
      .step_with_context(&mut context, 0)
      .unwrap_err();

    assert_eq!(error.kind_name(), "ResourceBudgetExceeded");
    assert_eq!(*output.borrow(), 0);
    assert_eq!(*calls.borrow(), 0);
    assert!(!runtime.is_poisoned());
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert_eq!(runtime_program_checkpoint_count(), 0);
  }

  #[test]
  fn explicit_reactive_turns_reuse_one_outer_program_checkpoint() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    reset_runtime_program_checkpoint_count();

    runtime.step_with_context(&mut context, 0).unwrap();
    assert_eq!(runtime_program_checkpoint_count(), 1);
    for _ in 0..99 {
      runtime.step_with_context(&mut context, 0).unwrap();
    }

    assert_eq!(runtime_program_checkpoint_count(), 1);
    assert_eq!(
      runtime.program_transaction_owner,
      Some(transaction_id),
    );
    runtime
      .abort_runtime_transaction(&mut context, "checkpoint test")
      .unwrap();
  }

  #[test]
  fn failed_implicit_turn_restores_program_and_removes_envelope() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, calls) = add_test_function(&mut runtime, Some(1));
    reset_runtime_program_checkpoint_count();

    let error = runtime.step(1).unwrap_err();

    assert_eq!(error.kind_name(), "GenericError");
    assert_eq!(*output.borrow(), 0);
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(runtime_program_checkpoint_count(), 0);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
  }

  #[test]
  fn failed_explicit_turn_releases_or_preserves_ownership_by_position() {
    let mut first_runtime = MechRuntime::builder().build().unwrap();
    let (first_output, _) =
      add_test_function(&mut first_runtime, Some(1));
    let mut first_context = first_runtime.runtime_context().unwrap();
    let first_transaction =
      first_runtime.begin_transaction(&mut first_context).unwrap();

    assert!(first_runtime
      .step_with_context(&mut first_context, 1)
      .is_err());
    assert_eq!(*first_output.borrow(), 0);
    assert_eq!(first_runtime.program_transaction_owner, None);
    assert!(first_runtime
      .active_execution_transaction(first_transaction)
      .unwrap()
      .program
      .is_none());
    first_runtime
      .abort_runtime_transaction(&mut first_context, "first failure")
      .unwrap();

    let mut later_runtime = MechRuntime::builder().build().unwrap();
    let (later_output, _) =
      add_test_function(&mut later_runtime, Some(2));
    let mut later_context = later_runtime.runtime_context().unwrap();
    let later_transaction =
      later_runtime.begin_transaction(&mut later_context).unwrap();
    later_runtime
      .step_with_context(&mut later_context, 1)
      .unwrap();

    assert!(later_runtime
      .step_with_context(&mut later_context, 1)
      .is_err());
    assert_eq!(*later_output.borrow(), 1);
    assert_eq!(
      later_runtime.program_transaction_owner,
      Some(later_transaction),
    );
    assert!(later_runtime
      .active_execution_transaction(later_transaction)
      .unwrap()
      .program
      .is_some());
    later_runtime
      .abort_runtime_transaction(&mut later_context, "later failure")
      .unwrap();
    assert_eq!(*later_output.borrow(), 0);
  }

  #[test]
  fn explicit_program_owner_excludes_other_reactive_transactions() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context_a = runtime.runtime_context().unwrap();
    let transaction_a = runtime.begin_transaction(&mut context_a).unwrap();
    runtime.step_with_context(&mut context_a, 0).unwrap();

    let mut context_b = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context_b).unwrap();
    let error = runtime
      .step_with_context(&mut context_b, 0)
      .unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeProgramBusy");
    assert_eq!(*output.borrow(), 1);

    runtime
      .put_object_with_context(
        &mut context_b,
        ObjectRecord::text(
          ObjectId(699),
          "note",
          "independent store work",
        ),
      )
      .unwrap();
    runtime
      .run_string_with_context(
        &mut context_a,
        "reactive-owner-source := 1",
      )
      .unwrap();
    runtime.step_with_context(&mut context_a, 0).unwrap();
    assert_eq!(*output.borrow(), 2);
    assert_eq!(
      runtime.program_transaction_owner,
      Some(transaction_a),
    );

    runtime
      .abort_runtime_transaction(&mut context_a, "restore owner A")
      .unwrap();
    assert_eq!(*output.borrow(), 0);
    assert!(runtime
      .program
      .root_symbol_value("reactive-owner-source")
      .is_err());
    assert!(runtime.get_object(ObjectId(699)).unwrap().is_none());
    runtime
      .abort_runtime_transaction(&mut context_b, "discard B store work")
      .unwrap();
  }

  #[test]
  fn failed_first_explicit_turn_retains_owner_when_rollback_is_incomplete() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let error: MechError = runtime
      .with_atomic_reactive_turn(
        &mut context,
        "incomplete_first_explicit_turn_rollback",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.stage_runtime_effect_with_context(
            context,
            PreparedRuntimeEffect::Transactional(Box::new(
              ReactiveTransactionalProbe {
                log: log.clone(),
                fail_prepare: false,
                fail_commit: false,
                fail_abort: true,
              },
            )),
          )?;
          Err::<(), _>(MechError::new(
            GenericError {
              msg: "deliberate first explicit turn failure".to_string(),
            },
            None,
          ))
        },
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert!(format!("{error:?}")
      .contains("deliberate first explicit turn failure"));
    assert!(format!("{error:?}")
      .contains("deliberate reactive abort failure"));
    assert_eq!(*output.borrow(), 0);
    assert_eq!(
      runtime.program_transaction_owner,
      Some(transaction_id),
    );
    assert!(runtime
      .active_execution_transaction(transaction_id)
      .unwrap()
      .program
      .is_some());
    assert!(runtime.is_poisoned());
    assert_eq!(*log.lock().unwrap(), vec!["abort"]);
  }

  fn limited_live_capability(
    id: CapabilityId,
    subject: &str,
    max_uses: u64,
  ) -> Arc<dyn Capability> {
    Arc::new(
      BasicCapability::new(
        id,
        &BasicSubject::new(subject),
        &BasicResource::new("db://reactive"),
        [BasicOperation::read()],
      )
      .with_constraints(
        BasicConstraints::default().with_max_uses(max_uses),
      ),
    )
  }

  fn reactive_capability_request(subject: &str) -> CapabilityRequest {
    CapabilityRequest::new(
      &BasicSubject::new(subject),
      &BasicOperation::read(),
      &BasicResource::new("db://reactive"),
    )
  }

  #[test]
  fn implicit_reactive_capability_use_commits_or_rolls_back_once() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .build()
      .unwrap();
    add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let id = CapabilityId(700);
    runtime
      .grant_capability_with_context(
        &mut administrative,
        limited_live_capability(id, &subject, 2),
      )
      .unwrap();
    let request = reactive_capability_request(&subject);

    let mut failed_context = runtime.runtime_context().unwrap();
    let failed: MResult<()> = runtime.with_atomic_reactive_turn(
      &mut failed_context,
      "failed_capability_turn",
      |runtime, context, journal| {
        initialize_program_journal(runtime, journal)?;
        runtime.check_capability_with_context(context, &request)?;
        Err(MechError::new(
          GenericError {
            msg: "deliberate failed capability turn".to_string(),
          },
          None,
        ))
      },
    );
    assert_eq!(failed.unwrap_err().kind_name(), "GenericError");
    assert_eq!(observed.successful_uses_for_test(id), 0);

    let mut successful_context = runtime.runtime_context().unwrap();
    runtime
      .with_atomic_reactive_turn(
        &mut successful_context,
        "successful_capability_turn",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.check_capability_with_context(context, &request)?;
          Ok(())
        },
      )
      .unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 1);
  }

  #[test]
  fn explicit_reactive_capability_reservations_commit_or_abort() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .build()
      .unwrap();
    add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let id = CapabilityId(701);
    runtime
      .grant_capability_with_context(
        &mut administrative,
        limited_live_capability(id, &subject, 3),
      )
      .unwrap();
    let request = reactive_capability_request(&subject);

    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .with_atomic_reactive_turn(
        &mut context,
        "explicit_capability_turn",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.check_capability_with_context(context, &request)?;
          Ok(())
        },
      )
      .unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 0);
    assert_eq!(
      runtime
        .active_execution_transaction(transaction_id)
        .unwrap()
        .capabilities
        .usage_deltas()
        .collect::<Vec<_>>(),
      vec![(id, 1)],
    );

    let failed: MResult<()> = runtime.with_atomic_reactive_turn(
      &mut context,
      "failed_later_capability_turn",
      |runtime, context, journal| {
        initialize_program_journal(runtime, journal)?;
        runtime.check_capability_with_context(context, &request)?;
        Err(MechError::new(
          GenericError {
            msg: "deliberate later capability failure".to_string(),
          },
          None,
        ))
      },
    );
    assert_eq!(failed.unwrap_err().kind_name(), "GenericError");
    assert_eq!(
      runtime
        .active_execution_transaction(transaction_id)
        .unwrap()
        .capabilities
        .usage_deltas()
        .collect::<Vec<_>>(),
      vec![(id, 1)],
    );
    assert_eq!(observed.successful_uses_for_test(id), 0);

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 1);

    let mut abort_context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut abort_context).unwrap();
    runtime
      .with_atomic_reactive_turn(
        &mut abort_context,
        "aborted_capability_turn",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.check_capability_with_context(context, &request)?;
          Ok(())
        },
      )
      .unwrap();
    runtime
      .abort_runtime_transaction(&mut abort_context, "discard reservation")
      .unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 1);
  }

  #[test]
  fn retryable_store_failure_commits_reserved_use_without_rerun() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .build()
      .unwrap();
    let (_, calls) = add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let capability_id = CapabilityId(702);
    runtime
      .grant_capability_with_context(
        &mut administrative,
        limited_live_capability(
          capability_id,
          &subject,
          1,
        ),
      )
      .unwrap();
    let request = reactive_capability_request(&subject);
    let missing_object = ObjectId(703);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
      .with_atomic_reactive_turn(
        &mut context,
        "retryable_capability_turn",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.check_capability_with_context(context, &request)?;
          Ok(())
        },
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(
          missing_object,
          "note",
          "staged update",
        ),
      )
      .unwrap();

    assert!(runtime
      .commit_runtime_transaction(&mut context)
      .is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(observed.successful_uses_for_test(capability_id), 0);
    assert_eq!(
      runtime
        .active_execution_transaction(transaction_id)
        .unwrap()
        .capabilities
        .usage_deltas()
        .collect::<Vec<_>>(),
      vec![(capability_id, 1)],
    );

    runtime
      .put_object(ObjectRecord::text(
        missing_object,
        "note",
        "durable baseline",
      ))
      .unwrap();
    runtime
      .commit_runtime_transaction(&mut context)
      .unwrap();

    assert_eq!(*calls.borrow(), 1);
    assert_eq!(observed.successful_uses_for_test(capability_id), 1);
    assert_eq!(
      runtime
        .get_object(missing_object)
        .unwrap()
        .unwrap()
        .data,
      b"staged update".to_vec(),
    );
  }

  #[test]
  fn provisional_capability_grant_and_use_commit_together() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .build()
      .unwrap();
    add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let subject = context.subject.clone();
    let capability_id = CapabilityId(704);
    let request = reactive_capability_request(&subject);
    runtime.begin_transaction(&mut context).unwrap();

    runtime
      .with_atomic_reactive_turn(
        &mut context,
        "provisional_grant_and_use",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.grant_capability_with_context(
            context,
            limited_live_capability(
              capability_id,
              &subject,
              1,
            ),
          )?;
          runtime.check_capability_with_context(context, &request)?;
          Ok(())
        },
      )
      .unwrap();

    assert_eq!(observed.successful_uses_for_test(capability_id), 0);
    assert!(observed.get(capability_id).unwrap().is_none());
    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert!(observed.get(capability_id).unwrap().is_some());
    assert_eq!(observed.successful_uses_for_test(capability_id), 1);
  }

  #[test]
  fn live_capability_use_commits_before_transactional_revocation() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
      .capability_kernel(kernel)
      .build()
      .unwrap();
    add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let capability_id = CapabilityId(705);
    runtime
      .grant_capability_with_context(
        &mut administrative,
        limited_live_capability(
          capability_id,
          &subject,
          1,
        ),
      )
      .unwrap();
    let request = reactive_capability_request(&subject);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    runtime
      .with_atomic_reactive_turn(
        &mut context,
        "live_use_then_revoke",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.check_capability_with_context(context, &request)?;
          runtime.revoke_capability_with_context(
            context,
            capability_id,
          )?;
          Ok(())
        },
      )
      .unwrap();

    assert_eq!(observed.successful_uses_for_test(capability_id), 0);
    assert!(!observed.is_revoked(capability_id).unwrap());
    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(observed.successful_uses_for_test(capability_id), 1);
    assert!(observed.is_revoked(capability_id).unwrap());
  }

  #[test]
  fn failed_reactive_turn_rolls_back_staged_object_and_program_state() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let object_id = ObjectId(800);

    let result: MResult<()> = runtime.with_atomic_reactive_turn(
      &mut context,
      "failed_object_turn",
      |runtime, context, journal| {
        initialize_program_journal(runtime, journal)?;
        runtime.put_object_with_context(
          context,
          ObjectRecord::text(object_id, "note", "provisional"),
        )?;
        Err(MechError::new(
          GenericError {
            msg: "deliberate object turn failure".to_string(),
          },
          None,
        ))
      },
    );

    assert_eq!(result.unwrap_err().kind_name(), "GenericError");
    assert_eq!(*output.borrow(), 0);
    assert!(runtime.get_object(object_id).unwrap().is_none());
    assert!(runtime.active_transactions.is_empty());
  }

  #[test]
  fn reactive_host_callback_uses_scoped_transaction_services() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
      .grant_capability(Arc::new(BasicCapability::new(
        CapabilityId(920),
        &BasicSubject::new(&subject),
        &BasicResource::new("host:demo/reactive-reenter"),
        [BasicOperation::new("call")],
      )))
      .unwrap();
    runtime
      .register_mech_host_function(
        ClosureHostFunction::new_runtime_managed(
          "demo/reactive-reenter",
          move |services, context, _args| {
            let object = ObjectRecord::text(
              ObjectId(922),
              "note",
              "reactive staging",
            );
            if services
              .get_object_with_context(context, object.id)?
              .is_some()
            {
              services.update_object_with_context(
                context,
                object,
              )?;
            } else {
              services.put_object_with_context(
                context,
                object,
              )?;
            }
            Ok(Value::F64(Ref::new(1.0)))
          },
        ),
      )
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
      .run_string_with_context(
        &mut context,
        "reactive-result := demo/reactive-reenter()",
      )
      .unwrap();

    runtime.step_with_context(&mut context, 0).unwrap();

    assert!(runtime.get_object(ObjectId(922)).unwrap().is_some());
  }

  #[test]
  fn pre_store_effect_failure_rolls_back_reactive_and_runtime_state() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let object_id = ObjectId(930);

    let error = runtime
      .with_atomic_reactive_turn(
        &mut context,
        "reactive_prepare_failure",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.put_object_with_context(
            context,
            ObjectRecord::text(
              object_id,
              "note",
              "must roll back",
            ),
          )?;
          runtime.stage_runtime_effect_with_context(
            context,
            PreparedRuntimeEffect::Transactional(Box::new(
              ReactiveTransactionalProbe {
                log: log.clone(),
                fail_prepare: true,
                fail_commit: false,
                fail_abort: false,
              },
            )),
          )?;
          Ok(())
        },
      )
      .unwrap_err();

    assert!(format!("{error:?}")
      .contains("deliberate reactive prepare failure"));
    assert_eq!(*output.borrow(), 0);
    assert!(runtime.get_object(object_id).unwrap().is_none());
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
    assert_eq!(*log.lock().unwrap(), vec!["prepare", "abort"]);
  }

  #[test]
  fn post_store_participant_failure_never_rolls_back_reactive_state() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let object_id = ObjectId(931);

    let error = runtime
      .with_atomic_reactive_turn(
        &mut context,
        "reactive_commit_failure",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.put_object_with_context(
            context,
            ObjectRecord::text(
              object_id,
              "note",
              "must remain committed",
            ),
          )?;
          runtime.stage_runtime_effect_with_context(
            context,
            PreparedRuntimeEffect::Transactional(Box::new(
              ReactiveTransactionalProbe {
                log: log.clone(),
                fail_prepare: false,
                fail_commit: true,
                fail_abort: false,
              },
            )),
          )?;
          Ok(())
        },
      )
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate");
    assert_eq!(*output.borrow(), 1);
    assert!(runtime.get_object(object_id).unwrap().is_some());
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(runtime.is_poisoned());
    assert_eq!(*log.lock().unwrap(), vec!["prepare", "commit"]);
  }

  #[test]
  fn after_commit_delivery_failure_keeps_reactive_state_and_health() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();

    runtime
      .with_atomic_reactive_turn(
        &mut context,
        "reactive_delivery_failure",
        |runtime, context, journal| {
          initialize_program_journal(runtime, journal)?;
          runtime.stage_runtime_effect_with_context(
            context,
            PreparedRuntimeEffect::AfterCommit(Box::new(
              ReactiveAfterCommitFailure,
            )),
          )?;
          Ok(())
        },
      )
      .unwrap();

    assert_eq!(*output.borrow(), 1);
    assert!(!runtime.is_poisoned());
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
      matches!(
        &event.kind,
        RuntimeEventKind::EffectDeliveryFailed { message, .. }
          if message.contains("deliberate reactive delivery failure")
      )
    }));
  }
}
