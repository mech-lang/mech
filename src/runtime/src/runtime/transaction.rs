// ---------------------------------------------------------------------------
// Transaction methods
// ---------------------------------------------------------------------------

// These methods handle the creation, retrieval, and management of transactions within the runtime. Transactions are used to group a set of operations together, allowing for atomic commits or rollbacks in case of errors. The methods:

// - `commit_transaction`: Commits a transaction record to the store and emits a TransactionCommitted event.
// - `get_transaction`: Retrieves a transaction record by its ID.
// - `list_transactions`: Lists transaction records with an optional limit.
// - `append_event`: Appends a runtime event to the store and returns its ID.
// - `get_event`: Retrieves a runtime event by its ID.
// - `list_events`: Lists runtime events with an optional limit.
// - `begin_transaction`: Starts a new transaction in the context and emits a TransactionStarted event.
// - `commit_runtime_transaction`: Commits the active transaction in the context, applying all staged changes to the store, and emits a TransactionCommitted event.
// - `abort_runtime_transaction`: Aborts the active transaction in the context with a given reason and emits a TransactionAborted event.
// - `active_transaction_mut`: Retrieves a mutable reference to an active transaction by its ID.
// - `context_transaction_id`: Retrieves the active transaction ID from the context.

use super::*;
use crate::{
  AccessSet, CapabilityKernelCheckpoint, RuntimeCommitOutcome,
  RuntimeEffectFailure, RuntimeEffectFailurePhase,
  RuntimeStoreCommitIndeterminate,
};

pub(super) enum RuntimeCommitResolution {
  Committed(RuntimeCommitOutcome),
  CommittedWithError {
    transaction_id: TransactionId,
    error: MechError,
  },
}

fn module_journal_validation_error(
  record_type: &'static str,
  identity: impl Into<String>,
  reason: impl Into<String>,
) -> MechError {
  MechError::new(
    RuntimeModuleJournalConflict {
      record_type,
      identity: identity.into(),
      reason: reason.into(),
    },
    None,
  )
}

impl MechRuntime {

  fn validate_runtime_module_journal(
    &self,
    transaction_id: TransactionId,
  ) -> MResult<()> {
    let journal =
      &self.active_execution_transaction(transaction_id)?.modules;
    if journal.is_empty() {
      return Ok(());
    }

    for module in journal.module_puts() {
      module.validate()?;
      if module.id != module_id(&module.name) {
        return Err(module_journal_validation_error(
          "module",
          module.id.to_string(),
          "module ID does not match its canonical URI",
        ));
      }
      if let Some(existing) = self.store.get_module(module.id)? {
        if existing.name != module.name {
          return Err(module_journal_validation_error(
            "module",
            module.id.to_string(),
            "committed module ID maps to another canonical URI",
          ));
        }
      }
      if let Some(existing) =
        self.store.find_module_by_name(&module.name)?
      {
        if existing.id != module.id {
          return Err(module_journal_validation_error(
            "module.name",
            module.name.clone(),
            "committed canonical URI maps to another module ID",
          ));
        }
      }
    }

    for version in journal.version_puts() {
      version.validate()?;
      version.validate_import_edges()?;
      if let Some(existing) =
        self.store.get_module_version(version.id)?
      {
        if existing != *version {
          return Err(module_journal_validation_error(
            "module_version",
            version.id.to_string(),
            "committed version ID maps to different contents",
          ));
        }
      }
      if journal.get_module(version.module).is_none()
        && self.store.get_module(version.module)?.is_none()
      {
        return Err(module_journal_validation_error(
          "module_version.owner",
          version.module.to_string(),
          format!(
            "owner of version {} is not visible",
            version.id,
          ),
        ));
      }
      for dependency in &version.dependencies {
        if journal.get_version(*dependency).is_none()
          && self.store.get_module_version(*dependency)?.is_none()
        {
          return Err(module_journal_validation_error(
            "module_version.dependency",
            dependency.to_string(),
            format!(
              "dependency of version {} is not visible",
              version.id,
            ),
          ));
        }
      }
      for edge in &version.import_edges {
        if journal.get_version(edge.dependency).is_none()
          && self
            .store
            .get_module_version(edge.dependency)?
            .is_none()
        {
          return Err(module_journal_validation_error(
            "module_version.import_edge",
            edge.dependency.to_string(),
            format!(
              "import-edge target of version {} is not visible",
              version.id,
            ),
          ));
        }
      }
    }

    Ok(())
  }

  pub fn commit_transaction(
    &mut self,
    transaction: TransactionRecord,
  ) -> MResult<TransactionId> {
    let mut context = self.context_for_transaction(&transaction)?;
    self.commit_transaction_with_context(&mut context, transaction)
  }

