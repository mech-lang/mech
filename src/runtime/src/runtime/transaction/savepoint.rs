use super::RuntimeContextCheckpoint;
use crate::RuntimeTransaction;

#[derive(Clone)]
pub(in crate::runtime) struct RuntimeOperationSavepoint {
    pub(in crate::runtime) store: RuntimeTransaction,
    pub(in crate::runtime) module_mark: usize,
    pub(in crate::runtime) effect_mark: usize,
    pub(in crate::runtime) capability_mark: usize,
    pub(in crate::runtime) context: RuntimeContextCheckpoint,
}
