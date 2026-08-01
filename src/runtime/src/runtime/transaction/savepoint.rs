use super::RuntimeContextCheckpoint;
use crate::RuntimeTransaction;
use crate::runtime::live_state::RuntimeLiveStateSnapshot;
use mech_engine::MechProgramCheckpoint;

#[derive(Clone)]
pub(in crate::runtime) struct RuntimeOperationSavepoint {
    pub(in crate::runtime) store: RuntimeTransaction,
    pub(in crate::runtime) module_mark: usize,
    pub(in crate::runtime) effect_mark: usize,
    pub(in crate::runtime) capability_mark: usize,
    pub(in crate::runtime) context: RuntimeContextCheckpoint,
}

#[derive(Clone)]
pub(in crate::runtime) struct RuntimeProgramOperationSavepoint {
    pub(in crate::runtime) program: MechProgramCheckpoint,
    pub(in crate::runtime) live: RuntimeLiveStateSnapshot,
    pub(in crate::runtime) runtime: RuntimeOperationSavepoint,
}