  pub fn commit_transaction_with_context(
    &mut self,
    context: &mut RuntimeContext,
    transaction: TransactionRecord,
  ) -> MResult<TransactionId> {
    self.ensure_runtime_mutation_allowed(
      "commit_transaction_with_context",
    )?;
    self.validate_context_for_runtime(context)?;
    context.charge_step()?;

    let id = self.store.commit_transaction(transaction)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TransactionCommitted {
        transaction_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn get_transaction(
    &self,
    id: TransactionId,
  ) -> MResult<Option<TransactionRecord>> {
    self.store.get_transaction(id)
  }

  pub fn list_transactions(
    &self,
    limit: Option<usize>,
  ) -> MResult<Vec<TransactionRecord>> {
    self.store.list_transactions(limit)
  }

  pub fn append_event(&mut self, event: RuntimeEvent) -> MResult<EventId> {
    self.ensure_runtime_mutation_allowed("append_event")?;
    self.store.append_event(event)
  }

  pub fn get_event(&self, id: EventId) -> MResult<Option<RuntimeEvent>> {
    self.store.get_event(id)
  }

  pub fn list_events(&self, limit: Option<usize>) -> MResult<Vec<RuntimeEvent>> {
    self.store.list_events(limit)
  }

  pub fn begin_transaction(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<TransactionId> {
    self.ensure_runtime_mutation_allowed("begin_transaction")?;
    self.reject_program_operation_reentrancy("begin_transaction")?;
    self.begin_runtime_transaction_internal(
      context,
      RuntimeExecutionTransactionMode::Explicit,
    )
  }

  pub(super) fn begin_runtime_transaction_internal(
    &mut self,
    context: &mut RuntimeContext,
    mode: RuntimeExecutionTransactionMode,
  ) -> MResult<TransactionId> {
    self.ensure_runtime_mutation_allowed(
      "begin_runtime_transaction_internal",
    )?;
    self.validate_context_for_runtime(context)?;

    if context.transaction.is_some() {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "begin_transaction",
          reason: "context already has an active transaction".to_string(),
        },
        None,
      ));
    }

    let context_baseline = RuntimeContextCheckpoint::capture(context);
    let id = self.next_transaction_id();
    let transaction = RuntimeExecutionTransaction::new(
      RuntimeTransaction::new(id, context.subject.clone()),
      mode,
      RuntimeTransactionContextIdentity::capture(context),
      context_baseline.clone(),
    );
    self.active_transactions.insert(id, transaction);
    context.transaction = Some(id);

    let started_event = match self.emit_event_immediate_to_context(
      context,
      RuntimeEventKind::TransactionStarted {
        transaction_id: id,
      },
    ) {
      Ok(event) => event,
      Err(error) => {
        self.active_transactions.remove(&id);
        context_baseline.restore_preserving_consumption(context);
        return Err(error);
      }
    };

    if let Err(error) = self
      .active_transaction_mut(id)?
      .record_event(started_event)
    {
      self.active_transactions.remove(&id);
      context_baseline.restore_preserving_consumption(context);
      return Err(error);
    }

    Ok(id)
  }

