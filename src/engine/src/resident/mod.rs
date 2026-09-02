mod activation;
mod arena;
mod artifact;
pub(crate) mod bench;
#[cfg(feature = "resident-artifact")]
mod budget;
mod candidate;
#[cfg(feature = "resident-artifact")]
pub(crate) mod composite;
mod full_write;
#[cfg(feature = "resident-artifact")]
pub(crate) mod general;
mod kernel;
#[cfg(feature = "resident-artifact")]
pub(crate) mod matrix_literal;
#[cfg(feature = "resident-artifact")]
pub(crate) mod numeric;
#[cfg(feature = "resident-artifact")]
pub(crate) mod set;
#[cfg(all(feature = "resident-artifact", feature = "table"))]
pub(crate) mod table;
#[cfg(feature = "resident-artifact")]
pub(crate) mod text;
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
    ActivationFacts, CapturedSignalInput, CapturedValueInput, ConcreteExecutionCase,
    OperationUnavailableForTarget, PreparedResidentTurn, ReactiveInstance, ResidentActivationError,
    ResidentActivationOptions, ResidentExecutionError, ResidentExternalAdmission,
    ResidentExternalPublicationAuthority, ResidentIntegrityMode, ResidentTurnSummary,
    ResidentValueBorrow, StateMigrationMapping, activate, activate_with_options,
    preflight_resident_target,
};

#[cfg(feature = "resident-artifact")]
#[doc(hidden)]
pub use general::{
    ActivatedInput, ActivatedInputSource, ActivatedPlan, ActivatedTurnStep,
    ResidentActivationPreflight, ResidentStructuralProbe, preflight_activation,
};

#[cfg(test)]
mod tests;
