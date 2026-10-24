mod activation;
mod arena;
mod artifact;
pub(crate) mod bench;
#[cfg(all(feature = "resident-artifact", feature = "integer_bits"))]
pub(crate) mod bits;
#[cfg(feature = "resident-artifact")]
pub(crate) mod budget;
mod candidate;
#[cfg(feature = "resident-artifact")]
pub(crate) mod composite;
#[cfg(all(feature = "resident-artifact", feature = "convert"))]
pub(crate) mod conversion;
mod full_write;
#[cfg(feature = "resident-artifact")]
pub(crate) mod general;
mod kernel;
#[cfg(all(feature = "resident-artifact", feature = "map_ops"))]
pub(crate) mod map_ops;
#[cfg(feature = "resident-artifact")]
pub(crate) mod matrix_literal;
#[cfg(all(
    feature = "resident-artifact",
    any(feature = "pure_text", feature = "map_ops")
))]
pub(crate) mod native_collection_semantics;
#[cfg(feature = "resident-artifact")]
pub(crate) mod native_factory_linkage;
#[cfg(feature = "resident-artifact")]
pub(crate) mod native_semantics;
#[cfg(feature = "resident-artifact")]
pub(crate) mod numeric;
#[cfg(all(feature = "resident-artifact", feature = "pure_text"))]
pub(crate) mod pure_text;
#[cfg(all(feature = "resident-artifact", feature = "sequence_sort"))]
pub(crate) mod sequence_sort;
#[cfg(feature = "resident-artifact")]
pub(crate) mod set;
#[cfg(all(feature = "resident-artifact", feature = "convert"))]
pub(crate) mod set_conversion;
#[cfg(all(feature = "resident-artifact", feature = "table"))]
pub(crate) mod table;
#[cfg(feature = "resident-artifact")]
pub(crate) mod text;
#[cfg(feature = "resident-artifact")]
pub(crate) mod text_index;
#[cfg(all(feature = "resident-artifact", feature = "pure_text"))]
pub(crate) mod text_ops;
mod workspace;
mod efficacy {
    pub(crate) mod ekf;
}

pub(crate) use activation::*;
pub(crate) use arena::*;
pub(crate) use artifact::*;
pub use candidate::ResidentExecutionError as ResidentCandidateExecutionError;
pub(crate) use candidate::{Candidate, ResidentEkfInstance, publish_epoch};
pub use full_write::{FULL_WRITE_ELEMENTS, PreparedResidentFullWrite, ResidentFullWrite};
pub(crate) use kernel::*;
pub(crate) use workspace::*;

#[cfg(feature = "resident-artifact")]
pub use general::{
    ActivationFacts, CapturedSignalInput, CapturedValueInput, ConcreteExecutionCase,
    OperationUnavailableForTarget, PreparedResidentTurn, ReactiveInstance, ResidentActivationError,
    ResidentActivationOptions, ResidentContinuationScheduler, ResidentContinuationWakeup,
    ResidentExecutionError, ResidentExternalAdmission, ResidentExternalPublicationAuthority,
    ResidentIntegrityMode, ResidentTurnSummary, ResidentValueBorrow, StateMigrationMapping,
    activate, activate_with_options, canonical_resource_activation_required,
    canonical_resource_effect_order, preflight_resident_target,
};

#[cfg(feature = "resident-artifact")]
#[doc(hidden)]
pub use general::{
    ActivatedInput, ActivatedInputSource, ActivatedPlan, ActivatedTurnStep,
    ResidentActivationPreflight, ResidentStructuralProbe, preflight_activation,
};

#[cfg(all(
    test,
    feature = "resident-artifact",
    feature = "pure_text",
    feature = "map_ops"
))]
mod canonical_text_tests;
#[cfg(test)]
mod tests;
