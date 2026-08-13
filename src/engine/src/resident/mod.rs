mod activation;
mod arena;
mod artifact;
pub(crate) mod bench;
mod candidate;
mod full_write;
#[cfg(feature = "resident-artifact")]
pub(crate) mod general;
mod kernel;
#[cfg(feature = "resident-artifact")]
pub(crate) mod numeric;
mod workspace;
mod efficacy {
    pub(crate) mod ekf;
}

pub(crate) use activation::*;
pub(crate) use arena::*;
pub(crate) use artifact::*;
pub use candidate::ResidentExecutionError as ResidentCandidateExecutionError;
pub(crate) use candidate::{Candidate, GateBInstance, publish_epoch};
pub use full_write::{FULL_WRITE_ELEMENTS, PreparedResidentFullWrite, ResidentFullWrite};
pub(crate) use kernel::*;
pub(crate) use workspace::*;

#[cfg(feature = "resident-artifact")]
pub use general::{
    ActivationFacts, CapturedSignalInput, CapturedValueInput, PreparedResidentTurn,
    ReactiveInstance, ResidentActivationError, ResidentActivationOptions, ResidentExecutionError,
    ResidentExternalAdmission, ResidentExternalPublicationAuthority, ResidentIntegrityMode,
    ResidentTurnSummary, ResidentValueBorrow, activate, activate_with_options,
};

#[cfg(feature = "resident-artifact")]
#[doc(hidden)]
pub use general::{
    ActivatedInput, ActivatedInputSource, ActivatedPlan, ActivatedTurnStep,
    ResidentActivationPreflight, ResidentStructuralProbe, preflight_activation,
};

#[cfg(test)]
mod tests;
