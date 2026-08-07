//! Runtime-owned effect journal and lifecycle mechanics.

use super::{RuntimeExecutionTransactionState, RuntimeHealth, RuntimePoisonRecord};
use crate::runtime::MechRuntime;
use crate::runtime::extension::{catch_extension, invoke_extension};
use crate::runtime::state::ScopedRuntimeState;
use crate::{
    ActiveRuntimeEffectPhase, PreparedRuntimeEffect, RuntimeContext, RuntimeEffectCleanupFailed,
    RuntimeEffectFailure, RuntimeEffectFailurePhase, RuntimeEffectId, RuntimeEffectMetadata,
    RuntimeEffectProtocol, RuntimeEffectRecord, RuntimeEventKind,
    RuntimeExternalCommitIndeterminate, RuntimeInvalidOperationError, TransactionId,
};
use mech_core::{MResult, MechError, Value};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum RuntimeEffectState {
    Staged,
    Prepared,
    Applied,
}

#[derive(Debug)]
pub(in crate::runtime) struct RuntimeEffectEntry {
    pub(in crate::runtime) id: RuntimeEffectId,
    pub(in crate::runtime) state: RuntimeEffectState,
    pub(in crate::runtime) effect: PreparedRuntimeEffect,
    resource_write: Option<RuntimeStagedResourceWrite>,
}

#[derive(Debug)]
struct RuntimeStagedResourceWrite {
    resource_identity: String,
    path: String,
    value: Value,
}

pub(in crate::runtime) struct RuntimeEffectStepFailure {
    pub(in crate::runtime) failure: RuntimeEffectFailure,
    pub(in crate::runtime) error: MechError,
}

pub(in crate::runtime) struct RuntimeTransactionalCommitReport {
    pub(in crate::runtime) committed: Vec<RuntimeEffectId>,
    pub(in crate::runtime) failures: Vec<RuntimeEffectStepFailure>,
    pub(in crate::runtime) participant_outcomes: Vec<String>,
}

#[derive(Debug, Default)]
pub(in crate::runtime) struct RuntimeEffectJournal {
    entries: Vec<RuntimeEffectEntry>,
    next_sequence: u64,
}

impl RuntimeEffectJournal {
    pub(in crate::runtime) fn new() -> Self {
        Self::default()
    }

    pub(in crate::runtime) fn mark(&self) -> usize {
        self.entries.len()
    }

    pub(in crate::runtime) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(in crate::runtime) fn records(&self) -> MResult<Vec<RuntimeEffectRecord>> {
        self.entries
            .iter()
            .map(|entry| {
                let (metadata, protocol) = effect_description(entry.id, &entry.effect)?;
                Ok(RuntimeEffectRecord::new(entry.id, metadata, protocol))
            })
            .collect()
    }

