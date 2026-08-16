//! Generic runtime operation health and savepoint coordination.

use super::{RuntimeContextCheckpoint, RuntimeOperationSavepoint};
use crate::runtime::MechRuntime;
use crate::runtime::state::ScopedRuntimeState;
use crate::{
    ActiveRuntimeEffectPhase, RuntimeContext, RuntimeEffectId, RuntimeEffectOperationReentrant,
    RuntimeEventKind, RuntimeHealth, RuntimeOperationRollbackFailed, RuntimePoisonRecord,
    RuntimePoisoned, TransactionId,
};
use mech_core::{MResult, MechError};
use std::collections::HashSet;

impl MechRuntime {
    #[cfg(feature = "runtime_bench_probes")]
    #[doc(hidden)]
    pub fn gate_a_capture_runtime_operation_savepoint(
        &self,
        context: &mut RuntimeContext,
    ) -> MResult<()> {
        let transaction_id = Self::context_transaction_id(context)?;
        let _savepoint = self.capture_runtime_operation_savepoint(context, transaction_id)?;
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
        self.reject_effect_reentrancy(operation)?;
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable(operation)?;
        Ok(())
    }

    pub(in crate::runtime) fn poison_runtime_operation(
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
            RuntimeOperationRollbackFailed {
                operation,
                transaction_id,
                original_error,
                rollback_failures,
            },
            None,
        )
    }

    pub(in crate::runtime) fn capture_runtime_operation_savepoint(
        &self,
        context: &mut RuntimeContext,
        transaction_id: TransactionId,
    ) -> MResult<RuntimeOperationSavepoint> {
        context.prepare_event_checkpoint();
        let transaction = self.active_runtime_transaction(transaction_id)?;
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
            transaction.store.gate_a_staged_item_count(),
        );
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
                let effect_failures = transaction.effects.rollback_to(savepoint.effect_mark);
                let capability_result = transaction
                    .capabilities
                    .rollback_to(savepoint.capability_mark);
                let module_result = transaction.modules.rollback_to(savepoint.module_mark);
                #[cfg(any(test, feature = "runtime_bench_probes"))]
                crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
                    savepoint.store.gate_a_staged_item_count(),
                );
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
            Some((effect_failures, capability_result, module_result, abortable_ids)) => {
                let failed_effects: HashSet<RuntimeEffectId> = effect_failures
                    .iter()
                    .map(|failure| failure.effect_id)
                    .collect();
                failures.extend(Self::describe_effect_failures(effect_failures));
                if let Err(error) = capability_result {
                    failures.push(format!("capability overlay rollback failed: {:?}", error,));
                }
                if let Err(error) = module_result {
                    failures.push(format!("module journal rollback failed: {:?}", error,));
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

        if let Err(error) = savepoint.context.restore_preserving_consumption(context) {
            failures.push(format!("context event mark restore failed: {:?}", error));
        }

        if let Err(error) = self.validate_context_for_runtime(context) {
            failures.push(format!("context restore invariant failed: {:?}", error));
        }

        failures
    }
}