  pub fn commit_runtime_transaction(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<TransactionId> {
    Ok(
      self
        .commit_runtime_transaction_detailed(context)?
        .transaction_id,
    )
  }

  pub fn commit_runtime_transaction_detailed(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<RuntimeCommitOutcome> {
    self.ensure_runtime_mutation_allowed("commit_runtime_transaction")?;
    self.reject_program_operation_reentrancy("commit_runtime_transaction")?;
    match self.commit_runtime_transaction_detailed_internal(context)? {
      RuntimeCommitResolution::Committed(outcome) => Ok(outcome),
      RuntimeCommitResolution::CommittedWithError {
        transaction_id,
        error,
      } => {
        let _ = transaction_id;
        Err(error)
      }
    }
  }

  pub(super) fn commit_runtime_transaction_internal(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<RuntimeCommitResolution> {
    self.commit_runtime_transaction_detailed_internal(context)
  }

  fn commit_runtime_transaction_detailed_internal(
    &mut self,
    context: &mut RuntimeContext,
  ) -> MResult<RuntimeCommitResolution> {
    self.validate_context_for_runtime(context)?;

    let transaction_id = Self::context_transaction_id(context)?;
    let (transaction_mode, has_program_baseline) = {
      let transaction =
        self.active_execution_transaction(transaction_id)?;
      (transaction.mode, transaction.program.is_some())
    };
    if has_program_baseline
      && self.program_transaction_owner != Some(transaction_id)
    {
      return self.coordinator_invariant_failure(
        "commit_runtime_transaction",
        Some(transaction_id),
        format!(
          "transaction {} contains a retained-program baseline but program ownership is {:?}",
          transaction_id,
          self.program_transaction_owner,
        ),
      );
    }
    #[cfg(feature = "invariant_define")]
    if transaction_mode
      == super::program_transaction::RuntimeExecutionTransactionMode::Explicit
      && self.program_transaction_owner == Some(transaction_id)
    {
      self.program.validate_integrity_constraints()?;
    }
    let access = self
      .active_execution_transaction(transaction_id)?
      .context_baseline
      .access_delta(context);

    let journal_failures = self
      .active_execution_transaction(transaction_id)?
      .effects
      .validate_active(transaction_id);
    if !journal_failures.is_empty() {
      return Err(self.poison_effect_cleanup(
        "commit_runtime_transaction",
        transaction_id,
        "effect journal invariant validation failed".to_string(),
        journal_failures,
      ));
    }

    self.validate_runtime_module_journal(transaction_id)?;

    {
      let transaction =
        self.active_execution_transaction_mut(transaction_id)?;
      if transaction.state != RuntimeExecutionTransactionState::Active {
        return Err(MechError::new(
          RuntimeInvalidOperationError {
            operation: "commit_runtime_transaction",
            reason: format!(
              "transaction {} is already committing",
              transaction_id,
            ),
          },
          None,
        ));
      }
      transaction.state = RuntimeExecutionTransactionState::Committing;
    }

    let mut envelope = self
      .active_transactions
      .remove(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })?;

    if envelope.effects.is_empty() && envelope.capabilities.is_empty() {
      let commit_event =
        self.make_event(RuntimeEventKind::TransactionCommitted {
          transaction_id,
        });
      let commit = match Self::build_runtime_store_commit(
        &mut envelope,
        &access,
        &commit_event,
      ) {
        Ok(commit) => commit,
        Err(error) => {
          envelope.state = RuntimeExecutionTransactionState::Active;
          self.active_transactions.insert(transaction_id, envelope);
          return Err(error);
        }
      };
      let id = match self.store.commit_runtime(commit) {
        Ok(id) => id,
        Err(error) => {
          if let Some(resolution) =
            self.resolve_indeterminate_store_commit(
              context,
              transaction_id,
              &error,
            )
          {
            return Ok(resolution);
          }
          envelope.state = RuntimeExecutionTransactionState::Active;
          self.active_transactions.insert(transaction_id, envelope);
          return Err(error);
        }
      };
      context.transaction = None;
      if self.program_transaction_owner == Some(transaction_id) {
        self.program_transaction_owner = None;
      }
      self.push_persisted_event_to_context(context, commit_event);
      return Ok(RuntimeCommitResolution::Committed(RuntimeCommitOutcome {
        transaction_id: id,
        delivery_failures: Vec::new(),
        audit_failures: Vec::new(),
      }));
    }

    let phase_guard = ScopedRuntimeState::enter(
      &self.active_effect_phase,
      ActiveRuntimeEffectPhase::Preparing,
    );
    let prepare_result = envelope.effects.prepare_transactional();
    drop(phase_guard);
    if let Err(step) = prepare_result {
      let original_error_text = format!("{:?}", step.error);
      let prepared_ids =
        envelope.effects.prepared_transactional_ids();
      let phase_guard = ScopedRuntimeState::enter(
        &self.active_effect_phase,
        ActiveRuntimeEffectPhase::Aborting,
      );
      let cleanup = envelope.effects.abort_prepared_reverse();
      drop(phase_guard);
      let failed_ids: HashSet<RuntimeEffectId> = cleanup
        .iter()
        .map(|failure| failure.effect_id)
        .collect();
      let _ = self.stage_effect_lifecycle_event(
        &mut envelope,
        context,
        RuntimeEventKind::EffectPreparationFailed {
          effect_id: step.failure.effect_id,
          message: step.failure.message.clone(),
        },
      );
      for effect_id in prepared_ids {
        if failed_ids.contains(&effect_id) {
          continue;
        }
        let _ = self.stage_effect_lifecycle_event(
          &mut envelope,
          context,
          RuntimeEventKind::EffectAborted { effect_id },
        );
      }
      envelope.state = RuntimeExecutionTransactionState::Active;
      self.active_transactions.insert(transaction_id, envelope);
      if cleanup.is_empty() {
        return Err(step.error);
      }
      return Err(self.poison_effect_cleanup(
        "commit_runtime_transaction",
        transaction_id,
        original_error_text,
        Self::describe_effect_failures(cleanup),
      ));
    }

    let capability_checkpoint = if envelope.capabilities.is_empty() {
      None
    } else {
      match self.capability_kernel.checkpoint() {
        Ok(checkpoint) => Some(checkpoint),
        Err(error) => {
          let original_error_text = format!("{:?}", error);
          let prepared_ids =
            envelope.effects.prepared_transactional_ids();
          let phase_guard = ScopedRuntimeState::enter(
            &self.active_effect_phase,
            ActiveRuntimeEffectPhase::Aborting,
          );
          let aborted = envelope.effects.abort_prepared_reverse();
          drop(phase_guard);
          let failed_ids: HashSet<RuntimeEffectId> = aborted
            .iter()
            .map(|failure| failure.effect_id)
            .collect();
          for effect_id in prepared_ids {
            if failed_ids.contains(&effect_id) {
              continue;
            }
            let _ = self.stage_effect_lifecycle_event(
              &mut envelope,
              context,
              RuntimeEventKind::EffectAborted { effect_id },
            );
          }
          envelope.state = RuntimeExecutionTransactionState::Active;
          self.active_transactions.insert(transaction_id, envelope);
          if aborted.is_empty() {
            return Err(error);
          }
          return Err(self.poison_effect_cleanup(
            "commit_runtime_transaction",
            transaction_id,
            original_error_text,
            Self::describe_effect_failures(aborted),
          ));
        }
      }
    };

    if let Err(error) = self.apply_capability_overlay(&envelope) {
      let original_error_text = format!("{:?}", error);
      let cleanup = self.cleanup_before_store_retry(
        &mut envelope,
        capability_checkpoint,
        context,
      );
      envelope.state = RuntimeExecutionTransactionState::Active;
      self.active_transactions.insert(transaction_id, envelope);
      if cleanup.is_empty() {
        return Err(error);
      }
      return Err(self.poison_effect_cleanup(
        "commit_runtime_transaction",
        transaction_id,
        original_error_text,
        cleanup,
      ));
    }

    let phase_guard = ScopedRuntimeState::enter(
      &self.active_effect_phase,
      ActiveRuntimeEffectPhase::Applying,
    );
    let apply_result = envelope.effects.apply_compensatable();
    drop(phase_guard);
    if let Err(step) = apply_result {
      let original_error_text = format!("{:?}", step.error);
      let cleanup = self.cleanup_before_store_retry(
        &mut envelope,
        capability_checkpoint,
        context,
      );
      envelope.state = RuntimeExecutionTransactionState::Active;
      self.active_transactions.insert(transaction_id, envelope);
      if cleanup.is_empty() {
        return Err(step.error);
      }
      return Err(self.poison_effect_cleanup(
        "commit_runtime_transaction",
        transaction_id,
        original_error_text,
        cleanup,
      ));
    }

    let commit_event = self.make_event(RuntimeEventKind::TransactionCommitted {
      transaction_id,
    });
    let commit = match Self::build_runtime_store_commit(
      &mut envelope,
      &access,
      &commit_event,
    ) {
      Ok(commit) => commit,
      Err(error) => {
        let original_error_text = format!("{:?}", error);
        let cleanup = self.cleanup_before_store_retry(
          &mut envelope,
          capability_checkpoint,
          context,
        );
        envelope.state = RuntimeExecutionTransactionState::Active;
        self.active_transactions.insert(transaction_id, envelope);
        if cleanup.is_empty() {
          return Err(error);
        }
        return Err(self.poison_effect_cleanup(
          "commit_runtime_transaction",
          transaction_id,
          original_error_text,
          cleanup,
        ));
      }
    };

    let id = match self.store.commit_runtime(commit) {
      Ok(id) => id,
      Err(error) => {
        if let Some(resolution) =
          self.resolve_indeterminate_store_commit(
            context,
            transaction_id,
            &error,
          )
        {
          return Ok(resolution);
        }
        let original_error_text = format!("{:?}", error);
        let cleanup = self.cleanup_before_store_retry(
          &mut envelope,
          capability_checkpoint,
          context,
        );
        envelope.state = RuntimeExecutionTransactionState::Active;
        self.active_transactions.insert(transaction_id, envelope);
        if cleanup.is_empty() {
          return Err(error);
        }
        return Err(self.poison_effect_cleanup(
          "commit_runtime_transaction",
          transaction_id,
          original_error_text,
          cleanup,
        ));
      }
    };

    let phase_guard = ScopedRuntimeState::enter(
      &self.active_effect_phase,
      ActiveRuntimeEffectPhase::Committing,
    );
    let commit_report = envelope.effects.commit_transactional();
    drop(phase_guard);

    context.transaction = None;
    if self.program_transaction_owner == Some(transaction_id) {
      self.program_transaction_owner = None;
    }
    self.push_persisted_event_to_context(context, commit_event);

    let mut audit_failures = Vec::new();
    for effect_id in commit_report.committed {
      if let Err(error) = self.emit_event_to_context(
        context,
        RuntimeEventKind::TransactionalEffectCommitted { effect_id },
      ) {
        audit_failures.push(RuntimeEffectFailure {
          effect_id,
          phase: RuntimeEffectFailurePhase::Audit,
          message: format!(
            "transactional effect commit audit failed: {:?}",
            error,
          ),
        });
      }
    }

    if !commit_report.failures.is_empty() {
      let failures: Vec<RuntimeEffectFailure> = commit_report
        .failures
        .into_iter()
        .map(|step| step.failure)
        .collect();
      let mut participant_outcomes = commit_report.participant_outcomes;
      for failure in &failures {
        if let Err(error) = self.emit_event_to_context(
          context,
          RuntimeEventKind::ExternalCommitIndeterminate {
            transaction_id: id,
            effect_id: failure.effect_id,
          },
        ) {
          participant_outcomes.push(format!(
            "external commit indeterminate audit for effect {} failed: {:?}",
            failure.effect_id,
            error,
          ));
        }
      }
      participant_outcomes.extend(
        audit_failures.iter().map(|failure| {
          format!(
            "effect {} audit failed: {}",
            failure.effect_id,
            failure.message,
          )
        }),
      );
      let error = self.poison_external_commit_indeterminate(
        id,
        failures,
        participant_outcomes,
      );
      return Ok(RuntimeCommitResolution::CommittedWithError {
        transaction_id: id,
        error,
      });
    }

    let after_commit_ids = envelope.effects.after_commit_ids();
    let phase_guard = ScopedRuntimeState::enter(
      &self.active_effect_phase,
      ActiveRuntimeEffectPhase::Delivering,
    );
    let delivery_failures = envelope.effects.deliver_after_commit();
    drop(phase_guard);
    for effect_id in after_commit_ids {
      let kind = match delivery_failures
        .iter()
        .find(|failure| failure.effect_id == effect_id)
      {
        Some(failure) => RuntimeEventKind::EffectDeliveryFailed {
          effect_id,
          message: failure.message.clone(),
        },
        None => RuntimeEventKind::EffectDelivered { effect_id },
      };
      if let Err(error) = self.emit_event_to_context(context, kind) {
        audit_failures.push(RuntimeEffectFailure {
          effect_id,
          phase: RuntimeEffectFailurePhase::Audit,
          message: format!("delivery audit event failed: {:?}", error),
        });
      }
    }

    Ok(RuntimeCommitResolution::Committed(RuntimeCommitOutcome {
      transaction_id: id,
      delivery_failures,
      audit_failures,
    }))
  }

  fn resolve_indeterminate_store_commit(
    &mut self,
    context: &mut RuntimeContext,
    transaction_id: TransactionId,
    error: &MechError,
  ) -> Option<RuntimeCommitResolution> {
    error
      .kind_as::<RuntimeStoreCommitIndeterminate>()?;
    context.transaction = None;
    if self.program_transaction_owner == Some(transaction_id) {
      self.program_transaction_owner = None;
    }
    self.health = RuntimeHealth::Poisoned(RuntimePoisonRecord {
      operation: "commit_runtime_transaction".to_string(),
      transaction_id: Some(transaction_id),
      original_error: format!("{error:?}"),
      rollback_failures: Vec::new(),
    });
    Some(RuntimeCommitResolution::CommittedWithError {
      transaction_id,
      error: error.clone(),
    })
  }

  fn cleanup_before_store_retry(
    &mut self,
    envelope: &mut RuntimeExecutionTransaction,
    capability_checkpoint: Option<Box<dyn CapabilityKernelCheckpoint>>,
    context: &mut RuntimeContext,
  ) -> Vec<String> {
    let compensated_ids =
      envelope.effects.applied_compensatable_ids();
    let compensated = {
      let _phase_guard = ScopedRuntimeState::enter(
        &self.active_effect_phase,
        ActiveRuntimeEffectPhase::Compensating,
      );
      envelope.effects.compensate_applied_reverse()
    };
    let capability_restore = capability_checkpoint
      .map(|checkpoint| {
        self.capability_kernel.restore(checkpoint)
      });
    let aborted_ids = envelope.effects.prepared_transactional_ids();
    let aborted = {
      let _phase_guard = ScopedRuntimeState::enter(
        &self.active_effect_phase,
        ActiveRuntimeEffectPhase::Aborting,
      );
      envelope.effects.abort_prepared_reverse()
    };

    let failed_compensations: HashSet<RuntimeEffectId> = compensated
      .iter()
      .map(|failure| failure.effect_id)
      .collect();
    let failed_aborts: HashSet<RuntimeEffectId> = aborted
      .iter()
      .map(|failure| failure.effect_id)
      .collect();
    for failure in &compensated {
      let _ = self.emit_effect_event_outside_transaction(
        context,
        RuntimeEventKind::EffectCompensationFailed {
          effect_id: failure.effect_id,
          message: failure.message.clone(),
        },
      );
    }
    let mut failures = Self::describe_effect_failures(compensated);
    if let Some(Err(error)) = capability_restore {
      failures.push(format!(
        "capability kernel checkpoint restore failed: {:?}",
        error,
      ));
    }
    failures.extend(Self::describe_effect_failures(aborted));
    for effect_id in compensated_ids {
      if failed_compensations.contains(&effect_id) {
        continue;
      }
      let _ = self.emit_effect_event_outside_transaction(
        context,
        RuntimeEventKind::EffectCompensated { effect_id },
      );
    }
    for effect_id in aborted_ids {
      if failed_aborts.contains(&effect_id) {
        continue;
      }
      let _ = self.stage_effect_lifecycle_event(
        envelope,
        context,
        RuntimeEventKind::EffectAborted { effect_id },
      );
    }
    failures
  }

  fn stage_effect_lifecycle_event(
    &mut self,
    envelope: &mut RuntimeExecutionTransaction,
    context: &mut RuntimeContext,
    kind: RuntimeEventKind,
  ) -> MResult<EventId> {
    let context_events_before = context.events.clone();
    let event = self.make_event(kind);
    let id = event.id;
    context.push_event(event.clone());
    self.trim_events_to_retention(&mut context.events);
    if let Err(error) = envelope.store.stage_event(event) {
      context.events = context_events_before;
      return Err(error);
    }
    Ok(id)
  }

  pub(super) fn emit_effect_event_outside_transaction(
    &mut self,
    context: &mut RuntimeContext,
    kind: RuntimeEventKind,
  ) -> MResult<EventId> {
    let context_events_before = context.events.clone();
    let event = self.make_event(kind);
    let id = event.id;
    context.push_event(event.clone());
    self.trim_events_to_retention(&mut context.events);
    if let Err(error) = self.store.append_event(event) {
      context.events = context_events_before;
      return Err(error);
    }
    Ok(id)
  }

  fn apply_capability_overlay(
    &mut self,
    envelope: &RuntimeExecutionTransaction,
  ) -> MResult<()> {
    for (_, capability) in envelope.capabilities.grants() {
      self
        .capability_kernel
        .grant(CapabilityGrant::new(capability))?;
    }
    for (capability, uses) in envelope.capabilities.usage_deltas() {
      self
        .capability_kernel
        .apply_usage_delta(capability, uses)?;
    }
    for capability in envelope.capabilities.revocations() {
      self
        .capability_kernel
        .revoke(CapabilityRevocation::new(capability))?;
    }
    Ok(())
  }

  fn build_runtime_store_commit(
    envelope: &mut RuntimeExecutionTransaction,
    access: &AccessSet,
    commit_event: &RuntimeEvent,
  ) -> MResult<RuntimeStoreCommit> {
    let transaction = &mut envelope.store;

    transaction.merge_read_set(&access.reads)?;
    transaction.merge_write_set(&access.writes)?;

    let staged_puts: Vec<ObjectRecord> =
      transaction.staged_puts().cloned().collect();
    let staged_updates: Vec<ObjectRecord> =
      transaction.staged_updates().cloned().collect();
    let staged_task_updates: Vec<TaskRecord> =
      transaction.staged_task_updates().cloned().collect();
    let staged_actor_updates: Vec<ActorRecord> =
      transaction.staged_actor_updates().cloned().collect();
    let staged_message_acks: Vec<(ActorId, MessageId)> = transaction
      .staged_message_acks()
      .flat_map(|(actor, messages)| {
        messages.iter().map(move |message| (*actor, *message))
      })
      .collect();
    let staged_message_enqueues: Vec<(ActorId, MessageRecord)> = transaction
      .staged_message_enqueues()
      .flat_map(|(actor, messages)| {
        messages.iter().cloned().map(move |message| (*actor, message))
      })
      .collect();

    let mut staged_events: Vec<RuntimeEvent> =
      transaction.staged_events().cloned().collect();
    staged_events.push(commit_event.clone());

    let mut transaction_snapshot = transaction.clone();
    transaction_snapshot.record_event(commit_event.id)?;
    let transaction_record = transaction_snapshot
      .into_record()?
      .with_effects(envelope.effects.records()?);

    Ok(RuntimeStoreCommit {
      transaction: transaction_record,
      module_puts: envelope.modules.module_puts().cloned().collect(),
      module_version_puts: envelope
        .modules
        .version_puts()
        .cloned()
        .collect(),
      capability_grants: envelope.capabilities.grants().collect(),
      capability_revocations: envelope
        .capabilities
        .revocations()
        .collect(),
      object_puts: staged_puts,
      object_updates: staged_updates,
      task_updates: staged_task_updates,
      actor_updates: staged_actor_updates,
      message_acks: staged_message_acks,
      message_enqueues: staged_message_enqueues,
      events: staged_events,
    })
  }

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

  pub(super) fn abort_runtime_transaction_internal(
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

  pub(super) fn abort_runtime_transaction_cleanup(
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

  pub(super) fn active_transaction_mut(
    &mut self,
    transaction_id: TransactionId,
  ) -> MResult<&mut RuntimeTransaction> {
    Ok(&mut self.active_execution_transaction_mut(transaction_id)?.store)
  }

  pub(super) fn active_execution_transaction(
    &self,
    transaction_id: TransactionId,
  ) -> MResult<&RuntimeExecutionTransaction> {
    self
      .active_transactions
      .get(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })
  }

  pub(super) fn active_execution_transaction_mut(
    &mut self,
    transaction_id: TransactionId,
  ) -> MResult<&mut RuntimeExecutionTransaction> {
    self
      .active_transactions
      .get_mut(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError { transaction_id },
          None,
        )
      })
  }

  pub(super) fn context_transaction_id(context: &RuntimeContext) -> MResult<TransactionId> {
    context.transaction.ok_or_else(|| {
      MechError::new(
        RuntimeInvalidOperationError {
          operation: "context_transaction_id",
          reason: "context has no active transaction".to_string(),
        },
        None,
      )
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::PreparedRuntimeEffect;
  use crate::runtime::test_support::{
    effects::{EffectLifecycleLog, TransactionalEffectProbe},
    events::event_count,
  };

  fn new_runtime() -> MechRuntime {
    MechRuntime::builder().build().unwrap()
  }

  #[test]
  fn transaction_commit_failure_is_atomic() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    runtime.begin_transaction(&mut context).unwrap();
    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "hello"),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(200), "note", "missing"),
      )
      .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());

