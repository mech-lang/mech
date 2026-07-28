use super::{
  RuntimeCapabilityOverlay,
  RuntimeContextCheckpoint,
  RuntimeEffectJournal,
  RuntimeModuleJournal,
  RuntimeTransactionContextIdentity,
};
use crate::runtime::{
  MechRuntime,
  RuntimeLiveStateSnapshot,
};
use crate::{
  RuntimeTransaction,
  RuntimeTransactionNotFoundError,
  TransactionId,
};
use mech_core::{
  MResult,
  MechError,
};
use mech_program::MechProgramCheckpoint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum RuntimeExecutionTransactionMode {
  Explicit,
  ImplicitModuleOperation,
  ImplicitProgramOperation,
  ImplicitReactiveTurn,
  ImplicitResourceOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum RuntimeExecutionTransactionState {
  Active,
  Committing,
}

#[derive(Clone)]
pub(in crate::runtime) struct RuntimeProgramBaseline {
  pub(in crate::runtime) program: MechProgramCheckpoint,
  pub(in crate::runtime) live: RuntimeLiveStateSnapshot,
}

pub(in crate::runtime) struct RuntimeExecutionTransaction {
  pub(in crate::runtime) store: RuntimeTransaction,
  pub(in crate::runtime) modules: RuntimeModuleJournal,
  pub(in crate::runtime) mode: RuntimeExecutionTransactionMode,
  pub(in crate::runtime) context_identity: RuntimeTransactionContextIdentity,
  pub(in crate::runtime) context_baseline: RuntimeContextCheckpoint,
  pub(in crate::runtime) program: Option<RuntimeProgramBaseline>,
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
      program: None,
      effects: RuntimeEffectJournal::new(),
      capabilities: RuntimeCapabilityOverlay::default(),
      state: RuntimeExecutionTransactionState::Active,
    }
  }
}
