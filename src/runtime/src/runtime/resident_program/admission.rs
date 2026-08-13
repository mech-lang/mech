use mech_engine::resident::ResidentActivationError;

use super::{ResidentRouteFailureClass, route_failure};

pub(crate) fn activation_failure(error: ResidentActivationError) -> mech_core::MechError {
    use ResidentActivationError::*;

    let class = match &error {
        LegacyOpaque { .. }
        | UnsupportedInteraction { .. }
        | UnsupportedDelivery { .. }
        | UnsupportedValue { .. }
        | TurnDimension { .. }
        | UnresolvedShape { .. }
        | UnsupportedConstruction { .. }
        | UnsupportedChangeDetection { .. }
        | InvalidAlias { .. }
        | InvalidNodeOutput { .. }
        | MissingResidentFactory { .. }
        | OutputMustBeStateBacked { .. } => ResidentRouteFailureClass::SemanticUnsupported,
        InvalidSnapshotRepresentation
        | MissingStateInitializer { .. }
        | InvalidExternalNode { .. }
        | InvalidConstraint { .. }
        | InvalidDependency { .. }
        | UnknownOutput { .. } => ResidentRouteFailureClass::InvalidArtifact,
        RegionSizeOverflow
        | KernelBind { .. }
        | ActivationKernel { .. }
        | ActiveCandidate
        | IncompatibleState { .. }
        | InvalidStateMigration
        | PlanGenerationExhausted
        | LayoutGenerationExhausted => ResidentRouteFailureClass::ActivationFailure,
    };
    route_failure(class, format!("resident activation failed: {error:?}"))
}

pub(crate) fn fallback_eligible(error: &mech_core::MechError) -> bool {
    error.kind_name() == "ResidentRouteFailure"
        && error.kind_message().starts_with("SemanticUnsupported:")
}
