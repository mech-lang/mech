//! Activation of the closed public EKF [`ProgramArtifact`](crate::ProgramArtifact).
//!
//! This is deliberately separate from the private Gate B control. Every logical
//! and physical binding in this module is derived from the finalized public
//! artifact and its admitted semantic closure.

use core::ops::Range;
use core::sync::atomic::AtomicU64;

use mech_core::{
    CellSlotId, InstanceEpoch, LayoutGeneration, NodeId, PlanGeneration, ProgramRevision,
    ReactiveInstanceId, SchemaId, SlotIndex,
};

use crate::efficacy::ekf::closure::{
    FrozenEkfArtifactClosure, FrozenEkfArtifactClosureError, FrozenEkfConstraint,
    FrozenEkfStateUpdate, value_f64s,
};
use crate::efficacy::ekf::operation::{EkfConstants, EkfKernel, EkfPredicate, EkfScratch};
use crate::{ArtifactSource, ProgramArtifact};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EkfScratchSlot {
    TrigonometricState,
    MotionJacobian,
    ControlJacobian,
    PredictedState,
    PredictedCovariance,
    LandmarkDeltaAndRange,
    PredictedMeasurement,
    MeasurementJacobian,
    InnovationCovariance,
    InverseInnovation,
    KalmanGain,
    Innovation,
    CorrectedState,
    CorrectedCovariance,
    SymmetrizedCovariance,
}

