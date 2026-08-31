//! Semantic admission and normalization for the frozen EKF artifact.

use mech_core::snapshot::SequenceView;
use mech_core::{
    AccessMode, AliasPolicy, CellSlotId, ChangeDetectionPolicy, ConstantId, DeliveryMode,
    DimensionExpr, ExecutionHostFunctionRequest, ExecutionResourceRequest, ExternalInteraction,
    FloatWidth, MResult, MechError, MechErrorKind, MechExecutionServices, NodeId,
    ObservationReplayPolicy, OperationContractId, OutputConstruction, OutputId, ProgramRevision,
    ResolvedOperationContract, ResourceDelivery, ResourceIntent, SchemaBody, SchemaId, ShapeRule,
    ValueCell, ValueData,
};
use nalgebra::DVector;

use crate::{
    ArtifactSource, CompilerPlanningConfig, CompilerPlanningProgram, ProgramArtifact,
    decode_program_artifact_bytecode_v1,
};

use super::catalog::frozen_ekf_compiler_catalog;
use super::operation::{
    EkfKernel, EkfPredicate, FROZEN_EKF_OPERATIONS, FrozenEkfOperation, FrozenEkfOperationSpec,
    FrozenEkfValueShape,
};

const TRACE: &[u8] = include_bytes!("../../../../../benchmarks/runtime/gate-b/ekf-input-v1.bin");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfInputClosure {
    pub name: &'static str,
    pub observation_node: NodeId,
    pub slot: CellSlotId,
    pub schema: SchemaId,
    pub request: ExecutionResourceRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfKernelNode {
    pub node: NodeId,
    pub operation: EkfKernel,
    pub inputs: Box<[ArtifactSource]>,
    pub output: CellSlotId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfPredicateNode {
    pub node: NodeId,
    pub operation: EkfPredicate,
    pub inputs: Box<[ArtifactSource]>,
    pub output: CellSlotId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfStateUpdate {
    pub node: NodeId,
    pub target: CellSlotId,
    pub candidate: CellSlotId,
    pub initializer: ConstantId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfConstraint {
    pub constraint: mech_core::IntegrityConstraintId,
    pub predicate: EkfPredicate,
    pub source: CellSlotId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfOutputClosure {
    pub output: OutputId,
    pub name: &'static str,
    pub source: CellSlotId,
    pub schema: SchemaId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfConstantClosure {
    pub dt: ConstantId,
    pub landmark: ConstantId,
    pub process_covariance: ConstantId,
    pub measurement_covariance: ConstantId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenEkfArtifactClosure {
    pub program_revision: ProgramRevision,
    pub input: FrozenEkfInputClosure,
    pub resident_kernels: Box<[FrozenEkfKernelNode]>,
    pub integrity_predicates: Box<[FrozenEkfPredicateNode]>,
    pub state_updates: Box<[FrozenEkfStateUpdate]>,
    pub constraints: Box<[FrozenEkfConstraint]>,
    pub constants: FrozenEkfConstantClosure,
    pub observation_adapter_nodes: Box<[NodeId]>,
    pub structural_alias_nodes: Box<[NodeId]>,
    pub output: FrozenEkfOutputClosure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrozenEkfArtifactClosureError {
    LegacyOpaqueContract {
        contract: OperationContractId,
    },
    UnexpectedExecutableNode {
        node: NodeId,
        operation: crate::OperationReference,
    },
    UnsupportedNodeContract {
        node: NodeId,
        contract: OperationContractId,
    },
    MissingFrozenOperation {
        operation: &'static str,
    },
    DuplicateFrozenOperation {
        operation: &'static str,
    },
    InvalidObservationBoundary,
    InvalidObservationAdapter,
    InvalidOperationWiring {
        node: NodeId,
        operation: &'static str,
    },
    InvalidStateUpdate,
    InvalidIntegrityPredicate,
    InvalidIntegrityConstraint,
    InvalidInput,
    InvalidOutput,
    InvalidInitializer,
    InvalidConstantBinding,
}

impl MechErrorKind for FrozenEkfArtifactClosureError {
    fn name(&self) -> &str {
        "FrozenEkfArtifactClosureError"
    }

    fn message(&self) -> String {
        format!("frozen EKF ProgramArtifact is not closed: {self:?}")
    }
}

impl FrozenEkfArtifactClosure {
    pub fn admit(
        artifact: &ProgramArtifact,
        request: &ExecutionResourceRequest,
    ) -> Result<Self, FrozenEkfArtifactClosureError> {
        if !artifact.inputs().is_empty() {
            return Err(FrozenEkfArtifactClosureError::InvalidInput);
        }
        if artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == crate::SlotRole::State)
            .count()
            != 2
        {
            return Err(FrozenEkfArtifactClosureError::InvalidStateUpdate);
        }
        for (index, contract) in artifact.contracts().iter().enumerate() {
            if matches!(contract, ResolvedOperationContract::LegacyOpaque(_)) {
                return Err(FrozenEkfArtifactClosureError::LegacyOpaqueContract {
                    contract: OperationContractId::new(index as u32),
                });
            }
        }

        let mut observation = None;
        let mut kernels = Vec::with_capacity(15);
        let mut predicates = Vec::with_capacity(3);
        let mut state_updates = Vec::with_capacity(2);
        let mut output_by_operation = std::collections::BTreeMap::new();

        for node in artifact.nodes() {
            let declared = declared_contract(artifact, node.node, node.contract)?;
            let inputs = node_inputs(artifact, node)?;
            let outputs = node_outputs(artifact, node)?;
            if node.operation.module_path.as_ref() == ["resource", "read"]
                && node.operation.operation_name == "read"
            {
                if observation.is_some()
                    || !inputs.is_empty()
                    || outputs.len() != 1
                    || !is_observation_contract(declared)
                    || !schema_matches(
                        artifact,
                        slot_schema(artifact, outputs[0])?,
                        FrozenEkfValueShape::Vector(4),
                    )
                {
                    return Err(FrozenEkfArtifactClosureError::InvalidObservationBoundary);
                }
                observation = Some((node.node, outputs[0]));
                continue;
            }

            if node.operation.module_path.as_ref() == ["ekf"] {
                let canonical = format!("ekf/{}", node.operation.operation_name);
                let Some(spec) = FROZEN_EKF_OPERATIONS
                    .iter()
                    .find(|spec| spec.canonical_name == canonical)
                else {
                    return Err(FrozenEkfArtifactClosureError::UnexpectedExecutableNode {
                        node: node.node,
                        operation: node.operation.clone(),
                    });
                };
                if output_by_operation
                    .insert(spec.canonical_name, outputs.first().copied())
                    .is_some()
                {
                    return Err(FrozenEkfArtifactClosureError::DuplicateFrozenOperation {
                        operation: spec.canonical_name,
                    });
                }
                validate_frozen_operation(artifact, node.node, spec, declared, &inputs, &outputs)?;
                match spec.operation {
                    FrozenEkfOperation::Kernel(operation) => kernels.push(FrozenEkfKernelNode {
                        node: node.node,
                        operation,
                        inputs,
                        output: outputs[0],
                    }),
                    FrozenEkfOperation::Predicate(operation) => {
                        predicates.push(FrozenEkfPredicateNode {
                            node: node.node,
                            operation,
                            inputs,
                            output: outputs[0],
                        })
                    }
                }
                continue;
            }

            if node.operation.module_path.as_ref() == ["core"]
                && node.operation.operation_name == "assign"
            {
                if inputs.len() != 1 || outputs.len() != 1 || !is_state_update_contract(declared) {
                    return Err(FrozenEkfArtifactClosureError::InvalidStateUpdate);
                }
                let ArtifactSource::Slot(candidate) = inputs[0] else {
                    return Err(FrozenEkfArtifactClosureError::InvalidStateUpdate);
                };
                let target = outputs[0];
                let target_declaration = artifact
                    .slots()
                    .get(target.get() as usize)
                    .filter(|slot| slot.slot == target && slot.role == crate::SlotRole::State)
                    .ok_or(FrozenEkfArtifactClosureError::InvalidStateUpdate)?;
                let crate::InitializerReference::Constant(initializer) = target_declaration
                    .initializer
                    .ok_or(FrozenEkfArtifactClosureError::InvalidInitializer)?;
                state_updates.push(FrozenEkfStateUpdate {
                    node: node.node,
                    target,
                    candidate,
                    initializer,
                });
                continue;
            }

            return Err(FrozenEkfArtifactClosureError::UnexpectedExecutableNode {
                node: node.node,
                operation: node.operation.clone(),
            });
        }

        for spec in FROZEN_EKF_OPERATIONS {
            if !output_by_operation.contains_key(spec.canonical_name) {
                return Err(FrozenEkfArtifactClosureError::MissingFrozenOperation {
                    operation: spec.canonical_name,
                });
            }
        }
        if kernels.len() != 15 || predicates.len() != 3 || state_updates.len() != 2 {
            return Err(FrozenEkfArtifactClosureError::InvalidStateUpdate);
        }

        let (observation_node, observation_slot) =
            observation.ok_or(FrozenEkfArtifactClosureError::InvalidObservationBoundary)?;
        validate_resource_request(request)
            .map_err(|_| FrozenEkfArtifactClosureError::InvalidObservationBoundary)?;
        let observation_schema = slot_schema(artifact, observation_slot)?;
        let input = FrozenEkfInputClosure {
            name: "frame",
            observation_node,
            slot: observation_slot,
            schema: observation_schema,
            request: request.clone(),
        };
        validate_observation_consumers(&kernels, observation_slot)?;

        validate_state_update_sources(&state_updates, &output_by_operation)?;
        let output = validate_output(artifact, &state_updates)?;
        validate_initializers(artifact, &state_updates, &output)?;
        let constants = validate_constants(artifact, &kernels)?;
        validate_operation_wiring(
            &kernels,
            &predicates,
            &state_updates,
            &output,
            &constants,
            observation_slot,
        )?;
        let constraints = validate_constraints(artifact, &predicates)?;

        Ok(Self {
            program_revision: artifact.revision(),
            input,
            resident_kernels: kernels.into_boxed_slice(),
            integrity_predicates: predicates.into_boxed_slice(),
            state_updates: state_updates.into_boxed_slice(),
            constraints,
            constants,
            observation_adapter_nodes: Box::new([]),
            structural_alias_nodes: Box::new([]),
            output,
        })
    }
}

fn declared_contract<'a>(
    artifact: &'a ProgramArtifact,
    node: NodeId,
    contract: OperationContractId,
) -> Result<&'a mech_core::DeclaredOperationContract, FrozenEkfArtifactClosureError> {
    match artifact.contracts().get(contract) {
        Some(ResolvedOperationContract::Declared(contract)) => Ok(contract),
        Some(ResolvedOperationContract::LegacyOpaque(_)) => {
            Err(FrozenEkfArtifactClosureError::LegacyOpaqueContract { contract })
        }
        None => Err(FrozenEkfArtifactClosureError::UnsupportedNodeContract { node, contract }),
    }
}

fn node_inputs(
    artifact: &ProgramArtifact,
    node: &crate::NodeDeclaration,
) -> Result<Box<[ArtifactSource]>, FrozenEkfArtifactClosureError> {
    artifact
        .bindings()
        .get(node.input_bindings.start as usize..node.input_bindings.end as usize)
        .ok_or(FrozenEkfArtifactClosureError::UnsupportedNodeContract {
            node: node.node,
            contract: node.contract,
        })?
        .iter()
        .map(|binding| match binding {
            crate::BindingDeclaration::Input { source, .. } => Ok(*source),
            _ => Err(FrozenEkfArtifactClosureError::UnsupportedNodeContract {
                node: node.node,
                contract: node.contract,
            }),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn node_outputs(
    artifact: &ProgramArtifact,
    node: &crate::NodeDeclaration,
) -> Result<Box<[CellSlotId]>, FrozenEkfArtifactClosureError> {
    artifact
        .bindings()
        .get(node.output_bindings.start as usize..node.output_bindings.end as usize)
        .ok_or(FrozenEkfArtifactClosureError::UnsupportedNodeContract {
            node: node.node,
            contract: node.contract,
        })?
        .iter()
        .map(|binding| match binding {
            crate::BindingDeclaration::Output { target, .. } => Ok(*target),
            _ => Err(FrozenEkfArtifactClosureError::UnsupportedNodeContract {
                node: node.node,
                contract: node.contract,
            }),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

fn slot_schema(
    artifact: &ProgramArtifact,
    slot: CellSlotId,
) -> Result<SchemaId, FrozenEkfArtifactClosureError> {
    artifact
        .slots()
        .get(slot.get() as usize)
        .filter(|declaration| declaration.slot == slot)
        .map(|declaration| declaration.schema)
        .ok_or(FrozenEkfArtifactClosureError::InvalidConstantBinding)
}

fn source_schema(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
) -> Result<SchemaId, FrozenEkfArtifactClosureError> {
    match source {
        ArtifactSource::Constant(constant) => artifact
            .constants()
            .get(constant)
            .map(|value| value.schema())
            .ok_or(FrozenEkfArtifactClosureError::InvalidConstantBinding),
        ArtifactSource::Slot(slot) => slot_schema(artifact, slot),
    }
}

fn schema_matches(
    artifact: &ProgramArtifact,
    schema: SchemaId,
    shape: FrozenEkfValueShape,
) -> bool {
    let Some(schema) = artifact.schemas().get(schema) else {
        return false;
    };
    match (shape, schema.body()) {
        (FrozenEkfValueShape::F64, SchemaBody::FloatingPoint(FloatWidth::W64)) => true,
        (FrozenEkfValueShape::Bool, SchemaBody::Bool) => true,
        (
            FrozenEkfValueShape::Vector(rows),
            SchemaBody::Matrix {
                element,
                dimensions,
            },
        ) => {
            matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W64))
                && dimensions.as_ref()
                    == [
                        DimensionExpr::Constant(rows as u64),
                        DimensionExpr::Constant(1),
                    ]
        }
        (
            FrozenEkfValueShape::Matrix { rows, columns },
            SchemaBody::Matrix {
                element,
                dimensions,
            },
        ) => {
            matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W64))
                && dimensions.as_ref()
                    == [
                        DimensionExpr::Constant(rows as u64),
                        DimensionExpr::Constant(columns as u64),
                    ]
        }
        _ => false,
    }
}

fn is_observation_contract(contract: &mech_core::DeclaredOperationContract) -> bool {
    contract.inputs.is_empty()
        && contract.outputs.len() == 1
        && contract.outputs[0].access == AccessMode::Write
        && contract.outputs[0].delivery == DeliveryMode::Signal
        && contract.outputs[0].construction
            == OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            }
        && contract.outputs[0].alias == AliasPolicy::NoAlias
        && contract.outputs[0].change_detection == ChangeDetectionPolicy::AlwaysChanged
        && matches!(
            contract.interaction,
            ExternalInteraction::Observation(ref observation)
                if observation.replay == ObservationReplayPolicy::CaptureAsInputFact
        )
}

fn validate_frozen_operation(
    artifact: &ProgramArtifact,
    node: NodeId,
    spec: &FrozenEkfOperationSpec,
    contract: &mech_core::DeclaredOperationContract,
    inputs: &[ArtifactSource],
    outputs: &[CellSlotId],
) -> Result<(), FrozenEkfArtifactClosureError> {
    let valid_contract = contract.interaction == ExternalInteraction::Pure
        && contract.inputs.len() == spec.inputs.len()
        && contract.inputs.iter().all(|input| {
            input.access == AccessMode::Read && input.delivery == DeliveryMode::Signal
        })
        && contract.outputs.len() == 1
        && contract.outputs[0].access == AccessMode::Write
        && contract.outputs[0].delivery == DeliveryMode::Signal
        && contract.outputs[0].construction
            == OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            }
        && contract.outputs[0].alias == AliasPolicy::NoAlias
        && valid_pure_change_detection(contract.outputs[0].change_detection, spec.change_detection);
    if !valid_contract || inputs.len() != spec.inputs.len() || outputs.len() != 1 {
        return Err(match spec.operation {
            FrozenEkfOperation::Kernel(_) => {
                FrozenEkfArtifactClosureError::UnsupportedNodeContract {
                    node,
                    contract: artifact.nodes()[node.get() as usize].contract,
                }
            }
            FrozenEkfOperation::Predicate(_) => {
                FrozenEkfArtifactClosureError::InvalidIntegrityPredicate
            }
        });
    }
    if !inputs.iter().zip(spec.inputs).all(|(source, shape)| {
        source_schema(artifact, *source).is_ok_and(|id| schema_matches(artifact, id, *shape))
    }) || !schema_matches(artifact, slot_schema(artifact, outputs[0])?, spec.output)
    {
        return Err(match spec.operation {
            FrozenEkfOperation::Kernel(_) => {
                FrozenEkfArtifactClosureError::UnsupportedNodeContract {
                    node,
                    contract: artifact.nodes()[node.get() as usize].contract,
                }
            }
            FrozenEkfOperation::Predicate(_) => {
                FrozenEkfArtifactClosureError::InvalidIntegrityPredicate
            }
        });
    }
    Ok(())
}

fn is_state_update_contract(contract: &mech_core::DeclaredOperationContract) -> bool {
    contract.interaction == ExternalInteraction::Pure
        && contract.inputs.len() == 1
        && contract.inputs[0].access == AccessMode::Read
        && contract.inputs[0].delivery == DeliveryMode::Signal
        && contract.outputs.len() == 1
        && contract.outputs[0].access == AccessMode::Write
        && contract.outputs[0].delivery == DeliveryMode::Signal
        && contract.outputs[0].construction
            == OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            }
        && contract.outputs[0].alias == AliasPolicy::NoAlias
        && valid_pure_change_detection(
            contract.outputs[0].change_detection,
            ChangeDetectionPolicy::KernelReported,
        )
}

fn valid_pure_change_detection(
    actual: ChangeDetectionPolicy,
    expected: ChangeDetectionPolicy,
) -> bool {
    !matches!(actual, ChangeDetectionPolicy::AlwaysChanged) && actual == expected
}

fn validate_state_update_sources(
    state_updates: &[FrozenEkfStateUpdate],
    output_by_operation: &std::collections::BTreeMap<&'static str, Option<CellSlotId>>,
) -> Result<(), FrozenEkfArtifactClosureError> {
    let expected = [
        output_by_operation
            .get("ekf/corrected-state")
            .copied()
            .flatten()
            .ok_or(FrozenEkfArtifactClosureError::InvalidStateUpdate)?,
        output_by_operation
            .get("ekf/covariance-symmetrization")
            .copied()
            .flatten()
            .ok_or(FrozenEkfArtifactClosureError::InvalidStateUpdate)?,
    ];
    if expected.iter().any(|candidate| {
        !state_updates
            .iter()
            .any(|update| update.candidate == *candidate)
    }) || state_updates
        .iter()
        .any(|update| !expected.contains(&update.candidate))
    {
        return Err(FrozenEkfArtifactClosureError::InvalidStateUpdate);
    }
    Ok(())
}

fn validate_observation_consumers(
    kernels: &[FrozenEkfKernelNode],
    observation: CellSlotId,
) -> Result<(), FrozenEkfArtifactClosureError> {
    let expected = [
        (EkfKernel::MotionJacobian, 1_usize),
        (EkfKernel::PredictedState, 1_usize),
        (EkfKernel::Innovation, 0_usize),
    ];
    for (operation, input) in expected {
        let node = kernels
            .iter()
            .find(|node| node.operation == operation)
            .ok_or(FrozenEkfArtifactClosureError::InvalidInput)?;
        if node.inputs.get(input) != Some(&ArtifactSource::Slot(observation)) {
            return Err(FrozenEkfArtifactClosureError::InvalidInput);
        }
    }
    if kernels
        .iter()
        .flat_map(|node| node.inputs.iter())
        .filter(|source| **source == ArtifactSource::Slot(observation))
        .count()
        != expected.len()
    {
        return Err(FrozenEkfArtifactClosureError::InvalidInput);
    }
    Ok(())
}

fn validate_output(
    artifact: &ProgramArtifact,
    state_updates: &[FrozenEkfStateUpdate],
) -> Result<FrozenEkfOutputClosure, FrozenEkfArtifactClosureError> {
    let [output] = artifact.outputs() else {
        return Err(FrozenEkfArtifactClosureError::InvalidOutput);
    };
    if output.name != "estimate"
        || !state_updates
            .iter()
            .any(|update| update.target == output.source)
        || !schema_matches(artifact, output.schema, FrozenEkfValueShape::Vector(3))
        || slot_schema(artifact, output.source)? != output.schema
    {
        return Err(FrozenEkfArtifactClosureError::InvalidOutput);
    }
    Ok(FrozenEkfOutputClosure {
        output: output.output,
        name: "estimate",
        source: output.source,
        schema: output.schema,
    })
}

fn validate_initializers(
    artifact: &ProgramArtifact,
    state_updates: &[FrozenEkfStateUpdate],
    output: &FrozenEkfOutputClosure,
) -> Result<(), FrozenEkfArtifactClosureError> {
    let state = state_updates
        .iter()
        .find(|update| update.target == output.source)
        .ok_or(FrozenEkfArtifactClosureError::InvalidInitializer)?;
    let covariance = state_updates
        .iter()
        .find(|update| update.target != output.source)
        .ok_or(FrozenEkfArtifactClosureError::InvalidInitializer)?;
    if value_f64s(artifact, state.initializer).as_deref() != Some(&[2.0, 1.0, 0.15])
        || value_f64s(artifact, covariance.initializer).as_deref()
            != Some(&[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05])
    {
        return Err(FrozenEkfArtifactClosureError::InvalidInitializer);
    }
    Ok(())
}

fn validate_constants(
    artifact: &ProgramArtifact,
    kernels: &[FrozenEkfKernelNode],
) -> Result<FrozenEkfConstantClosure, FrozenEkfArtifactClosureError> {
    let constant = |operation: EkfKernel, input: usize, expected: &[f64]| {
        let node = kernels
            .iter()
            .find(|node| node.operation == operation)
            .ok_or(FrozenEkfArtifactClosureError::InvalidConstantBinding)?;
        let ArtifactSource::Constant(constant) = node
            .inputs
            .get(input)
            .copied()
            .ok_or(FrozenEkfArtifactClosureError::InvalidConstantBinding)?
        else {
            return Err(FrozenEkfArtifactClosureError::InvalidConstantBinding);
        };
        if value_f64s(artifact, constant).as_deref() != Some(expected) {
            return Err(FrozenEkfArtifactClosureError::InvalidConstantBinding);
        }
        Ok(constant)
    };
    let dt = constant(EkfKernel::MotionJacobian, 3, &[0.05])?;
    let landmark = constant(EkfKernel::LandmarkDeltaAndRange, 1, &[25.0, -10.0])?;
    let process_covariance =
        constant(EkfKernel::PredictedCovariance, 3, &[0.04, 0.0, 0.0, 0.0025])?;
    let measurement_covariance = constant(
        EkfKernel::InnovationCovariance,
        2,
        &[0.25, 0.0, 0.0, 0.0009],
    )?;
    let allowed = [dt, landmark, process_covariance, measurement_covariance];
    if kernels
        .iter()
        .flat_map(|node| node.inputs.iter())
        .filter_map(|source| match source {
            ArtifactSource::Constant(constant) => Some(*constant),
            ArtifactSource::Slot(_) => None,
        })
        .any(|constant| !allowed.contains(&constant))
    {
        return Err(FrozenEkfArtifactClosureError::InvalidConstantBinding);
    }
    Ok(FrozenEkfConstantClosure {
        dt,
        landmark,
        process_covariance,
        measurement_covariance,
    })
}

fn validate_operation_wiring(
    kernels: &[FrozenEkfKernelNode],
    predicates: &[FrozenEkfPredicateNode],
    state_updates: &[FrozenEkfStateUpdate],
    output: &FrozenEkfOutputClosure,
    constants: &FrozenEkfConstantClosure,
    observation: CellSlotId,
) -> Result<(), FrozenEkfArtifactClosureError> {
    let kernel_output = |operation| {
        let spec = super::operation::operation_spec(FrozenEkfOperation::Kernel(operation));
        kernels
            .iter()
            .find(|node| node.operation == operation)
            .map(|node| ArtifactSource::Slot(node.output))
            .ok_or(FrozenEkfArtifactClosureError::MissingFrozenOperation {
                operation: spec.canonical_name,
            })
    };
    let state = state_updates
        .iter()
        .find(|update| update.target == output.source)
        .ok_or(FrozenEkfArtifactClosureError::InvalidStateUpdate)?;
    let covariance = state_updates
        .iter()
        .find(|update| update.target != output.source)
        .ok_or(FrozenEkfArtifactClosureError::InvalidStateUpdate)?;
    let state_source = ArtifactSource::Slot(state.target);
    let covariance_source = ArtifactSource::Slot(covariance.target);
    let observation_source = ArtifactSource::Slot(observation);
    let constant = ArtifactSource::Constant;

    for node in kernels {
        let expected = match node.operation {
            EkfKernel::TrigonometricState => vec![state_source],
            EkfKernel::MotionJacobian => vec![
                state_source,
                observation_source,
                kernel_output(EkfKernel::TrigonometricState)?,
                constant(constants.dt),
            ],
            EkfKernel::ControlJacobian => vec![
                kernel_output(EkfKernel::TrigonometricState)?,
                constant(constants.dt),
            ],
            EkfKernel::PredictedState => vec![
                state_source,
                observation_source,
                kernel_output(EkfKernel::TrigonometricState)?,
                constant(constants.dt),
            ],
            EkfKernel::PredictedCovariance => vec![
                covariance_source,
                kernel_output(EkfKernel::MotionJacobian)?,
                kernel_output(EkfKernel::ControlJacobian)?,
                constant(constants.process_covariance),
            ],
            EkfKernel::LandmarkDeltaAndRange => vec![
                kernel_output(EkfKernel::PredictedState)?,
                constant(constants.landmark),
            ],
            EkfKernel::PredictedMeasurement => vec![
                kernel_output(EkfKernel::PredictedState)?,
                kernel_output(EkfKernel::LandmarkDeltaAndRange)?,
            ],
            EkfKernel::MeasurementJacobian => {
                vec![kernel_output(EkfKernel::LandmarkDeltaAndRange)?]
            }
            EkfKernel::InnovationCovariance => vec![
                kernel_output(EkfKernel::PredictedCovariance)?,
                kernel_output(EkfKernel::MeasurementJacobian)?,
                constant(constants.measurement_covariance),
            ],
            EkfKernel::Solve2x2 => vec![kernel_output(EkfKernel::InnovationCovariance)?],
            EkfKernel::KalmanGain => vec![
                kernel_output(EkfKernel::PredictedCovariance)?,
                kernel_output(EkfKernel::MeasurementJacobian)?,
                kernel_output(EkfKernel::Solve2x2)?,
            ],
            EkfKernel::Innovation => vec![
                observation_source,
                kernel_output(EkfKernel::PredictedMeasurement)?,
            ],
            EkfKernel::CorrectedState => vec![
                kernel_output(EkfKernel::PredictedState)?,
                kernel_output(EkfKernel::KalmanGain)?,
                kernel_output(EkfKernel::Innovation)?,
            ],
            EkfKernel::JosephCovarianceUpdate => vec![
                kernel_output(EkfKernel::PredictedCovariance)?,
                kernel_output(EkfKernel::MeasurementJacobian)?,
                kernel_output(EkfKernel::KalmanGain)?,
                constant(constants.measurement_covariance),
            ],
            EkfKernel::CovarianceSymmetrization => {
                vec![kernel_output(EkfKernel::JosephCovarianceUpdate)?]
            }
        };
        if node.inputs.as_ref() != expected.as_slice() {
            return Err(FrozenEkfArtifactClosureError::InvalidOperationWiring {
                node: node.node,
                operation: super::operation::operation_spec(FrozenEkfOperation::Kernel(
                    node.operation,
                ))
                .canonical_name,
            });
        }
    }

    for node in predicates {
        let expected = match node.operation {
            EkfPredicate::CandidateFinite => vec![
                kernel_output(EkfKernel::CorrectedState)?,
                kernel_output(EkfKernel::CovarianceSymmetrization)?,
            ],
            EkfPredicate::CovariancePositiveDiagonal | EkfPredicate::CovarianceSymmetric => {
                vec![kernel_output(EkfKernel::CovarianceSymmetrization)?]
            }
        };
        if node.inputs.as_ref() != expected.as_slice() {
            return Err(FrozenEkfArtifactClosureError::InvalidOperationWiring {
                node: node.node,
                operation: super::operation::operation_spec(FrozenEkfOperation::Predicate(
                    node.operation,
                ))
                .canonical_name,
            });
        }
    }

    if ArtifactSource::Slot(state.candidate) != kernel_output(EkfKernel::CorrectedState)?
        || ArtifactSource::Slot(covariance.candidate)
            != kernel_output(EkfKernel::CovarianceSymmetrization)?
    {
        return Err(FrozenEkfArtifactClosureError::InvalidStateUpdate);
    }
    Ok(())
}

fn validate_constraints(
    artifact: &ProgramArtifact,
    predicates: &[FrozenEkfPredicateNode],
) -> Result<Box<[FrozenEkfConstraint]>, FrozenEkfArtifactClosureError> {
    if artifact.constraints().len() != 3 {
        return Err(FrozenEkfArtifactClosureError::InvalidIntegrityConstraint);
    }
    let mut constraints = Vec::with_capacity(3);
    for constraint in artifact.constraints() {
        let Some(ResolvedOperationContract::Declared(contract)) =
            artifact.contracts().get(constraint.contract)
        else {
            return Err(FrozenEkfArtifactClosureError::InvalidIntegrityConstraint);
        };
        let [ArtifactSource::Slot(source)] = constraint.inputs.as_ref() else {
            return Err(FrozenEkfArtifactClosureError::InvalidIntegrityConstraint);
        };
        let predicate = predicates
            .iter()
            .find(|predicate| predicate.output == *source)
            .ok_or(FrozenEkfArtifactClosureError::InvalidIntegrityConstraint)?;
        if constraint.operation.module_path.as_ref() != ["integrity"]
            || constraint.operation.operation_name != "assert"
            || contract.interaction != ExternalInteraction::Pure
            || contract.inputs.len() != 1
            || contract.inputs[0].access != AccessMode::Read
            || contract.inputs[0].delivery != DeliveryMode::Signal
            || contract.outputs.len() != 0
        {
            return Err(FrozenEkfArtifactClosureError::InvalidIntegrityConstraint);
        }
        constraints.push(FrozenEkfConstraint {
            constraint: constraint.constraint,
            predicate: predicate.operation,
            source: *source,
        });
    }
    if predicates.iter().any(|predicate| {
        constraints
            .iter()
            .filter(|constraint| constraint.predicate == predicate.operation)
            .count()
            != 1
    }) {
        return Err(FrozenEkfArtifactClosureError::InvalidIntegrityConstraint);
    }
    Ok(constraints.into_boxed_slice())
}

pub(crate) fn value_f64s(artifact: &ProgramArtifact, constant: ConstantId) -> Option<Vec<f64>> {
    match artifact.constants().get(constant)?.data() {
        ValueData::F64(value) => Some(vec![value.to_f64()]),
        ValueData::Matrix(matrix) => match matrix.elements() {
            SequenceView::F64(values) => Some(values.iter().map(|value| value.to_f64()).collect()),
            _ => None,
        },
        _ => None,
    }
}

fn validate_resource_request(request: &ExecutionResourceRequest) -> Result<(), ()> {
    if request.intent == ResourceIntent::Read
        && request.delivery == ResourceDelivery::Live
        && request.operation == "read"
        && request.context_name == "frame"
        && request.base_uri == "gate-d://ekf/frame"
        && request.path == "sample"
    {
        Ok(())
    } else {
        Err(())
    }
}

#[derive(Clone, Debug)]
pub struct FrozenLiveBinding {
    pub interpreter_id: u64,
    pub request: ExecutionResourceRequest,
    pub target: ValueCell,
}

#[derive(Debug)]
pub struct FrozenEkfCompilationServices {
    frame: ValueCell,
    planning_frame: ValueCell,
    pub planned_reads: Vec<ExecutionResourceRequest>,
    pub reads: Vec<ExecutionResourceRequest>,
    pub live_bindings: Vec<FrozenLiveBinding>,
}

impl FrozenEkfCompilationServices {
    pub fn from_frozen_trace() -> Self {
        let frame: [f64; 4] = (0..4)
            .map(|index| {
                f64::from_le_bytes(
                    TRACE[index * 8..index * 8 + 8]
                        .try_into()
                        .expect("frozen frame contains four f64 values"),
                )
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("frozen frame contains exactly four f64 values");
        Self::from_frames(frame, [11.25, -0.375, 22.5, 0.125])
    }

    pub fn from_frames(frame: [f64; 4], planning_frame: [f64; 4]) -> Self {
        let frame_value = |values: [f64; 4]| {
            ValueCell::from_exact_matrix_ref(
                mech_core::Ref::new(DVector::from_vec(values.to_vec())),
                4,
                1,
            )
            .expect("the frozen EKF frame is a canonical four-element vector")
        };
        Self {
            frame: frame_value(frame),
            planning_frame: frame_value(planning_frame),
            planned_reads: Vec::new(),
            reads: Vec::new(),
            live_bindings: Vec::new(),
        }
    }

    fn validate_request(request: &ExecutionResourceRequest) -> MResult<()> {
        if validate_resource_request(request).is_err() {
            return Err(frozen_service_error(format!(
                "unexpected frozen EKF resource request: {request:?}"
            )));
        }
        Ok(())
    }
}

impl Default for FrozenEkfCompilationServices {
    fn default() -> Self {
        Self::from_frozen_trace()
    }
}

impl MechExecutionServices for FrozenEkfCompilationServices {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        _arguments: &[mech_core::Value],
    ) -> MResult<mech_core::Value> {
        Err(frozen_service_error(format!(
            "host call is outside the frozen EKF fixture: {request:?}"
        )))
    }

    fn plan_resource_read_output(
        &mut self,
        request: &ExecutionResourceRequest,
    ) -> MResult<mech_core::Value> {
        Self::validate_request(request)?;
        self.planned_reads.push(request.clone());
        self.planning_frame.snapshot()
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<mech_core::Value> {
        Self::validate_request(request)?;
        if self
            .reads
            .first()
            .is_some_and(|existing| existing != request)
        {
            return Err(frozen_service_error("multiple logical EKF observations"));
        }
        if self.reads.is_empty() {
            self.reads.push(request.clone());
        }
        self.frame.snapshot()
    }

    fn write_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        _value: &mech_core::Value,
    ) -> MResult<()> {
        Err(frozen_service_error(format!(
            "resource write is outside the frozen EKF fixture: {request:?}"
        )))
    }

    fn bind_live_resource(
        &mut self,
        interpreter_id: u64,
        request: &ExecutionResourceRequest,
        target: ValueCell,
    ) -> MResult<()> {
        Self::validate_request(request)?;
        if let Some(existing) = self
            .live_bindings
            .iter()
            .find(|binding| binding.interpreter_id == interpreter_id && binding.request == *request)
        {
            if existing.target.same_cell(&target) {
                return Ok(());
            }
            return Err(frozen_service_error(
                "live EKF observation rebound to a different target",
            ));
        }
        self.live_bindings.push(FrozenLiveBinding {
            interpreter_id,
            request: request.clone(),
            target,
        });
        Ok(())
    }
}

#[derive(Debug)]
pub struct FrozenEkfCompilation {
    pub source_artifact: ProgramArtifact,
    pub source_closure: FrozenEkfArtifactClosure,
    pub bytecode: Vec<u8>,
    pub decoded_artifact: ProgramArtifact,
    pub decoded_closure: FrozenEkfArtifactClosure,
    pub resource_request: ExecutionResourceRequest,
}

struct FrozenEkfCompiledProduct {
    source_artifact: ProgramArtifact,
    bytecode: Vec<u8>,
    decoded_artifact: ProgramArtifact,
    resource_request: ExecutionResourceRequest,
}

pub fn compile_frozen_ekf_source(
    source: &str,
    services: &mut dyn MechExecutionServices,
) -> MResult<FrozenEkfCompilation> {
    let product = compile_frozen_ekf_product(source, services)?;
    let source_closure =
        FrozenEkfArtifactClosure::admit(&product.source_artifact, &product.resource_request)
            .map_err(frozen_closure_error)?;
    let decoded_closure =
        FrozenEkfArtifactClosure::admit(&product.decoded_artifact, &product.resource_request)
            .map_err(frozen_closure_error)?;
    Ok(FrozenEkfCompilation {
        source_artifact: product.source_artifact,
        source_closure,
        bytecode: product.bytecode,
        decoded_artifact: product.decoded_artifact,
        decoded_closure,
        resource_request: product.resource_request,
    })
}

fn compile_frozen_ekf_product(
    source: &str,
    services: &mut dyn MechExecutionServices,
) -> MResult<FrozenEkfCompiledProduct> {
    let catalog = frozen_ekf_compiler_catalog()?;
    let mut program = CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        catalog.clone(),
    );
    for spec in FROZEN_EKF_OPERATIONS {
        let export = catalog
            .module_export("ekf", spec.module_item)
            .expect("the frozen catalog installs every EKF module export");
        program.bind_compiler_catalog_export(export, spec.canonical_name)?;
    }
    let tree = mech_syntax::parser::parse(source.trim())?;
    program.plan_tree_with_services(&tree, services)?;
    let (source_artifact, bytecode) = program.compile_program_product()?.into_parts();
    let parsed = mech_core::ParsedProgram::from_bytes(&bytecode)?;
    let resource_request = parsed
        .requirements
        .iter()
        .filter_map(|requirement| match requirement {
            mech_core::ApplicationRequirement::Resource(request)
                if request.intent == ResourceIntent::Read =>
            {
                Some(request.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let [resource_request] = resource_request.as_slice() else {
        return Err(frozen_service_error(format!(
            "expected one observation requirement, found {}",
            resource_request.len(),
        )));
    };
    FrozenEkfCompilationServices::validate_request(resource_request)?;
    let decoded_artifact = decode_program_artifact_bytecode_v1(&bytecode).map_err(|error| {
        frozen_service_error(format!(
            "unable to decode frozen EKF ProgramArtifact: {error:?}"
        ))
    })?;
    Ok(FrozenEkfCompiledProduct {
        source_artifact,
        bytecode,
        decoded_artifact,
        resource_request: resource_request.clone(),
    })
}

fn frozen_closure_error(error: FrozenEkfArtifactClosureError) -> MechError {
    MechError::new(error, None).with_compiler_loc()
}

fn frozen_service_error(message: impl Into<String>) -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str =
        include_str!("../../../../../tests/architecture/resident-activation/ekf-source-v1.mec");

    #[test]
    fn ordinary_source_reaches_the_program_artifact_compiler() -> MResult<()> {
        let mut services = FrozenEkfCompilationServices::default();
        let compilation = compile_frozen_ekf_source(SOURCE, &mut services)?;
        assert_eq!(
            compilation.source_artifact.revision(),
            compilation.decoded_artifact.revision()
        );
        assert_eq!(compilation.source_closure, compilation.decoded_closure);
        assert_eq!(compilation.source_closure.resident_kernels.len(), 15);
        assert_eq!(compilation.source_closure.integrity_predicates.len(), 3);
        assert_eq!(compilation.source_closure.state_updates.len(), 2);
        assert_eq!(compilation.source_closure.constraints.len(), 3);
        assert_eq!(services.reads.len(), 1);
        assert_eq!(services.live_bindings.len(), 1);
        Ok(())
    }

    fn state_initializers(artifact: &ProgramArtifact) -> (Vec<f64>, Vec<f64>) {
        let mut values = artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == crate::SlotRole::State)
            .map(|slot| {
                let crate::InitializerReference::Constant(initializer) =
                    slot.initializer.expect("state initializer");
                value_f64s(artifact, initializer).expect("numeric state initializer")
            })
            .collect::<Vec<_>>();
        values.sort_by_key(Vec::len);
        assert_eq!(values.len(), 2);
        (values.remove(0), values.remove(0))
    }

    #[test]
    fn generic_declaration_snapshots_are_the_initializer_authority() -> MResult<()> {
        let mut base_services = FrozenEkfCompilationServices::default();
        let base = compile_frozen_ekf_product(SOURCE, &mut base_services)?;
        assert_eq!(
            state_initializers(&base.source_artifact),
            (
                vec![2.0, 1.0, 0.15],
                vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05]
            )
        );
        assert_eq!(
            base.source_artifact.revision(),
            base.decoded_artifact.revision()
        );

        let changed_state =
            SOURCE.replace("~state := [2.0; 1.0; 0.15]", "~state := [3.0; 1.0; 0.15]");
        let mut state_services = FrozenEkfCompilationServices::default();
        let state = compile_frozen_ekf_product(&changed_state, &mut state_services)?;
        let (state_initializer, state_covariance) = state_initializers(&state.source_artifact);
        assert_eq!(state_initializer, vec![3.0, 1.0, 0.15]);
        assert_eq!(
            state_covariance,
            state_initializers(&base.source_artifact).1
        );
        assert_ne!(
            state.source_artifact.revision(),
            base.source_artifact.revision()
        );
        assert_eq!(
            state.source_artifact.revision(),
            state.decoded_artifact.revision()
        );

        let changed_covariance = SOURCE.replace(
            "~covariance := [1.0, 0.0, 0.0; 0.0, 1.0, 0.0; 0.0, 0.0, 0.05]",
            "~covariance := [2.0, 0.0, 0.0; 0.0, 1.0, 0.0; 0.0, 0.0, 0.05]",
        );
        let mut covariance_services = FrozenEkfCompilationServices::default();
        let covariance = compile_frozen_ekf_product(&changed_covariance, &mut covariance_services)?;
        let (covariance_state, covariance_initializer) =
            state_initializers(&covariance.source_artifact);
        assert_eq!(
            covariance_state,
            state_initializers(&base.source_artifact).0
        );
        assert_eq!(
            covariance_initializer,
            vec![2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05]
        );
        assert_ne!(
            covariance.source_artifact.revision(),
            base.source_artifact.revision()
        );
        assert_eq!(
            covariance.source_artifact.revision(),
            covariance.decoded_artifact.revision()
        );
        Ok(())
    }

    #[test]
    fn observation_payload_does_not_change_initializers_or_revision_before_admission() -> MResult<()>
    {
        let mut left_services =
            FrozenEkfCompilationServices::from_frames([1.0, 2.0, 3.0, 4.0], [5.0, 6.0, 7.0, 8.0]);
        let mut right_services = FrozenEkfCompilationServices::from_frames(
            [-10.0, 20.0, -30.0, 40.0],
            [50.0, -60.0, 70.0, -80.0],
        );
        let left = compile_frozen_ekf_product(SOURCE, &mut left_services)?;
        let right = compile_frozen_ekf_product(SOURCE, &mut right_services)?;
        assert_eq!(
            state_initializers(&left.source_artifact),
            state_initializers(&right.source_artifact)
        );
        assert_eq!(
            left.source_artifact.revision(),
            right.source_artifact.revision()
        );
        assert_eq!(left.bytecode, right.bytecode);
        Ok(())
    }

    #[test]
    fn second_observation_root_is_rejected_before_activation() {
        let source = SOURCE.replacen(
            "frame := @trace/sample",
            "frame := @trace/sample\n@trace-2 := gate-d://ekf/frame{:read(sample)}\nframe-2 := @trace-2/sample",
            1,
        );
        let mut services = FrozenEkfCompilationServices::default();
        let error = compile_frozen_ekf_source(&source, &mut services).unwrap_err();
        assert!(error.display_message().contains("observation"));
    }

    #[test]
    fn always_changed_is_rejected_for_pure_kernels_and_state_updates() {
        assert!(!valid_pure_change_detection(
            ChangeDetectionPolicy::AlwaysChanged,
            ChangeDetectionPolicy::KernelReported,
        ));
        assert!(!valid_pure_change_detection(
            ChangeDetectionPolicy::AlwaysChanged,
            ChangeDetectionPolicy::ExactScalar,
        ));
    }

    #[test]
    fn same_shaped_noncanonical_operation_wiring_is_rejected() {
        let rewired_sources = [
            SOURCE.replace(
                "ekf/predicted-measurement(predicted-state,\n  delta-range)",
                "ekf/predicted-measurement(delta-range,\n  predicted-state)",
            ),
            SOURCE.replace(
                "gain,\n  measurement-covariance)",
                "gain,\n  process-covariance)",
            ),
        ];
        for source in rewired_sources {
            let mut services = FrozenEkfCompilationServices::default();
            let error = compile_frozen_ekf_source(&source, &mut services).unwrap_err();
            assert!(
                error.display_message().contains("InvalidOperationWiring"),
                "unexpected admission error: {error:?}",
            );
        }
    }
}
