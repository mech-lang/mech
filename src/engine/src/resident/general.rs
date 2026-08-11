//! Schema-driven activation for the pre-launch dense numeric resident profile.

use core::ops::Range;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeMap, BTreeSet};

use mech_core::snapshot::{F64Bits, SequenceView, SnapshotValidationContext};
use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, CellSlotId, ChangeDetectionPolicy, ConstantId,
    DeliveryMode, DimensionExpr, DimensionLifetime, ExternalInteraction, FunctionCatalog,
    InstanceEpoch, IntegrityConstraintId, LayoutGeneration, NodeId, ObservationReplayPolicy,
    OutputConstruction, PlanGeneration, ProgramRevision, ReactiveInstanceId,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentPortLayout, ResidentShape,
    ResidentValueKind, ResidentValueMut, ResidentValueRef, SchemaBody, SchemaId, SchemaKey,
    ShapeInstance, SlotIndex, Value, ValueData, ValueDataDraft, ValueDraft,
};

use crate::{
    ArtifactSource, BindingDeclaration, InitializerReference, ProducerReference, ProgramArtifact,
    SlotRole,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentStorageClass {
    Constant,
    Input,
    State,
    Scratch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentRegion {
    pub kind: ResidentValueKind,
    pub offset: usize,
    pub len: usize,
    pub shape: ResidentShape,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSlot {
    pub artifact_id: CellSlotId,
    pub physical_index: SlotIndex,
    pub schema: SchemaId,
    pub schema_key: SchemaKey,
    pub shape: ShapeInstance,
    pub storage: ResidentStorageClass,
    pub region: ResidentRegion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentReadLocation {
    Constant(ResidentRegion),
    Input(ResidentRegion),
    State {
        slot: CellSlotId,
        region: ResidentRegion,
    },
    Scratch(ResidentRegion),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentWriteLocation {
    pub slot: CellSlotId,
    pub storage: ResidentStorageClass,
    pub region: ResidentRegion,
}

#[derive(Clone, Debug)]
pub struct ActivatedNode {
    pub artifact_node: NodeId,
    pub reads: Range<u32>,
    pub write: ResidentWriteLocation,
    pub construction: OutputConstruction,
    pub change_detection: ChangeDetectionPolicy,
    pub kernel: BoundResidentKernel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ActivatedNodeIndex(u32);

impl ActivatedNodeIndex {
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTopology {
    pub linear_node_order: Box<[ActivatedNodeIndex]>,
    pub same_turn_downstream_offsets: Box<[u32]>,
    pub same_turn_downstream_nodes: Box<[ActivatedNodeIndex]>,
    pub turn_root_nodes: Box<[ActivatedNodeIndex]>,
    pub same_turn_downstream_masks: Box<[Box<[u64]>]>,
    pub turn_root_mask: Box<[u64]>,
    pub mandatory_candidate_mask: Box<[u64]>,
}

impl DependencyTopology {
    pub fn same_turn_downstream(&self, node: ActivatedNodeIndex) -> &[ActivatedNodeIndex] {
        let index = node.0 as usize;
        let start = self.same_turn_downstream_offsets[index] as usize;
        let end = self.same_turn_downstream_offsets[index + 1] as usize;
        &self.same_turn_downstream_nodes[start..end]
    }

    pub fn word_len(&self) -> usize {
        self.turn_root_mask.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivatedInput {
    pub slot: SlotIndex,
    pub schema: SchemaId,
    pub region: ResidentRegion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivatedOutput {
    pub slot: SlotIndex,
    pub schema: SchemaId,
    pub schema_key: SchemaKey,
    pub shape: ShapeInstance,
    pub region: ResidentRegion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivatedConstraint {
    pub artifact_id: IntegrityConstraintId,
    pub predicate: ResidentReadLocation,
}

#[derive(Clone, Debug)]
pub struct ActivatedPlan {
    pub program_revision: ProgramRevision,
    pub plan_generation: PlanGeneration,
    pub layout_generation: LayoutGeneration,
    pub slots: Box<[ResolvedSlot]>,
    pub nodes: Box<[ActivatedNode]>,
    pub reads: Box<[ResidentReadLocation]>,
    pub topology: DependencyTopology,
    pub inputs: Box<[ActivatedInput]>,
    pub outputs: Box<[ActivatedOutput]>,
    pub constraints: Box<[ActivatedConstraint]>,
    pub activation_nodes: Box<[NodeId]>,
    activation_steps: Box<[ActivatedOnceNode]>,
    pub(crate) schemas: mech_core::SchemaTable,
    pub(crate) constant_regions: Box<[ResidentRegion]>,
    pub(crate) state_slots: Box<[CellSlotId]>,
    pub(crate) activation_sizes: ResidentArenaSizes,
}

#[derive(Clone, Debug)]
struct ActivatedOnceNode {
    artifact_node: NodeId,
    sources: Box<[ArtifactSource]>,
    write: ResidentRegion,
    kernel: BoundResidentKernel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentArenaSizes {
    pub bools: usize,
    pub indexes: usize,
    pub f64s: usize,
}

impl ResidentArenaSizes {
    fn allocate(&mut self, kind: ResidentValueKind, len: usize) -> Option<usize> {
        let cursor = match kind {
            ResidentValueKind::Bool => &mut self.bools,
            ResidentValueKind::Index => &mut self.indexes,
            ResidentValueKind::F64 => &mut self.f64s,
        };
        let offset = *cursor;
        *cursor = cursor.checked_add(len)?;
        Some(offset)
    }
}

#[derive(Clone, Debug, Default)]
pub struct TypedResidentArena {
    bools: Box<[u8]>,
    indexes: Box<[u64]>,
    f64s: Box<[f64]>,
}

impl TypedResidentArena {
    fn allocate(sizes: ResidentArenaSizes) -> Self {
        Self {
            bools: vec![0; sizes.bools].into_boxed_slice(),
            indexes: vec![0; sizes.indexes].into_boxed_slice(),
            f64s: vec![0.0; sizes.f64s].into_boxed_slice(),
        }
    }

    pub fn bool_storage(&self) -> &[u8] {
        &self.bools
    }

    pub fn index_storage(&self) -> &[u64] {
        &self.indexes
    }

    pub fn f64_storage(&self) -> &[f64] {
        &self.f64s
    }

    pub(crate) fn read(&self, region: ResidentRegion) -> ResidentValueRef<'_> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => ResidentValueRef::Bool(&self.bools[range]),
            ResidentValueKind::Index => ResidentValueRef::Index(&self.indexes[range]),
            ResidentValueKind::F64 => ResidentValueRef::F64(&self.f64s[range]),
        }
    }

    pub(crate) fn write(&mut self, region: ResidentRegion) -> ResidentValueMut<'_> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => ResidentValueMut::Bool(&mut self.bools[range]),
            ResidentValueKind::Index => ResidentValueMut::Index(&mut self.indexes[range]),
            ResidentValueKind::F64 => ResidentValueMut::F64(&mut self.f64s[range]),
        }
    }

    fn copy_region_from(
        &mut self,
        target: ResidentRegion,
        source: &TypedResidentArena,
        source_region: ResidentRegion,
    ) {
        debug_assert_eq!(target.kind, source_region.kind);
        debug_assert_eq!(target.len, source_region.len);
        match (self.write(target), source.read(source_region)) {
            (ResidentValueMut::Bool(target), ResidentValueRef::Bool(source)) => {
                target.copy_from_slice(source)
            }
            (ResidentValueMut::Index(target), ResidentValueRef::Index(source)) => {
                target.copy_from_slice(source)
            }
            (ResidentValueMut::F64(target), ResidentValueRef::F64(source)) => {
                target.copy_from_slice(source)
            }
            _ => unreachable!("resident region kinds were checked"),
        }
    }
}

#[derive(Clone, Debug)]
struct StateVersion {
    slot: CellSlotId,
    region: ResidentRegion,
    epochs: [Option<InstanceEpoch>; 2],
}

#[derive(Clone, Debug)]
pub struct StateArena {
    buffers: [TypedResidentArena; 2],
    versions: Box<[StateVersion]>,
}

impl StateArena {
    fn new(sizes: ResidentArenaSizes, slots: &[ResolvedSlot]) -> Self {
        let versions = slots
            .iter()
            .filter(|slot| slot.storage == ResidentStorageClass::State)
            .map(|slot| StateVersion {
                slot: slot.artifact_id,
                region: slot.region,
                epochs: [Some(InstanceEpoch::ZERO), None],
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            buffers: [
                TypedResidentArena::allocate(sizes),
                TypedResidentArena::allocate(sizes),
            ],
            versions,
        }
    }

    fn version(&self, slot: CellSlotId) -> &StateVersion {
        self.versions
            .iter()
            .find(|version| version.slot == slot)
            .expect("activated state slot has a version record")
    }

    fn version_mut(&mut self, slot: CellSlotId) -> &mut StateVersion {
        self.versions
            .iter_mut()
            .find(|version| version.slot == slot)
            .expect("activated state slot has a version record")
    }

    pub fn published_buffer(&self, slot: CellSlotId, epoch: InstanceEpoch) -> usize {
        let version = self.version(slot);
        version
            .epochs
            .iter()
            .enumerate()
            .filter_map(|(index, tag)| tag.filter(|tag| *tag <= epoch).map(|tag| (tag, index)))
            .max_by_key(|(tag, _)| *tag)
            .map(|(_, index)| index)
            .expect("activated state retains a version at or before the published epoch")
    }

    pub fn epochs(&self, slot: CellSlotId) -> [Option<InstanceEpoch>; 2] {
        self.version(slot).epochs
    }

    pub fn candidate_bytes(&self) -> usize {
        self.buffers[0].bools.len()
            + self.buffers[0].indexes.len() * core::mem::size_of::<u64>()
            + self.buffers[0].f64s.len() * core::mem::size_of::<f64>()
    }

    pub fn dual_payload_bytes(&self) -> usize {
        self.candidate_bytes() * 2
    }

    fn read_published(&self, slot: CellSlotId, epoch: InstanceEpoch) -> ResidentValueRef<'_> {
        let version = self.version(slot);
        let buffer = self.published_buffer(slot, epoch);
        self.buffers[buffer].read(version.region)
    }

    fn initialize(
        &mut self,
        slot: CellSlotId,
        value: &Value,
    ) -> Result<(), ResidentActivationError> {
        let region = self.version(slot).region;
        write_value(&mut self.buffers[0], region, value)?;
        Ok(())
    }

    fn install_migrated(
        &mut self,
        slot: CellSlotId,
        epoch: InstanceEpoch,
        source: &StateArena,
        source_slot: CellSlotId,
        source_epoch: InstanceEpoch,
    ) {
        let target = self.version(slot).region;
        let source_region = source.version(source_slot).region;
        let source_buffer = source.published_buffer(source_slot, source_epoch);
        self.buffers[0].copy_region_from(target, &source.buffers[source_buffer], source_region);
        self.version_mut(slot).epochs = [Some(epoch), None];
    }
}

#[derive(Clone, Debug)]
pub struct TurnWorkspace {
    pub(crate) input: TypedResidentArena,
    pub(crate) scratch: TypedResidentArena,
    pub(crate) dirty_bits: Box<[u64]>,
    pub(crate) executed_bits: Box<[u64]>,
    pub(crate) initialized_output_bits: Box<[u64]>,
    pub(crate) touched_slots: Vec<SlotIndex>,
    pub(crate) changed_slots: Vec<SlotIndex>,
}

impl TurnWorkspace {
    fn new(
        input_sizes: ResidentArenaSizes,
        scratch_sizes: ResidentArenaSizes,
        plan: &ActivatedPlan,
    ) -> Self {
        let words = plan.topology.word_len();
        Self {
            input: TypedResidentArena::allocate(input_sizes),
            scratch: TypedResidentArena::allocate(scratch_sizes),
            dirty_bits: vec![0; words].into_boxed_slice(),
            executed_bits: vec![0; words].into_boxed_slice(),
            initialized_output_bits: vec![0; words].into_boxed_slice(),
            touched_slots: Vec::with_capacity(plan.state_slots.len()),
            changed_slots: Vec::with_capacity(plan.state_slots.len()),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActivationFacts {
    pub slot_shapes: BTreeMap<CellSlotId, ShapeInstance>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateMigrationPolicy {
    PreserveCompatibleRejectIncompatible,
    PreserveCompatibleResetIncompatible,
}

#[derive(Clone, Copy, Debug)]
pub enum ResidentValueBorrow<'a> {
    Bool {
        shape: ResidentShape,
        values: &'a [u8],
    },
    Index {
        shape: ResidentShape,
        values: &'a [u64],
    },
    F64 {
        shape: ResidentShape,
        values: &'a [f64],
    },
}

impl ResidentValueBorrow<'_> {
    pub fn len(&self) -> usize {
        match self {
            Self::Bool { values, .. } => values.len(),
            Self::Index { values, .. } => values.len(),
            Self::F64 { values, .. } => values.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct ReactiveInstance {
    pub id: ReactiveInstanceId,
    pub plan: ActivatedPlan,
    pub activation: TypedResidentArena,
    pub state: StateArena,
    pub workspace: TurnWorkspace,
    published_epoch: AtomicU64,
    next_epoch: Option<InstanceEpoch>,
    candidate_active: bool,
}

impl ReactiveInstance {
    pub fn published_epoch(&self) -> InstanceEpoch {
        InstanceEpoch::new(self.published_epoch.load(Ordering::Acquire))
    }

    pub fn next_epoch(&self) -> Option<InstanceEpoch> {
        self.next_epoch
    }

    pub fn output_borrow(&self, output: usize) -> Option<ResidentValueBorrow<'_>> {
        let output = self.plan.outputs.get(output)?;
        let slot = self.plan.slots.get(output.slot.get() as usize)?;
        let value = self
            .state
            .read_published(slot.artifact_id, self.published_epoch());
        Some(match value {
            ResidentValueRef::Bool(values) => ResidentValueBorrow::Bool {
                shape: output.region.shape,
                values,
            },
            ResidentValueRef::Index(values) => ResidentValueBorrow::Index {
                shape: output.region.shape,
                values,
            },
            ResidentValueRef::F64(values) => ResidentValueBorrow::F64 {
                shape: output.region.shape,
                values,
            },
        })
    }

    pub fn state_borrow(&self, slot: CellSlotId) -> Option<ResidentValueBorrow<'_>> {
        let resolved = self.plan.slots.get(slot.get() as usize)?;
        if resolved.storage != ResidentStorageClass::State {
            return None;
        }
        let value = self.state.read_published(slot, self.published_epoch());
        Some(match value {
            ResidentValueRef::Bool(values) => ResidentValueBorrow::Bool {
                shape: resolved.region.shape,
                values,
            },
            ResidentValueRef::Index(values) => ResidentValueBorrow::Index {
                shape: resolved.region.shape,
                values,
            },
            ResidentValueRef::F64(values) => ResidentValueBorrow::F64 {
                shape: resolved.region.shape,
                values,
            },
        })
    }

    pub fn copied_output(&self, output: usize) -> Result<Value, ResidentActivationError> {
        let declaration = self
            .plan
            .outputs
            .get(output)
            .ok_or(ResidentActivationError::UnknownOutput { output })?;
        let borrowed = self
            .output_borrow(output)
            .ok_or(ResidentActivationError::UnknownOutput { output })?;
        let scalar = !matches!(
            self.plan
                .schemas
                .entry(declaration.schema)
                .expect("activated output schema remains present")
                .schema()
                .body(),
            SchemaBody::Matrix { .. }
        );
        let data = match borrowed {
            ResidentValueBorrow::Bool { values, .. } if scalar => {
                ValueDataDraft::Bool(values[0] != 0)
            }
            ResidentValueBorrow::Index { values, .. } if scalar => ValueDataDraft::Index(values[0]),
            ResidentValueBorrow::F64 { values, .. } if scalar => {
                ValueDataDraft::F64(F64Bits::from_f64(values[0]))
            }
            ResidentValueBorrow::Bool { values, .. } => ValueDataDraft::Matrix(
                values
                    .iter()
                    .map(|value| ValueDataDraft::Bool(*value != 0))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::Index { values, .. } => ValueDataDraft::Matrix(
                values
                    .iter()
                    .copied()
                    .map(ValueDataDraft::Index)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ResidentValueBorrow::F64 { values, .. } => ValueDataDraft::Matrix(
                values
                    .iter()
                    .copied()
                    .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        };
        ValueDraft {
            schema: declaration.schema,
            shape_values: declaration
                .shape
                .parameter_values()
                .to_vec()
                .into_boxed_slice(),
            data,
        }
        .finalize(&SnapshotValidationContext::new(&self.plan.schemas))
        .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)
    }

    pub fn reactivate(
        &mut self,
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        facts: &ActivationFacts,
        migration: StateMigrationPolicy,
    ) -> Result<(), ResidentActivationError> {
        if self.candidate_active {
            return Err(ResidentActivationError::ActiveCandidate);
        }
        if artifact.revision() == self.plan.program_revision {
            return Ok(());
        }
        let next_plan = self
            .plan
            .plan_generation
            .checked_next()
            .map_err(|_| ResidentActivationError::PlanGenerationExhausted)?;
        let mut replacement = activate(self.id, artifact, catalog, facts)?;
        let same_layout = physical_layout_eq(&self.plan, &replacement.plan);
        let next_layout = if same_layout {
            self.plan.layout_generation
        } else {
            self.plan
                .layout_generation
                .checked_next()
                .map_err(|_| ResidentActivationError::LayoutGenerationExhausted)?
        };
        let epoch = self.published_epoch();
        for target in replacement
            .plan
            .slots
            .iter()
            .filter(|slot| slot.storage == ResidentStorageClass::State)
        {
            let compatible = self
                .plan
                .slots
                .get(target.artifact_id.get() as usize)
                .filter(|source| {
                    source.artifact_id == target.artifact_id
                        && source.storage == ResidentStorageClass::State
                        && source.schema_key == target.schema_key
                        && source.shape == target.shape
                });
            if let Some(source) = compatible {
                replacement.state.install_migrated(
                    target.artifact_id,
                    epoch,
                    &self.state,
                    source.artifact_id,
                    epoch,
                );
            } else if migration == StateMigrationPolicy::PreserveCompatibleRejectIncompatible {
                return Err(ResidentActivationError::IncompatibleState {
                    slot: target.artifact_id,
                });
            }
        }
        replacement.plan.plan_generation = next_plan;
        replacement.plan.layout_generation = next_layout;
        replacement
            .published_epoch
            .store(epoch.get(), Ordering::Relaxed);
        replacement.next_epoch = self.next_epoch;
        *self = replacement;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentActivationError {
    LegacyOpaque {
        node: NodeId,
    },
    UnsupportedInteraction {
        node: NodeId,
    },
    UnsupportedDelivery {
        node: NodeId,
    },
    UnsupportedValue {
        schema: SchemaId,
    },
    TurnDimension {
        schema: SchemaId,
    },
    UnresolvedShape {
        slot: CellSlotId,
    },
    RegionSizeOverflow,
    InvalidSnapshotRepresentation,
    MissingStateInitializer {
        slot: CellSlotId,
    },
    UnsupportedConstruction {
        node: NodeId,
    },
    InvalidAlias {
        node: NodeId,
    },
    InvalidNodeOutput {
        node: NodeId,
    },
    InvalidConstraint {
        constraint: IntegrityConstraintId,
    },
    MissingResidentFactory {
        node: NodeId,
    },
    KernelBind {
        node: NodeId,
        error: ResidentKernelBindError,
    },
    ActivationKernel {
        node: NodeId,
    },
    InvalidDependency {
        node: NodeId,
    },
    OutputMustBeStateBacked {
        slot: CellSlotId,
    },
    UnknownOutput {
        output: usize,
    },
    ActiveCandidate,
    IncompatibleState {
        slot: CellSlotId,
    },
    PlanGenerationExhausted,
    LayoutGenerationExhausted,
}

pub fn activate(
    id: ReactiveInstanceId,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    facts: &ActivationFacts,
) -> Result<ReactiveInstance, ResidentActivationError> {
    let classification = classify_nodes(artifact)?;
    let layout = build_layout(artifact, facts, &classification)?;
    let (plan, input_sizes, state_sizes, scratch_sizes, activation_sizes) =
        build_plan(artifact, catalog, classification, layout)?;
    let mut activation = TypedResidentArena::allocate(activation_sizes);
    for raw in 0..artifact.constants().len() {
        let constant = ConstantId::new(raw as u32);
        let value = artifact
            .constants()
            .get(constant)
            .expect("dense constant store");
        write_value(&mut activation, plan.constant_regions[raw], value)?;
    }
    execute_activation_graph(artifact, &plan, &mut activation)?;
    let mut state = StateArena::new(state_sizes, &plan.slots);
    for slot in plan
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
    {
        let declaration = &artifact.slots()[slot.artifact_id.get() as usize];
        let Some(InitializerReference::Constant(constant)) = declaration.initializer else {
            return Err(ResidentActivationError::MissingStateInitializer {
                slot: slot.artifact_id,
            });
        };
        let value = artifact.constants().get(constant).ok_or(
            ResidentActivationError::MissingStateInitializer {
                slot: slot.artifact_id,
            },
        )?;
        state.initialize(slot.artifact_id, value)?;
    }
    let workspace = TurnWorkspace::new(input_sizes, scratch_sizes, &plan);
    Ok(ReactiveInstance {
        id,
        plan,
        activation,
        state,
        workspace,
        published_epoch: AtomicU64::new(InstanceEpoch::ZERO.get()),
        next_epoch: Some(InstanceEpoch::new(1)),
        candidate_active: false,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeClass {
    Observation,
    Activation,
    Turn,
}

fn classify_nodes(artifact: &ProgramArtifact) -> Result<Box<[NodeClass]>, ResidentActivationError> {
    let mut classes = vec![NodeClass::Turn; artifact.nodes().len()];
    let mut activation = BTreeSet::<NodeId>::new();
    for node in artifact.nodes() {
        let Some(contract) = artifact.contracts().get(node.contract) else {
            return Err(ResidentActivationError::LegacyOpaque { node: node.node });
        };
        let mech_core::ResolvedOperationContract::Declared(contract) = contract else {
            return Err(ResidentActivationError::LegacyOpaque { node: node.node });
        };
        if contract
            .inputs
            .iter()
            .any(|input| input.delivery != DeliveryMode::Signal)
            || contract
                .outputs
                .iter()
                .any(|output| output.delivery != DeliveryMode::Signal)
        {
            return Err(ResidentActivationError::UnsupportedDelivery { node: node.node });
        }
        if matches!(
            contract.interaction,
            ExternalInteraction::Observation(observation)
                if observation.replay == ObservationReplayPolicy::CaptureAsInputFact
        ) {
            classes[node.node.get() as usize] = NodeClass::Observation;
        } else if contract.interaction != ExternalInteraction::Pure {
            return Err(ResidentActivationError::UnsupportedInteraction { node: node.node });
        }
    }
    loop {
        let before = activation.len();
        for node in artifact.nodes() {
            if classes[node.node.get() as usize] == NodeClass::Observation {
                continue;
            }
            let inputs = node_inputs(artifact, node.node)?;
            if inputs.iter().all(|source| match source {
                ArtifactSource::Constant(_) => true,
                ArtifactSource::Slot(slot) => {
                    let slot = &artifact.slots()[slot.get() as usize];
                    slot.role != SlotRole::State
                        && matches!(
                            slot.producer,
                            ProducerReference::NodeOutput { node, .. }
                                if activation.contains(&node)
                        )
                }
            }) {
                activation.insert(node.node);
            }
        }
        if before == activation.len() {
            break;
        }
    }
    for node in activation {
        classes[node.get() as usize] = NodeClass::Activation;
    }
    for node in artifact.nodes() {
        let mech_core::ResolvedOperationContract::Declared(contract) = artifact
            .contracts()
            .get(node.contract)
            .expect("validated contract")
        else {
            unreachable!()
        };
        if contract.outputs.len() != 1 {
            return Err(ResidentActivationError::InvalidNodeOutput { node: node.node });
        }
        let output = &contract.outputs[0];
        match classes[node.node.get() as usize] {
            NodeClass::Observation => {}
            NodeClass::Activation => {
                if !matches!(
                    output.construction,
                    OutputConstruction::FullWrite { .. } | OutputConstruction::Build { .. }
                ) || output.alias != AliasPolicy::NoAlias
                {
                    return Err(ResidentActivationError::UnsupportedConstruction {
                        node: node.node,
                    });
                }
            }
            NodeClass::Turn => match output.construction {
                OutputConstruction::FullWrite { .. } => {
                    if output.access != AccessMode::Write || output.alias != AliasPolicy::NoAlias {
                        return Err(ResidentActivationError::InvalidAlias { node: node.node });
                    }
                }
                OutputConstruction::ReadModifyWrite { base_input, .. } => {
                    if output.access != AccessMode::ReadWrite
                        || output.alias != (AliasPolicy::MayAlias { input: base_input })
                        || !matches!(
                            node_inputs(artifact, node.node)?.get(base_input as usize),
                            Some(ArtifactSource::Slot(base))
                                if node_output_slot(artifact, node.node)? == *base
                        )
                    {
                        return Err(ResidentActivationError::InvalidAlias { node: node.node });
                    }
                }
                _ => {
                    return Err(ResidentActivationError::UnsupportedConstruction {
                        node: node.node,
                    });
                }
            },
        }
    }
    Ok(classes.into_boxed_slice())
}

struct LayoutBuild {
    slots: Box<[ResolvedSlot]>,
    constant_regions: Box<[ResidentRegion]>,
    sizes: [ResidentArenaSizes; 4],
}

fn class_index(class: ResidentStorageClass) -> usize {
    match class {
        ResidentStorageClass::Constant => 0,
        ResidentStorageClass::Input => 1,
        ResidentStorageClass::State => 2,
        ResidentStorageClass::Scratch => 3,
    }
}

fn build_layout(
    artifact: &ProgramArtifact,
    facts: &ActivationFacts,
    classes: &[NodeClass],
) -> Result<LayoutBuild, ResidentActivationError> {
    let mut sizes = [ResidentArenaSizes::default(); 4];
    let mut constant_regions = Vec::with_capacity(artifact.constants().len());
    for raw in 0..artifact.constants().len() {
        let value = artifact
            .constants()
            .get(ConstantId::new(raw as u32))
            .unwrap();
        let (kind, shape) = value_layout(artifact, value)?;
        let len = shape
            .len()
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        let offset = sizes[0]
            .allocate(kind, len)
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        constant_regions.push(ResidentRegion {
            kind,
            offset,
            len,
            shape,
        });
    }
    let mut slots = Vec::with_capacity(artifact.slots().len());
    for declaration in artifact.slots() {
        let producer_class = match declaration.producer {
            ProducerReference::Input(_) => Some(NodeClass::Observation),
            ProducerReference::NodeOutput { node, .. } => Some(classes[node.get() as usize]),
        };
        let storage = match declaration.role {
            SlotRole::Input => ResidentStorageClass::Input,
            SlotRole::State => ResidentStorageClass::State,
            SlotRole::Derived if producer_class == Some(NodeClass::Observation) => {
                ResidentStorageClass::Input
            }
            SlotRole::Derived if producer_class == Some(NodeClass::Activation) => {
                ResidentStorageClass::Constant
            }
            SlotRole::Derived => ResidentStorageClass::Scratch,
        };
        let shape = slot_shape(artifact, declaration.slot, facts)?;
        let (kind, resident_shape) = schema_layout(artifact, declaration.schema, &shape)?;
        let len = resident_shape
            .len()
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        let class = class_index(storage);
        let offset = sizes[class]
            .allocate(kind, len)
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        let schema = artifact.schemas().entry(declaration.schema).unwrap();
        slots.push(ResolvedSlot {
            artifact_id: declaration.slot,
            physical_index: SlotIndex::new(declaration.slot.get()),
            schema: declaration.schema,
            schema_key: schema.key(),
            shape,
            storage,
            region: ResidentRegion {
                kind,
                offset,
                len,
                shape: resident_shape,
            },
        });
    }
    Ok(LayoutBuild {
        slots: slots.into_boxed_slice(),
        constant_regions: constant_regions.into_boxed_slice(),
        sizes,
    })
}

fn slot_shape(
    artifact: &ProgramArtifact,
    slot: CellSlotId,
    facts: &ActivationFacts,
) -> Result<ShapeInstance, ResidentActivationError> {
    let declaration = &artifact.slots()[slot.get() as usize];
    if let Some(InitializerReference::Constant(constant)) = declaration.initializer {
        return Ok(artifact.constants().get(constant).unwrap().shape().clone());
    }
    if let Some(shape) = facts.slot_shapes.get(&slot) {
        return Ok(shape.clone());
    }
    let schema = artifact
        .schemas()
        .entry(declaration.schema)
        .unwrap()
        .schema();
    if schema.dimension_parameters().is_empty() {
        return schema
            .instantiate_shape(Box::new([]))
            .map_err(|_| ResidentActivationError::UnresolvedShape { slot });
    }
    Err(ResidentActivationError::UnresolvedShape { slot })
}

fn value_layout(
    artifact: &ProgramArtifact,
    value: &Value,
) -> Result<(ResidentValueKind, ResidentShape), ResidentActivationError> {
    schema_layout(artifact, value.schema(), value.shape())
}

fn schema_layout(
    artifact: &ProgramArtifact,
    schema: SchemaId,
    shape: &ShapeInstance,
) -> Result<(ResidentValueKind, ResidentShape), ResidentActivationError> {
    let schema_entry = artifact.schemas().entry(schema).unwrap();
    if schema_entry
        .schema()
        .dimension_parameters()
        .iter()
        .any(|parameter| parameter.lifetime() == DimensionLifetime::Turn)
    {
        return Err(ResidentActivationError::TurnDimension { schema });
    }
    let kind = |body: &SchemaBody| match body {
        SchemaBody::Bool => Some(ResidentValueKind::Bool),
        SchemaBody::Index => Some(ResidentValueKind::Index),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => Some(ResidentValueKind::F64),
        _ => None,
    };
    match schema_entry.schema().body() {
        body @ (SchemaBody::Bool
        | SchemaBody::Index
        | SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)) => {
            Ok((kind(body).unwrap(), ResidentShape::SCALAR))
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 => {
            let kind = kind(element).ok_or(ResidentActivationError::UnsupportedValue { schema })?;
            let rows = evaluate_dimension(&dimensions[0], shape.parameter_values())?;
            let columns = evaluate_dimension(&dimensions[1], shape.parameter_values())?;
            Ok((
                kind,
                ResidentShape {
                    rows: u32::try_from(rows)
                        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
                    columns: u32::try_from(columns)
                        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
                },
            ))
        }
        _ => Err(ResidentActivationError::UnsupportedValue { schema }),
    }
}

fn evaluate_dimension(
    expression: &DimensionExpr,
    values: &[u64],
) -> Result<u64, ResidentActivationError> {
    match expression {
        DimensionExpr::Hole => Err(ResidentActivationError::RegionSizeOverflow),
        DimensionExpr::Constant(value) => Ok(*value),
        DimensionExpr::Parameter(id) => values
            .get(id.get() as usize)
            .copied()
            .ok_or(ResidentActivationError::RegionSizeOverflow),
        DimensionExpr::Add(terms) => terms.iter().try_fold(0_u64, |sum, term| {
            sum.checked_add(evaluate_dimension(term, values)?)
                .ok_or(ResidentActivationError::RegionSizeOverflow)
        }),
        DimensionExpr::Multiply(terms) => terms.iter().try_fold(1_u64, |product, term| {
            product
                .checked_mul(evaluate_dimension(term, values)?)
                .ok_or(ResidentActivationError::RegionSizeOverflow)
        }),
        DimensionExpr::Min(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min()
            .ok_or(ResidentActivationError::RegionSizeOverflow),
        DimensionExpr::Max(terms) => terms
            .iter()
            .map(|term| evaluate_dimension(term, values))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .ok_or(ResidentActivationError::RegionSizeOverflow),
    }
}

fn build_plan(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    classes: Box<[NodeClass]>,
    layout: LayoutBuild,
) -> Result<
    (
        ActivatedPlan,
        ResidentArenaSizes,
        ResidentArenaSizes,
        ResidentArenaSizes,
        ResidentArenaSizes,
    ),
    ResidentActivationError,
> {
    let mut reads = Vec::new();
    let mut nodes = Vec::new();
    let mut activation_steps = Vec::new();
    let mut artifact_to_activated = vec![None; artifact.nodes().len()];
    for node in artifact.nodes() {
        let class = classes[node.node.get() as usize];
        if class == NodeClass::Observation {
            continue;
        }
        let input_sources = node_inputs(artifact, node.node)?;
        let output_slot = node_output_slot(artifact, node.node)?;
        let output = &layout.slots[output_slot.get() as usize];
        let mech_core::ResolvedOperationContract::Declared(contract) =
            artifact.contracts().get(node.contract).unwrap()
        else {
            unreachable!()
        };
        let output_contract = &contract.outputs[0];
        let base = match output_contract.construction {
            OutputConstruction::ReadModifyWrite { base_input, .. } => Some(base_input as usize),
            _ => None,
        };
        let input_layouts = input_sources
            .iter()
            .map(|source| source_port_layout(artifact, &layout, *source))
            .collect::<Result<Vec<_>, _>>()?;
        let output_layout = slot_port_layout(output);
        let factory = catalog
            .resident_factory(&node.operation.module_path, &node.operation.operation_name)
            .ok_or(ResidentActivationError::MissingResidentFactory { node: node.node })?;
        let kernel = (factory.factory)(&ResidentKernelBindRequest {
            contract: artifact.contracts().get(node.contract).unwrap(),
            inputs: &input_layouts,
            output: output_layout,
        })
        .map_err(|error| ResidentActivationError::KernelBind {
            node: node.node,
            error,
        })?;
        if class == NodeClass::Activation {
            activation_steps.push(ActivatedOnceNode {
                artifact_node: node.node,
                sources: input_sources.into_boxed_slice(),
                write: output.region,
                kernel,
            });
            continue;
        }
        let activated = ActivatedNodeIndex(nodes.len() as u32);
        artifact_to_activated[node.node.get() as usize] = Some(activated);
        let read_start = reads.len() as u32;
        for (ordinal, source) in input_sources.iter().enumerate() {
            if Some(ordinal) != base {
                reads.push(resolve_read(artifact, &layout, *source)?);
            }
        }
        nodes.push(ActivatedNode {
            artifact_node: node.node,
            reads: read_start..reads.len() as u32,
            write: ResidentWriteLocation {
                slot: output_slot,
                storage: output.storage,
                region: output.region,
            },
            construction: output_contract.construction.clone(),
            change_detection: output_contract.change_detection,
            kernel,
        });
    }
    let topology = build_topology(artifact, &classes, &nodes, &artifact_to_activated)?;
    let inputs = layout
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::Input)
        .map(|slot| ActivatedInput {
            slot: slot.physical_index,
            schema: slot.schema,
            region: slot.region,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let outputs = artifact
        .outputs()
        .iter()
        .map(|output| {
            let slot = &layout.slots[output.source.get() as usize];
            if slot.storage != ResidentStorageClass::State {
                return Err(ResidentActivationError::OutputMustBeStateBacked {
                    slot: output.source,
                });
            }
            Ok(ActivatedOutput {
                slot: slot.physical_index,
                schema: slot.schema,
                schema_key: slot.schema_key,
                shape: slot.shape.clone(),
                region: slot.region,
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let constraints = artifact
        .constraints()
        .iter()
        .map(|constraint| {
            if constraint.operation.module_path.as_ref() != ["integrity"]
                || constraint.operation.operation_name != "assert"
                || constraint.inputs.len() != 1
            {
                return Err(ResidentActivationError::InvalidConstraint {
                    constraint: constraint.constraint,
                });
            }
            let predicate = resolve_read(artifact, &layout, constraint.inputs[0])?;
            let layout = source_port_layout(artifact, &layout, constraint.inputs[0])?;
            if layout.kind != ResidentValueKind::Bool || layout.shape != ResidentShape::SCALAR {
                return Err(ResidentActivationError::InvalidConstraint {
                    constraint: constraint.constraint,
                });
            }
            Ok(ActivatedConstraint {
                artifact_id: constraint.constraint,
                predicate,
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let state_slots = layout
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::State)
        .map(|slot| slot.artifact_id)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let activation_nodes = artifact
        .nodes()
        .iter()
        .filter(|node| classes[node.node.get() as usize] == NodeClass::Activation)
        .map(|node| node.node)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let input_sizes = layout.sizes[1];
    let state_sizes = layout.sizes[2];
    let scratch_sizes = layout.sizes[3];
    let activation_sizes = layout.sizes[0];
    let plan = ActivatedPlan {
        program_revision: artifact.revision(),
        plan_generation: PlanGeneration::ZERO,
        layout_generation: LayoutGeneration::ZERO,
        slots: layout.slots,
        nodes: nodes.into_boxed_slice(),
        reads: reads.into_boxed_slice(),
        topology,
        inputs,
        outputs,
        constraints,
        activation_nodes,
        activation_steps: activation_steps.into_boxed_slice(),
        schemas: artifact.schemas().clone(),
        constant_regions: layout.constant_regions,
        state_slots,
        activation_sizes,
    };
    Ok((
        plan,
        input_sizes,
        state_sizes,
        scratch_sizes,
        activation_sizes,
    ))
}

fn execute_activation_graph(
    _artifact: &ProgramArtifact,
    plan: &ActivatedPlan,
    arena: &mut TypedResidentArena,
) -> Result<(), ResidentActivationError> {
    for step in &plan.activation_steps {
        let inputs = step
            .sources
            .iter()
            .map(|source| owned_activation_input(plan, arena, *source))
            .collect::<Result<Vec<_>, _>>()?;
        let refs = inputs
            .iter()
            .map(OwnedResidentValue::as_ref)
            .collect::<Vec<_>>();
        step.kernel
            .execute(&refs, arena.write(step.write))
            .map_err(|_| ResidentActivationError::ActivationKernel {
                node: step.artifact_node,
            })?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
enum OwnedResidentValue {
    Bool(Box<[u8]>),
    Index(Box<[u64]>),
    F64(Box<[f64]>),
}

impl OwnedResidentValue {
    fn as_ref(&self) -> ResidentValueRef<'_> {
        match self {
            Self::Bool(values) => ResidentValueRef::Bool(values),
            Self::Index(values) => ResidentValueRef::Index(values),
            Self::F64(values) => ResidentValueRef::F64(values),
        }
    }
}

fn owned_activation_input(
    plan: &ActivatedPlan,
    arena: &TypedResidentArena,
    source: ArtifactSource,
) -> Result<OwnedResidentValue, ResidentActivationError> {
    let region = match source {
        ArtifactSource::Constant(constant) => plan.constant_regions[constant.get() as usize],
        ArtifactSource::Slot(slot) => {
            let slot = &plan.slots[slot.get() as usize];
            if slot.storage != ResidentStorageClass::Constant {
                return Err(ResidentActivationError::InvalidDependency {
                    node: NodeId::new(0),
                });
            }
            slot.region
        }
    };
    Ok(match arena.read(region) {
        ResidentValueRef::Bool(values) => {
            OwnedResidentValue::Bool(values.to_vec().into_boxed_slice())
        }
        ResidentValueRef::Index(values) => {
            OwnedResidentValue::Index(values.to_vec().into_boxed_slice())
        }
        ResidentValueRef::F64(values) => {
            OwnedResidentValue::F64(values.to_vec().into_boxed_slice())
        }
    })
}

fn build_topology(
    artifact: &ProgramArtifact,
    classes: &[NodeClass],
    nodes: &[ActivatedNode],
    artifact_to_activated: &[Option<ActivatedNodeIndex>],
) -> Result<DependencyTopology, ResidentActivationError> {
    let mut downstream = vec![Vec::<ActivatedNodeIndex>::new(); nodes.len()];
    let mut roots = Vec::<ActivatedNodeIndex>::new();
    let mut latest_state_writer = BTreeMap::<CellSlotId, ActivatedNodeIndex>::new();
    for node in artifact.nodes() {
        if classes[node.node.get() as usize] != NodeClass::Turn {
            continue;
        }
        let current = artifact_to_activated[node.node.get() as usize].unwrap();
        let mut root = false;
        for source in node_inputs(artifact, node.node)? {
            let ArtifactSource::Slot(slot_id) = source else {
                continue;
            };
            let slot = &artifact.slots()[slot_id.get() as usize];
            let parent = if slot.role == SlotRole::State {
                latest_state_writer.get(&slot_id).copied()
            } else {
                match slot.producer {
                    ProducerReference::NodeOutput { node, .. } => {
                        artifact_to_activated[node.get() as usize]
                    }
                    ProducerReference::Input(_) => None,
                }
            };
            if let Some(parent) = parent {
                if !downstream[parent.get() as usize].contains(&current) {
                    downstream[parent.get() as usize].push(current);
                }
            } else if slot.role == SlotRole::State
                || classes
                    .get(match slot.producer {
                        ProducerReference::NodeOutput { node, .. } => node.get() as usize,
                        ProducerReference::Input(_) => usize::MAX,
                    })
                    .copied()
                    == Some(NodeClass::Observation)
                || slot.role == SlotRole::Input
            {
                root = true;
            }
        }
        if root {
            roots.push(current);
        }
        let output = node_output_slot(artifact, node.node)?;
        if artifact.slots()[output.get() as usize].role == SlotRole::State {
            latest_state_writer.insert(output, current);
        }
    }
    let words = nodes.len().div_ceil(64);
    let mut masks = Vec::with_capacity(nodes.len());
    for children in &downstream {
        let mut mask = vec![0_u64; words].into_boxed_slice();
        for child in children {
            set_bit(&mut mask, child.get() as usize);
        }
        masks.push(mask);
    }
    let mut root_mask = vec![0_u64; words].into_boxed_slice();
    for root in &roots {
        set_bit(&mut root_mask, root.get() as usize);
    }
    let mut mandatory = vec![0_u64; words].into_boxed_slice();
    for (index, node) in nodes.iter().enumerate() {
        if node.write.storage == ResidentStorageClass::State {
            set_bit(&mut mandatory, index);
        }
    }
    for constraint in artifact.constraints() {
        let Some(ArtifactSource::Slot(slot)) = constraint.inputs.first().copied() else {
            continue;
        };
        let ProducerReference::NodeOutput { node, .. } =
            artifact.slots()[slot.get() as usize].producer
        else {
            continue;
        };
        if let Some(activated) = artifact_to_activated[node.get() as usize] {
            set_bit(&mut mandatory, activated.get() as usize);
        }
    }
    let mut offsets = Vec::with_capacity(nodes.len() + 1);
    let mut values = Vec::new();
    offsets.push(0_u32);
    for children in downstream {
        values.extend(children);
        offsets.push(values.len() as u32);
    }
    Ok(DependencyTopology {
        linear_node_order: (0..nodes.len())
            .map(|index| ActivatedNodeIndex(index as u32))
            .collect(),
        same_turn_downstream_offsets: offsets.into_boxed_slice(),
        same_turn_downstream_nodes: values.into_boxed_slice(),
        turn_root_nodes: roots.into_boxed_slice(),
        same_turn_downstream_masks: masks.into_boxed_slice(),
        turn_root_mask: root_mask,
        mandatory_candidate_mask: mandatory,
    })
}

fn set_bit(words: &mut [u64], bit: usize) {
    words[bit / 64] |= 1_u64 << (bit % 64);
}

fn resolve_read(
    artifact: &ProgramArtifact,
    layout: &LayoutBuild,
    source: ArtifactSource,
) -> Result<ResidentReadLocation, ResidentActivationError> {
    Ok(match source {
        ArtifactSource::Constant(constant) => {
            ResidentReadLocation::Constant(layout.constant_regions[constant.get() as usize])
        }
        ArtifactSource::Slot(slot) => {
            let resolved = &layout.slots[slot.get() as usize];
            match resolved.storage {
                ResidentStorageClass::Constant => ResidentReadLocation::Constant(resolved.region),
                ResidentStorageClass::Input => ResidentReadLocation::Input(resolved.region),
                ResidentStorageClass::State => ResidentReadLocation::State {
                    slot,
                    region: resolved.region,
                },
                ResidentStorageClass::Scratch => ResidentReadLocation::Scratch(resolved.region),
            }
        }
    })
}

fn source_port_layout(
    artifact: &ProgramArtifact,
    layout: &LayoutBuild,
    source: ArtifactSource,
) -> Result<ResidentPortLayout, ResidentActivationError> {
    match source {
        ArtifactSource::Constant(constant) => {
            let value = artifact.constants().get(constant).unwrap();
            let region = layout.constant_regions[constant.get() as usize];
            Ok(ResidentPortLayout {
                schema: value.schema_key(),
                kind: region.kind,
                shape: region.shape,
            })
        }
        ArtifactSource::Slot(slot) => Ok(slot_port_layout(&layout.slots[slot.get() as usize])),
    }
}

fn slot_port_layout(slot: &ResolvedSlot) -> ResidentPortLayout {
    ResidentPortLayout {
        schema: slot.schema_key,
        kind: slot.region.kind,
        shape: slot.region.shape,
    }
}

fn node_inputs(
    artifact: &ProgramArtifact,
    node: NodeId,
) -> Result<Vec<ArtifactSource>, ResidentActivationError> {
    let declaration = artifact
        .nodes()
        .get(node.get() as usize)
        .ok_or(ResidentActivationError::InvalidDependency { node })?;
    artifact.bindings()
        [declaration.input_bindings.start as usize..declaration.input_bindings.end as usize]
        .iter()
        .map(|binding| match binding {
            BindingDeclaration::Input { source, .. } => Ok(*source),
            BindingDeclaration::Output { .. } => {
                Err(ResidentActivationError::InvalidDependency { node })
            }
        })
        .collect()
}

fn node_output_slot(
    artifact: &ProgramArtifact,
    node: NodeId,
) -> Result<CellSlotId, ResidentActivationError> {
    let declaration = artifact
        .nodes()
        .get(node.get() as usize)
        .ok_or(ResidentActivationError::InvalidDependency { node })?;
    let bindings = &artifact.bindings()
        [declaration.output_bindings.start as usize..declaration.output_bindings.end as usize];
    let [BindingDeclaration::Output { target, .. }] = bindings else {
        return Err(ResidentActivationError::InvalidNodeOutput { node });
    };
    Ok(*target)
}

fn write_value(
    arena: &mut TypedResidentArena,
    region: ResidentRegion,
    value: &Value,
) -> Result<(), ResidentActivationError> {
    match (arena.write(region), value.data()) {
        (ResidentValueMut::Bool(target), ValueData::Bool(value)) if target.len() == 1 => {
            target[0] = u8::from(*value);
        }
        (ResidentValueMut::Index(target), ValueData::Index(value)) if target.len() == 1 => {
            target[0] = *value;
        }
        (ResidentValueMut::F64(target), ValueData::F64(value)) if target.len() == 1 => {
            target[0] = value.to_f64();
        }
        (ResidentValueMut::Bool(target), ValueData::Matrix(matrix)) => {
            let SequenceView::Bool(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (target, source) in target.iter_mut().zip(source) {
                *target = u8::from(*source);
            }
        }
        (ResidentValueMut::Index(target), ValueData::Matrix(matrix)) => {
            let SequenceView::Index(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            target.copy_from_slice(source);
        }
        (ResidentValueMut::F64(target), ValueData::Matrix(matrix)) => {
            let SequenceView::F64(source) = matrix.elements() else {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            };
            if target.len() != source.len() {
                return Err(ResidentActivationError::InvalidSnapshotRepresentation);
            }
            for (target, source) in target.iter_mut().zip(source) {
                *target = source.to_f64();
            }
        }
        _ => return Err(ResidentActivationError::InvalidSnapshotRepresentation),
    }
    Ok(())
}

fn physical_layout_eq(left: &ActivatedPlan, right: &ActivatedPlan) -> bool {
    left.slots.len() == right.slots.len()
        && left.slots.iter().zip(&right.slots).all(|(left, right)| {
            left.artifact_id == right.artifact_id
                && left.schema_key == right.schema_key
                && left.shape == right.shape
                && left.storage == right.storage
                && left.region == right.region
        })
}