impl From<EkfKernel> for EkfScratchSlot {
    fn from(value: EkfKernel) -> Self {
        match value {
            EkfKernel::TrigonometricState => Self::TrigonometricState,
            EkfKernel::MotionJacobian => Self::MotionJacobian,
            EkfKernel::ControlJacobian => Self::ControlJacobian,
            EkfKernel::PredictedState => Self::PredictedState,
            EkfKernel::PredictedCovariance => Self::PredictedCovariance,
            EkfKernel::LandmarkDeltaAndRange => Self::LandmarkDeltaAndRange,
            EkfKernel::PredictedMeasurement => Self::PredictedMeasurement,
            EkfKernel::MeasurementJacobian => Self::MeasurementJacobian,
            EkfKernel::InnovationCovariance => Self::InnovationCovariance,
            EkfKernel::Solve2x2 => Self::InverseInnovation,
            EkfKernel::KalmanGain => Self::KalmanGain,
            EkfKernel::Innovation => Self::Innovation,
            EkfKernel::CorrectedState => Self::CorrectedState,
            EkfKernel::JosephCovarianceUpdate => Self::CorrectedCovariance,
            EkfKernel::CovarianceSymmetrization => Self::SymmetrizedCovariance,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EkfPredicateSlot {
    CandidateFinite,
    CovariancePositiveDiagonal,
    CovarianceSymmetric,
}

impl From<EkfPredicate> for EkfPredicateSlot {
    fn from(value: EkfPredicate) -> Self {
        match value {
            EkfPredicate::CandidateFinite => Self::CandidateFinite,
            EkfPredicate::CovariancePositiveDiagonal => Self::CovariancePositiveDiagonal,
            EkfPredicate::CovarianceSymmetric => Self::CovarianceSymmetric,
        }
    }
}

impl EkfPredicateSlot {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::CandidateFinite => 0,
            Self::CovariancePositiveDiagonal => 1,
            Self::CovarianceSymmetric => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentStorageLocation {
    InputFrame,
    State,
    Covariance,
    Scratch(EkfScratchSlot),
    Predicate(EkfPredicateSlot),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActivatedRead {
    ConstantF64(f64),
    ConstantVector2([f64; 2]),
    ConstantMatrix2([f64; 4]),
    InputFrame,
    PublishedState,
    PublishedCovariance,
    Scratch(EkfScratchSlot),
    Predicate(EkfPredicateSlot),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivatedWrite {
    CandidateState,
    CandidateCovariance,
    Scratch(EkfScratchSlot),
    Predicate(EkfPredicateSlot),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivatedNodeKind {
    Kernel(EkfKernel),
    Predicate(EkfPredicate),
    StateCopy {
        source: ActivatedReadLocation,
        target: ActivatedWrite,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivatedReadLocation {
    Scratch(EkfScratchSlot),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ActivatedNodeIndex(pub(crate) u32);

impl ActivatedNodeIndex {
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedSlot {
    pub artifact_id: CellSlotId,
    pub physical_index: SlotIndex,
    pub schema: SchemaId,
    pub location: ResidentStorageLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivatedNode {
    pub artifact_node: NodeId,
    pub kind: ActivatedNodeKind,
    pub reads: Range<u32>,
    pub writes: Range<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivatedConstraint {
    pub predicate: EkfPredicateSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivatedInput {
    pub slot: SlotIndex,
    pub schema: SchemaId,
    pub location: ResidentStorageLocation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivatedOutput {
    pub slot: SlotIndex,
    pub schema: SchemaId,
    pub location: ResidentStorageLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTopology {
    pub linear_node_order: Box<[ActivatedNodeIndex]>,
    pub same_turn_downstream_offsets: Box<[u32]>,
    pub same_turn_downstream_nodes: Box<[ActivatedNodeIndex]>,
    pub turn_root_nodes: Box<[ActivatedNodeIndex]>,
    pub(crate) same_turn_downstream_masks: Box<[u32]>,
    pub(crate) turn_root_mask: u32,
    pub(crate) mandatory_candidate_mask: u32,
}

impl DependencyTopology {
    pub fn same_turn_downstream(&self, node: ActivatedNodeIndex) -> &[ActivatedNodeIndex] {
        let index = node.0 as usize;
        let start = self.same_turn_downstream_offsets[index] as usize;
        let end = self.same_turn_downstream_offsets[index + 1] as usize;
        &self.same_turn_downstream_nodes[start..end]
    }
}

#[derive(Clone, Debug)]
pub struct ActivatedPlan {
    pub program_revision: ProgramRevision,
    pub plan_generation: PlanGeneration,
    pub layout_generation: LayoutGeneration,
    pub slots: Box<[ResolvedSlot]>,
    pub nodes: Box<[ActivatedNode]>,
    pub reads: Box<[ActivatedRead]>,
    pub writes: Box<[ActivatedWrite]>,
    pub topology: DependencyTopology,
    pub constraints: Box<[ActivatedConstraint]>,
    pub input: ActivatedInput,
    pub output: ActivatedOutput,
    pub(crate) state_slot: SlotIndex,
    pub(crate) covariance_slot: SlotIndex,
    pub(crate) constants: EkfConstants,
}

#[derive(Clone, Debug)]
pub(crate) struct VersionedValue<T> {
    pub(crate) buffers: [T; 2],
    pub(crate) epochs: [Option<InstanceEpoch>; 2],
}

impl<T: Copy> VersionedValue<T> {
    fn activate(initial: T) -> Self {
        Self {
            buffers: [initial, initial],
            epochs: [Some(InstanceEpoch::ZERO), None],
        }
    }

    pub(crate) fn published_index(&self, epoch: InstanceEpoch) -> usize {
        match self.epochs {
            [Some(left), _] if left == epoch => 0,
            [_, Some(right)] if right == epoch => 1,
            _ => panic!("published resident epoch has no typed artifact buffer"),
        }
    }

    pub(crate) fn begin_candidate(
        &mut self,
        base: InstanceEpoch,
        working: InstanceEpoch,
    ) -> (usize, usize) {
        let published = self.published_index(base);
        let candidate = 1 - published;
        self.epochs[candidate] = Some(working);
        (published, candidate)
    }

    pub(crate) fn reject(&mut self, candidate: usize, working: InstanceEpoch) {
        debug_assert_eq!(self.epochs[candidate], Some(working));
        self.epochs[candidate] = None;
    }
}

#[derive(Clone, Debug)]
pub struct StateArena {
    pub(crate) state: VersionedValue<[f64; 3]>,
    pub(crate) covariance: VersionedValue<[f64; 9]>,
}

impl StateArena {
    fn activate(initial_state: [f64; 3], initial_covariance: [f64; 9]) -> Self {
        Self {
            state: VersionedValue::activate(initial_state),
            covariance: VersionedValue::activate(initial_covariance),
        }
    }

    pub fn candidate_bytes(&self) -> usize {
        core::mem::size_of::<[f64; 3]>() + core::mem::size_of::<[f64; 9]>()
    }

    pub(crate) fn begin_candidate(
        &mut self,
        base: InstanceEpoch,
        working: InstanceEpoch,
    ) -> (usize, usize) {
        let state = self.state.begin_candidate(base, working);
        let covariance = self.covariance.begin_candidate(base, working);
        debug_assert_eq!(state, covariance);
        state
    }

    pub(crate) fn reject_candidate(&mut self, candidate: usize, working: InstanceEpoch) {
        self.state.reject(candidate, working);
        self.covariance.reject(candidate, working);
    }
}

#[derive(Debug)]
pub struct TurnWorkspace {
    pub(crate) input: [f64; 4],
    pub(crate) scratch: EkfScratch,
    pub(crate) predicates: [bool; 3],
    pub(crate) slot_epoch_marks: Box<[InstanceEpoch]>,
    pub(crate) touched_slots: Vec<SlotIndex>,
    pub(crate) changed_slots: Vec<SlotIndex>,
    pub(crate) dirty_node_mask: u32,
    pub(crate) executed_node_mask: u32,
    pub(crate) initialized_node_mask: u32,
}

impl TurnWorkspace {
    fn activate(plan: &ActivatedPlan) -> Self {
        Self {
            input: [0.0; 4],
            scratch: EkfScratch::default(),
            predicates: [false; 3],
            slot_epoch_marks: vec![InstanceEpoch::ZERO; plan.slots.len()].into_boxed_slice(),
            touched_slots: Vec::with_capacity(2),
            changed_slots: Vec::with_capacity(2),
            dirty_node_mask: 0,
            executed_node_mask: 0,
            initialized_node_mask: 0,
        }
    }

    pub fn input(&self) -> &[f64; 4] {
        &self.input
    }

    pub fn predicate_values(&self) -> &[bool; 3] {
        &self.predicates
    }

    pub fn executed_node_count(&self) -> usize {
        self.executed_node_mask.count_ones() as usize
    }

    pub(crate) fn begin(&mut self, frame: [f64; 4]) {
        self.input = frame;
        self.touched_slots.clear();
        self.changed_slots.clear();
        self.dirty_node_mask = 0;
        self.executed_node_mask = 0;
    }

    pub(crate) fn record_execution(&mut self, node: ActivatedNodeIndex) {
        self.executed_node_mask |= 1_u32 << node.0;
    }

    pub(crate) fn record_dirty(&mut self, node: ActivatedNodeIndex) {
        self.dirty_node_mask |= 1_u32 << node.0;
    }

    pub(crate) fn output_initialized(&self, node: ActivatedNodeIndex) -> bool {
        self.initialized_node_mask & (1_u32 << node.0) != 0
    }

    pub(crate) fn record_output_initialized(&mut self, node: ActivatedNodeIndex) {
        self.initialized_node_mask |= 1_u32 << node.0;
    }
}

#[derive(Debug)]
pub struct ReactiveInstance {
    pub id: ReactiveInstanceId,
    pub plan: ActivatedPlan,
    pub state: StateArena,
    pub workspace: TurnWorkspace,
    pub(crate) published_epoch: AtomicU64,
    pub(crate) next_epoch: Option<InstanceEpoch>,
}

impl ReactiveInstance {
    pub fn published_epoch(&self) -> InstanceEpoch {
        InstanceEpoch::new(
            self.published_epoch
                .load(core::sync::atomic::Ordering::Acquire),
        )
    }

    pub fn next_epoch(&self) -> Option<InstanceEpoch> {
        self.next_epoch
    }

    pub fn logical_binding_projection(&self) -> ActivationProjection {
        ActivationProjection {
            program_revision: self.plan.program_revision,
            slots: self
                .plan
                .slots
                .iter()
                .map(|slot| (slot.artifact_id, slot.physical_index, slot.location))
                .collect(),
            nodes: self
                .plan
                .nodes
                .iter()
                .map(|node| (node.artifact_node, node.kind))
                .collect(),
            constant_bits: constants_bits(&self.plan.constants),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationProjection {
    pub program_revision: ProgramRevision,
    pub slots: Box<[(CellSlotId, SlotIndex, ResidentStorageLocation)]>,
    pub nodes: Box<[(NodeId, ActivatedNodeKind)]>,
    pub constant_bits: Box<[u64]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentActivationError {
    ArtifactClosure(FrozenEkfArtifactClosureError),
    NonDenseSlot {
        slot: CellSlotId,
        expected: CellSlotId,
    },
    DuplicatePhysicalBinding {
        slot: CellSlotId,
    },
    MissingPhysicalBinding {
        slot: CellSlotId,
    },
    UnsupportedRead {
        node: NodeId,
        source: ArtifactSource,
    },
    UnsupportedWrite {
        node: NodeId,
        slot: CellSlotId,
    },
    InvalidConstantRepresentation,
    MissingStateInitializer,
    InvalidDependency {
        node: NodeId,
    },
}

impl From<FrozenEkfArtifactClosureError> for ResidentActivationError {
    fn from(value: FrozenEkfArtifactClosureError) -> Self {
        Self::ArtifactClosure(value)
    }
}

pub fn activate(
    id: ReactiveInstanceId,
    artifact: &ProgramArtifact,
    request: &mech_core::ExecutionResourceRequest,
) -> Result<ReactiveInstance, ResidentActivationError> {
    // Admission is intentionally repeated here. A closure produced earlier by
    // compilation is evidence, not authority at the runtime boundary.
    let closure = FrozenEkfArtifactClosure::admit(artifact, request)?;
    let plan = build_plan(artifact, &closure)?;
    let (initial_state, initial_covariance) = initial_state(artifact, &closure)?;

    // Persistent storage is allocated only after all semantic resolution has
    // succeeded. Activation itself never begins or executes a candidate turn.
    let state = StateArena::activate(initial_state, initial_covariance);
    let workspace = TurnWorkspace::activate(&plan);
    Ok(ReactiveInstance {
        id,
        plan,
        state,
        workspace,
        published_epoch: AtomicU64::new(InstanceEpoch::ZERO.get()),
        next_epoch: Some(InstanceEpoch::new(1)),
    })
}

fn build_plan(
    artifact: &ProgramArtifact,
    closure: &FrozenEkfArtifactClosure,
) -> Result<ActivatedPlan, ResidentActivationError> {
    let mut locations = vec![None; artifact.slots().len()];
    for (index, slot) in artifact.slots().iter().enumerate() {
        let expected = CellSlotId::new(index as u32);
        if slot.slot != expected {
            return Err(ResidentActivationError::NonDenseSlot {
                slot: slot.slot,
                expected,
            });
        }
    }
    bind_location(
        &mut locations,
        closure.input.slot,
        ResidentStorageLocation::InputFrame,
    )?;
    for update in &closure.state_updates {
        let location = if update.target == closure.output.source {
            ResidentStorageLocation::State
        } else {
            ResidentStorageLocation::Covariance
        };
        bind_location(&mut locations, update.target, location)?;
    }
    for node in &closure.resident_kernels {
        bind_location(
            &mut locations,
            node.output,
            ResidentStorageLocation::Scratch(node.operation.into()),
        )?;
    }
    for node in &closure.integrity_predicates {
        bind_location(
            &mut locations,
            node.output,
            ResidentStorageLocation::Predicate(node.operation.into()),
        )?;
    }

    let slots = artifact
        .slots()
        .iter()
        .enumerate()
        .map(
            |(index, slot)| -> Result<ResolvedSlot, ResidentActivationError> {
                Ok(ResolvedSlot {
                    artifact_id: slot.slot,
                    physical_index: SlotIndex::new(index as u32),
                    schema: slot.schema,
                    location: locations[index].ok_or(
                        ResidentActivationError::MissingPhysicalBinding { slot: slot.slot },
                    )?,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

    let mut reads = Vec::new();
    let mut writes = Vec::new();
    let mut nodes = Vec::with_capacity(20);
    let mut artifact_to_activated = vec![None; artifact.nodes().len()];
    for declaration in artifact.nodes() {
        if declaration.node == closure.input.observation_node {
            continue;
        }
        let read_start = reads.len() as u32;
        let write_start = writes.len() as u32;
        let kind = if let Some(kernel) = closure
            .resident_kernels
            .iter()
            .find(|node| node.node == declaration.node)
        {
            reads.extend(
                kernel
                    .inputs
                    .iter()
                    .map(|source| {
                        activated_read(artifact, closure, &locations, declaration.node, *source)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
            writes.push(ActivatedWrite::Scratch(kernel.operation.into()));
            ActivatedNodeKind::Kernel(kernel.operation)
        } else if let Some(predicate) = closure
            .integrity_predicates
            .iter()
            .find(|node| node.node == declaration.node)
        {
            reads.extend(
                predicate
                    .inputs
                    .iter()
                    .map(|source| {
                        activated_read(artifact, closure, &locations, declaration.node, *source)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
            writes.push(ActivatedWrite::Predicate(predicate.operation.into()));
            ActivatedNodeKind::Predicate(predicate.operation)
        } else if let Some(update) = closure
            .state_updates
            .iter()
            .find(|update| update.node == declaration.node)
        {
            let source = scratch_source(&locations, update)?;
            let target = match locations[update.target.get() as usize] {
                Some(ResidentStorageLocation::State) => ActivatedWrite::CandidateState,
                Some(ResidentStorageLocation::Covariance) => ActivatedWrite::CandidateCovariance,
                _ => {
                    return Err(ResidentActivationError::UnsupportedWrite {
                        node: declaration.node,
                        slot: update.target,
                    });
                }
            };
            reads.push(ActivatedRead::Scratch(source));
            writes.push(target);
            ActivatedNodeKind::StateCopy {
                source: ActivatedReadLocation::Scratch(source),
                target,
            }
        } else {
            return Err(ResidentActivationError::InvalidDependency {
                node: declaration.node,
            });
        };
        let activated_index = ActivatedNodeIndex(nodes.len() as u32);
        artifact_to_activated[declaration.node.get() as usize] = Some(activated_index);
        nodes.push(ActivatedNode {
            artifact_node: declaration.node,
            kind,
            reads: read_start..reads.len() as u32,
            writes: write_start..writes.len() as u32,
        });
    }
    if nodes.len() != 20 {
        return Err(ResidentActivationError::InvalidDependency {
            node: closure.input.observation_node,
        });
    }
    let topology = build_topology(artifact, closure, &nodes, &artifact_to_activated)?;
    let constants = materialize_constants(artifact, closure)?;
    let constraints = closure
        .constraints
        .iter()
        .map(activated_constraint)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let input_slot = SlotIndex::new(closure.input.slot.get());
    let output_slot = SlotIndex::new(closure.output.source.get());
    let covariance_slot = slots
        .iter()
        .find(|slot| slot.location == ResidentStorageLocation::Covariance)
        .map(|slot| slot.physical_index)
        .ok_or(ResidentActivationError::MissingPhysicalBinding {
            slot: closure.output.source,
        })?;
    Ok(ActivatedPlan {
        program_revision: closure.program_revision,
        plan_generation: PlanGeneration::ZERO,
        layout_generation: LayoutGeneration::ZERO,
        slots: slots.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        reads: reads.into_boxed_slice(),
        writes: writes.into_boxed_slice(),
        topology,
        constraints,
        input: ActivatedInput {
            slot: input_slot,
            schema: closure.input.schema,
            location: ResidentStorageLocation::InputFrame,
        },
        output: ActivatedOutput {
            slot: output_slot,
            schema: closure.output.schema,
            location: ResidentStorageLocation::State,
        },
        state_slot: output_slot,
        covariance_slot,
        constants,
    })
}

fn bind_location(
    locations: &mut [Option<ResidentStorageLocation>],
    slot: CellSlotId,
    location: ResidentStorageLocation,
) -> Result<(), ResidentActivationError> {
    let target = locations
        .get_mut(slot.get() as usize)
        .ok_or(ResidentActivationError::MissingPhysicalBinding { slot })?;
    if target.replace(location).is_some() {
        return Err(ResidentActivationError::DuplicatePhysicalBinding { slot });
    }
    Ok(())
}

fn activated_read(
    artifact: &ProgramArtifact,
    closure: &FrozenEkfArtifactClosure,
    locations: &[Option<ResidentStorageLocation>],
    node: NodeId,
    source: ArtifactSource,
) -> Result<ActivatedRead, ResidentActivationError> {
    match source {
        ArtifactSource::Slot(slot) => match locations.get(slot.get() as usize).copied().flatten() {
            Some(ResidentStorageLocation::InputFrame) => Ok(ActivatedRead::InputFrame),
            Some(ResidentStorageLocation::State) => Ok(ActivatedRead::PublishedState),
            Some(ResidentStorageLocation::Covariance) => Ok(ActivatedRead::PublishedCovariance),
            Some(ResidentStorageLocation::Scratch(slot)) => Ok(ActivatedRead::Scratch(slot)),
            Some(ResidentStorageLocation::Predicate(slot)) => Ok(ActivatedRead::Predicate(slot)),
            None => Err(ResidentActivationError::UnsupportedRead { node, source }),
        },
        ArtifactSource::Constant(constant) => {
            let values = value_f64s(artifact, constant)
                .ok_or(ResidentActivationError::InvalidConstantRepresentation)?;
            if constant == closure.constants.dt {
                let [value] = values.as_slice() else {
                    return Err(ResidentActivationError::InvalidConstantRepresentation);
                };
                Ok(ActivatedRead::ConstantF64(*value))
            } else if constant == closure.constants.landmark {
                Ok(ActivatedRead::ConstantVector2(array(values)?))
            } else if constant == closure.constants.process_covariance
                || constant == closure.constants.measurement_covariance
            {
                Ok(ActivatedRead::ConstantMatrix2(array(values)?))
            } else {
                Err(ResidentActivationError::UnsupportedRead { node, source })
            }
        }
    }
}

fn scratch_source(
    locations: &[Option<ResidentStorageLocation>],
    update: &FrozenEkfStateUpdate,
) -> Result<EkfScratchSlot, ResidentActivationError> {
    match locations
        .get(update.candidate.get() as usize)
        .copied()
        .flatten()
    {
        Some(ResidentStorageLocation::Scratch(slot)) => Ok(slot),
        _ => Err(ResidentActivationError::UnsupportedRead {
            node: update.node,
            source: ArtifactSource::Slot(update.candidate),
        }),
    }
}

fn activated_constraint(constraint: &FrozenEkfConstraint) -> ActivatedConstraint {
    ActivatedConstraint {
        predicate: constraint.predicate.into(),
    }
}

fn build_topology(
    artifact: &ProgramArtifact,
    closure: &FrozenEkfArtifactClosure,
    nodes: &[ActivatedNode],
    artifact_to_activated: &[Option<ActivatedNodeIndex>],
) -> Result<DependencyTopology, ResidentActivationError> {
    let mut producer = vec![None; artifact.slots().len()];
    for node in closure.resident_kernels.iter() {
        producer[node.output.get() as usize] = artifact_to_activated[node.node.get() as usize];
    }
    for node in closure.integrity_predicates.iter() {
        producer[node.output.get() as usize] = artifact_to_activated[node.node.get() as usize];
    }
    let mut downstream = vec![Vec::new(); nodes.len()];
    let mut roots = Vec::new();
    for (node_index, node) in nodes.iter().enumerate() {
        let activated = ActivatedNodeIndex(node_index as u32);
        let reads = &artifact.bindings()[artifact.nodes()[node.artifact_node.get() as usize]
            .input_bindings
            .start as usize
            ..artifact.nodes()[node.artifact_node.get() as usize]
                .input_bindings
                .end as usize];
        let mut has_turn_root = false;
        for binding in reads {
            let crate::BindingDeclaration::Input { source, .. } = binding else {
                return Err(ResidentActivationError::InvalidDependency {
                    node: node.artifact_node,
                });
            };
            match source {
                ArtifactSource::Constant(_) => {}
                ArtifactSource::Slot(slot) => {
                    if *slot == closure.input.slot
                        || closure
                            .state_updates
                            .iter()
                            .any(|update| update.target == *slot)
                    {
                        has_turn_root = true;
                    } else if let Some(parent) = producer[slot.get() as usize] {
                        if parent.0 >= activated.0 {
                            return Err(ResidentActivationError::InvalidDependency {
                                node: node.artifact_node,
                            });
                        }
                        if !downstream[parent.0 as usize].contains(&activated) {
                            downstream[parent.0 as usize].push(activated);
                        }
                    }
                }
            }
        }
        if has_turn_root {
            roots.push(activated);
        }
    }
    for list in &mut downstream {
        list.sort_by_key(|node| node.0);
    }
    roots.sort_by_key(|node| node.0);
    roots.dedup();
    debug_assert!(nodes.len() <= u32::BITS as usize);
    let downstream_masks = downstream
        .iter()
        .map(|nodes| {
            nodes
                .iter()
                .fold(0_u32, |mask, node| mask | (1_u32 << node.0))
        })
        .collect::<Box<[_]>>();
    let turn_root_mask = roots
        .iter()
        .fold(0_u32, |mask, node| mask | (1_u32 << node.0));
    let mandatory_candidate_mask = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            matches!(
                node.kind,
                ActivatedNodeKind::Predicate(_) | ActivatedNodeKind::StateCopy { .. }
            )
        })
        .fold(0_u32, |mask, (index, _)| mask | (1_u32 << index));
    let mut offsets = Vec::with_capacity(nodes.len() + 1);
    let mut values = Vec::new();
    offsets.push(0);
    for list in downstream {
        values.extend(list);
        offsets.push(values.len() as u32);
    }
    Ok(DependencyTopology {
        linear_node_order: (0..nodes.len())
            .map(|index| ActivatedNodeIndex(index as u32))
            .collect(),
        same_turn_downstream_offsets: offsets.into_boxed_slice(),
        same_turn_downstream_nodes: values.into_boxed_slice(),
        turn_root_nodes: roots.into_boxed_slice(),
        same_turn_downstream_masks: downstream_masks,
        turn_root_mask,
        mandatory_candidate_mask,
    })
}

fn materialize_constants(
    artifact: &ProgramArtifact,
    closure: &FrozenEkfArtifactClosure,
) -> Result<EkfConstants, ResidentActivationError> {
    Ok(EkfConstants {
        dt: array::<1>(
            value_f64s(artifact, closure.constants.dt)
                .ok_or(ResidentActivationError::InvalidConstantRepresentation)?,
        )?[0],
        landmark: array(
            value_f64s(artifact, closure.constants.landmark)
                .ok_or(ResidentActivationError::InvalidConstantRepresentation)?,
        )?,
        process_covariance: array(
            value_f64s(artifact, closure.constants.process_covariance)
                .ok_or(ResidentActivationError::InvalidConstantRepresentation)?,
        )?,
        measurement_covariance: array(
            value_f64s(artifact, closure.constants.measurement_covariance)
                .ok_or(ResidentActivationError::InvalidConstantRepresentation)?,
        )?,
    })
}

fn initial_state(
    artifact: &ProgramArtifact,
    closure: &FrozenEkfArtifactClosure,
) -> Result<([f64; 3], [f64; 9]), ResidentActivationError> {
    let state = closure
        .state_updates
        .iter()
        .find(|update| update.target == closure.output.source)
        .ok_or(ResidentActivationError::MissingStateInitializer)?;
    let covariance = closure
        .state_updates
        .iter()
        .find(|update| update.target != closure.output.source)
        .ok_or(ResidentActivationError::MissingStateInitializer)?;
    Ok((
        array(
            value_f64s(artifact, state.initializer)
                .ok_or(ResidentActivationError::MissingStateInitializer)?,
        )?,
        array(
            value_f64s(artifact, covariance.initializer)
                .ok_or(ResidentActivationError::MissingStateInitializer)?,
        )?,
    ))
}

fn array<const N: usize>(values: Vec<f64>) -> Result<[f64; N], ResidentActivationError> {
    values
        .try_into()
        .map_err(|_| ResidentActivationError::InvalidConstantRepresentation)
}

fn constants_bits(constants: &EkfConstants) -> Box<[u64]> {
    core::iter::once(constants.dt)
        .chain(constants.landmark)
        .chain(constants.process_covariance)
        .chain(constants.measurement_covariance)
        .map(f64::to_bits)
        .collect()
}