    pub(in crate::runtime) fn prepared_transactional_ids(&self) -> Vec<RuntimeEffectId> {
        self.entries
            .iter()
            .filter_map(|entry| {
                if entry.state == RuntimeEffectState::Prepared
                    && matches!(entry.effect, PreparedRuntimeEffect::Transactional(_))
                {
                    Some(entry.id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(in crate::runtime) fn applied_compensatable_ids(&self) -> Vec<RuntimeEffectId> {
        self.entries
            .iter()
            .filter_map(|entry| {
                if entry.state == RuntimeEffectState::Applied
                    && matches!(entry.effect, PreparedRuntimeEffect::Compensatable(_))
                {
                    Some(entry.id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(in crate::runtime) fn after_commit_ids(&self) -> Vec<RuntimeEffectId> {
        self.entries
            .iter()
            .filter_map(|entry| {
                if matches!(entry.effect, PreparedRuntimeEffect::AfterCommit(_)) {
                    Some(entry.id)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(in crate::runtime) fn abortable_ids(&self) -> Vec<RuntimeEffectId> {
        self.entries
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

    pub(in crate::runtime) fn abortable_ids_after(&self, mark: usize) -> Vec<RuntimeEffectId> {
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

    pub(in crate::runtime) fn validate_active(&self, transaction: TransactionId) -> Vec<String> {
        let mut failures = Vec::new();
        let mut previous_sequence = None;

        for entry in &self.entries {
            if entry.id.transaction != transaction {
                failures.push(format!(
                    "effect {} belongs to transaction {}, expected {}",
                    entry.id, entry.id.transaction, transaction,
                ));
            }
            if entry.state != RuntimeEffectState::Staged {
                failures.push(format!(
                    "effect {} entered active commit in state {:?}",
                    entry.id, entry.state,
                ));
            }
            if previous_sequence.is_some_and(|previous| entry.id.sequence <= previous) {
                failures.push(format!(
                    "effect {} is not in strictly increasing sequence order",
                    entry.id,
                ));
            }
            previous_sequence = Some(entry.id.sequence);
        }

        if previous_sequence.is_some_and(|sequence| self.next_sequence <= sequence) {
            failures.push(format!(
                "effect next sequence {} does not advance past the journal tail",
                self.next_sequence,
            ));
        }

        failures
    }

    #[cfg(test)]
    pub(in crate::runtime) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(in crate::runtime) fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub(in crate::runtime) fn stage(
        &mut self,
        transaction: TransactionId,
        effect: PreparedRuntimeEffect,
    ) -> RuntimeEffectId {
        self.stage_entry(transaction, effect, None)
    }

    pub(in crate::runtime) fn stage_resource_write(
        &mut self,
        transaction: TransactionId,
        effect: PreparedRuntimeEffect,
        resource_identity: String,
        path: String,
        value: Value,
    ) -> RuntimeEffectId {
        self.stage_entry(
            transaction,
            effect,
            Some(RuntimeStagedResourceWrite {
                resource_identity,
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

    pub(in crate::runtime) fn staged_resource_value(
        &self,
        resource_identity: &str,
        path: &str,
    ) -> Option<Value> {
        self.entries.iter().rev().find_map(|entry| {
            let write = entry.resource_write.as_ref()?;
            if write.resource_identity == resource_identity && write.path == path {
                Some(write.value.clone())
            } else {
                None
            }
        })
    }

    pub(in crate::runtime) fn rollback_to(&mut self, mark: usize) -> Vec<RuntimeEffectFailure> {
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

    pub(in crate::runtime) fn abort_all(&mut self) -> Vec<RuntimeEffectFailure> {
        self.rollback_to(0)
    }

    pub(in crate::runtime) fn prepare_transactional(
        &mut self,
    ) -> Result<(), RuntimeEffectStepFailure> {
        for entry in &mut self.entries {
            if entry.state != RuntimeEffectState::Staged {
                continue;
            }
            let PreparedRuntimeEffect::Transactional(effect) = &mut entry.effect else {
                continue;
            };
            if let Err(error) = invoke_extension(
                format!("transactional effect {}", entry.id),
                "prepare",
                || effect.prepare(),
            ) {
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

    pub(in crate::runtime) fn abort_prepared_reverse(&mut self) -> Vec<RuntimeEffectFailure> {
        let mut failures = Vec::new();
        for entry in self.entries.iter_mut().rev() {
            if entry.state != RuntimeEffectState::Prepared {
                continue;
            }
            let PreparedRuntimeEffect::Transactional(effect) = &mut entry.effect else {
                continue;
            };
            match invoke_extension(
                format!("transactional effect {}", entry.id),
                "abort",
                || effect.abort(),
            ) {
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

    pub(in crate::runtime) fn apply_compensatable(
        &mut self,
    ) -> Result<(), RuntimeEffectStepFailure> {
        for entry in &mut self.entries {
            if entry.state != RuntimeEffectState::Staged {
                continue;
            }
            let PreparedRuntimeEffect::Compensatable(effect) = &mut entry.effect else {
                continue;
            };
            if let Err(error) = invoke_extension(
                format!("compensatable effect {}", entry.id),
                "apply",
                || effect.apply(),
            ) {
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

    pub(in crate::runtime) fn compensate_applied_reverse(&mut self) -> Vec<RuntimeEffectFailure> {
        let mut failures = Vec::new();
        for entry in self.entries.iter_mut().rev() {
            if entry.state != RuntimeEffectState::Applied {
                continue;
            }
            let PreparedRuntimeEffect::Compensatable(effect) = &mut entry.effect else {
                continue;
            };
            match invoke_extension(
                format!("compensatable effect {}", entry.id),
                "compensate",
                || effect.compensate(),
            ) {
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

    pub(in crate::runtime) fn commit_transactional(&mut self) -> RuntimeTransactionalCommitReport {
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
            match invoke_extension(
                format!("transactional effect {}", entry.id),
                "commit",
                || effect.commit(),
            ) {
                Ok(()) => {
                    outcomes.push(format!("transactional effect {} committed", entry.id,));
                    committed.push(entry.id);
                }
                Err(error) => {
                    let step =
                        effect_step_failure(entry.id, RuntimeEffectFailurePhase::Commit, error);
                    outcomes.push(format!(
                        "transactional effect {} commit failed: {}",
                        entry.id, step.failure.message,
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

    pub(in crate::runtime) fn deliver_after_commit(&mut self) -> Vec<RuntimeEffectFailure> {
        let mut failures = Vec::new();
        for entry in &mut self.entries {
            let PreparedRuntimeEffect::AfterCommit(effect) = &mut entry.effect else {
                continue;
            };
            if let Err(error) = invoke_extension(
                format!("after-commit effect {}", entry.id),
                "deliver",
                || effect.deliver(),
            ) {
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

fn abort_effect_entry(entry: &mut RuntimeEffectEntry) -> Result<(), RuntimeEffectFailure> {
    let result = match (&mut entry.effect, entry.state) {
        (
            PreparedRuntimeEffect::Transactional(effect),
            RuntimeEffectState::Staged | RuntimeEffectState::Prepared | RuntimeEffectState::Applied,
        ) => invoke_extension(
            format!("transactional effect {}", entry.id),
            "abort",
            || effect.abort(),
        )
        .map_err(|error| (RuntimeEffectFailurePhase::Abort, error)),
        (PreparedRuntimeEffect::Compensatable(effect), RuntimeEffectState::Applied) => {
            invoke_extension(
                format!("compensatable effect {}", entry.id),
                "compensate",
                || effect.compensate(),
            )
            .map_err(|error| (RuntimeEffectFailurePhase::Compensate, error))
        }
        (
            PreparedRuntimeEffect::Compensatable(effect),
            RuntimeEffectState::Staged | RuntimeEffectState::Prepared,
        ) => invoke_extension(
            format!("compensatable effect {}", entry.id),
            "abort",
            || effect.abort(),
        )
        .map_err(|error| (RuntimeEffectFailurePhase::Abort, error)),
        (PreparedRuntimeEffect::AfterCommit(_), _) => Ok(()),
    };

    result.map_err(|(phase, error)| RuntimeEffectFailure {
        effect_id: entry.id,
        phase,
        message: format!("{:?}", error),
    })
}

fn effect_description(
    id: RuntimeEffectId,
    effect: &PreparedRuntimeEffect,
) -> MResult<(RuntimeEffectMetadata, RuntimeEffectProtocol)> {
    catch_extension(format!("effect {id}"), "metadata", || {
        (effect.metadata(), effect.protocol())
    })
    .map_err(|panic| panic.into_error())
}

impl MechRuntime {
    pub(in crate::runtime) fn describe_effect_failures(
        failures: impl IntoIterator<Item = RuntimeEffectFailure>,
    ) -> Vec<String> {
        failures
            .into_iter()
            .map(|failure| {
                format!(
                    "effect {} {:?} failed: {}",
                    failure.effect_id, failure.phase, failure.message,
                )
            })
            .collect()
    }

    pub(in crate::runtime) fn poison_effect_cleanup(
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

    pub(in crate::runtime) fn poison_external_commit_indeterminate(
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
            failed_effects, transaction_id,
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
        self.ensure_runtime_mutation_allowed("stage_runtime_effect_with_context")?;
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
        let (metadata, protocol) = catch_extension("prepared runtime effect", "metadata", || {
            (effect.metadata(), effect.protocol())
        })
        .map_err(|panic| panic.into_error())?;
        let cost = metadata.cost;
        context.charge_bytes(cost.bytes)?;
        context.charge_items(cost.items)?;
        let transaction = self.active_execution_transaction(transaction_id)?;
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
            transaction.store.gate_a_staged_item_count(),
        );
        let store_before = transaction.store.clone();
        let effect_mark = transaction.effects.mark();
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
                let transaction = self.active_execution_transaction_mut(transaction_id)?;
                transaction.store = store_before;
                transaction.effects.rollback_to(effect_mark)
            };
            drop(phase_guard);
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

    pub(in crate::runtime) fn stage_runtime_resource_effect_with_context(
        &mut self,
        context: &mut RuntimeContext,
        effect: PreparedRuntimeEffect,
        resource_identity: String,
        path: String,
        value: Value,
    ) -> MResult<RuntimeEffectId> {
        self.ensure_runtime_mutation_allowed("stage_runtime_resource_effect_with_context")?;
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
        let (metadata, protocol) = catch_extension("prepared runtime effect", "metadata", || {
            (effect.metadata(), effect.protocol())
        })
        .map_err(|panic| panic.into_error())?;
        let cost = metadata.cost;
        context.charge_bytes(cost.bytes)?;
        context.charge_items(cost.items)?;
        let transaction = self.active_execution_transaction(transaction_id)?;
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
            transaction.store.gate_a_staged_item_count(),
        );
        let store_before = transaction.store.clone();
        let effect_mark = transaction.effects.mark();
        let effect_id = self
            .active_execution_transaction_mut(transaction_id)?
            .effects
            .stage_resource_write(transaction_id, effect, resource_identity, path, value);

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
                let transaction = self.active_execution_transaction_mut(transaction_id)?;
                transaction.store = store_before;
                transaction.effects.rollback_to(effect_mark)
            };
            drop(phase_guard);
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

    pub(in crate::runtime) fn execute_runtime_effect_immediately(
        &mut self,
        mut effect: PreparedRuntimeEffect,
    ) -> MResult<RuntimeEffectId> {
        self.ensure_runtime_mutation_allowed("execute_runtime_effect_immediately")?;

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
                            effect_id, error,
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
#[path = "tests/effects/mod.rs"]
mod tests;
