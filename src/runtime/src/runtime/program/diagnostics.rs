use mech_core::ResidentKernelError;
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
        UnsupportedControlLayout { .. }
        | LegacyOpaque { .. }
        | UnsupportedInteraction { .. }
        | UnsupportedDelivery { .. }
        | UnsupportedValue { .. }
        | InitializerUnavailableAtActivation { .. }
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
        | ResidentMemoryPlanRejected { .. }
        | MemoryRuntime { .. }
        | StaticSelectorResolutionLimit { .. }
        | KernelBind { .. }
        | ActivationKernel { .. }
        | ActivationKernelExecution { .. }
        | ActiveCandidate
        | OutputUnavailable { .. }
        | IncompatibleState { .. }
        | InvalidStateMigration
        | SuspendedContinuationMigration
        | PlanGenerationExhausted
        | LayoutGenerationExhausted => ResidentRouteFailureClass::ActivationFailure,
    };
    route_failure(class, format!("resident activation failed: {error:?}"))
}

pub(crate) fn activation_failure_for_artifact(
    artifact: &ProgramArtifact,
    error: ResidentActivationError,
) -> mech_core::MechError {
    if let ResidentActivationError::ActivationKernelExecution {
        node,
        error:
            ResidentKernelError::ProviderStatus {
                operation,
                status,
                code,
            },
    } = &error
    {
        return route_failure(
            ResidentRouteFailureClass::ActivationFailure,
            format!(
                "dynamic kernel `{operation}` returned {status} (status {code}) during resident activation at {node:?}"
            ),
        );
    }
    let operation_node = match &error {
        ResidentActivationError::UnsupportedControlLayout { node }
        | ResidentActivationError::LegacyOpaque { node }
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
                    "resident activation failed at {node:?} ({}): {error:?}",
                    match &declaration.body {
                        mech_engine::ExecutableNodeBody::Operation(operation) =>
                            operation.operation.canonical_name(),
                        mech_engine::ExecutableNodeBody::Match(_) => "Typed match".to_owned(),
                        mech_engine::ExecutableNodeBody::Activation(_) => {
                            "Activation scope".to_owned()
                        }
                        mech_engine::ExecutableNodeBody::Comprehension(control) => match control
                            .kind
                        {
                            mech_engine::ComprehensionKind::Matrix =>
                                "Matrix comprehension".to_owned(),
                            mech_engine::ComprehensionKind::MatrixPreserveShape =>
                                "Shape-preserving matrix comprehension".to_owned(),
                            mech_engine::ComprehensionKind::MatrixHorizontalBlocks { .. } =>
                                "Horizontal block matrix comprehension".to_owned(),
                            mech_engine::ComprehensionKind::Set => "Set comprehension".to_owned(),
                        },
                        mech_engine::ExecutableNodeBody::Fsm(control) => {
                            format!("FSM {}", control.machine)
                        }
                        mech_engine::ExecutableNodeBody::Iteration(control) => {
                            iteration_description(control.count)
                        }
                    },
                ),
            );
        }
    }
    activation_failure(error)
}

fn iteration_description(count: mech_engine::IterationCount) -> String {
    match count {
        mech_engine::IterationCount::Literal(count) => format!("Counted application ({count})"),
        mech_engine::IterationCount::Elements { input } => {
            format!("Element-wise counted application (count input {input})")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::iteration_description;
    use mech_engine::IterationCount;

    #[test]
    fn counted_application_diagnostics_preserve_literal_counts() {
        assert_eq!(
            iteration_description(IterationCount::Literal(2)),
            "Counted application (2)"
        );
        assert_eq!(
            iteration_description(IterationCount::Literal(u64::MAX)),
            "Counted application (18446744073709551615)"
        );
    }

    #[test]
    fn element_wise_application_diagnostics_identify_the_count_input() {
        assert_eq!(
            iteration_description(IterationCount::Elements { input: 1 }),
            "Element-wise counted application (count input 1)"
        );
    }
}
