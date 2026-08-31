use super::ActivationPatternCapture;
use crate::CompiledPattern;

#[derive(Clone)]
pub(super) struct PreflightActivationArm {
    pub(super) pattern: CompiledPattern,
    pub(super) captures: Vec<ActivationPatternCapture>,
}
pub(super) struct PreflightPatternedActivation {
    pub(super) arms: Vec<PreflightActivationArm>,
}
