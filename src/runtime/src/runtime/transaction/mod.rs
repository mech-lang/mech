//! Private runtime transaction coordination.
//!
//! Public transaction protocols and models remain in `crate::transaction`.

mod abort;
mod capabilities;
mod commit;
mod context;
pub(super) mod effects;
mod envelope;
mod health;
mod modules;
mod operation;
mod savepoint;

pub(super) use capabilities::{RuntimeCapabilityOverlay, check_transactional_capability};
pub(in crate::runtime) use commit::RuntimeCommitResolution;
pub(super) use context::{RuntimeContextCheckpoint, RuntimeTransactionContextIdentity};
pub(super) use effects::RuntimeEffectJournal;
pub(super) use envelope::{
    RuntimeExecutionTransaction, RuntimeExecutionTransactionMode, RuntimeExecutionTransactionState,
};
pub use health::{RuntimeHealth, RuntimePoisonRecord};
pub(super) use modules::RuntimeModuleJournal;
pub(super) use savepoint::RuntimeOperationSavepoint;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
