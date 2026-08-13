//! Runtime transaction commit and durable-publication protocol.

use super::{
    RuntimeContextCheckpoint, RuntimeExecutionTransaction, RuntimeExecutionTransactionMode,
    RuntimeExecutionTransactionState, RuntimeHealth, RuntimePoisonRecord,
    RuntimeTransactionContextIdentity,
};
use crate::runtime::MechRuntime;
use crate::runtime::state::ScopedRuntimeState;
use crate::{
    AccessSet, ActiveRuntimeEffectPhase, ActorId, ActorRecord, CapabilityGrant,
    CapabilityKernelCheckpoint, CapabilityRevocation, EventId, MessageId, MessageRecord,
    ObjectRecord, RuntimeCommitOutcome, RuntimeContext, RuntimeEffectFailure,
    RuntimeEffectFailurePhase, RuntimeEffectId, RuntimeEvent, RuntimeEventKind,
    RuntimeInvalidOperationError, RuntimeModuleJournalConflict, RuntimeStoreCommit,
    RuntimeStoreCommitIndeterminate, RuntimeTransaction, RuntimeTransactionNotFoundError,
    TaskRecord, TransactionId, TransactionRecord, module_id,
};
use mech_core::{MResult, MechError};
use std::collections::HashSet;

pub(in crate::runtime) enum RuntimeCommitResolution {
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
    fn validate_runtime_module_journal(&self, transaction_id: TransactionId) -> MResult<()> {
        let journal = &self.active_execution_transaction(transaction_id)?.modules;
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
            if let Some(existing) = self.store.find_module_by_name(&module.name)? {
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
            if let Some(existing) = self.store.get_module_version(version.id)? {
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
                    format!("owner of version {} is not visible", version.id,),
                ));
            }
            for dependency in &version.dependencies {
                if journal.get_version(*dependency).is_none()
                    && self.store.get_module_version(*dependency)?.is_none()
                {
                    return Err(module_journal_validation_error(
                        "module_version.dependency",
                        dependency.to_string(),
                        format!("dependency of version {} is not visible", version.id,),
                    ));
                }
            }
            for edge in &version.import_edges {
                if journal.get_version(edge.dependency).is_none()
                    && self.store.get_module_version(edge.dependency)?.is_none()
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

    pub fn commit_transaction(&mut self, transaction: TransactionRecord) -> MResult<TransactionId> {
        let mut context = self.context_for_transaction(&transaction)?;
        self.commit_transaction_with_context(&mut context, transaction)
    }

    pub fn commit_transaction_with_context(
        &mut self,
        context: &mut RuntimeContext,
        transaction: TransactionRecord,
    ) -> MResult<TransactionId> {
        self.ensure_runtime_mutation_allowed("commit_transaction_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;

        let id = self.store.commit_transaction(transaction)?;

        self.emit_event_to_context(
            context,
            RuntimeEventKind::TransactionCommitted { transaction_id: id },
        )?;

        Ok(id)
    }

    pub fn get_transaction(&self, id: TransactionId) -> MResult<Option<TransactionRecord>> {
        self.store.get_transaction(id)
    }

    pub fn list_transactions(&self, limit: Option<usize>) -> MResult<Vec<TransactionRecord>> {
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

    pub fn begin_transaction(&mut self, context: &mut RuntimeContext) -> MResult<TransactionId> {
        self.ensure_runtime_mutation_allowed("begin_transaction")?;
        self.reject_program_operation_reentrancy("begin_transaction")?;
        self.begin_runtime_transaction_internal(context, RuntimeExecutionTransactionMode::Explicit)
    }

    pub(in crate::runtime) fn begin_runtime_transaction_internal(
        &mut self,
        context: &mut RuntimeContext,
        mode: RuntimeExecutionTransactionMode,
    ) -> MResult<TransactionId> {
        self.ensure_runtime_mutation_allowed("begin_runtime_transaction_internal")?;
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
            RuntimeEventKind::TransactionStarted { transaction_id: id },
        ) {
            Ok(event) => event,
            Err(error) => {
                self.active_transactions.remove(&id);
                context_baseline.restore_preserving_consumption(context)?;
                context.events.finish_transaction_scope()?;
                return Err(error);
            }
        };

        if let Err(error) = self.active_transaction_mut(id)?.record_event(started_event) {
            self.active_transactions.remove(&id);
            context_baseline.restore_preserving_consumption(context)?;
            context.events.finish_transaction_scope()?;
            return Err(error);
        }

        Ok(id)
    }

    pub fn commit_runtime_transaction(
        &mut self,
        context: &mut RuntimeContext,
    ) -> MResult<TransactionId> {
        Ok(self
            .commit_runtime_transaction_detailed(context)?
            .transaction_id)
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

    pub(in crate::runtime) fn commit_runtime_transaction_internal(
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
        let has_program_baseline = self
            .active_execution_transaction(transaction_id)?
            .program
            .is_some();
        #[cfg(feature = "invariant_define")]
        let transaction_mode = self.active_execution_transaction(transaction_id)?.mode;
        if has_program_baseline && self.program_transaction_owner != Some(transaction_id) {
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
        if transaction_mode == RuntimeExecutionTransactionMode::Explicit
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
            let transaction = self.active_execution_transaction_mut(transaction_id)?;
            if transaction.state != RuntimeExecutionTransactionState::Active {
                return Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "commit_runtime_transaction",
                        reason: format!("transaction {} is already committing", transaction_id,),
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
                MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None)
            })?;

        if envelope.effects.is_empty() && envelope.capabilities.is_empty() {
            let commit_event =
                self.make_event(RuntimeEventKind::TransactionCommitted { transaction_id });
            let commit =
                match Self::build_runtime_store_commit(&mut envelope, &access, &commit_event) {
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
                        self.resolve_indeterminate_store_commit(context, transaction_id, &error)
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
            #[cfg(feature = "resident-routing")]
            if envelope.claims_legacy_program_owner {
                self.publish_legacy_program_owner();
            }
            self.push_persisted_event_to_context(context, commit_event)?;
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
            let prepared_ids = envelope.effects.prepared_transactional_ids();
            let phase_guard = ScopedRuntimeState::enter(
                &self.active_effect_phase,
                ActiveRuntimeEffectPhase::Aborting,
            );
            let cleanup = envelope.effects.abort_prepared_reverse();
            drop(phase_guard);
            let failed_ids: HashSet<RuntimeEffectId> =
                cleanup.iter().map(|failure| failure.effect_id).collect();
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
                    let prepared_ids = envelope.effects.prepared_transactional_ids();
                    let phase_guard = ScopedRuntimeState::enter(
                        &self.active_effect_phase,
                        ActiveRuntimeEffectPhase::Aborting,
                    );
                    let aborted = envelope.effects.abort_prepared_reverse();
                    drop(phase_guard);
                    let failed_ids: HashSet<RuntimeEffectId> =
                        aborted.iter().map(|failure| failure.effect_id).collect();
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
            let cleanup =
                self.cleanup_before_store_retry(&mut envelope, capability_checkpoint, context);
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
            let cleanup =
                self.cleanup_before_store_retry(&mut envelope, capability_checkpoint, context);
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

        let commit_event =
            self.make_event(RuntimeEventKind::TransactionCommitted { transaction_id });
        let commit = match Self::build_runtime_store_commit(&mut envelope, &access, &commit_event) {
            Ok(commit) => commit,
            Err(error) => {
                let original_error_text = format!("{:?}", error);
                let cleanup =
                    self.cleanup_before_store_retry(&mut envelope, capability_checkpoint, context);
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
                    self.resolve_indeterminate_store_commit(context, transaction_id, &error)
                {
                    return Ok(resolution);
                }
                let original_error_text = format!("{:?}", error);
                let cleanup =
                    self.cleanup_before_store_retry(&mut envelope, capability_checkpoint, context);
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
        #[cfg(feature = "resident-routing")]
        if envelope.claims_legacy_program_owner {
            self.publish_legacy_program_owner();
        }
        self.push_persisted_event_to_context(context, commit_event)?;

        let mut audit_failures = Vec::new();
        for effect_id in commit_report.committed {
            if let Err(error) = self.emit_event_to_context(
                context,
                RuntimeEventKind::TransactionalEffectCommitted { effect_id },
            ) {
                audit_failures.push(RuntimeEffectFailure {
                    effect_id,
                    phase: RuntimeEffectFailurePhase::Audit,
                    message: format!("transactional effect commit audit failed: {:?}", error,),
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
                        failure.effect_id, error,
                    ));
                }
            }
            participant_outcomes.extend(audit_failures.iter().map(|failure| {
                format!(
                    "effect {} audit failed: {}",
                    failure.effect_id, failure.message,
                )
            }));
            let error =
                self.poison_external_commit_indeterminate(id, failures, participant_outcomes);
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
        error.kind_as::<RuntimeStoreCommitIndeterminate>()?;
        context.transaction = None;
        if self.program_transaction_owner == Some(transaction_id) {
            self.program_transaction_owner = None;
        }
        let mut rollback_failures = Vec::new();
        if let Err(compaction_error) = context.finish_event_transaction_scope() {
            rollback_failures.push(format!(
                "context event compaction after indeterminate store commit failed: {:?}",
                compaction_error,
            ));
        }
        self.health = RuntimeHealth::Poisoned(RuntimePoisonRecord {
            operation: "commit_runtime_transaction".to_string(),
            transaction_id: Some(transaction_id),
            original_error: format!("{error:?}"),
            rollback_failures,
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
        let compensated_ids = envelope.effects.applied_compensatable_ids();
        let compensated = {
            let _phase_guard = ScopedRuntimeState::enter(
                &self.active_effect_phase,
                ActiveRuntimeEffectPhase::Compensating,
            );
            envelope.effects.compensate_applied_reverse()
        };
        let capability_restore =
            capability_checkpoint.map(|checkpoint| self.capability_kernel.restore(checkpoint));
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
        let failed_aborts: HashSet<RuntimeEffectId> =
            aborted.iter().map(|failure| failure.effect_id).collect();
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
        let event = self.make_event(kind);
        let id = event.id;
        envelope.store.stage_event(event.clone())?;
        context.push_event(event);
        self.apply_context_event_retention(context);
        Ok(id)
    }

    pub(in crate::runtime) fn emit_effect_event_outside_transaction(
        &mut self,
        context: &mut RuntimeContext,
        kind: RuntimeEventKind,
    ) -> MResult<EventId> {
        let event = self.make_event(kind);
        let id = event.id;
        self.store.append_event(event.clone())?;
        context.push_event(event);
        self.apply_context_event_retention(context);
        Ok(id)
    }

    fn apply_capability_overlay(&mut self, envelope: &RuntimeExecutionTransaction) -> MResult<()> {
        for (_, capability) in envelope.capabilities.grants() {
            self.capability_kernel
                .grant(CapabilityGrant::new(capability))?;
        }
        for (capability, uses) in envelope.capabilities.usage_deltas() {
            self.capability_kernel.apply_usage_delta(capability, uses)?;
        }
        for capability in envelope.capabilities.revocations() {
            self.capability_kernel
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

        let staged_puts: Vec<ObjectRecord> = transaction.staged_puts().cloned().collect();
        let staged_updates: Vec<ObjectRecord> = transaction.staged_updates().cloned().collect();
        let staged_task_updates: Vec<TaskRecord> =
            transaction.staged_task_updates().cloned().collect();
        let staged_actor_updates: Vec<ActorRecord> =
            transaction.staged_actor_updates().cloned().collect();
        let staged_message_acks: Vec<(ActorId, MessageId)> = transaction
            .staged_message_acks()
            .flat_map(|(actor, messages)| messages.iter().map(move |message| (*actor, *message)))
            .collect();
        let staged_message_enqueues: Vec<(ActorId, MessageRecord)> = transaction
            .staged_message_enqueues()
            .flat_map(|(actor, messages)| {
                messages
                    .iter()
                    .cloned()
                    .map(move |message| (*actor, message))
            })
            .collect();

        let mut staged_events: Vec<RuntimeEvent> = transaction.staged_events().cloned().collect();
        staged_events.push(commit_event.clone());

        let mut transaction_snapshot = transaction.clone();
        transaction_snapshot.record_event(commit_event.id)?;
        let transaction_record = transaction_snapshot
            .into_record()?
            .with_effects(envelope.effects.records()?);

        Ok(RuntimeStoreCommit {
            transaction: transaction_record,
            module_puts: envelope.modules.module_puts().cloned().collect(),
            module_version_puts: envelope.modules.version_puts().cloned().collect(),
            capability_grants: envelope.capabilities.grants().collect(),
            capability_revocations: envelope.capabilities.revocations().collect(),
            object_puts: staged_puts,
            object_updates: staged_updates,
            task_updates: staged_task_updates,
            actor_updates: staged_actor_updates,
            message_acks: staged_message_acks,
            message_enqueues: staged_message_enqueues,
            events: staged_events,
        })
    }
}