    assert!(runtime.get_object(ObjectId(100)).unwrap().is_none());
    assert!(runtime.get_object(ObjectId(200)).unwrap().is_none());
    assert!(runtime.get_transaction(TransactionId(1)).unwrap().is_none());

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
      event_count(
        &events,
        |kind| kind == &RuntimeEventKind::ObjectCreated {
          object_id: ObjectId(100),
        },
      ),
      0,
    );
    assert_eq!(
      event_count(
        &events,
        |kind| kind == &RuntimeEventKind::ObjectUpdated {
          object_id: ObjectId(200),
        },
      ),
      0,
    );
  }

  #[test]
  fn module_journal_validation_precedes_effect_preparation() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id =
      runtime.begin_transaction(&mut context).unwrap();
    runtime
      .active_execution_transaction_mut(transaction_id)
      .unwrap()
      .modules
      .stage_version(ModuleVersionRecord::new(
        ModuleVersionId(10),
        module_id("memory://missing.mec"),
        1,
      ))
      .unwrap();
    let lifecycle = EffectLifecycleLog::default();
    runtime
      .active_execution_transaction_mut(transaction_id)
      .unwrap()
      .effects
      .stage(
        transaction_id,
        PreparedRuntimeEffect::Transactional(Box::new(
          TransactionalEffectProbe::new(
            "module-validation-probe",
            lifecycle.clone(),
          ),
        )),
      );

    let error =
      runtime.commit_runtime_transaction(&mut context).unwrap_err();

    assert!(
      error.kind_as::<RuntimeModuleJournalConflict>().is_some(),
    );
    assert!(lifecycle.observations().is_empty());
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(matches!(runtime.health(), RuntimeHealth::Healthy));
  }

  #[test]
  fn transaction_commit_failure_keeps_transaction_active() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "hello"),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(200), "note", "missing"),
      )
      .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
      .abort_runtime_transaction(&mut context, "failed commit")
      .unwrap();
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn transaction_abort_discards_staged_events() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "hello"),
      )
      .unwrap();

    let staged_event_id = context
      .events
      .iter()
      .find(|event| {
        event.kind == (RuntimeEventKind::ObjectCreated {
          object_id: ObjectId(100),
        })
      })
      .map(|event| event.id)
      .unwrap();

    runtime
      .abort_runtime_transaction(&mut context, "abort")
      .unwrap();

    assert!(!context.events.iter().any(|event| event.id == staged_event_id));
    assert!(runtime.get_event(staged_event_id).unwrap().is_none());
    assert!(runtime.get_object(ObjectId(100)).unwrap().is_none());
    assert!(runtime.get_transaction(transaction_id).unwrap().is_none());

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
      event_count(
        &events,
        |kind| kind == &RuntimeEventKind::TransactionStarted { transaction_id },
      ),
      1,
    );
    assert_eq!(
      event_count(
        &events,
        |kind| kind == &RuntimeEventKind::TransactionAborted {
          transaction_id,
          message: "abort".to_string(),
        },
      ),
      1,
    );
  }

  #[test]
  fn transaction_commit_persists_staged_events_once() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let started_id = context
      .events
      .iter()
      .find(|event| {
        event.kind == (RuntimeEventKind::TransactionStarted { transaction_id })
      })
      .map(|event| event.id)
      .unwrap();

    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "hello"),
      )
      .unwrap();
    runtime
      .update_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(100), "note", "updated"),
      )
      .unwrap();

    let staged_event_ids: Vec<EventId> = context
      .events
      .iter()
      .filter(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::ObjectCreated { .. }
            | RuntimeEventKind::ObjectUpdated { .. }
        )
      })
      .map(|event| event.id)
      .collect();

    assert_eq!(
      runtime.commit_runtime_transaction(&mut context).unwrap(),
      transaction_id,
    );

    let object = runtime.get_object(ObjectId(100)).unwrap().unwrap();
    assert_eq!(object.data, b"updated");

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
      event_count(
        &events,
        |kind| kind == &RuntimeEventKind::ObjectCreated {
          object_id: ObjectId(100),
        },
      ),
      1,
    );
    assert_eq!(
      event_count(
        &events,
        |kind| kind == &RuntimeEventKind::ObjectUpdated {
          object_id: ObjectId(100),
        },
      ),
      1,
    );
    assert_eq!(
      event_count(
        &events,
        |kind| kind == &RuntimeEventKind::TransactionCommitted { transaction_id },
      ),
      1,
    );
    let commit_event_id = context
      .events
      .iter()
      .find(|event| {
        event.kind == (RuntimeEventKind::TransactionCommitted { transaction_id })
      })
      .map(|event| event.id)
      .unwrap();
    assert_eq!(
      events
        .iter()
        .filter(|event| event.id == commit_event_id)
        .count(),
      1,
    );

    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    assert!(record.events.contains(&started_id));
    assert!(record.events.contains(&commit_event_id));
    for event_id in &staged_event_ids {
      assert!(record.events.contains(event_id));
      assert_eq!(
        events.iter().filter(|event| event.id == *event_id).count(),
        1,
      );
    }

    let mut unique = record.events.clone();
    unique.sort_by_key(|id| id.as_u128());
    unique.dedup();
    assert_eq!(unique.len(), record.events.len());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, None);
  }
  #[test]
  fn rejects_foreign_runtime_context_before_object_write_and_events() {
    let runtime_a = new_runtime();
    let mut runtime_b = new_runtime();
    let mut context = runtime_a.runtime_context().unwrap();
    let events_before = runtime_b.list_events(None).unwrap();

    assert!(runtime_b
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(900), "note", "foreign"),
      )
      .is_err());

    assert!(runtime_b.get_object(ObjectId(900)).unwrap().is_none());
    assert_eq!(runtime_b.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
  }

  #[test]
  fn nonexistent_transaction_context_does_not_fall_through_to_durable_writes() {
    let mut runtime = new_runtime();
    runtime.put_actor(ActorRecord::new(ActorId(1), "actor:1")).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.transaction = Some(TransactionId(404));
    let events_before = runtime.list_events(None).unwrap();

    assert!(runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(901), "note", "missing-tx"),
      )
      .is_err());
    assert!(runtime
      .send_message_with_context(&mut context, ActorId(1), "ping", b"missing-tx".to_vec())
      .is_err());

    assert!(runtime.get_object(ObjectId(901)).unwrap().is_none());
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
  }

  #[test]
  fn transaction_subject_mismatch_cannot_stage_commit_or_abort_owner_can_finish() {
    let mut runtime = new_runtime();
    runtime.put_actor(ActorRecord::new(ActorId(1), "owner")).unwrap();
    let mut owner_context = runtime.runtime_context().unwrap();
    owner_context.subject = "owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut owner_context).unwrap();
    let events_after_begin = runtime.list_events(None).unwrap();

    let mut other_context = runtime.runtime_context().unwrap();
    other_context.subject = "other".to_string();
    other_context.transaction = Some(transaction_id);

    assert!(runtime
      .put_object_with_context(
        &mut other_context,
        ObjectRecord::text(ObjectId(902), "note", "wrong-owner"),
      )
      .is_err());
    assert!(runtime
      .send_message_with_context(&mut other_context, ActorId(1), "ping", b"wrong-owner".to_vec())
      .is_err());
    assert!(runtime.commit_runtime_transaction(&mut other_context).is_err());
    assert!(runtime.abort_runtime_transaction(&mut other_context, "wrong-owner").is_err());

    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.get_object(ObjectId(902)).unwrap().is_none());
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_after_begin);
    assert!(other_context.events.is_empty());

    assert_eq!(runtime.commit_runtime_transaction(&mut owner_context).unwrap(), transaction_id);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
  }

  #[test]
  fn stale_aborted_transaction_context_is_rejected_not_durable() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let mut stale_context = context.clone();
    runtime.abort_runtime_transaction(&mut context, "rollback").unwrap();
    let events_after_abort = runtime.list_events(None).unwrap();

    assert!(runtime
      .put_object_with_context(
        &mut stale_context,
        ObjectRecord::text(ObjectId(903), "note", "stale"),
      )
      .is_err());

    assert_eq!(stale_context.transaction, Some(transaction_id));
    assert!(runtime.get_object(ObjectId(903)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_after_abort);
  }

  #[test]
  fn foreign_context_rejected_before_host_and_capability_boundaries() {
    let runtime_a = new_runtime();
    let mut runtime_b = new_runtime();
    let mut context = runtime_a.runtime_context().unwrap();
    let events_before = runtime_b.list_events(None).unwrap();

    assert!(runtime_b
      .call_host_with_context(&mut context, HostCall::new("missing/host", Vec::new()))
      .is_err());
    assert!(runtime_b
      .check_capability_with_context(
        &mut context,
        &CapabilityRequest::from_keys("subject", "op", "resource"),
      )
      .is_err());

    assert_eq!(runtime_b.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
  }

  #[test]
  fn historical_transaction_record_context_is_valid_without_active_transaction() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    context.subject = "historical-owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    let mut record_context = runtime.context_for_transaction(&record).unwrap();

    assert_eq!(record_context.runtime, runtime.id);
    assert_eq!(record_context.subject, record.subject);
    assert_eq!(record_context.transaction, None);
    runtime
      .put_object_with_context(
        &mut record_context,
        ObjectRecord::text(ObjectId(905), "note", "historical"),
      )
      .unwrap();
    assert!(runtime.get_object(ObjectId(905)).unwrap().is_some());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.get_transaction(transaction_id).unwrap().is_some());
  }

  #[test]
  fn active_transaction_must_continue_with_original_context() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    context.subject = "owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .put_object_with_context(
        &mut context,
        ObjectRecord::text(ObjectId(906), "note", "staged"),
      )
      .unwrap();
    assert!(runtime.get_object(ObjectId(906)).unwrap().is_none());
    assert_eq!(
      runtime.commit_runtime_transaction(&mut context).unwrap(),
      transaction_id,
    );
    assert!(runtime.get_object(ObjectId(906)).unwrap().is_some());
  }

  #[test]
  fn store_read_panic_is_converted_and_runtime_recovers() {
    let mut store = InMemoryStore::new();
    store.panic_on_get_object_for_test();
    let mut runtime = MechRuntime::builder()
      .store(store)
      .build()
      .unwrap();

    let error = runtime.get_object(ObjectId(1)).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate store read panic"));
    assert!(!runtime.is_poisoned());
    runtime.run_string("store-read-recovery := 1.0").unwrap();
  }

  #[test]
  fn store_commit_panic_is_indeterminate_and_never_rolled_back() {
    let mut store = InMemoryStore::new();
    store.panic_on_commit_runtime_for_test();
    let mut runtime = MechRuntime::builder()
      .store(store)
      .build()
      .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
      .run_string_with_context(
        &mut context,
        "store-commit-panic-state := 42.0",
      )
      .unwrap();

    let error = runtime
      .commit_runtime_transaction_detailed(&mut context)
      .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeStoreCommitIndeterminate");
    assert!(format!("{error:?}").contains("deliberate store commit panic"));
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    let retained = runtime
      .root_symbol_value("store-commit-panic-state")
      .unwrap();
    match retained.as_value() {
      Value::F64(value) => assert_eq!(*value.borrow(), 42.0),
      other => panic!("expected retained f64 value, got {other:?}"),
    }
  }

}
