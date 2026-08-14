use super::{
    RuntimeCapabilityOverlay, RuntimeContextCheckpoint, RuntimeEffectJournal, RuntimeModuleJournal,
    RuntimeTransactionContextIdentity,
};
use crate::runtime::MechRuntime;
use crate::{RuntimeTransaction, RuntimeTransactionNotFoundError, TransactionId};
use mech_core::{MResult, MechError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum RuntimeExecutionTransactionMode {
    Explicit,
    ImplicitModuleOperation,
    ImplicitResourceOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum RuntimeExecutionTransactionState {
    Active,
    Committing,
}

pub(in crate::runtime) struct RuntimeExecutionTransaction {
    pub(in crate::runtime) store: RuntimeTransaction,
    pub(in crate::runtime) modules: RuntimeModuleJournal,
    pub(in crate::runtime) mode: RuntimeExecutionTransactionMode,
    pub(in crate::runtime) context_identity: RuntimeTransactionContextIdentity,
    pub(in crate::runtime) context_baseline: RuntimeContextCheckpoint,
    pub(in crate::runtime) effects: RuntimeEffectJournal,
    pub(in crate::runtime) capabilities: RuntimeCapabilityOverlay,
    pub(in crate::runtime) state: RuntimeExecutionTransactionState,
}

impl RuntimeExecutionTransaction {
    pub(in crate::runtime) fn new(
        store: RuntimeTransaction,
        mode: RuntimeExecutionTransactionMode,
        context_identity: RuntimeTransactionContextIdentity,
        context_baseline: RuntimeContextCheckpoint,
    ) -> Self {
        Self {
            store,
            modules: RuntimeModuleJournal::new(),
            mode,
            context_identity,
            context_baseline,
            effects: RuntimeEffectJournal::new(),
            capabilities: RuntimeCapabilityOverlay::default(),
            state: RuntimeExecutionTransactionState::Active,
        }
    }
}

impl MechRuntime {
    pub(in crate::runtime) fn active_transaction_mut(
        &mut self,
        transaction_id: TransactionId,
    ) -> MResult<&mut RuntimeTransaction> {
        Ok(&mut self.active_execution_transaction_mut(transaction_id)?.store)
    }

    pub(in crate::runtime) fn active_execution_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> MResult<&RuntimeExecutionTransaction> {
        self.active_transactions
            .get(&transaction_id)
            .ok_or_else(|| MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None))
    }

    pub(in crate::runtime) fn active_execution_transaction_mut(
        &mut self,
        transaction_id: TransactionId,
    ) -> MResult<&mut RuntimeExecutionTransaction> {
        self.active_transactions
            .get_mut(&transaction_id)
            .ok_or_else(|| MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None))
    }
}
