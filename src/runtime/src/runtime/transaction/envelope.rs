use super::{
    RuntimeCapabilityOverlay, RuntimeContextCheckpoint, RuntimeEffectJournal, RuntimeModuleJournal,
    RuntimeTransactionContextIdentity,
};
use crate::runtime::MechRuntime;
use crate::{RuntimeTransaction, RuntimeTransactionNotFoundError, TransactionId};
use mech_core::{MResult, MechError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum RuntimeTransactionScope {
    Explicit,
    ImplicitModuleOperation,
    ImplicitResourceOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum ActiveRuntimeTransactionState {
    Active,
    Committing,
}

pub(in crate::runtime) struct ActiveRuntimeTransaction {
    pub(in crate::runtime) store: RuntimeTransaction,
    pub(in crate::runtime) modules: RuntimeModuleJournal,
    pub(in crate::runtime) scope: RuntimeTransactionScope,
    pub(in crate::runtime) context_identity: RuntimeTransactionContextIdentity,
    pub(in crate::runtime) context_baseline: RuntimeContextCheckpoint,
    pub(in crate::runtime) effects: RuntimeEffectJournal,
    pub(in crate::runtime) capabilities: RuntimeCapabilityOverlay,
    pub(in crate::runtime) state: ActiveRuntimeTransactionState,
}

impl ActiveRuntimeTransaction {
    pub(in crate::runtime) fn new(
        store: RuntimeTransaction,
        scope: RuntimeTransactionScope,
        context_identity: RuntimeTransactionContextIdentity,
        context_baseline: RuntimeContextCheckpoint,
    ) -> Self {
        Self {
            store,
            modules: RuntimeModuleJournal::new(),
            scope,
            context_identity,
            context_baseline,
            effects: RuntimeEffectJournal::new(),
            capabilities: RuntimeCapabilityOverlay::default(),
            state: ActiveRuntimeTransactionState::Active,
        }
    }
}

impl MechRuntime {
    pub(in crate::runtime) fn active_transaction_mut(
        &mut self,
        transaction_id: TransactionId,
    ) -> MResult<&mut RuntimeTransaction> {
        Ok(&mut self.active_runtime_transaction_mut(transaction_id)?.store)
    }

    pub(in crate::runtime) fn active_runtime_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> MResult<&ActiveRuntimeTransaction> {
        self.active_transactions
            .get(&transaction_id)
            .ok_or_else(|| MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None))
    }

    pub(in crate::runtime) fn active_runtime_transaction_mut(
        &mut self,
        transaction_id: TransactionId,
    ) -> MResult<&mut ActiveRuntimeTransaction> {
        self.active_transactions
            .get_mut(&transaction_id)
            .ok_or_else(|| MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None))
    }
}
