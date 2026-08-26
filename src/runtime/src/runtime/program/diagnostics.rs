#[cfg(feature = "resident-routing-source")]
use mech_engine::resident::ResidentExecutionError;
use mech_engine::{ProgramArtifact, resident::ResidentActivationError};

use super::{ResidentRouteFailureClass, route_failure};

#[cfg(feature = "resident-routing-source")]
pub(crate) fn projection_refresh_failure(error: ResidentExecutionError) -> mech_core::MechError {
    route_failure(
        ResidentRouteFailureClass::ActivationFailure,
        format!("resident projection refresh failed: {error:?}"),
    )
}

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
        | MissingResidentFactory { .. } => ResidentRouteFailureClass::SemanticUnsupported,
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

pub(crate) fn activation_failure_for_artifact(
    artifact: &ProgramArtifact,
    error: ResidentActivationError,
) -> mech_core::MechError {
    let operation_node = match &error {
        ResidentActivationError::LegacyOpaque { node }
        | ResidentActivationError::MissingResidentFactory { node }
        | ResidentActivationError::UnsupportedConstruction { node }
        | ResidentActivationError::KernelBind { node, .. } => Some(*node),
        _ => None,
    };
    if let Some(node) = operation_node {
        if let Some(declaration) = artifact.nodes().iter().find(|entry| entry.node == node) {
            return route_failure(
                ResidentRouteFailureClass::SemanticUnsupported,
                format!(
                    "resident activation failed at {node:?} ({}/{}): {error:?}",
                    declaration.operation.module_path.join("/"),
                    declaration.operation.operation_name,
                ),
            );
        }
    }
    activation_failure(error)
}
