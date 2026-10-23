//! Shared admission diagnostics, independent of resident session execution.
use mech_core::{MechError, MechErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentRouteFailureClass {
    SemanticUnsupported,
    MultipleRootsUnsupported,
    ProviderUnavailable,
    ProviderContractMismatch,
    AuthorizationDenied,
    InvalidArtifact,
    InvalidBytecode,
    ActivationFailure,
    InternalFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentRouteFailure {
    pub class: ResidentRouteFailureClass,
    pub reason: String,
}

impl MechErrorKind for ResidentRouteFailure {
    fn name(&self) -> &str {
        "ResidentRouteFailure"
    }

    fn message(&self) -> String {
        format!("{:?}: {}", self.class, self.reason)
    }
}

pub(crate) fn route_failure(
    class: ResidentRouteFailureClass,
    reason: impl Into<String>,
) -> mech_core::MechError {
    MechError::new(
        ResidentRouteFailure {
            class,
            reason: reason.into(),
        },
        None,
    )
}
