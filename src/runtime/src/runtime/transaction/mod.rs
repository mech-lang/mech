//! Private runtime transaction coordination.
//!
//! Public transaction protocols and models remain in `crate::transaction`.

mod abort;
mod capabilities;
mod commit;
mod context;
mod envelope;
mod effects;
mod health;
mod modules;
mod program;
mod reactive;
mod savepoint;

pub(super) use capabilities::{
  check_transactional_capability,
  RuntimeCapabilityOverlay,
};
pub(in crate::runtime) use commit::RuntimeCommitResolution;
pub(super) use context::{
  RuntimeContextCheckpoint,
  RuntimeTransactionContextIdentity,
};
pub(super) use effects::RuntimeEffectJournal;
pub(super) use envelope::{
  RuntimeExecutionTransaction,
  RuntimeExecutionTransactionMode,
  RuntimeExecutionTransactionState,
  RuntimeProgramBaseline,
};
pub use health::{RuntimeHealth, RuntimePoisonRecord};
pub(super) use modules::RuntimeModuleJournal;
pub(super) use program::{
  ActiveRuntimeProgramOperation,
  RuntimeProgramOwnershipAcquisition,
};
pub(super) use reactive::PreparedRuntimeHostInput;
pub(super) use savepoint::{
  RuntimeOperationSavepoint,
  RuntimeProgramOperationSavepoint,
};

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
