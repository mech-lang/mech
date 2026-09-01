//! Schema-driven activation for the pre-launch dense numeric resident profile.

mod execution;

pub use execution::*;

use core::ops::Range;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirementId, BoundResidentKernel, CellSlotId,
    ChangeDetectionPolicy, ConstantId, DeliveryMode, DimensionExpr, DimensionLifetime,
    ExecutionTarget, ExecutionTargetSet, ExternalInteraction, FunctionCatalog, InputId,
    InstanceEpoch, IntegrityConstraintId, LayoutGeneration, NodeId, ObservationReplayPolicy,
    OutputConstruction, PlanGeneration, ProgramRevision, ReactiveInstanceId,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelInputs, ResidentPortLayout,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef, SchemaBody, SchemaId,
    SchemaKey, ShapeInstance, SlotIndex, Value,
};
use sha2::{Digest, Sha256};

use crate::{
    ArtifactSource, BindingDeclaration, InitializerReference, OperationReference,
    ProducerReference, ProgramArtifact, SlotRole,
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
    pub role: SlotRole,
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

impl ResidentReadLocation {
    pub const fn region(self) -> ResidentRegion {
        match self {
            Self::Constant(region) | Self::Input(region) | Self::Scratch(region) => region,
            Self::State { region, .. } => region,
        }
    }
}

const F64_ACTIVATION_ARENA: u32 = 0;
const F64_INPUT_ARENA: u32 = 1;
const F64_SCRATCH_ARENA: u32 = 2;
const F64_STATE_ARENA_BASE: u8 = 3;
const F64_STATE_SLOT_BIT: u32 = 1 << 31;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct F64ReadTapeEntry {
    selector: u32,
    start: u32,
    end: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentWriteLocation {
    pub slot: CellSlotId,
    pub storage: ResidentStorageClass,
    pub region: ResidentRegion,
}

#[derive(Clone, Debug)]
pub struct ActivatedKernelNode {
    pub artifact_node: NodeId,
    pub reads: Range<u32>,
    pub write: ResidentWriteLocation,
    pub construction: OutputConstruction,
    pub change_detection: ChangeDetectionPolicy,
    pub(crate) reads_state: bool,
    pub(crate) scratch_prefix_reads: bool,
    pub kernel: BoundResidentKernel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivatedExternalNode {
    pub artifact_node: NodeId,
    pub requirement: ApplicationRequirementId,
    pub interaction: ExternalInteraction,
    pub payload: ResidentReadLocation,
    pub captured_payload: ResidentRegion,
    pub effect_ordinal: u32,
    pub payload_schema: SchemaId,
    pub payload_shape: ShapeInstance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentEffectIntent {
    pub artifact_node: NodeId,
    pub requirement: ApplicationRequirementId,
    pub ordinal: u32,
}

#[derive(Clone, Debug)]
pub enum ActivatedTurnStep {
    Kernel(ActivatedKernelNode),
    External(ActivatedExternalNode),
}

impl ActivatedTurnStep {
    pub const fn artifact_node(&self) -> NodeId {
        match self {
            Self::Kernel(node) => node.artifact_node,
            Self::External(node) => node.artifact_node,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ActivatedNodeIndex(u32);

impl ActivatedNodeIndex {
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SingleWordScheduleEntry {
    node: ActivatedNodeIndex,
    node_bit: u64,
    downstream: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTopology {
    pub linear_node_order: Box<[ActivatedNodeIndex]>,
    pub(crate) single_word_schedule: Box<[SingleWordScheduleEntry]>,
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
    pub artifact_slot: CellSlotId,
    pub slot: SlotIndex,
    pub schema: SchemaId,
    pub schema_key: SchemaKey,
    pub shape: ShapeInstance,
    pub region: ResidentRegion,
    pub source: ActivatedInputSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivatedInputSource {
    DeclaredInput {
        input: InputId,
    },
    Observation {
        node: NodeId,
        requirement: ApplicationRequirementId,
    },
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
struct ActivatedOutputMaterialization {
    target: CellSlotId,
    source: ResidentReadLocation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivatedConstraint {
    pub artifact_id: IntegrityConstraintId,
    pub predicate: ResidentReadLocation,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResidentIntegrityMode {
    #[default]
    Checked,
    Unchecked,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResidentExternalAdmission {
    #[default]
    Deny,
    /// Admit external nodes into the structural plan only. The engine owns no
    /// provider authority, and ordinary resident publication remains closed.
    StructuralOnly,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentActivationOptions {
    pub integrity: ResidentIntegrityMode,
    pub external: ResidentExternalAdmission,
}

#[derive(Clone, Debug)]
pub struct ActivatedPlan {
    pub program_revision: ProgramRevision,
    pub activation_facts_fingerprint: [u8; 32],
    pub plan_generation: PlanGeneration,
    pub layout_generation: LayoutGeneration,
    pub slots: Box<[ResolvedSlot]>,
    pub steps: Box<[ActivatedTurnStep]>,
    external_step_count: usize,
    pure_kernel_steps: Option<Box<[ActivatedKernelNode]>>,
    pub reads: Box<[ResidentReadLocation]>,
    f64_read_tape: Option<Box<[F64ReadTapeEntry]>>,
    execution_node_order: Box<[ActivatedNodeIndex]>,
    execution_node_mask: Box<[u64]>,
    pub integrity_mode: ResidentIntegrityMode,
    pub external_admission: ResidentExternalAdmission,
    pub topology: DependencyTopology,
    pub inputs: Box<[ActivatedInput]>,
    pub outputs: Box<[ActivatedOutput]>,
    output_materializations: Box<[ActivatedOutputMaterialization]>,
    pub constraints: Box<[ActivatedConstraint]>,
    pub activation_nodes: Box<[NodeId]>,
    activation_steps: Box<[ActivatedOnceNode]>,
    pub(crate) schemas: mech_core::SchemaTable,
    pub(crate) constant_regions: Box<[ResidentRegion]>,
    pub(crate) state_slots: Box<[CellSlotId]>,
    pub(crate) rmw_state_slots: Box<[CellSlotId]>,
    pub(crate) state_hash_seed: u64,
    pub(crate) effect_payload_sizes: ResidentArenaSizes,
}

impl ActivatedPlan {
    pub fn schemas(&self) -> &mech_core::SchemaTable {
        &self.schemas
    }

    pub fn execution_node_count(&self) -> usize {
        self.execution_node_order.len()
    }

    pub fn has_external_steps(&self) -> bool {
        self.external_step_count != 0
    }

    pub(crate) fn has_observation_inputs(&self) -> bool {
        self.inputs
            .iter()
            .any(|input| matches!(input.source, ActivatedInputSource::Observation { .. }))
    }

    pub fn external_effect_count(&self) -> usize {
        self.steps
            .iter()
            .filter(|step| {
                matches!(
                    step,
                    ActivatedTurnStep::External(external)
                        if matches!(external.interaction, ExternalInteraction::Effect(_))
                )
            })
            .count()
    }

    /// Replaces one activated kernel in every execution representation.
    ///
    /// This is intentionally hidden and exists for fault-injection contract
    /// tests. Pure plans retain a flattened kernel cache for the hot path, so
    /// mutating the diagnostic `steps` view alone would not inject a fault into
    /// the executor that production actually uses.
    #[doc(hidden)]
    pub fn replace_kernel_for_test(
        &mut self,
        index: usize,
        kernel: BoundResidentKernel,
    ) -> BoundResidentKernel {
        let ActivatedTurnStep::Kernel(node) = &mut self.steps[index] else {
            panic!("activated step {index} is not a resident kernel")
        };
        let previous = core::mem::replace(&mut node.kernel, kernel.clone());
        if let Some(nodes) = &mut self.pure_kernel_steps {
            nodes[index].kernel = kernel;
        }
        previous
    }

    /// Changes one activated kernel's dirty-propagation policy in every
    /// execution representation. See [`Self::replace_kernel_for_test`].
    #[doc(hidden)]
    pub fn set_change_detection_for_test(&mut self, index: usize, policy: ChangeDetectionPolicy) {
        let ActivatedTurnStep::Kernel(node) = &mut self.steps[index] else {
            panic!("activated step {index} is not a resident kernel")
        };
        node.change_detection = policy;
        if let Some(nodes) = &mut self.pure_kernel_steps {
            nodes[index].change_detection = policy;
        }
    }
}

#[derive(Clone, Debug)]
struct ActivatedOnceNode {
    artifact_node: NodeId,
    sources: Box<[ArtifactSource]>,
    storage: ResidentStorageClass,
    write: ResidentRegion,
    kernel: BoundResidentKernel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentArenaSizes {
    pub bools: usize,
    pub indexes: usize,
    pub f64s: usize,
    pub strings: usize,
    pub snapshots: usize,
}

impl ResidentArenaSizes {
    fn allocate(&mut self, kind: ResidentValueKind, len: usize) -> Option<usize> {
        let cursor = match kind {
            ResidentValueKind::Bool => &mut self.bools,
            ResidentValueKind::Index => &mut self.indexes,
            ResidentValueKind::F64 => &mut self.f64s,
            ResidentValueKind::String => &mut self.strings,
            ResidentValueKind::Snapshot => &mut self.snapshots,
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
    strings: Box<[String]>,
    snapshots: Box<[Option<Value>]>,
}

impl TypedResidentArena {
    fn allocate(sizes: ResidentArenaSizes) -> Self {
        Self {
            bools: vec![0; sizes.bools].into_boxed_slice(),
            indexes: vec![1; sizes.indexes].into_boxed_slice(),
            f64s: vec![0.0; sizes.f64s].into_boxed_slice(),
            strings: vec![String::new(); sizes.strings].into_boxed_slice(),
            snapshots: vec![None; sizes.snapshots].into_boxed_slice(),
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

    pub fn string_storage(&self) -> &[String] {
        &self.strings
    }

    pub fn snapshot_storage(&self) -> &[Option<Value>] {
        &self.snapshots
    }

    pub(crate) fn read(&self, region: ResidentRegion) -> ResidentValueRef<'_> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => ResidentValueRef::Bool(&self.bools[range]),
            ResidentValueKind::Index => ResidentValueRef::Index(&self.indexes[range]),
            ResidentValueKind::F64 => ResidentValueRef::F64(&self.f64s[range]),
            ResidentValueKind::String => ResidentValueRef::String(&self.strings[range]),
            ResidentValueKind::Snapshot => ResidentValueRef::Snapshot(&self.snapshots[range]),
        }
    }

    pub(crate) fn read_f64(&self, region: ResidentRegion) -> Option<&[f64]> {
        (region.kind == ResidentValueKind::F64)
            .then(|| self.f64s.get(region.offset..region.offset + region.len))?
    }

    #[inline(always)]
    pub(crate) fn write(&mut self, region: ResidentRegion) -> ResidentValueMut<'_> {
        let range = region.offset..region.offset + region.len;
        match region.kind {
            ResidentValueKind::Bool => ResidentValueMut::Bool(&mut self.bools[range]),
            ResidentValueKind::Index => ResidentValueMut::Index(&mut self.indexes[range]),
            ResidentValueKind::F64 => ResidentValueMut::F64(&mut self.f64s[range]),
            ResidentValueKind::String => ResidentValueMut::String(&mut self.strings[range]),
            ResidentValueKind::Snapshot => ResidentValueMut::Snapshot(&mut self.snapshots[range]),
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
            (ResidentValueMut::String(target), ResidentValueRef::String(source)) => {
                target.clone_from_slice(source)
            }
            (ResidentValueMut::Snapshot(target), ResidentValueRef::Snapshot(source)) => {
                target.clone_from_slice(source)
            }
            _ => unreachable!("resident region kinds were checked"),
        }
    }

    fn copy_region_within(&mut self, target: ResidentRegion, source: ResidentRegion) {
        debug_assert_eq!(target.kind, source.kind);
        debug_assert_eq!(target.len, source.len);
        let source = source.offset..source.offset + source.len;
        match target.kind {
            ResidentValueKind::Bool => self.bools.copy_within(source, target.offset),
            ResidentValueKind::Index => self.indexes.copy_within(source, target.offset),
            ResidentValueKind::F64 => self.f64s.copy_within(source, target.offset),
            ResidentValueKind::String => {
                let values = self.strings[source].to_vec();
                self.strings[target.offset..target.offset + target.len].clone_from_slice(&values);
            }
            ResidentValueKind::Snapshot => {
                let values = self.snapshots[source].to_vec();
                self.snapshots[target.offset..target.offset + target.len].clone_from_slice(&values);
            }
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
    version_by_slot: Box<[Option<usize>]>,
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
        let mut version_by_slot = vec![None; slots.len()];
        for (index, version) in versions.iter().enumerate() {
            version_by_slot[version.slot.get() as usize] = Some(index);
        }
        Self {
            buffers: [
                TypedResidentArena::allocate(sizes),
                TypedResidentArena::allocate(sizes),
            ],
            versions,
            version_by_slot: version_by_slot.into_boxed_slice(),
        }
    }

    fn version(&self, slot: CellSlotId) -> &StateVersion {
        let index = self.version_by_slot[slot.get() as usize]
            .expect("activated state slot has a version record");
        &self.versions[index]
    }

    fn version_mut(&mut self, slot: CellSlotId) -> &mut StateVersion {
        let index = self.version_by_slot[slot.get() as usize]
            .expect("activated state slot has a version record");
        &mut self.versions[index]
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
            + self.buffers[0].strings.len() * core::mem::size_of::<String>()
            + self.buffers[0].snapshots.len() * core::mem::size_of::<Option<Value>>()
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
        crate::resident_value_adapter::write_value(&mut self.buffers[0], region, value)?;
        Ok(())
    }

    fn initialize_from_arena(
        &mut self,
        slot: CellSlotId,
        source: &TypedResidentArena,
        source_region: ResidentRegion,
    ) {
        let target = self.version(slot).region;
        self.buffers[0].copy_region_from(target, source, source_region);
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
    pub(crate) all_outputs_initialized: bool,
    pub(crate) touched_slots: Vec<SlotIndex>,
    pub(crate) changed_slots: Vec<SlotIndex>,
    pub(crate) effect_intents: Vec<ResidentEffectIntent>,
    pub(crate) effect_payloads: TypedResidentArena,
    state_f64_arena_by_slot: Box<[u8]>,
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
            all_outputs_initialized: false,
            touched_slots: Vec::with_capacity(plan.state_slots.len()),
            changed_slots: Vec::with_capacity(plan.state_slots.len()),
            effect_intents: Vec::with_capacity(
                plan.steps
                    .iter()
                    .filter(|step| matches!(step, ActivatedTurnStep::External(_)))
                    .count(),
            ),
            effect_payloads: TypedResidentArena::allocate(plan.effect_payload_sizes),
            state_f64_arena_by_slot: vec![0; plan.slots.len()].into_boxed_slice(),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateMigrationMapping {
    pub source: CellSlotId,
    pub target: CellSlotId,
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
    String {
        shape: ResidentShape,
        values: &'a [String],
    },
    Snapshot {
        shape: ResidentShape,
        values: &'a [Option<Value>],
    },
}

impl ResidentValueBorrow<'_> {
    pub fn len(&self) -> usize {
        match self {
            Self::Bool { values, .. } => values.len(),
            Self::Index { values, .. } => values.len(),
            Self::F64 { values, .. } => values.len(),
            Self::String { values, .. } => values.len(),
            Self::Snapshot { values, .. } => values.len(),
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
    candidate_epoch: Option<InstanceEpoch>,
}

/// Trusted cross-crate authority for publishing an externally coordinated
/// resident candidate.
///
/// # Safety
///
/// This unsafe trait is a deliberately narrow cross-crate trust boundary. It
/// exists because Rust cannot express "implementable only by mech-runtime".
/// Implementors must remain inaccessible outside a coordinator that completes
/// every authorization, provider, receipt, outbox, and cleanup obligation
/// before invoking external publication. It must not become a general runtime
/// privilege or capability mechanism.
#[doc(hidden)]
pub unsafe trait ResidentExternalPublicationAuthority {}

impl ReactiveInstance {
    pub fn published_epoch(&self) -> InstanceEpoch {
        InstanceEpoch::new(self.published_epoch.load(Ordering::Acquire))
    }

    pub fn has_active_candidate(&self) -> bool {
        self.candidate_active
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
            ResidentValueRef::String(values) => ResidentValueBorrow::String {
                shape: output.region.shape,
                values,
            },
            ResidentValueRef::Snapshot(values) => ResidentValueBorrow::Snapshot {
                shape: output.region.shape,
                values,
            },
        })
    }

    pub fn state_borrow(&self, slot: CellSlotId) -> Option<ResidentValueBorrow<'_>> {
        let resolved = self.plan.slots.get(slot.get() as usize)?;
        if resolved.role != SlotRole::State {
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
            ResidentValueRef::String(values) => ResidentValueBorrow::String {
                shape: resolved.region.shape,
                values,
            },
            ResidentValueRef::Snapshot(values) => ResidentValueBorrow::Snapshot {
                shape: resolved.region.shape,
                values,
            },
        })
    }

    /// Install compatible persistent storage from another live instance
    /// without changing this instance's already-validated plan.
    ///
    /// Interactive hosts activate replacement programs in isolation before
    /// calling this method. Mutable state and materialized output projections
    /// share the state arena and migrate together, keeping the replacement's
    /// published snapshot coherent. Migration is staged in a cloned arena, so
    /// an invalid or incompatible mapping cannot partially mutate the candidate.
    pub fn migrate_compatible_state_from(
        &mut self,
        source: &ReactiveInstance,
        state_map: &[StateMigrationMapping],
    ) -> Result<(), ResidentActivationError> {
        if self.candidate_active || source.candidate_active {
            return Err(ResidentActivationError::ActiveCandidate);
        }

        let mut targets = BTreeSet::<CellSlotId>::new();
        let mut sources = BTreeSet::<CellSlotId>::new();
        let mut migrated = self.state.clone();
        let target_epoch = self.published_epoch();
        let source_epoch = source.published_epoch();

        for mapping in state_map {
            if !targets.insert(mapping.target) || !sources.insert(mapping.source) {
                return Err(ResidentActivationError::InvalidStateMigration);
            }
            let Some(target) = self.plan.slots.get(mapping.target.get() as usize) else {
                return Err(ResidentActivationError::InvalidStateMigration);
            };
            let Some(source_slot) = source.plan.slots.get(mapping.source.get() as usize) else {
                return Err(ResidentActivationError::InvalidStateMigration);
            };
            if target.role != source_slot.role
                || !matches!(target.role, SlotRole::State | SlotRole::Output)
                || target.schema_key != source_slot.schema_key
                || target.shape != source_slot.shape
            {
                return Err(ResidentActivationError::IncompatibleState {
                    slot: mapping.target,
                });
            }
            migrated.install_migrated(
                mapping.target,
                target_epoch,
                &source.state,
                mapping.source,
                source_epoch,
            );
        }

        self.state = migrated;
        Ok(())
    }

    pub fn reactivate(
        &mut self,
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        facts: &ActivationFacts,
        migration: StateMigrationPolicy,
    ) -> Result<(), ResidentActivationError> {
        self.reactivate_with_state_map(artifact, catalog, facts, migration, &[])
    }

    pub fn reactivate_with_state_map(
        &mut self,
        artifact: &ProgramArtifact,
        catalog: &FunctionCatalog,
        facts: &ActivationFacts,
        migration: StateMigrationPolicy,
        state_map: &[StateMigrationMapping],
    ) -> Result<(), ResidentActivationError> {
        if self.candidate_active {
            return Err(ResidentActivationError::ActiveCandidate);
        }
        if artifact.revision() == self.plan.program_revision
            && activation_facts_fingerprint(facts) == self.plan.activation_facts_fingerprint
        {
            return Ok(());
        }
        let next_plan = self
            .plan
            .plan_generation
            .checked_next()
            .map_err(|_| ResidentActivationError::PlanGenerationExhausted)?;
        let mut replacement = activate_with_options(
            self.id,
            artifact,
            catalog,
            facts,
            ResidentActivationOptions {
                integrity: self.plan.integrity_mode,
                external: self.plan.external_admission,
            },
        )?;
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
        let mut mappings = BTreeMap::<CellSlotId, CellSlotId>::new();
        let mut sources = BTreeSet::<CellSlotId>::new();
        if artifact.revision() == self.plan.program_revision && state_map.is_empty() {
            for slot in replacement
                .plan
                .slots
                .iter()
                .filter(|slot| slot.role == SlotRole::State)
            {
                mappings.insert(slot.artifact_id, slot.artifact_id);
                sources.insert(slot.artifact_id);
            }
        }
        for mapping in state_map {
            if mappings.insert(mapping.target, mapping.source).is_some()
                || !sources.insert(mapping.source)
            {
                return Err(ResidentActivationError::InvalidStateMigration);
            }
        }
        for target in mappings.keys() {
            if replacement
                .plan
                .slots
                .get(target.get() as usize)
                .is_none_or(|slot| slot.role != SlotRole::State)
            {
                return Err(ResidentActivationError::InvalidStateMigration);
            }
        }
        for target in replacement
            .plan
            .slots
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
        {
            let compatible = mappings.get(&target.artifact_id).and_then(|source_id| {
                self.plan
                    .slots
                    .get(source_id.get() as usize)
                    .filter(|source| {
                        source.role == SlotRole::State
                            && source.schema_key == target.schema_key
                            && source.shape == target.shape
                    })
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
    UnsupportedChangeDetection {
        node: NodeId,
    },
    InvalidAlias {
        node: NodeId,
    },
    InvalidNodeOutput {
        node: NodeId,
    },
    InvalidExternalNode {
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
    UnknownOutput {
        output: usize,
    },
    ActiveCandidate,
    IncompatibleState {
        slot: CellSlotId,
    },
    InvalidStateMigration,
    PlanGenerationExhausted,
    LayoutGenerationExhausted,
}

pub fn activate(
    id: ReactiveInstanceId,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    facts: &ActivationFacts,
) -> Result<ReactiveInstance, ResidentActivationError> {
    activate_with_options(
        id,
        artifact,
        catalog,
        facts,
        ResidentActivationOptions::default(),
    )
}

pub fn activate_with_options(
    id: ReactiveInstanceId,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    facts: &ActivationFacts,
    options: ResidentActivationOptions,
) -> Result<ReactiveInstance, ResidentActivationError> {
    activate_internal(id, artifact, catalog, facts, options)
}

#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct ResidentActivationPreflight {
    pub plan: ActivatedPlan,
    pub concrete_cases: Box<[ConcreteExecutionCase]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteExecutionCase {
    pub node: NodeId,
    pub operation: OperationReference,
    pub input_schemas: Box<[SchemaId]>,
    pub output_schema: SchemaId,
    pub targets: ExecutionTargetSet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationUnavailableForTarget {
    pub node: Option<NodeId>,
    pub operation: Option<OperationReference>,
    pub target: ExecutionTarget,
    pub reason: String,
}

/// Validates and plans resident activation without allocating an instance
/// identity or creating mutable resident state.
#[doc(hidden)]
pub fn preflight_activation(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    facts: &ActivationFacts,
    options: ResidentActivationOptions,
) -> Result<ResidentActivationPreflight, ResidentActivationError> {
    preflight_state_initializers(artifact)?;
    let classification = classify_nodes(artifact, options.external)?;
    let layout = build_layout(artifact, facts, &classification)?;
    let facts_fingerprint = activation_facts_fingerprint(facts);
    let (plan, _, _, _, _) = build_plan(
        artifact,
        catalog,
        classification,
        layout,
        facts_fingerprint,
        options,
    )?;
    Ok(ResidentActivationPreflight {
        plan,
        concrete_cases: resident_concrete_execution_cases(artifact)?,
    })
}

/// Produces the concrete resident capability witness before any resident
/// instance or target-specific artifact is emitted. An unsupported concrete
/// layout is reported as target unavailability rather than escaping later as
/// a missing factory during loading.
pub fn preflight_resident_target(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    facts: &ActivationFacts,
    options: ResidentActivationOptions,
) -> Result<ResidentActivationPreflight, OperationUnavailableForTarget> {
    preflight_activation(artifact, catalog, facts, options).map_err(|error| {
        let node = resident_activation_error_node(&error);
        let operation = node.and_then(|node| {
            artifact
                .nodes()
                .get(node.get() as usize)
                .map(|node| node.operation.clone())
        });
        OperationUnavailableForTarget {
            node,
            operation,
            target: ExecutionTarget::ResidentCpu,
            reason: format!("{error:?}"),
        }
    })
}

fn resident_activation_error_node(error: &ResidentActivationError) -> Option<NodeId> {
    match error {
        ResidentActivationError::LegacyOpaque { node }
        | ResidentActivationError::UnsupportedInteraction { node }
        | ResidentActivationError::UnsupportedDelivery { node }
        | ResidentActivationError::UnsupportedConstruction { node }
        | ResidentActivationError::UnsupportedChangeDetection { node }
        | ResidentActivationError::InvalidAlias { node }
        | ResidentActivationError::InvalidNodeOutput { node }
        | ResidentActivationError::InvalidExternalNode { node }
        | ResidentActivationError::MissingResidentFactory { node }
        | ResidentActivationError::KernelBind { node, .. }
        | ResidentActivationError::ActivationKernel { node }
        | ResidentActivationError::InvalidDependency { node } => Some(*node),
        _ => None,
    }
}

fn resident_concrete_execution_cases(
    artifact: &ProgramArtifact,
) -> Result<Box<[ConcreteExecutionCase]>, ResidentActivationError> {
    artifact
        .nodes()
        .iter()
        .filter(|node| {
            artifact
                .contracts()
                .get(node.contract)
                .is_some_and(|contract| {
                    matches!(
                        contract,
                        mech_core::ResolvedOperationContract::Declared(contract)
                            if contract.interaction == ExternalInteraction::Pure
                    )
                })
        })
        .map(|node| {
            let input_schemas = node_inputs(artifact, node.node)?
                .into_iter()
                .map(|source| match source {
                    ArtifactSource::Constant(constant) => artifact
                        .constants()
                        .get(constant)
                        .map(Value::schema)
                        .ok_or(ResidentActivationError::InvalidDependency { node: node.node }),
                    ArtifactSource::Slot(slot) => artifact
                        .slots()
                        .get(slot.get() as usize)
                        .map(|slot| slot.schema)
                        .ok_or(ResidentActivationError::InvalidDependency { node: node.node }),
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_boxed_slice();
            let output = node_output_slot(artifact, node.node)?;
            let output_schema = artifact
                .slots()
                .get(output.get() as usize)
                .map(|slot| slot.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?;
            Ok(ConcreteExecutionCase {
                node: node.node,
                operation: node.operation.clone(),
                input_schemas,
                output_schema,
                targets: ExecutionTargetSet::RESIDENT_CPU,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Vec::into_boxed_slice)
}

/// Activates an external resident instance structurally. Safe engine consumers
/// can prepare and abort its candidates, but only the runtime coordinator owns
/// the audited authority required to publish them.
pub fn activate_external(
    id: ReactiveInstanceId,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    facts: &ActivationFacts,
    integrity: ResidentIntegrityMode,
) -> Result<ReactiveInstance, ResidentActivationError> {
    activate_internal(
        id,
        artifact,
        catalog,
        facts,
        ResidentActivationOptions {
            integrity,
            external: ResidentExternalAdmission::StructuralOnly,
        },
    )
}

fn activate_internal(
    id: ReactiveInstanceId,
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    facts: &ActivationFacts,
    options: ResidentActivationOptions,
) -> Result<ReactiveInstance, ResidentActivationError> {
    preflight_state_initializers(artifact)?;
    let classification = classify_nodes(artifact, options.external)?;
    let layout = build_layout(artifact, facts, &classification)?;
    let facts_fingerprint = activation_facts_fingerprint(facts);
    let (plan, input_sizes, state_sizes, scratch_sizes, activation_sizes) = build_plan(
        artifact,
        catalog,
        classification,
        layout,
        facts_fingerprint,
        options,
    )?;
    let mut activation = TypedResidentArena::allocate(activation_sizes);
    for raw in 0..artifact.constants().len() {
        let constant = ConstantId::new(raw as u32);
        let value = artifact
            .constants()
            .get(constant)
            .expect("dense constant store");
        crate::resident_value_adapter::write_value(
            &mut activation,
            plan.constant_regions[raw],
            value,
        )?;
    }
    execute_activation_graph(&plan, &mut activation)?;
    let mut state = StateArena::new(state_sizes, &plan.slots);
    for slot in plan
        .slots
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
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
    for materialization in plan.output_materializations.iter().copied() {
        let declaration = &artifact.slots()[materialization.target.get() as usize];
        if let Some(InitializerReference::Constant(constant)) = declaration.initializer {
            let value = artifact
                .constants()
                .get(constant)
                .ok_or(ResidentActivationError::InvalidSnapshotRepresentation)?;
            state.initialize(materialization.target, value)?;
        } else if let ResidentReadLocation::Constant(source) = materialization.source {
            state.initialize_from_arena(materialization.target, &activation, source);
        }
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
        candidate_epoch: None,
    })
}

fn preflight_state_initializers(artifact: &ProgramArtifact) -> Result<(), ResidentActivationError> {
    for slot in artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
    {
        let Some(InitializerReference::Constant(constant)) = slot.initializer else {
            return Err(ResidentActivationError::MissingStateInitializer { slot: slot.slot });
        };
        if artifact.constants().get(constant).is_none() {
            return Err(ResidentActivationError::MissingStateInitializer { slot: slot.slot });
        }
    }
    Ok(())
}

fn activation_facts_fingerprint(facts: &ActivationFacts) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update((facts.slot_shapes.len() as u64).to_le_bytes());
    for (slot, shape) in &facts.slot_shapes {
        hash.update(slot.get().to_le_bytes());
        hash.update(shape.canonical_bytes());
    }
    hash.finalize().into()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeClass {
    Observation,
    Activation,
    Turn,
    External,
}

fn classify_nodes(
    artifact: &ProgramArtifact,
    external_admission: ResidentExternalAdmission,
) -> Result<Box<[NodeClass]>, ResidentActivationError> {
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
        match contract.interaction {
            ExternalInteraction::Pure => {}
            ExternalInteraction::Observation(observation)
                if observation.replay == ObservationReplayPolicy::CaptureAsInputFact =>
            {
                classes[node.node.get() as usize] = NodeClass::Observation;
            }
            ExternalInteraction::Effect(_) | ExternalInteraction::TransactionalExternal(_)
                if external_admission == ResidentExternalAdmission::StructuralOnly =>
            {
                classes[node.node.get() as usize] = NodeClass::External;
            }
            _ => return Err(ResidentActivationError::UnsupportedInteraction { node: node.node }),
        }
    }
    loop {
        let before = activation.len();
        for node in artifact.nodes() {
            if matches!(
                classes[node.node.get() as usize],
                NodeClass::Observation | NodeClass::External
            ) {
                continue;
            }
            let output = node_output_slot(artifact, node.node)?;
            if artifact.slots()[output.get() as usize].role != SlotRole::Derived {
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
        match classes[node.node.get() as usize] {
            NodeClass::Observation => {
                let [output] = contract.outputs.as_ref() else {
                    return Err(ResidentActivationError::InvalidNodeOutput { node: node.node });
                };
                let output_slot = node_output_slot(artifact, node.node)?;
                let output_role = artifact.slots()[output_slot.get() as usize].role;
                if output_role != SlotRole::Derived {
                    return Err(ResidentActivationError::InvalidNodeOutput { node: node.node });
                }
                if !matches!(output.construction, OutputConstruction::FullWrite { .. }) {
                    return Err(ResidentActivationError::UnsupportedConstruction {
                        node: node.node,
                    });
                }
            }
            NodeClass::Activation => {
                let [output] = contract.outputs.as_ref() else {
                    return Err(ResidentActivationError::InvalidNodeOutput { node: node.node });
                };
                let output_slot = node_output_slot(artifact, node.node)?;
                let output_role = artifact.slots()[output_slot.get() as usize].role;
                if output_role != SlotRole::Derived
                    || !matches!(
                        output.construction,
                        OutputConstruction::FullWrite { .. } | OutputConstruction::Build { .. }
                    )
                    || output.alias != AliasPolicy::NoAlias
                {
                    return Err(ResidentActivationError::UnsupportedConstruction {
                        node: node.node,
                    });
                }
            }
            NodeClass::Turn => {
                let [output] = contract.outputs.as_ref() else {
                    return Err(ResidentActivationError::InvalidNodeOutput { node: node.node });
                };
                let output_slot = node_output_slot(artifact, node.node)?;
                let output_role = artifact.slots()[output_slot.get() as usize].role;
                match output.construction {
                    OutputConstruction::FullWrite { .. } => {
                        if matches!(output_role, SlotRole::Input)
                            || output.access != AccessMode::Write
                            || output.alias != AliasPolicy::NoAlias
                        {
                            return Err(ResidentActivationError::InvalidAlias { node: node.node });
                        }
                    }
                    OutputConstruction::Build { .. } => {
                        if output_role != SlotRole::Derived
                            || output.access != AccessMode::Write
                            || output.alias != AliasPolicy::NoAlias
                        {
                            return Err(ResidentActivationError::InvalidAlias { node: node.node });
                        }
                    }
                    OutputConstruction::ReadModifyWrite { base_input, .. } => {
                        if output_role != SlotRole::State
                            || output.access != AccessMode::ReadWrite
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
                }
            }
            NodeClass::External => {
                if contract.inputs.len() != 1
                    || !contract.outputs.is_empty()
                    || !node.output_bindings.is_empty()
                    || node.requirement.is_none()
                {
                    return Err(ResidentActivationError::InvalidExternalNode { node: node.node });
                }
            }
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
            ProducerReference::Output { .. } => None,
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
            SlotRole::Output => ResidentStorageClass::State,
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
            role: declaration.role,
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
    if let ProducerReference::Output { source, .. } = declaration.producer {
        return match source {
            ArtifactSource::Constant(constant) => {
                Ok(artifact.constants().get(constant).unwrap().shape().clone())
            }
            ArtifactSource::Slot(source) => slot_shape(artifact, source, facts),
        };
    }
    if let Some(shape) = facts.slot_shapes.get(&slot) {
        let schema = artifact
            .schemas()
            .entry(declaration.schema)
            .expect("validated slot schema")
            .schema();
        return schema
            .instantiate_shape(shape.parameter_values().to_vec().into_boxed_slice())
            .map_err(|_| ResidentActivationError::UnresolvedShape { slot });
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
        SchemaBody::String => Some(ResidentValueKind::String),
        _ => None,
    };
    match schema_entry.schema().body() {
        body @ (SchemaBody::Bool
        | SchemaBody::Index
        | SchemaBody::String
        | SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)) => {
            Ok((kind(body).unwrap(), ResidentShape::SCALAR))
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 && kind(element).is_some() => {
            let kind = kind(element).expect("guarded resident matrix element kind");
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
        _ => Ok((ResidentValueKind::Snapshot, ResidentShape::SCALAR)),
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
    activation_facts_fingerprint: [u8; 32],
    options: ResidentActivationOptions,
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
    let mut steps = Vec::new();
    let mut activation_steps = Vec::new();
    let mut artifact_to_activated = vec![None; artifact.nodes().len()];
    let mut effect_ordinal = 0_u32;
    let mut effect_payload_sizes = ResidentArenaSizes::default();
    for node in artifact.nodes() {
        let class = classes[node.node.get() as usize];
        if class == NodeClass::Observation {
            continue;
        }
        let input_sources = node_inputs(artifact, node.node)?;
        let mech_core::ResolvedOperationContract::Declared(contract) =
            artifact.contracts().get(node.contract).unwrap()
        else {
            unreachable!()
        };
        if class == NodeClass::External {
            let [source] = input_sources.as_slice() else {
                return Err(ResidentActivationError::InvalidExternalNode { node: node.node });
            };
            let requirement = node
                .requirement
                .ok_or(ResidentActivationError::InvalidExternalNode { node: node.node })?;
            let payload = resolve_read(&layout, *source)?;
            let payload_layout = source_port_layout(artifact, &layout, *source)?;
            let payload_shape = match source {
                ArtifactSource::Slot(slot) => layout.slots[slot.get() as usize].shape.clone(),
                ArtifactSource::Constant(constant) => {
                    artifact.constants().get(*constant).unwrap().shape().clone()
                }
            };
            let payload_region = payload.region();
            let captured_payload = ResidentRegion {
                kind: payload_region.kind,
                offset: effect_payload_sizes
                    .allocate(payload_region.kind, payload_region.len)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?,
                len: payload_region.len,
                shape: payload_region.shape,
            };
            let activated = ActivatedNodeIndex(steps.len() as u32);
            artifact_to_activated[node.node.get() as usize] = Some(activated);
            steps.push(ActivatedTurnStep::External(ActivatedExternalNode {
                artifact_node: node.node,
                requirement,
                interaction: contract.interaction.clone(),
                payload,
                captured_payload,
                effect_ordinal,
                payload_schema: payload_layout.schema_id,
                payload_shape,
            }));
            effect_ordinal = effect_ordinal
                .checked_add(1)
                .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            continue;
        }
        let output_slot = node_output_slot(artifact, node.node)?;
        let output = &layout.slots[output_slot.get() as usize];
        let output_contract = &contract.outputs[0];
        if output_contract.change_detection == ChangeDetectionPolicy::SemanticHash
            || (output_contract.change_detection == ChangeDetectionPolicy::ExactScalar
                && output.region.len != 1)
        {
            return Err(ResidentActivationError::UnsupportedChangeDetection { node: node.node });
        }
        let base = match output_contract.construction {
            OutputConstruction::ReadModifyWrite { base_input, .. } => Some(base_input as usize),
            _ => None,
        };
        let input_layouts = input_sources
            .iter()
            .map(|source| source_port_layout(artifact, &layout, *source))
            .collect::<Result<Vec<_>, _>>()?;
        let output_layout = slot_port_layout(output);
        let bind_request = ResidentKernelBindRequest {
            contract: artifact.contracts().get(node.contract).unwrap(),
            schemas: artifact.schemas(),
            inputs: &input_layouts,
            output: output_layout,
        };
        let kernel = if let Some(factory) =
            catalog.resident_factory(&node.operation.module_path, &node.operation.operation_name)
        {
            (factory.factory)(&bind_request)
        } else {
            #[cfg(feature = "dynamic-modules")]
            {
                crate::function::bind_dynamic_resident_operation(
                    &node.operation.module_path,
                    &node.operation.operation_name,
                    &bind_request,
                )
                .ok_or(ResidentActivationError::MissingResidentFactory { node: node.node })?
            }
            #[cfg(not(feature = "dynamic-modules"))]
            {
                return Err(ResidentActivationError::MissingResidentFactory { node: node.node });
            }
        }
        .map_err(|error| ResidentActivationError::KernelBind {
            node: node.node,
            error,
        })?;
        if class == NodeClass::Activation {
            activation_steps.push(ActivatedOnceNode {
                artifact_node: node.node,
                sources: input_sources.into_boxed_slice(),
                storage: output.storage,
                write: output.region,
                kernel,
            });
            continue;
        }
        let activated = ActivatedNodeIndex(steps.len() as u32);
        artifact_to_activated[node.node.get() as usize] = Some(activated);
        let read_start = reads.len() as u32;
        let mut reads_state = false;
        for (ordinal, source) in input_sources.iter().enumerate() {
            if Some(ordinal) != base {
                let read = resolve_read(&layout, *source)?;
                reads_state |= matches!(read, ResidentReadLocation::State { .. });
                reads.push(read);
            }
        }
        let scratch_prefix_reads = output.storage == ResidentStorageClass::Scratch
            && reads[read_start as usize..].iter().all(|read| match read {
                ResidentReadLocation::Scratch(region) if region.kind == output.region.kind => {
                    region.offset + region.len <= output.region.offset
                }
                _ => true,
            });
        steps.push(ActivatedTurnStep::Kernel(ActivatedKernelNode {
            artifact_node: node.node,
            reads: read_start..reads.len() as u32,
            write: ResidentWriteLocation {
                slot: output_slot,
                storage: output.storage,
                region: output.region,
            },
            construction: output_contract.construction.clone(),
            change_detection: output_contract.change_detection,
            reads_state,
            scratch_prefix_reads,
            kernel,
        }));
    }
    let activation_steps = order_activation_steps(artifact, &classes, activation_steps)?;
    let mut topology = build_topology(artifact, &classes, &steps, &artifact_to_activated)?;
    let execution_node_order = build_execution_node_order(
        artifact,
        &steps,
        &artifact_to_activated,
        &topology,
        options.integrity,
    );
    let mut execution_node_mask = vec![0_u64; topology.word_len()].into_boxed_slice();
    for node in &execution_node_order {
        set_bit(&mut execution_node_mask, node.get() as usize);
    }
    topology.single_word_schedule = build_single_word_schedule(
        &execution_node_order,
        &topology.same_turn_downstream_masks,
        topology.word_len(),
    );
    let inputs = layout
        .slots
        .iter()
        .filter(|slot| slot.storage == ResidentStorageClass::Input)
        .map(|slot| ActivatedInput {
            artifact_slot: slot.artifact_id,
            slot: slot.physical_index,
            schema: slot.schema,
            schema_key: slot.schema_key,
            shape: slot.shape.clone(),
            region: slot.region,
            source: activated_input_source(artifact, slot.artifact_id),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let outputs = artifact
        .outputs()
        .iter()
        .map(|output| {
            let slot = &layout.slots[output.source.get() as usize];
            debug_assert_eq!(slot.storage, ResidentStorageClass::State);
            ActivatedOutput {
                slot: slot.physical_index,
                schema: slot.schema,
                schema_key: slot.schema_key,
                shape: slot.shape.clone(),
                region: slot.region,
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let output_materializations = artifact
        .slots()
        .iter()
        .filter_map(|slot| match (slot.role, slot.producer) {
            (SlotRole::Output, ProducerReference::Output { source, .. }) => Some(
                resolve_read(&layout, source).map(|source| ActivatedOutputMaterialization {
                    target: slot.slot,
                    source,
                }),
            ),
            _ => None,
        })
        .collect::<Result<Vec<_>, ResidentActivationError>>()?
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
            let predicate = resolve_read(&layout, constraint.inputs[0])?;
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
    let state_hash_seed = state_hash_seed(artifact.revision(), &layout.slots, &state_slots);
    let rmw_state_slots = steps
        .iter()
        .filter_map(|step| match step {
            ActivatedTurnStep::Kernel(node)
                if node.write.storage == ResidentStorageClass::State
                    && matches!(
                        node.construction,
                        OutputConstruction::ReadModifyWrite { .. }
                    ) =>
            {
                Some(node.write.slot)
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let activation_nodes = activation_steps
        .iter()
        .map(|step| step.artifact_node)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let input_sizes = layout.sizes[1];
    let state_sizes = layout.sizes[2];
    let scratch_sizes = layout.sizes[3];
    let activation_sizes = layout.sizes[0];
    let f64_read_tape = build_f64_read_tape(&reads);
    let external_step_count = steps
        .iter()
        .filter(|step| matches!(step, ActivatedTurnStep::External(_)))
        .count();
    let pure_kernel_steps = (external_step_count == 0).then(|| {
        steps
            .iter()
            .map(|step| match step {
                ActivatedTurnStep::Kernel(node) => node.clone(),
                ActivatedTurnStep::External(_) => {
                    unreachable!("pure resident plan contains an external step")
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    });
    let plan = ActivatedPlan {
        program_revision: artifact.revision(),
        activation_facts_fingerprint,
        plan_generation: PlanGeneration::ZERO,
        layout_generation: LayoutGeneration::ZERO,
        slots: layout.slots,
        steps: steps.into_boxed_slice(),
        external_step_count,
        pure_kernel_steps,
        reads: reads.into_boxed_slice(),
        f64_read_tape,
        execution_node_order,
        execution_node_mask,
        integrity_mode: options.integrity,
        external_admission: options.external,
        topology,
        inputs,
        outputs,
        output_materializations,
        constraints,
        activation_nodes,
        activation_steps,
        schemas: artifact.schemas().clone(),
        constant_regions: layout.constant_regions,
        state_slots,
        rmw_state_slots,
        state_hash_seed,
        effect_payload_sizes,
    };
    Ok((
        plan,
        input_sizes,
        state_sizes,
        scratch_sizes,
        activation_sizes,
    ))
}

fn build_execution_node_order(
    artifact: &ProgramArtifact,
    steps: &[ActivatedTurnStep],
    artifact_to_activated: &[Option<ActivatedNodeIndex>],
    topology: &DependencyTopology,
    integrity: ResidentIntegrityMode,
) -> Box<[ActivatedNodeIndex]> {
    if integrity == ResidentIntegrityMode::Checked {
        return topology.linear_node_order.clone();
    }
    let mut omitted = vec![false; steps.len()];
    for constraint in artifact.constraints() {
        let Some(ArtifactSource::Slot(slot)) = constraint.inputs.first().copied() else {
            continue;
        };
        let ProducerReference::NodeOutput { node, .. } =
            artifact.slots()[slot.get() as usize].producer
        else {
            continue;
        };
        let Some(activated) = artifact_to_activated[node.get() as usize] else {
            continue;
        };
        if let ActivatedTurnStep::Kernel(candidate) = &steps[activated.get() as usize] {
            if candidate.write.storage == ResidentStorageClass::Scratch
                && topology.same_turn_downstream(activated).is_empty()
            {
                omitted[activated.get() as usize] = true;
            }
        }
    }
    topology
        .linear_node_order
        .iter()
        .copied()
        .filter(|node| !omitted[node.get() as usize])
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn build_f64_read_tape(reads: &[ResidentReadLocation]) -> Option<Box<[F64ReadTapeEntry]>> {
    reads
        .iter()
        .map(|read| {
            let (selector, region) = match *read {
                ResidentReadLocation::Constant(region) => (F64_ACTIVATION_ARENA, region),
                ResidentReadLocation::Input(region) => (F64_INPUT_ARENA, region),
                ResidentReadLocation::Scratch(region) => (F64_SCRATCH_ARENA, region),
                ResidentReadLocation::State { slot, region } if slot.get() < F64_STATE_SLOT_BIT => {
                    (F64_STATE_SLOT_BIT | slot.get(), region)
                }
                ResidentReadLocation::State { .. } => return None,
            };
            if region.kind != ResidentValueKind::F64 {
                return None;
            }
            Some(F64ReadTapeEntry {
                selector,
                start: u32::try_from(region.offset).ok()?,
                end: u32::try_from(region.offset.checked_add(region.len)?).ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()
        .map(Vec::into_boxed_slice)
}

fn state_hash_seed(
    revision: ProgramRevision,
    slots: &[ResolvedSlot],
    state_slots: &[CellSlotId],
) -> u64 {
    let mut hash = 0x243f_6a88_85a3_08d3_u64;
    for chunk in revision.as_bytes().chunks_exact(8) {
        hash = fold_hash_word(hash, u64::from_le_bytes(chunk.try_into().unwrap()));
    }
    for artifact_slot in state_slots {
        let slot = &slots[artifact_slot.get() as usize];
        hash = fold_hash_word(hash, u64::from(slot.artifact_id.get()));
        for chunk in slot.schema_key.as_bytes().chunks_exact(8) {
            hash = fold_hash_word(hash, u64::from_le_bytes(chunk.try_into().unwrap()));
        }
        hash = fold_hash_word(hash, slot.shape.parameter_values().len() as u64);
        for value in slot.shape.parameter_values() {
            hash = fold_hash_word(hash, *value);
        }
        hash = fold_hash_word(hash, u64::from(slot.region.shape.rows));
        hash = fold_hash_word(hash, u64::from(slot.region.shape.columns));
    }
    hash
}

#[inline(always)]
fn fold_hash_word(hash: u64, word: u64) -> u64 {
    (hash.rotate_left(17) ^ word).wrapping_mul(0xd6e8_feb8_6659_fd93)
}

fn execute_activation_graph(
    plan: &ActivatedPlan,
    arena: &mut TypedResidentArena,
) -> Result<(), ResidentActivationError> {
    for step in &plan.activation_steps {
        if step.storage != ResidentStorageClass::Constant {
            return Err(ResidentActivationError::InvalidDependency {
                node: step.artifact_node,
            });
        }
        let inputs = step
            .sources
            .iter()
            .map(|source| owned_activation_input(plan, arena, *source))
            .collect::<Result<Vec<_>, _>>()?;
        step.kernel
            .execute(&OwnedActivationInputs(&inputs), arena.write(step.write))
            .map_err(|_| ResidentActivationError::ActivationKernel {
                node: step.artifact_node,
            })?;
    }
    Ok(())
}

fn order_activation_steps(
    artifact: &ProgramArtifact,
    classes: &[NodeClass],
    steps: Vec<ActivatedOnceNode>,
) -> Result<Box<[ActivatedOnceNode]>, ResidentActivationError> {
    let by_node = steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.artifact_node, index))
        .collect::<BTreeMap<_, _>>();
    let mut downstream = vec![Vec::<usize>::new(); steps.len()];
    for (current, step) in steps.iter().enumerate() {
        for source in &step.sources {
            let ArtifactSource::Slot(slot) = source else {
                continue;
            };
            let ProducerReference::NodeOutput { node, .. } =
                artifact.slots()[slot.get() as usize].producer
            else {
                continue;
            };
            if classes[node.get() as usize] != NodeClass::Activation {
                return Err(ResidentActivationError::InvalidDependency {
                    node: step.artifact_node,
                });
            }
            let parent = *by_node
                .get(&node)
                .ok_or(ResidentActivationError::InvalidDependency {
                    node: step.artifact_node,
                })?;
            if !downstream[parent].contains(&current) {
                downstream[parent].push(current);
            }
        }
    }
    let keys = steps
        .iter()
        .map(|step| step.artifact_node.get())
        .collect::<Vec<_>>();
    let order = stable_topological_order(&downstream, &keys).ok_or_else(|| {
        ResidentActivationError::InvalidDependency {
            node: steps
                .first()
                .map_or(NodeId::new(0), |step| step.artifact_node),
        }
    })?;
    let mut pending = steps.into_iter().map(Some).collect::<Vec<_>>();
    Ok(order
        .into_iter()
        .map(|index| pending[index].take().expect("unique activation order"))
        .collect::<Vec<_>>()
        .into_boxed_slice())
}

fn stable_topological_order(downstream: &[Vec<usize>], keys: &[u32]) -> Option<Vec<usize>> {
    let mut indegree = vec![0_usize; downstream.len()];
    for children in downstream {
        for child in children {
            indegree[*child] = indegree[*child].checked_add(1)?;
        }
    }
    let mut ready = indegree
        .iter()
        .enumerate()
        .filter(|(_, degree)| **degree == 0)
        .map(|(index, _)| (keys[index], index))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(downstream.len());
    while let Some(entry) = ready.pop_first() {
        let index = entry.1;
        order.push(index);
        for child in &downstream[index] {
            indegree[*child] -= 1;
            if indegree[*child] == 0 {
                ready.insert((keys[*child], *child));
            }
        }
    }
    (order.len() == downstream.len()).then_some(order)
}

#[derive(Clone, Debug)]
enum OwnedResidentValue {
    Bool(Box<[u8]>),
    Index(Box<[u64]>),
    F64(Box<[f64]>),
    String(Box<[String]>),
    Snapshot(Box<[Option<Value>]>),
}

impl OwnedResidentValue {
    fn as_ref(&self) -> ResidentValueRef<'_> {
        match self {
            Self::Bool(values) => ResidentValueRef::Bool(values),
            Self::Index(values) => ResidentValueRef::Index(values),
            Self::F64(values) => ResidentValueRef::F64(values),
            Self::String(values) => ResidentValueRef::String(values),
            Self::Snapshot(values) => ResidentValueRef::Snapshot(values),
        }
    }
}

struct OwnedActivationInputs<'a>(&'a [OwnedResidentValue]);

impl ResidentKernelInputs for OwnedActivationInputs<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        self.0.get(index).map(OwnedResidentValue::as_ref)
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
        ResidentValueRef::String(values) => {
            OwnedResidentValue::String(values.to_vec().into_boxed_slice())
        }
        ResidentValueRef::Snapshot(values) => {
            OwnedResidentValue::Snapshot(values.to_vec().into_boxed_slice())
        }
    })
}

fn build_topology(
    artifact: &ProgramArtifact,
    classes: &[NodeClass],
    steps: &[ActivatedTurnStep],
    artifact_to_activated: &[Option<ActivatedNodeIndex>],
) -> Result<DependencyTopology, ResidentActivationError> {
    let mut downstream = vec![Vec::<ActivatedNodeIndex>::new(); steps.len()];
    let mut latest_state_writer = BTreeMap::<CellSlotId, ActivatedNodeIndex>::new();
    let mut direct_input_consumers = Vec::<ActivatedNodeIndex>::new();
    let mut prior_state_consumers = Vec::<ActivatedNodeIndex>::new();
    for node in artifact.nodes() {
        if !matches!(
            classes[node.node.get() as usize],
            NodeClass::Turn | NodeClass::External
        ) {
            continue;
        }
        let current = artifact_to_activated[node.node.get() as usize].unwrap();
        for source in node_inputs(artifact, node.node)? {
            let ArtifactSource::Slot(slot_id) = source else {
                continue;
            };
            let slot = &artifact.slots()[slot_id.get() as usize];
            let reads_turn_input = match slot.producer {
                ProducerReference::Input(_) => true,
                ProducerReference::NodeOutput { node, .. } => {
                    classes[node.get() as usize] == NodeClass::Observation
                }
                ProducerReference::Output { .. } => false,
            };
            if reads_turn_input && !direct_input_consumers.contains(&current) {
                direct_input_consumers.push(current);
            }
            let parent = if slot.role == SlotRole::State {
                let parent = latest_state_writer.get(&slot_id).copied();
                if parent.is_none() && !prior_state_consumers.contains(&current) {
                    prior_state_consumers.push(current);
                }
                parent
            } else {
                match slot.producer {
                    ProducerReference::NodeOutput { node, .. } => {
                        artifact_to_activated[node.get() as usize]
                    }
                    ProducerReference::Input(_) => None,
                    ProducerReference::Output { .. } => None,
                }
            };
            if let Some(parent) = parent {
                if !downstream[parent.get() as usize].contains(&current) {
                    downstream[parent.get() as usize].push(current);
                }
            }
        }
        if classes[node.node.get() as usize] == NodeClass::Turn {
            let output = node_output_slot(artifact, node.node)?;
            if artifact.slots()[output.get() as usize].role == SlotRole::State {
                latest_state_writer.insert(output, current);
            }
        }
    }
    let mut indegree = vec![0_usize; steps.len()];
    for children in &downstream {
        for child in children {
            indegree[child.get() as usize] += 1;
        }
    }
    let mut roots = indegree
        .iter()
        .enumerate()
        .filter(|(_, degree)| **degree == 0)
        .map(|(index, _)| ActivatedNodeIndex(index as u32))
        .collect::<Vec<_>>();
    for consumer in direct_input_consumers {
        if !roots.contains(&consumer) {
            roots.push(consumer);
        }
    }
    // A read from the published state is a next-turn dependency, not a
    // same-turn edge. Its consumer must run at the start of every accepted
    // turn even when its other (same-turn) inputs retain the same value. If it
    // is seeded only by changed parents, a recurrence such as `x = x + dt`
    // stalls whenever `dt` is constant.
    for consumer in prior_state_consumers {
        if !roots.contains(&consumer) {
            roots.push(consumer);
        }
    }
    roots.sort_by_key(|node| node.get());
    let order_source = downstream
        .iter()
        .map(|children| {
            children
                .iter()
                .map(|child| child.get() as usize)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let keys = steps
        .iter()
        .map(|step| step.artifact_node().get())
        .collect::<Vec<_>>();
    let linear_node_order = stable_topological_order(&order_source, &keys)
        .ok_or_else(|| ResidentActivationError::InvalidDependency {
            node: steps
                .first()
                .map_or(NodeId::new(0), ActivatedTurnStep::artifact_node),
        })?
        .into_iter()
        .map(|index| ActivatedNodeIndex(index as u32))
        .collect::<Vec<_>>();
    let words = steps.len().div_ceil(64);
    let mut masks = Vec::with_capacity(steps.len());
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
    let mut offsets = Vec::with_capacity(steps.len() + 1);
    let mut values = Vec::new();
    offsets.push(0_u32);
    for children in downstream {
        values.extend(children);
        offsets.push(values.len() as u32);
    }
    Ok(DependencyTopology {
        linear_node_order: linear_node_order.into_boxed_slice(),
        single_word_schedule: Box::new([]),
        same_turn_downstream_offsets: offsets.into_boxed_slice(),
        same_turn_downstream_nodes: values.into_boxed_slice(),
        turn_root_nodes: roots.into_boxed_slice(),
        same_turn_downstream_masks: masks.into_boxed_slice(),
        turn_root_mask: root_mask,
        mandatory_candidate_mask: mandatory,
    })
}

fn build_single_word_schedule(
    node_order: &[ActivatedNodeIndex],
    downstream_masks: &[Box<[u64]>],
    words: usize,
) -> Box<[SingleWordScheduleEntry]> {
    if words != 1 {
        return Box::new([]);
    }
    node_order
        .iter()
        .map(|node| {
            let index = node.get() as usize;
            SingleWordScheduleEntry {
                node: *node,
                node_bit: 1_u64 << index,
                downstream: downstream_masks[index][0],
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn set_bit(words: &mut [u64], bit: usize) {
    words[bit / 64] |= 1_u64 << (bit % 64);
}

fn resolve_read(
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
                schema_id: value.schema(),
                schema_key: value.schema_key(),
                kind: region.kind,
                shape: region.shape,
                shape_instance: value.shape().clone(),
                resolved_selector: resident_resolved_selector(value),
            })
        }
        ArtifactSource::Slot(slot) => Ok(slot_port_layout(&layout.slots[slot.get() as usize])),
    }
}

fn slot_port_layout(slot: &ResolvedSlot) -> ResidentPortLayout {
    ResidentPortLayout {
        schema_id: slot.schema,
        schema_key: slot.schema_key,
        kind: slot.region.kind,
        shape: slot.region.shape,
        shape_instance: slot.shape.clone(),
        resolved_selector: None,
    }
}

fn resident_resolved_selector(
    value: &mech_core::Value,
) -> Option<mech_core::ResidentResolvedSelector> {
    use mech_core::ResidentResolvedSelector;

    let one_based = match value.data() {
        mech_core::ValueData::Index(value) | mech_core::ValueData::U64(value) => u128::from(*value),
        mech_core::ValueData::U8(value) => u128::from(*value),
        mech_core::ValueData::U16(value) => u128::from(*value),
        mech_core::ValueData::U32(value) => u128::from(*value),
        mech_core::ValueData::U128(value) => *value,
        mech_core::ValueData::I8(value) if *value >= 0 => *value as u128,
        mech_core::ValueData::I16(value) if *value >= 0 => *value as u128,
        mech_core::ValueData::I32(value) if *value >= 0 => *value as u128,
        mech_core::ValueData::I64(value) if *value >= 0 => *value as u128,
        mech_core::ValueData::I128(value) if *value >= 0 => *value as u128,
        mech_core::ValueData::F32(value)
            if value.to_f32().is_finite()
                && value.to_f32() >= 1.0
                && value.to_f32().fract() == 0.0 =>
        {
            value.to_f32() as u128
        }
        mech_core::ValueData::F64(value)
            if value.to_f64().is_finite()
                && value.to_f64() >= 1.0
                && value.to_f64().fract() == 0.0 =>
        {
            value.to_f64() as u128
        }
        mech_core::ValueData::Id(value) => return Some(ResidentResolvedSelector::Id(*value)),
        _ => return None,
    };
    let ordinal = usize::try_from(one_based.checked_sub(1)?).ok()?;
    Some(ResidentResolvedSelector::Ordinal(ordinal))
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

fn activated_input_source(artifact: &ProgramArtifact, slot: CellSlotId) -> ActivatedInputSource {
    match artifact.slots()[slot.get() as usize].producer {
        ProducerReference::Input(input) => ActivatedInputSource::DeclaredInput { input },
        ProducerReference::NodeOutput { node, .. } => {
            let requirement = artifact.nodes()[node.get() as usize]
                .requirement
                .expect("validated observation has an application requirement");
            ActivatedInputSource::Observation { node, requirement }
        }
        ProducerReference::Output { .. } => {
            unreachable!("published output slots are not resident inputs")
        }
    }
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
