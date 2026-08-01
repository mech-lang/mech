use super::ActivationPatternCapture;
use crate::{CompiledPattern, ValueKind};

#[derive(Clone)]
pub(super) struct PreflightActivationArm {
    pub(super) pattern: CompiledPattern,
    pub(super) captures: Vec<ActivationPatternCapture>,
}
pub(super) struct PreflightPatternedActivation {
    pub(super) trigger_kind: ValueKind,
    pub(super) arms: Vec<PreflightActivationArm>,
}
