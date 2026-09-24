//! Schema-driven activation for the pre-launch dense numeric resident profile.

mod comprehension;
mod execution;
pub use comprehension::{
    ActivatedCollectionStep, ActivatedComprehensionNode, ActivatedPatternBinding,
};
mod live;

pub use execution::*;

use core::ops::Range;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use mech_core::snapshot::{
    SnapshotValidationContext, ValueDataDraft, ValueDraft, schema_data_language_eq,
    schema_data_partial_cmp, schema_data_snapshot_eq,
};
use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirementId, BoundCall, BoundResidentKernel,
    CallMemoryPlanningRequest, CardinalitySpec, CellSlotId, ChangeDetectionPolicy, ConstantId,
    CurrentMemoryFootprint, DeliveryMode, DimensionExpr, DimensionLifetime, ExecutionTarget,
    ExecutionTargetSet, ExternalInteraction, FunctionCatalog, ImplementationMemoryClass, InputId,
    InstanceEpoch, IntegrityConstraintId, LayoutGeneration, MemoryFootprintWitness, MemoryLifetime,
    MemoryPlanError, MemoryPlanPoint, NodeId, ObservationReplayPolicy, OutputConstruction,
    PlanGeneration, PlannedArenaElement, PlannedArenaProjection, ProgramRevision,
    ReactiveInstanceId, RegionAccessPlan, ResidentBuildContext, ResidentKernelBindError,
    ResidentKernelBindRequest, ResidentKernelInputs, ResidentOperationKey, ResidentPortLayout,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef, ResolvedRangeMode,
    ResolvedSelectionMode, ResolvedType, SchemaBody, SchemaId, SchemaKey, ShapeInstance, ShapeRule,
    SlotIndex, TargetMemoryProfile, Value, execute_conversion_draft, plan_call_memory,
    plan_explicit_cast,
};
use sha2::{Digest, Sha256};

use crate::memory_planner::{
    PlannedValueClass, ProgramMemoryPlan, ResidentValuePlanInput,
    attach_resident_call_memory_template, finalize_resident_current_footprints,
    plan_program_memory_template, plan_resident_arenas, plan_resident_effect_payload,
    resident_arena_id, resident_payload_arena_id, resident_storage_descriptor, schedule_points,
};
use crate::memory_runtime::ManagedProgramMemory;
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
    pub activation_fixed_shape: bool,
    pub storage: ResidentStorageClass,
    pub region: ResidentRegion,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResidentReadLocation {
    Constant(ResidentRegion),
    Input(ResidentRegion),
    /// A bound invocation argument backed by an input before suspension and
    /// by its retained capture frame after suspension.
    LexicalInput(ResidentRegion),
    State {
        slot: CellSlotId,
        region: ResidentRegion,
    },
    Scratch(ResidentRegion),
}

impl ResidentReadLocation {
    pub const fn region(self) -> ResidentRegion {
        match self {
            Self::Constant(region)
            | Self::Input(region)
            | Self::LexicalInput(region)
            | Self::Scratch(region) => region,
            Self::State { region, .. } => region,
        }
    }
}

const F64_ACTIVATION_ARENA: u32 = 0;
const F64_INPUT_ARENA: u32 = 1;
const F64_SCRATCH_ARENA: u32 = 2;
const F64_STATE_ARENA_BASE: u8 = 3;
const F64_STATE_SLOT_BIT: u32 = 1 << 31;
const MAX_STATIC_SELECTOR_SOURCE_STEPS: usize = 65_536;

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
    pub(crate) memory_node: NodeId,
    pub reads: Range<u32>,
    pub write: ResidentWriteLocation,
    pub construction: OutputConstruction,
    pub(crate) rmw_base: Option<ResidentReadLocation>,
    pub(crate) rmw_previous: Option<ResidentRegion>,
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
pub struct ActivatedControlStep {
    pub node: ActivatedNodeIndex,
    /// Prefix of the block's shared local inventory that is initialized when
    /// this child executes.
    pub retained_local_count: u32,
    /// Sorted local indices already owned by the child's call contract.
    pub excluded_locals: Box<[u32]>,
}

#[derive(Clone, Debug)]
pub struct ActivatedControlBlock {
    pub steps: std::sync::Arc<[ActivatedControlStep]>,
    /// Pattern bindings followed by operation outputs in source order. Steps
    /// share this inventory and retain only a prefix plus compact exclusions.
    pub locals: std::sync::Arc<[ResidentRegion]>,
    pub yield_value: ResidentReadLocation,
    pub yield_layout: ResidentPortLayout,
}

#[derive(Clone, Debug)]
pub struct ActivatedMatchArm {
    pub pattern: ActivatedMatchPattern,
    pub binding_regions: Box<[ResidentRegion]>,
    pub guard_regions: Box<[ResidentRegion]>,
    pub guard: Option<ActivatedControlBlock>,
    pub body: ActivatedControlBlock,
}

#[derive(Clone, Copy, Debug)]
pub struct ActivatedPatternValue {
    pub location: ResidentReadLocation,
    pub schema: SchemaId,
}

#[derive(Clone, Debug)]
pub enum ActivatedMatchPattern {
    Literal(ResidentReadLocation),
    Wildcard,
    Bind,
    Structural {
        pattern: crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
        work: u64,
        binding_count: u64,
        equality_count: u64,
        snapshot_finalization_count: u64,
        clone_depth: u64,
    },
}

#[derive(Clone, Debug)]
pub struct ActivatedMatchNode {
    pub artifact_node: NodeId,
    /// A physical control identity with no call-local memory contract of its
    /// own. Nested matches must not borrow the enclosing comprehension's
    /// materialization contract merely because they share an artifact owner.
    pub budget_node: NodeId,
    pub scrutinee: ResidentReadLocation,
    pub scrutinee_schema: SchemaId,
    pub scrutinee_shape_values: Box<[u64]>,
    pub write: ResidentWriteLocation,
    pub arms: Box<[ActivatedMatchArm]>,
    pub locals: Box<[ResidentRegion]>,
    pub continuation: bool,
    pub capture_sources: Box<[ResidentReadLocation]>,
}

#[derive(Clone, Copy, Debug)]
pub struct ActivatedRecursiveCall {
    pub artifact_node: NodeId,
    pub target: ActivatedNodeIndex,
    pub argument: ResidentReadLocation,
    pub write: ResidentWriteLocation,
}

#[derive(Clone, Copy, Debug)]
pub struct ActivatedSuspension {
    pub artifact_node: NodeId,
    pub target: ActivatedNodeIndex,
    pub argument: ResidentReadLocation,
}

#[derive(Clone, Copy, Debug)]
pub struct ActivatedPublication {
    pub artifact_node: NodeId,
    pub target: ActivatedNodeIndex,
    pub value: ResidentReadLocation,
}

#[derive(Clone, Debug)]
pub enum ActivatedTurnStep {
    Match(ActivatedMatchNode),
    Recur(ActivatedRecursiveCall),
    Suspend(ActivatedSuspension),
    Publish(ActivatedPublication),
    Comprehension(std::sync::Arc<ActivatedComprehensionNode>),
    Kernel(ActivatedKernelNode),
    External(ActivatedExternalNode),
}

struct ActivatedMemorySite {
    artifact_node: NodeId,
    memory_node: NodeId,
    reads: Range<u32>,
    write: ResidentWriteLocation,
    construction: OutputConstruction,
    rmw_base: Option<ResidentReadLocation>,
}

impl ActivatedTurnStep {
    fn memory_site(&self) -> Option<ActivatedMemorySite> {
        Some(match self {
            Self::Kernel(node) => ActivatedMemorySite {
                artifact_node: node.artifact_node,
                memory_node: node.memory_node,
                reads: node.reads.clone(),
                write: node.write,
                construction: node.construction.clone(),
                rmw_base: node.rmw_base,
            },
            Self::Comprehension(node) => ActivatedMemorySite {
                artifact_node: node.artifact_node,
                memory_node: node.memory_node,
                reads: node.reads.clone(),
                write: node.write,
                construction: comprehension::materialization_construction(),
                rmw_base: None,
            },
            _ => return None,
        })
    }

    pub fn artifact_node(&self) -> NodeId {
        match self {
            Self::Kernel(node) => node.artifact_node,
            Self::Match(node) => node.artifact_node,
            Self::Recur(node) => node.artifact_node,
            Self::Suspend(node) => node.artifact_node,
            Self::Publish(node) => node.artifact_node,
            Self::Comprehension(node) => node.artifact_node,
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
    pub same_turn_dependency_masks: Box<[Box<[u64]>]>,
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
    producer: Option<ActivatedNodeIndex>,
}

fn output_materialization_depends_on_match(
    plan: &ActivatedPlan,
    materialization: ActivatedOutputMaterialization,
    match_index: usize,
    match_region: ResidentRegion,
) -> bool {
    read_location_depends_on_match(plan, materialization.source, match_index, match_region)
}

fn read_location_depends_on_match(
    plan: &ActivatedPlan,
    location: ResidentReadLocation,
    match_index: usize,
    match_region: ResidentRegion,
) -> bool {
    if location == ResidentReadLocation::Scratch(match_region) {
        return true;
    }
    let producer = plan.steps.iter().position(|step| {
        let write = match step {
            ActivatedTurnStep::Kernel(node) => Some(node.write),
            ActivatedTurnStep::Match(node) => Some(node.write),
            ActivatedTurnStep::Recur(node) => Some(node.write),
            ActivatedTurnStep::Comprehension(node) => Some(node.write),
            ActivatedTurnStep::External(_)
            | ActivatedTurnStep::Suspend(_)
            | ActivatedTurnStep::Publish(_) => None,
        };
        write.is_some_and(|write| {
            location
                == match write.storage {
                    ResidentStorageClass::Constant => ResidentReadLocation::Constant(write.region),
                    ResidentStorageClass::Input => ResidentReadLocation::Input(write.region),
                    ResidentStorageClass::State => ResidentReadLocation::State {
                        slot: write.slot,
                        region: write.region,
                    },
                    ResidentStorageClass::Scratch => ResidentReadLocation::Scratch(write.region),
                }
        })
    });
    producer.is_some_and(|producer| {
        plan.topology.same_turn_dependency_masks[match_index]
            .get(producer / 64)
            .is_some_and(|word| word & (1 << (producer % 64)) != 0)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivatedConstraint {
    pub artifact_id: IntegrityConstraintId,
    pub predicate: ResidentReadLocation,
    pub producer: Option<ActivatedNodeIndex>,
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResidentActivationOptions {
    pub integrity: ResidentIntegrityMode,
    pub external: ResidentExternalAdmission,
    /// Optional caller-owned aggregate backing budget. Its shared ownership
    /// includes old/candidate program coexistence, independently of per-call
    /// output and execution-work limits.
    pub memory_budget: Option<mech_core::ManagedMemoryBudget>,
}

#[derive(Clone, Debug)]
pub struct ActivatedPlan {
    pub program_revision: ProgramRevision,
    /// Identity of caller-supplied activation facts. Deterministic facts
    /// completed from the artifact belong to this plan, not its request key.
    pub activation_facts_fingerprint: [u8; 32],
    pub plan_generation: PlanGeneration,
    pub layout_generation: LayoutGeneration,
    pub memory_plan: ProgramMemoryPlan,
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
    /// Artifact input slots that schedule a turn. Inputs used only as
    /// activation captures are retained as sampled values instead.
    pub turn_trigger_inputs: Box<[CellSlotId]>,
    /// Per-activation input roots. Runtime turns use this to schedule exactly
    /// the scopes whose trigger observations arrived in the admitted batch.
    activation_turn_inputs: Box<
        [(
            ActivatedNodeIndex,
            Box<[CellSlotId]>,
            Box<[ActivatedNodeIndex]>,
            Box<[ActivatedNodeIndex]>,
        )],
    >,
    pub outputs: Box<[ActivatedOutput]>,
    output_materializations: Box<[ActivatedOutputMaterialization]>,
    pub constraints: Box<[ActivatedConstraint]>,
    pub activation_nodes: Box<[NodeId]>,
    activation_steps: Box<[ActivatedOnceNode]>,
    pub(crate) schemas: std::sync::Arc<mech_core::SchemaTable>,
    pub(crate) structural_projections: execution::StructuralProjectionTable,
    pub(crate) constant_regions: Box<[ResidentRegion]>,
    pub(crate) state_slots: Box<[CellSlotId]>,
    pub(crate) rmw_state_slots: Box<[CellSlotId]>,
    pub(crate) state_hash_seed: u64,
}

impl ActivatedPlan {
    pub fn schemas(&self) -> &mech_core::SchemaTable {
        &self.schemas
    }

    pub fn execution_node_count(&self) -> usize {
        self.execution_node_order.len()
    }

    pub fn has_only_kernel_steps(&self) -> bool {
        self.pure_kernel_steps.is_some()
    }

    pub fn has_external_steps(&self) -> bool {
        self.external_step_count != 0
    }

    pub fn has_input_free_activation_roots(&self) -> bool {
        self.activation_turn_inputs
            .iter()
            .any(|(_, inputs, _, _)| inputs.is_empty())
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
    base_input: Option<usize>,
    storage: ResidentStorageClass,
    write: ResidentRegion,
    body: ActivatedOnceBody,
}

#[derive(Clone, Debug)]
enum ActivatedOnceBody {
    Kernel(BoundResidentKernel),
    Control(ActivatedNodeIndex),
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
    fn from_memory_plan_buffer(
        plan: &ProgramMemoryPlan,
        class: ResidentStorageClass,
        buffer: u8,
    ) -> Result<Self, ResidentActivationError> {
        let class = match class {
            ResidentStorageClass::Constant => PlannedValueClass::Constant,
            ResidentStorageClass::Input => PlannedValueClass::Input,
            ResidentStorageClass::State => PlannedValueClass::State,
            ResidentStorageClass::Scratch => PlannedValueClass::Scratch,
        };
        let capacity = |kind, bytes: usize| -> Result<usize, ResidentActivationError> {
            let arena = resident_arena_id((class, kind, buffer))
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
            let bytes =
                u64::try_from(bytes).map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
            let capacity = plan
                .arenas
                .iter()
                .find(|candidate| candidate.id == arena)
                .map(|arena| arena.capacity_bytes)
                .unwrap_or(0);
            if bytes == 0 || capacity % bytes != 0 {
                return Err(ResidentActivationError::RegionSizeOverflow);
            }
            usize::try_from(capacity / bytes)
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)
        };
        Ok(Self {
            bools: capacity(ResidentValueKind::Bool, core::mem::size_of::<u8>())?,
            indexes: capacity(ResidentValueKind::Index, core::mem::size_of::<u64>())?,
            f64s: capacity(ResidentValueKind::F64, core::mem::size_of::<f64>())?,
            strings: capacity(ResidentValueKind::String, core::mem::size_of::<String>())?,
            snapshots: capacity(
                ResidentValueKind::Snapshot,
                core::mem::size_of::<Option<Value>>(),
            )?,
        })
    }
}

enum ResidentLane<T: PlannedArenaElement> {
    Managed(PlannedArenaProjection<T>),
    #[cfg(test)]
    Testing(Box<[T]>),
}

impl<T: PlannedArenaElement + core::fmt::Debug> core::fmt::Debug for ResidentLane<T> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Managed(values) => values.fmt(formatter),
            #[cfg(test)]
            Self::Testing(values) => values.fmt(formatter),
        }
    }
}

impl<T: PlannedArenaElement> core::ops::Deref for ResidentLane<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Managed(values) => values,
            #[cfg(test)]
            Self::Testing(values) => values,
        }
    }
}

impl<T: PlannedArenaElement> core::ops::DerefMut for ResidentLane<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Managed(values) => values,
            #[cfg(test)]
            Self::Testing(values) => values,
        }
    }
}

#[derive(Debug)]
pub struct TypedResidentArena {
    bools: ResidentLane<u8>,
    indexes: ResidentLane<u64>,
    f64s: ResidentLane<f64>,
    strings: ResidentLane<String>,
    snapshots: ResidentLane<Option<Value>>,
    // Kept after the lanes: payload bytes are destroyed before their capacity
    // reservations. Fixed numeric arenas need neither this owner nor metadata.
    payload_budget: Option<std::rc::Rc<super::budget::payload::ResidentPayloadOwner>>,
}

fn resident_lane<T: PlannedArenaElement>(
    plan: &ProgramMemoryPlan,
    class: ResidentStorageClass,
    kind: ResidentValueKind,
    buffer: u8,
    len: usize,
    memory: &ManagedProgramMemory,
) -> Result<ResidentLane<T>, ResidentActivationError> {
    let class = match class {
        ResidentStorageClass::Constant => PlannedValueClass::Constant,
        ResidentStorageClass::Input => PlannedValueClass::Input,
        ResidentStorageClass::State => PlannedValueClass::State,
        ResidentStorageClass::Scratch => PlannedValueClass::Scratch,
    };
    let arena_id = resident_arena_id((class, kind, buffer))
        .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
    let Some(arena) = plan.arenas.iter().find(|arena| arena.id == arena_id) else {
        return PlannedArenaProjection::empty()
            .map(ResidentLane::Managed)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error });
    };
    if arena.capacity_bytes == 0 {
        return PlannedArenaProjection::empty()
            .map(ResidentLane::Managed)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error });
    }
    memory
        .domain()
        .project_host_arena(memory.realized(), arena.id, len)
        .map(ResidentLane::Managed)
        .map_err(|error| ResidentActivationError::MemoryRuntime { error })
}

impl TypedResidentArena {
    fn allocate_from_plan(
        plan: &ProgramMemoryPlan,
        class: ResidentStorageClass,
        memory: &ManagedProgramMemory,
    ) -> Result<Self, ResidentActivationError> {
        Self::allocate_from_plan_buffer(plan, class, 0, memory)
    }

    fn allocate_from_plan_buffer(
        plan: &ProgramMemoryPlan,
        class: ResidentStorageClass,
        buffer: u8,
        memory: &ManagedProgramMemory,
    ) -> Result<Self, ResidentActivationError> {
        ensure_resident_plan_admitted(plan)?;
        let sizes = ResidentArenaSizes::from_memory_plan_buffer(plan, class, buffer)?;
        let payload_budget = if sizes.strings != 0 || sizes.snapshots != 0 {
            memory
                .domain()
                .memory_budget()
                .map(|budget| {
                    let planned_class = match class {
                        ResidentStorageClass::Constant => PlannedValueClass::Constant,
                        ResidentStorageClass::Input => PlannedValueClass::Input,
                        ResidentStorageClass::State => PlannedValueClass::State,
                        ResidentStorageClass::Scratch => PlannedValueClass::Scratch,
                    };
                    let string_arena =
                        resident_arena_id((planned_class, ResidentValueKind::String, buffer))
                            .map_err(|error| {
                                ResidentActivationError::ResidentMemoryPlanRejected { error }
                            })?;
                    let snapshot_arena =
                        resident_arena_id((planned_class, ResidentValueKind::Snapshot, buffer))
                            .map_err(|error| {
                                ResidentActivationError::ResidentMemoryPlanRejected { error }
                            })?;
                    let payload_arena =
                        crate::memory_planner::resident_payload_arena_id((planned_class, buffer))
                            .map_err(
                            |error| ResidentActivationError::ResidentMemoryPlanRejected { error },
                        )?;
                    let mut string_prepaid = budget
                        .reserve_capacity(0)
                        .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
                    let mut snapshot_prepaid = budget
                        .reserve_capacity(0)
                        .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
                    for allocation in plan
                        .allocations
                        .iter()
                        .filter(|allocation| allocation.placement.arena == payload_arena)
                    {
                        if allocation.capacity_bytes == 0 {
                            continue;
                        }
                        if plan.allocations.iter().any(|fixed| {
                            fixed.placement.arena == string_arena && fixed.owner == allocation.owner
                        }) {
                            let key = memory
                                .domain()
                                .plan_object_key(memory.realized().revision(), allocation.id)
                                .map_err(|error| ResidentActivationError::MemoryRuntime {
                                    error,
                                })?;
                            let transferred = memory
                                .domain()
                                .transfer_resident_payload_budget_capacity(memory.realized(), key)
                                .map_err(|error| ResidentActivationError::MemoryRuntime {
                                    error,
                                })?;
                            string_prepaid
                                .merge_capacity(transferred)
                                .map_err(|error| ResidentActivationError::MemoryRuntime {
                                    error,
                                })?;
                        }
                        if plan.allocations.iter().any(|fixed| {
                            fixed.placement.arena == snapshot_arena
                                && fixed.owner == allocation.owner
                        }) {
                            let key = memory
                                .domain()
                                .plan_object_key(memory.realized().revision(), allocation.id)
                                .map_err(|error| ResidentActivationError::MemoryRuntime {
                                    error,
                                })?;
                            let transferred = memory
                                .domain()
                                .transfer_resident_payload_budget_capacity(memory.realized(), key)
                                .map_err(|error| ResidentActivationError::MemoryRuntime {
                                    error,
                                })?;
                            snapshot_prepaid
                                .merge_capacity(transferred)
                                .map_err(|error| ResidentActivationError::MemoryRuntime {
                                    error,
                                })?;
                        }
                    }
                    super::budget::payload::ResidentPayloadOwner::new(
                        budget,
                        string_prepaid,
                        snapshot_prepaid,
                        sizes.strings,
                        sizes.snapshots,
                    )
                    .map_err(|error| ResidentActivationError::MemoryRuntime { error })
                })
                .transpose()?
        } else {
            None
        };
        let mut indexes = resident_lane(
            plan,
            class,
            ResidentValueKind::Index,
            buffer,
            sizes.indexes,
            memory,
        )?;
        // Index is one-based, so zero is not a valid initialized value. Keep
        // every freshly realized Index lane semantically initialized to the
        // same minimum value used by the pre-cutover resident arena.
        indexes.fill(1);
        Ok(Self {
            bools: resident_lane(
                plan,
                class,
                ResidentValueKind::Bool,
                buffer,
                sizes.bools,
                memory,
            )?,
            indexes,
            f64s: resident_lane(
                plan,
                class,
                ResidentValueKind::F64,
                buffer,
                sizes.f64s,
                memory,
            )?,
            strings: resident_lane(
                plan,
                class,
                ResidentValueKind::String,
                buffer,
                sizes.strings,
                memory,
            )?,
            snapshots: resident_lane(
                plan,
                class,
                ResidentValueKind::Snapshot,
                buffer,
                sizes.snapshots,
                memory,
            )?,
            payload_budget,
        })
    }

    #[cfg(test)]
    fn allocate_projected_sizes(sizes: ResidentArenaSizes) -> Self {
        fn lane<T: PlannedArenaElement>(len: usize) -> ResidentLane<T> {
            ResidentLane::Testing(
                core::iter::repeat_with(T::default)
                    .take(len)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        }
        Self {
            bools: lane(sizes.bools),
            indexes: lane(sizes.indexes),
            f64s: lane(sizes.f64s),
            strings: lane(sizes.strings),
            snapshots: lane(sizes.snapshots),
            payload_budget: None,
        }
    }

    fn lane_bytes(&self, kind: ResidentValueKind) -> Result<u64, ResidentActivationError> {
        let bytes = match kind {
            ResidentValueKind::Bool => Some(self.bools.len()),
            ResidentValueKind::Index => self.indexes.len().checked_mul(core::mem::size_of::<u64>()),
            ResidentValueKind::F64 => self.f64s.len().checked_mul(core::mem::size_of::<f64>()),
            ResidentValueKind::String => self
                .strings
                .len()
                .checked_mul(core::mem::size_of::<String>()),
            ResidentValueKind::Snapshot => self
                .snapshots
                .len()
                .checked_mul(core::mem::size_of::<Option<Value>>()),
        }
        .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        u64::try_from(bytes).map_err(|_| ResidentActivationError::RegionSizeOverflow)
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

    pub(crate) fn prepare_payload_write(
        &self,
        region: ResidentRegion,
    ) -> mech_core::MemoryRuntimeResult<Option<super::budget::payload::ResidentPayloadScope>> {
        if !super::budget::payload::is_payload(region.kind) {
            return Ok(None);
        }
        self.payload_budget
            .as_ref()
            .map(|owner| owner.begin(region))
            .transpose()
    }

    pub(crate) fn finish_payload_write(
        &mut self,
        region: ResidentRegion,
        scope: Option<super::budget::payload::ResidentPayloadScope>,
    ) -> mech_core::MemoryRuntimeResult<()> {
        if let Some(scope) = scope {
            scope.finish(self.write(region))?;
        }
        Ok(())
    }

    pub(crate) fn abort_payload_write(
        &mut self,
        region: ResidentRegion,
        scope: Option<super::budget::payload::ResidentPayloadScope>,
    ) -> mech_core::MemoryRuntimeResult<()> {
        if let Some(scope) = scope {
            scope.abort(self.write(region))?;
        }
        Ok(())
    }

    pub(crate) fn discard_payload_write(&mut self, region: ResidentRegion) {
        if !super::budget::payload::is_payload(region.kind) {
            return;
        }
        if let Some(owner) = self.payload_budget.clone() {
            owner
                .discard(region, self.write(region))
                .expect("resident payload ownership must balance during candidate abort");
        } else {
            match self.write(region) {
                ResidentValueMut::String(values) => values.fill_with(String::new),
                ResidentValueMut::Snapshot(values) => values.fill(None),
                _ => unreachable!("payload kind checked above"),
            }
        }
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
    ) -> mech_core::MemoryRuntimeResult<()> {
        debug_assert_eq!(target.kind, source_region.kind);
        debug_assert_eq!(target.len, source_region.len);
        let scope = self.prepare_payload_write(target)?;
        if let Some(scope) = &scope {
            scope.admit_copy(source.read(source_region), 0)?;
        }
        if let Some(scope) = &scope {
            scope.start();
        }
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
                for (target, source) in target.iter_mut().zip(source) {
                    *target = source.clone();
                }
            }
            (ResidentValueMut::Snapshot(target), ResidentValueRef::Snapshot(source)) => {
                target.clone_from_slice(source)
            }
            _ => unreachable!("resident region kinds were checked"),
        }
        self.finish_payload_write(target, scope)
    }

    fn copy_region_within(
        &mut self,
        target: ResidentRegion,
        source: ResidentRegion,
    ) -> mech_core::MemoryRuntimeResult<()> {
        debug_assert_eq!(target.kind, source.kind);
        debug_assert_eq!(target.len, source.len);
        let scope = self.prepare_payload_write(target)?;
        if let Some(scope) = &scope {
            let header = match source.kind {
                ResidentValueKind::String => core::mem::size_of::<String>(),
                ResidentValueKind::Snapshot => core::mem::size_of::<Option<Value>>(),
                _ => 0,
            };
            let bytes = (source.len as u64).checked_mul(header as u64).ok_or(
                mech_core::MemoryRuntimeError::IdentityExhausted {
                    identity: "resident copy staging bytes",
                },
            )?;
            scope.admit_copy(self.read(source), bytes)?;
            scope.start();
        }
        let source = source.offset..source.offset + source.len;
        match target.kind {
            ResidentValueKind::Bool => self.bools.copy_within(source, target.offset),
            ResidentValueKind::Index => self.indexes.copy_within(source, target.offset),
            ResidentValueKind::F64 => self.f64s.copy_within(source, target.offset),
            ResidentValueKind::String => {
                let values = self.strings[source].to_vec();
                for (target, value) in self.strings[target.offset..target.offset + target.len]
                    .iter_mut()
                    .zip(values)
                {
                    *target = value;
                }
            }
            ResidentValueKind::Snapshot => {
                let values = self.snapshots[source].to_vec();
                for (target, value) in self.snapshots[target.offset..target.offset + target.len]
                    .iter_mut()
                    .zip(values)
                {
                    *target = value;
                }
            }
        }
        self.finish_payload_write(target, scope)
    }
}

#[derive(Clone, Debug)]
struct StateVersion {
    slot: CellSlotId,
    region: ResidentRegion,
    epochs: [Option<InstanceEpoch>; 2],
}

#[derive(Debug)]
pub struct StateArena {
    buffers: [TypedResidentArena; 2],
    versions: Box<[StateVersion]>,
    version_by_slot: Box<[Option<usize>]>,
}

impl StateArena {
    fn new(
        plan: &ProgramMemoryPlan,
        slots: &[ResolvedSlot],
        memory: &ManagedProgramMemory,
    ) -> Result<Self, ResidentActivationError> {
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
        Ok(Self {
            buffers: [
                TypedResidentArena::allocate_from_plan_buffer(
                    plan,
                    ResidentStorageClass::State,
                    0,
                    memory,
                )?,
                TypedResidentArena::allocate_from_plan_buffer(
                    plan,
                    ResidentStorageClass::State,
                    1,
                    memory,
                )?,
            ],
            versions,
            version_by_slot: version_by_slot.into_boxed_slice(),
        })
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
    ) -> Result<(), ResidentActivationError> {
        let target = self.version(slot).region;
        self.buffers[0]
            .copy_region_from(target, source, source_region)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })
    }

    fn install_migrated(
        &mut self,
        slot: CellSlotId,
        epoch: InstanceEpoch,
        source: &StateArena,
        source_slot: CellSlotId,
        source_epoch: InstanceEpoch,
    ) -> Result<(), ResidentActivationError> {
        let target = self.version(slot).region;
        let source_region = source.version(source_slot).region;
        let source_buffer = source.published_buffer(source_slot, source_epoch);
        self.buffers[0]
            .copy_region_from(target, &source.buffers[source_buffer], source_region)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
        self.version_mut(slot).epochs = [Some(epoch), None];
        Ok(())
    }

    fn abort_payloads(&mut self, working: InstanceEpoch) {
        for version_index in 0..self.versions.len() {
            let region = self.versions[version_index].region;
            for buffer in 0..2 {
                if self.versions[version_index].epochs[buffer] == Some(working) {
                    self.buffers[buffer].discard_payload_write(region);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TurnWorkspace {
    pub(crate) input: TypedResidentArena,
    pub(crate) scratch: TypedResidentArena,
    pub(crate) dirty_bits: Box<[u64]>,
    pub(crate) suppressed_activation_bits: Box<[u64]>,
    pub(crate) executed_bits: Box<[u64]>,
    pub(crate) initialized_output_bits: Box<[u64]>,
    pub(crate) all_outputs_initialized: bool,
    pub(crate) touched_slots: Vec<SlotIndex>,
    pub(crate) changed_slots: Vec<SlotIndex>,
    pub(crate) effect_intents: Vec<ResidentEffectIntent>,
    pub(crate) effect_payloads: TypedResidentArena,
    pub(crate) rmw_previous: TypedResidentArena,
    state_f64_arena_by_slot: Box<[u8]>,
    // Only activation-invariant fixed-width plans are cached. Payload-bearing
    // values and deferred regions still supply live facts on every execution.
    fixed_turn_plans: Box<[Option<std::sync::Arc<crate::memory_planner::TurnMemoryPlan>>]>,
    recursive_scrutinees: Vec<RecursiveFrame>,
    continuation_candidates: Vec<Option<ResidentContinuation>>,
    completed_continuations: Box<[u64]>,
    continuation_publications: Box<[u64]>,
    candidate_output_ready: Box<[bool]>,
    continuation_capture_frames: Vec<Box<[(ResidentReadLocation, OwnedResidentValue)]>>,
    active_resume_state: Option<OwnedResidentValue>,
}

#[derive(Clone, Copy, Debug)]
struct RecursiveFrame {
    target: ActivatedNodeIndex,
    argument: ResidentReadLocation,
}

impl TurnWorkspace {
    fn new(
        plan: &ActivatedPlan,
        memory: &ManagedProgramMemory,
    ) -> Result<Self, ResidentActivationError> {
        let words = plan.topology.word_len();
        Ok(Self {
            input: TypedResidentArena::allocate_from_plan(
                &plan.memory_plan,
                ResidentStorageClass::Input,
                memory,
            )?,
            scratch: TypedResidentArena::allocate_from_plan(
                &plan.memory_plan,
                ResidentStorageClass::Scratch,
                memory,
            )?,
            dirty_bits: vec![0; words].into_boxed_slice(),
            suppressed_activation_bits: vec![0; words].into_boxed_slice(),
            executed_bits: vec![0; words].into_boxed_slice(),
            initialized_output_bits: vec![0; plan.steps.len().div_ceil(64)].into_boxed_slice(),
            all_outputs_initialized: false,
            touched_slots: Vec::with_capacity(plan.state_slots.len()),
            changed_slots: Vec::with_capacity(plan.state_slots.len()),
            effect_intents: Vec::with_capacity(
                plan.steps
                    .iter()
                    .filter(|step| matches!(step, ActivatedTurnStep::External(_)))
                    .count(),
            ),
            effect_payloads: TypedResidentArena::allocate_from_plan_buffer(
                &plan.memory_plan,
                ResidentStorageClass::Scratch,
                1,
                memory,
            )?,
            rmw_previous: TypedResidentArena::allocate_from_plan_buffer(
                &plan.memory_plan,
                ResidentStorageClass::Scratch,
                2,
                memory,
            )?,
            state_f64_arena_by_slot: vec![0; plan.slots.len()].into_boxed_slice(),
            fixed_turn_plans: vec![None; plan.steps.len()].into_boxed_slice(),
            recursive_scrutinees: Vec::new(),
            continuation_candidates: vec![None; plan.steps.len()],
            completed_continuations: vec![0; plan.steps.len().div_ceil(64)].into_boxed_slice(),
            continuation_publications: vec![0; plan.steps.len().div_ceil(64)].into_boxed_slice(),
            candidate_output_ready: vec![false; plan.outputs.len()].into_boxed_slice(),
            continuation_capture_frames: Vec::new(),
            active_resume_state: None,
        })
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
    transient_budget: Option<std::rc::Rc<super::budget::payload::ResidentPayloadOwner>>,
    published_epoch: AtomicU64,
    next_epoch: Option<InstanceEpoch>,
    candidate_active: bool,
    candidate_epoch: Option<InstanceEpoch>,
    continuations: Vec<Option<ResidentContinuation>>,
    ready_continuations: std::collections::VecDeque<ActivatedNodeIndex>,
    published_continuations: Box<[u64]>,
    completed_continuation_roots: Box<[u64]>,
    output_ready: Box<[bool]>,
    // Declared last so every typed lane projection is destroyed before the
    // realization releases its arena owners.
    _managed_memory: ManagedProgramMemory,
}

/// Scheduler token for one coalesced, generation-bound FSM continuation.
/// Tokens become stale when an instance is reset or reactivated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentContinuationWakeup {
    instance: ReactiveInstanceId,
    plan_generation: PlanGeneration,
    layout_generation: LayoutGeneration,
    continuation: ActivatedNodeIndex,
}

impl ResidentContinuationWakeup {
    pub const fn instance(self) -> ReactiveInstanceId {
        self.instance
    }

    pub const fn plan_generation(self) -> PlanGeneration {
        self.plan_generation
    }

    pub const fn layout_generation(self) -> LayoutGeneration {
        self.layout_generation
    }
}

/// Collects a bounded, fair batch of generation-bound continuation wakeups.
///
/// A collection pass visits each instance at most once, so one async chain
/// cannot resume twice in one drain. The cursor advances after every selected
/// instance and therefore interleaves independently runnable instances across
/// bounded drains. Execution remains with the runtime coordinator, which can
/// install that turn's current external captures before accepting the token.
#[derive(Debug, Default)]
pub struct ResidentContinuationScheduler {
    next_instance: usize,
}

impl ResidentContinuationScheduler {
    pub const fn new() -> Self {
        Self { next_instance: 0 }
    }

    pub fn collect_ready(
        &mut self,
        instances: &[&ReactiveInstance],
        max_work: usize,
    ) -> Box<[ResidentContinuationWakeup]> {
        if instances.is_empty() || max_work == 0 {
            return Box::new([]);
        }
        let start = self.next_instance % instances.len();
        let mut selected = Vec::with_capacity(max_work.min(instances.len()));
        let mut last_visited = start;
        for offset in 0..instances.len() {
            let index = (start + offset) % instances.len();
            last_visited = index;
            if let Some(wakeup) = instances[index].continuation_wakeup() {
                selected.push(wakeup);
                self.next_instance = (index + 1) % instances.len();
                if selected.len() == max_work {
                    break;
                }
            }
        }
        if selected.is_empty() {
            self.next_instance = (last_visited + 1) % instances.len();
        }
        selected.into_boxed_slice()
    }
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
    pub(crate) fn memory_budget(&self) -> Option<mech_core::ManagedMemoryBudget> {
        self._managed_memory.domain().memory_budget()
    }
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
        if !self.output_ready.get(output).copied().unwrap_or(false) {
            return None;
        }
        self.output_borrow_at(output, self.published_epoch())
    }

    /// True when this instance owns at least one published FSM continuation
    /// that can resume on a later turn.
    pub fn has_ready_continuation(&self) -> bool {
        !self.ready_continuations.is_empty()
    }

    pub fn ready_continuation_count(&self) -> usize {
        self.ready_continuations.len()
    }

    pub fn continuation_wakeup(&self) -> Option<ResidentContinuationWakeup> {
        self.ready_continuations
            .front()
            .copied()
            .map(|continuation| ResidentContinuationWakeup {
                instance: self.id,
                plan_generation: self.plan.plan_generation,
                layout_generation: self.plan.layout_generation,
                continuation,
            })
    }

    pub fn accepts_continuation_wakeup(&self, wakeup: ResidentContinuationWakeup) -> bool {
        wakeup.instance == self.id
            && wakeup.plan_generation == self.plan.plan_generation
            && wakeup.layout_generation == self.plan.layout_generation
            && self.ready_continuations.front() == Some(&wakeup.continuation)
    }

    pub(crate) fn output_borrow_at(
        &self,
        output: usize,
        epoch: InstanceEpoch,
    ) -> Option<ResidentValueBorrow<'_>> {
        let output = self.plan.outputs.get(output)?;
        let slot = self.plan.slots.get(output.slot.get() as usize)?;
        let value = self.state.read_published(slot.artifact_id, epoch);
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
    /// published snapshot coherent. Every mapping is validated before any
    /// retained lane is changed, so a recoverable validation failure cannot
    /// partially mutate the candidate or allocate an unplanned arena clone.
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
        }

        for mapping in state_map {
            let target = self.state.version(mapping.target).region;
            let candidate = 1 - self.state.published_buffer(mapping.target, target_epoch);
            let source_region = source.state.version(mapping.source).region;
            let source_buffer = source.state.published_buffer(mapping.source, source_epoch);
            self.state.buffers[candidate]
                .copy_region_from(target, &source.state.buffers[source_buffer], source_region)
                .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
        }
        // Every payload copy and ownership transfer has succeeded. Publish
        // only the prepared existing state regions; no fallible work follows.
        for mapping in state_map {
            let candidate = 1 - self.state.published_buffer(mapping.target, target_epoch);
            let mut epochs = [None, None];
            epochs[candidate] = Some(target_epoch);
            self.state.version_mut(mapping.target).epochs = epochs;
            let target_slot = self.plan.slots[mapping.target.get() as usize].physical_index;
            let source_slot = source.plan.slots[mapping.source.get() as usize].physical_index;
            if let Some(source_output) = source
                .plan
                .outputs
                .iter()
                .position(|output| output.slot == source_slot)
            {
                for (target_output, output) in self.plan.outputs.iter().enumerate() {
                    if output.slot == target_slot {
                        self.output_ready[target_output] = source.output_ready[source_output];
                    }
                }
            }
        }
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
                memory_budget: self._managed_memory.domain().memory_budget(),
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
                )?;
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
    UnsupportedControlLayout {
        node: NodeId,
    },
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
        slot: Option<CellSlotId>,
    },
    UnresolvedShape {
        slot: CellSlotId,
    },
    RegionSizeOverflow,
    ResidentMemoryPlanRejected {
        error: MemoryPlanError,
    },
    MemoryRuntime {
        error: mech_core::MemoryRuntimeError,
    },
    InvalidSnapshotRepresentation,
    MissingStateInitializer {
        slot: CellSlotId,
    },
    /// The artifact declares an initializer that this target cannot evaluate
    /// without live inputs or state. No state has been published.
    InitializerUnavailableAtActivation {
        slot: CellSlotId,
        source: CellSlotId,
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
    StaticSelectorResolutionLimit {
        slot: CellSlotId,
    },
    UnknownOutput {
        output: usize,
    },
    OutputUnavailable {
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

fn ensure_resident_plan_admitted(plan: &ProgramMemoryPlan) -> Result<(), ResidentActivationError> {
    if let Some(violation) = plan.budget_violations.first() {
        return Err(ResidentActivationError::ResidentMemoryPlanRejected {
            error: MemoryPlanError::TargetLimitExceeded {
                violation: violation.clone(),
            },
        });
    }
    Ok(())
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
    /// Static selector identities retained by the resident/native plan for
    /// each artifact input. Dynamic slot inputs remain `None`.
    pub input_resolved_selectors: Box<[Option<mech_core::ResidentResolvedSelector>]>,
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
    let classification = classify_nodes(artifact, options.external)?;
    preflight_state_initializers(artifact, &classification)?;
    let schedule = build_activation_schedule(artifact, &classification)?;
    let facts_fingerprint = activation_facts_fingerprint(facts);
    let facts = complete_activation_shape_facts(artifact, facts, &classification, &schedule)?;
    let layout = build_layout(artifact, &facts, &classification, &schedule.positions)?;
    let mut static_selectors = ArtifactStaticSelectorResolver::new(artifact);
    let plan = build_plan(
        artifact,
        catalog,
        classification,
        schedule,
        layout,
        facts_fingerprint,
        options,
        &mut static_selectors,
    )?;
    Ok(ResidentActivationPreflight {
        plan,
        concrete_cases: resident_concrete_execution_cases(artifact, &mut static_selectors)?,
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
        let node = resident_activation_error_node(artifact, &error);
        let operation = node.and_then(|node| {
            artifact.nodes().get(node.get() as usize).and_then(|node| {
                node.as_operation()
                    .map(|operation| operation.operation.clone())
            })
        });
        OperationUnavailableForTarget {
            node,
            operation,
            target: ExecutionTarget::ResidentCpu,
            reason: format!("{error:?}"),
        }
    })
}

fn resident_activation_error_node(
    artifact: &ProgramArtifact,
    error: &ResidentActivationError,
) -> Option<NodeId> {
    match error {
        ResidentActivationError::UnsupportedControlLayout { node }
        | ResidentActivationError::LegacyOpaque { node }
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
        ResidentActivationError::TurnDimension {
            slot: Some(slot), ..
        } => match artifact.slots().get(slot.get() as usize)?.producer {
            ProducerReference::NodeOutput { node, .. } => Some(node),
            _ => None,
        },
        _ => None,
    }
}

fn resident_concrete_execution_cases(
    artifact: &ProgramArtifact,
    static_selectors: &mut ArtifactStaticSelectorResolver,
) -> Result<Box<[ConcreteExecutionCase]>, ResidentActivationError> {
    let mut cases = artifact
        .nodes()
        .iter()
        .filter_map(|node| node.as_operation())
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
            let mut input_schemas = Vec::new();
            let mut input_resolved_selectors = Vec::new();
            for source in node_inputs(artifact, node.node)? {
                match source {
                    ArtifactSource::Constant(constant) => {
                        let value = artifact.constants().get(constant).ok_or(
                            ResidentActivationError::InvalidDependency { node: node.node },
                        )?;
                        input_schemas.push(value.schema());
                        input_resolved_selectors.push(
                            static_selectors
                                .resolve(artifact, ArtifactSource::Constant(constant))?,
                        );
                    }
                    ArtifactSource::Slot(slot) => {
                        input_schemas.push(
                            artifact
                                .slots()
                                .get(slot.get() as usize)
                                .map(|slot| slot.schema)
                                .ok_or(ResidentActivationError::InvalidDependency {
                                    node: node.node,
                                })?,
                        );
                        input_resolved_selectors
                            .push(static_selectors.resolve(artifact, ArtifactSource::Slot(slot))?);
                    }
                }
            }
            let output = node_output_slot(artifact, node.node)?;
            let output_schema = artifact
                .slots()
                .get(output.get() as usize)
                .map(|slot| slot.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?;
            Ok(ConcreteExecutionCase {
                node: node.node,
                operation: node.operation.clone(),
                input_schemas: input_schemas.into_boxed_slice(),
                input_resolved_selectors: input_resolved_selectors.into_boxed_slice(),
                output_schema,
                targets: ExecutionTargetSet::RESIDENT_CPU,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    for node in artifact.nodes() {
        if let crate::ExecutableNodeBody::Comprehension(control) = &node.body {
            let sources = node_inputs(artifact, node.node)?
                .into_iter()
                .map(Some)
                .collect::<Vec<_>>();
            append_comprehension_execution_cases(
                artifact,
                node.node,
                control,
                &sources,
                static_selectors,
                &mut cases,
            )?;
            continue;
        }
        let control = match &node.body {
            crate::ExecutableNodeBody::Match(control)
            | crate::ExecutableNodeBody::Activation(control) => control,
            _ => continue,
        };
        let sources = node_inputs(artifact, node.node)?
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>();
        append_match_execution_cases(
            artifact,
            node.node,
            control,
            &sources,
            static_selectors,
            &mut cases,
        )?;
    }
    Ok(cases.into_boxed_slice())
}

fn append_comprehension_execution_cases(
    artifact: &ProgramArtifact,
    owner: NodeId,
    control: &crate::ComprehensionDeclaration,
    sources: &[Option<ArtifactSource>],
    static_selectors: &mut ArtifactStaticSelectorResolver,
    cases: &mut Vec<ConcreteExecutionCase>,
) -> Result<(), ResidentActivationError> {
    for operation in control.steps.iter().filter_map(|step| match step {
        crate::ComprehensionStep::Operation(operation) => Some(operation),
        _ => None,
    }) {
        let inputs = operation
            .inputs
            .iter()
            .map(|value| match *value {
                crate::ComprehensionValue::Constant(id) => Some(ArtifactSource::Constant(id)),
                crate::ComprehensionValue::Input(ordinal) => sources[ordinal as usize],
                crate::ComprehensionValue::Local(_) => None,
            })
            .collect::<Vec<_>>();
        match &operation.body {
            crate::ControlOperationBody::Operation {
                operation: reference,
                contract: contract_id,
            } => {
                let Some(mech_core::ResolvedOperationContract::Declared(contract)) =
                    artifact.contracts().get(*contract_id)
                else {
                    unreachable!("validated lexical operation")
                };
                let selectors = inputs
                    .iter()
                    .map(|source| {
                        source
                            .map(|source| static_selectors.resolve(artifact, source))
                            .transpose()
                            .map(Option::flatten)
                    })
                    .collect::<Result<Box<[_]>, _>>()?;
                cases.push(ConcreteExecutionCase {
                    node: owner,
                    operation: reference.clone(),
                    input_schemas: contract.inputs.iter().map(|port| port.schema).collect(),
                    input_resolved_selectors: selectors,
                    output_schema: operation.schema,
                    targets: ExecutionTargetSet::RESIDENT_CPU,
                });
            }
            crate::ControlOperationBody::Match(nested) => append_match_execution_cases(
                artifact,
                owner,
                nested,
                &inputs,
                static_selectors,
                cases,
            )?,
            crate::ControlOperationBody::Comprehension(nested) => {
                append_comprehension_execution_cases(
                    artifact,
                    owner,
                    nested,
                    &inputs,
                    static_selectors,
                    cases,
                )?;
            }
            crate::ControlOperationBody::Recur(_)
            | crate::ControlOperationBody::Suspend
            | crate::ControlOperationBody::Publish => {}
        }
    }
    Ok(())
}

fn append_match_execution_cases(
    artifact: &ProgramArtifact,
    owner: NodeId,
    control: &crate::MatchDeclaration,
    sources: &[Option<ArtifactSource>],
    static_selectors: &mut ArtifactStaticSelectorResolver,
    cases: &mut Vec<ConcreteExecutionCase>,
) -> Result<(), ResidentActivationError> {
    for arm in &control.arms {
        for block in arm.guard.iter().chain(core::iter::once(&arm.body)) {
            for operation in &block.operations {
                let inputs = operation
                    .inputs
                    .iter()
                    .map(|value| match *value {
                        crate::ControlValue::Constant(id) => Some(ArtifactSource::Constant(id)),
                        crate::ControlValue::Parameter { ordinal, .. } => {
                            let input = match block.parameters[ordinal as usize].source {
                                crate::ControlParameterSource::Scrutinee => control.scrutinee,
                                crate::ControlParameterSource::PatternBinding(local) => {
                                    match &arm.pattern {
                                        crate::MatchPattern::Structural(
                                            crate::CollectionPattern::Bind { local: bound, .. },
                                        ) if *bound == local => control.scrutinee,
                                        _ => return None,
                                    }
                                }
                                crate::ControlParameterSource::Capture(index) => {
                                    control.captures[index as usize].input
                                }
                            };
                            sources[input as usize]
                        }
                        crate::ControlValue::Local { .. } => None,
                    })
                    .collect::<Vec<_>>();
                match &operation.body {
                    crate::ControlOperationBody::Match(nested) => append_match_execution_cases(
                        artifact,
                        owner,
                        nested,
                        &inputs,
                        static_selectors,
                        cases,
                    )?,
                    crate::ControlOperationBody::Comprehension(nested) => {
                        append_comprehension_execution_cases(
                            artifact,
                            owner,
                            nested,
                            &inputs,
                            static_selectors,
                            cases,
                        )?;
                    }
                    crate::ControlOperationBody::Recur(_)
                    | crate::ControlOperationBody::Suspend
                    | crate::ControlOperationBody::Publish => {}
                    crate::ControlOperationBody::Operation {
                        operation: reference,
                        contract,
                    } => {
                        let Some(mech_core::ResolvedOperationContract::Declared(contract)) =
                            artifact.contracts().get(*contract)
                        else {
                            return Err(ResidentActivationError::LegacyOpaque { node: owner });
                        };
                        let selectors = inputs
                            .into_iter()
                            .map(|source| {
                                source
                                    .map(|source| static_selectors.resolve(artifact, source))
                                    .transpose()
                                    .map(Option::flatten)
                            })
                            .collect::<Result<Box<[_]>, _>>()?;
                        cases.push(ConcreteExecutionCase {
                            node: owner,
                            operation: reference.clone(),
                            input_schemas: contract.inputs.iter().map(|port| port.schema).collect(),
                            input_resolved_selectors: selectors,
                            output_schema: operation.schema,
                            targets: ExecutionTargetSet::RESIDENT_CPU,
                        });
                    }
                }
            }
        }
    }
    Ok(())
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
            memory_budget: None,
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
    let memory_budget = options.memory_budget.clone();
    let classification = classify_nodes(artifact, options.external)?;
    preflight_state_initializers(artifact, &classification)?;
    let schedule = build_activation_schedule(artifact, &classification)?;
    // Reactivation identity belongs to the caller-supplied activation facts.
    // Shapes completed deterministically from the artifact are derived plan
    // state and must not make an unchanged request look like a new one.
    let facts_fingerprint = activation_facts_fingerprint(facts);
    let facts = complete_activation_shape_facts(artifact, facts, &classification, &schedule)?;
    let layout = build_layout(artifact, &facts, &classification, &schedule.positions)?;
    let mut static_selectors = ArtifactStaticSelectorResolver::new(artifact);
    let plan = build_plan(
        artifact,
        catalog,
        classification,
        schedule,
        layout,
        facts_fingerprint,
        options,
        &mut static_selectors,
    )?;
    let managed_memory =
        ManagedProgramMemory::realize_with_memory_budget(&plan.memory_plan, memory_budget.as_ref())
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    let transient_budget = memory_budget
        .clone()
        .map(|budget| {
            let empty = budget.reserve_capacity(0)?;
            let strings = budget.reserve_capacity(0)?;
            super::budget::payload::ResidentPayloadOwner::new(budget, strings, empty, 0, 0)
        })
        .transpose()
        .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    let mut activation = TypedResidentArena::allocate_from_plan(
        &plan.memory_plan,
        ResidentStorageClass::Constant,
        &managed_memory,
    )?;
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
    let state = StateArena::new(&plan.memory_plan, &plan.slots, &managed_memory)?;
    let workspace = TurnWorkspace::new(&plan, &managed_memory)?;
    let continuations = vec![None; plan.steps.len()];
    let output_ready = plan
        .outputs
        .iter()
        .map(|output| {
            plan.output_materializations
                .iter()
                .filter(|materialization| {
                    plan.slots[materialization.target.get() as usize].physical_index == output.slot
                })
                .all(|materialization| {
                    !plan.steps.iter().enumerate().any(|(index, step)| {
                        matches!(
                            step,
                            ActivatedTurnStep::Match(control)
                                if control.continuation
                                    && output_materialization_depends_on_match(
                                        &plan,
                                        *materialization,
                                        index,
                                        control.write.region,
                                    )
                        )
                    })
                })
        })
        .collect();
    let continuation_words = plan.steps.len().div_ceil(64);
    let mut instance = ReactiveInstance {
        id,
        plan,
        activation,
        state,
        workspace,
        transient_budget,
        published_epoch: AtomicU64::new(InstanceEpoch::ZERO.get()),
        next_epoch: Some(InstanceEpoch::new(1)),
        candidate_active: false,
        candidate_epoch: None,
        continuations,
        ready_continuations: std::collections::VecDeque::new(),
        published_continuations: vec![0; continuation_words].into_boxed_slice(),
        completed_continuation_roots: vec![0; continuation_words].into_boxed_slice(),
        output_ready,
        _managed_memory: managed_memory,
    };
    for index in 0..instance.plan.activation_steps.len() {
        let step = &instance.plan.activation_steps[index];
        if let ActivatedOnceBody::Control(control) = &step.body {
            let control = *control;
            let node = step.artifact_node;
            instance
                .execute_activation_control(control)
                .map_err(|error| match error {
                    ResidentExecutionError::MemoryRuntime { error } => {
                        ResidentActivationError::MemoryRuntime { error }
                    }
                    _ => ResidentActivationError::ActivationKernel { node },
                })?;
        } else {
            execute_activation_kernel(
                step,
                &instance.plan,
                &mut instance.activation,
                instance.transient_budget.as_ref(),
            )?;
        }
    }
    for slot in instance
        .plan
        .slots
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
    {
        let declaration = &artifact.slots()[slot.artifact_id.get() as usize];
        match declaration.initializer {
            Some(InitializerReference::Constant(constant)) => {
                let value = artifact.constants().get(constant).ok_or(
                    ResidentActivationError::MissingStateInitializer {
                        slot: slot.artifact_id,
                    },
                )?;
                instance.state.initialize(slot.artifact_id, value)?;
            }
            Some(InitializerReference::Activation(source)) => {
                let source = &instance.plan.slots[source.get() as usize];
                if source.storage != ResidentStorageClass::Constant {
                    return Err(
                        ResidentActivationError::InitializerUnavailableAtActivation {
                            slot: slot.artifact_id,
                            source: source.artifact_id,
                        },
                    );
                }
                instance.state.initialize_from_arena(
                    slot.artifact_id,
                    &instance.activation,
                    source.region,
                )?;
            }
            None => {
                return Err(ResidentActivationError::MissingStateInitializer {
                    slot: slot.artifact_id,
                });
            }
        }
    }
    for materialization in instance.plan.output_materializations.iter().copied() {
        let declaration = &artifact.slots()[materialization.target.get() as usize];
        if let Some(InitializerReference::Constant(constant)) = declaration.initializer {
            let value = artifact
                .constants()
                .get(constant)
                .ok_or(ResidentActivationError::InvalidSnapshotRepresentation)?;
            instance.state.initialize(materialization.target, value)?;
        } else if let ResidentReadLocation::Constant(source) = materialization.source {
            instance.state.initialize_from_arena(
                materialization.target,
                &instance.activation,
                source,
            )?;
        }
    }
    finalize_resident_backing_footprints(
        artifact,
        &mut instance.plan,
        &instance.activation,
        &instance.state,
        &instance.workspace,
    )?;
    ensure_resident_plan_admitted(&instance.plan.memory_plan)?;
    audit_resident_backings(
        artifact,
        &instance.plan,
        &instance.activation,
        &instance.state,
        &instance.workspace,
    )?;
    instance.prepare_fixed_turn_plans()?;
    Ok(instance)
}

fn finalize_resident_backing_footprints(
    artifact: &ProgramArtifact,
    activated: &mut ActivatedPlan,
    activation: &TypedResidentArena,
    state: &StateArena,
    workspace: &TurnWorkspace,
) -> Result<(), ResidentActivationError> {
    let mut footprints = BTreeMap::new();
    for allocation in &activated.memory_plan.allocations {
        if footprints.contains_key(&allocation.owner) {
            continue;
        }
        let Some(value) =
            resident_observation_value(activated, allocation, activation, state, workspace)
        else {
            continue;
        };
        footprints.insert(
            allocation.owner.clone(),
            resident_value_observed_footprint(artifact, value)?,
        );
    }
    finalize_resident_current_footprints(&mut activated.memory_plan, &footprints)
        .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })
}

fn audit_resident_backings(
    artifact: &ProgramArtifact,
    activated: &ActivatedPlan,
    activation: &TypedResidentArena,
    state: &StateArena,
    workspace: &TurnWorkspace,
) -> Result<(), ResidentActivationError> {
    let plan = &activated.memory_plan;
    let arenas = [
        (PlannedValueClass::Constant, 0_u8, activation),
        (PlannedValueClass::Input, 0, &workspace.input),
        (PlannedValueClass::State, 0, &state.buffers[0]),
        (PlannedValueClass::State, 1, &state.buffers[1]),
        (PlannedValueClass::Scratch, 0, &workspace.scratch),
        (PlannedValueClass::Scratch, 1, &workspace.effect_payloads),
        (PlannedValueClass::Scratch, 2, &workspace.rmw_previous),
    ];
    for (class, buffer, backing) in arenas {
        for kind in [
            ResidentValueKind::Bool,
            ResidentValueKind::Index,
            ResidentValueKind::F64,
            ResidentValueKind::String,
            ResidentValueKind::Snapshot,
        ] {
            let arena = resident_arena_id((class, kind, buffer))
                .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
            let planned = plan
                .arenas
                .iter()
                .find(|candidate| candidate.id == arena)
                .map_or(0, |candidate| candidate.capacity_bytes);
            let observed = backing.lane_bytes(kind)?;
            if observed != planned {
                let object = plan
                    .arenas
                    .iter()
                    .find(|candidate| candidate.id == arena)
                    .and_then(|candidate| candidate.members.first())
                    .copied()
                    .unwrap_or_else(|| mech_core::MemoryObjectId::new(0));
                return Err(ResidentActivationError::ResidentMemoryPlanRejected {
                    error: MemoryPlanError::ObservationExceeded {
                        mismatch: mech_core::MemoryPlanAuditMismatch {
                            object,
                            field: "arena_capacity_bytes",
                            planned,
                            observed,
                        },
                    },
                });
            }
        }
    }
    let observations = plan
        .allocations
        .iter()
        .map(|allocation| {
            let value = plan
                .values
                .iter()
                .find(|value| value.object == allocation.id);
            let observed =
                resident_observation_value(activated, allocation, activation, state, workspace)
                    .map(|value| resident_value_observed_footprint(artifact, value))
                    .transpose()?;
            Ok(mech_core::MemoryPlanObservation {
                object: allocation.id,
                current_bytes: if allocation.role == mech_core::AllocationRole::VariablePayload {
                    observed.map_or(allocation.current_bytes, |footprint| {
                        footprint.payload_bytes
                    })
                } else {
                    allocation.current_bytes
                },
                capacity_bytes: allocation.capacity_bytes,
                payload_bytes: value
                    .and_then(|_| observed)
                    .map_or(0, |footprint| footprint.payload_bytes),
                retained_nodes: value
                    .and_then(|_| observed)
                    .map_or(0, |footprint| footprint.retained_nodes),
                logical_elements: observed.map_or_else(
                    || value.map_or(0, |value| value.layout.current_elements),
                    |footprint| footprint.logical_elements,
                ),
            })
        })
        .collect::<Result<Vec<_>, ResidentActivationError>>()?;
    crate::memory_planner::audit_program_memory_plan(plan, &observations)
        .and_then(|report| report.assert_conformant())
        .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })
}

fn resident_observation_value<'a>(
    activated: &'a ActivatedPlan,
    allocation: &mech_core::AllocationPlan,
    activation: &'a TypedResidentArena,
    state: &'a StateArena,
    workspace: &'a TurnWorkspace,
) -> Option<ResidentValueRef<'a>> {
    match allocation.owner {
        mech_core::MemoryObjectOwner::Constant(constant) => activated
            .constant_regions
            .get(constant.get() as usize)
            .copied()
            .map(|region| activation.read(region)),
        mech_core::MemoryObjectOwner::Slot(slot) => {
            let resolved = activated.slots.get(slot.get() as usize)?;
            Some(match resolved.storage {
                ResidentStorageClass::Constant => activation.read(resolved.region),
                ResidentStorageClass::Input => workspace.input.read(resolved.region),
                ResidentStorageClass::State => {
                    state.buffers[resident_state_buffer(allocation)].read(resolved.region)
                }
                ResidentStorageClass::Scratch => workspace.scratch.read(resolved.region),
            })
        }
        mech_core::MemoryObjectOwner::NodeScratch { .. } => {
            // Block locals are value-bearing scratch objects in the same R5
            // plan. Kernel-private scratch has no value slot and stays absent.
            let value = activated
                .memory_plan
                .values
                .iter()
                .find(|value| value.object == allocation.id)?;
            let slot = activated.slots.get(value.slot.get() as usize)?;
            (slot.storage == ResidentStorageClass::Scratch)
                .then(|| workspace.scratch.read(slot.region))
        }
        _ => None,
    }
}

fn resident_state_buffer(allocation: &mech_core::AllocationPlan) -> usize {
    for buffer in [0_u8, 1] {
        if resident_payload_arena_id((PlannedValueClass::State, buffer))
            .is_ok_and(|arena| arena == allocation.placement.arena)
        {
            return usize::from(buffer);
        }
        for kind in [
            ResidentValueKind::Bool,
            ResidentValueKind::Index,
            ResidentValueKind::F64,
            ResidentValueKind::String,
            ResidentValueKind::Snapshot,
        ] {
            if resident_arena_id((PlannedValueClass::State, kind, buffer))
                .is_ok_and(|arena| arena == allocation.placement.arena)
            {
                return usize::from(buffer);
            }
        }
    }
    0
}

fn resident_value_observed_footprint(
    artifact: &ProgramArtifact,
    value: ResidentValueRef<'_>,
) -> Result<CurrentMemoryFootprint, ResidentActivationError> {
    let logical_elements =
        u64::try_from(value.len()).map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
    let mut footprint = CurrentMemoryFootprint {
        logical_elements,
        ..CurrentMemoryFootprint::default()
    };
    match value {
        ResidentValueRef::String(values) => {
            footprint.payload_bytes = values.iter().try_fold(0_u64, |total, value| {
                total
                    .checked_add(
                        u64::try_from(value.len())
                            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
                    )
                    .ok_or(ResidentActivationError::RegionSizeOverflow)
            })?;
        }
        ResidentValueRef::Snapshot(values) => {
            for value in values.iter().flatten() {
                let retained = value
                    .retained_footprint(artifact.schemas())
                    .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)?;
                footprint.payload_bytes = footprint
                    .payload_bytes
                    .checked_add(retained.retained_bytes)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?;
                footprint.encoded_bytes = footprint
                    .encoded_bytes
                    .checked_add(retained.encoded_bytes)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?;
                footprint.retained_nodes = footprint
                    .retained_nodes
                    .checked_add(retained.node_count)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            }
        }
        ResidentValueRef::Bool(_) | ResidentValueRef::Index(_) | ResidentValueRef::F64(_) => {}
    }
    Ok(footprint)
}

fn preflight_state_initializers(
    artifact: &ProgramArtifact,
    classes: &[NodeClass],
) -> Result<(), ResidentActivationError> {
    for slot in artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
    {
        match slot.initializer {
            Some(InitializerReference::Constant(constant))
                if artifact.constants().get(constant).is_some() => {}
            Some(InitializerReference::Activation(source)) => {
                let declaration = &artifact.slots()[source.get() as usize];
                if declaration.role != SlotRole::Derived
                    || !matches!(declaration.producer,
                    ProducerReference::NodeOutput { node, .. } if classes[node.get() as usize] == NodeClass::Activation)
                {
                    return Err(
                        ResidentActivationError::InitializerUnavailableAtActivation {
                            slot: slot.slot,
                            source,
                        },
                    );
                }
            }
            _ => return Err(ResidentActivationError::MissingStateInitializer { slot: slot.slot }),
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
        if matches!(&node.body, crate::ExecutableNodeBody::Fsm(_)) {
            return Err(ResidentActivationError::UnsupportedControlLayout { node: node.node });
        }
        let Some(node) = node.as_operation() else {
            continue;
        };
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
            // Activation scopes are turn owners even when their trigger and
            // captures are closed. Loading may allocate their lexical control
            // but must never execute the scope body.
            if matches!(&node.body, crate::ExecutableNodeBody::Activation(_)) {
                continue;
            }
            if matches!(
                classes[node.node.get() as usize],
                NodeClass::Observation | NodeClass::External
            ) {
                continue;
            }
            if matches!(
                &node.body,
                crate::ExecutableNodeBody::Match(control) if control.contains_suspend()
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
        let Some(node) = node.as_operation() else {
            continue;
        };
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
                let construction_supported = match output.construction {
                    OutputConstruction::FullWrite { .. } | OutputConstruction::Build { .. } => {
                        output.access == AccessMode::Write && output.alias == AliasPolicy::NoAlias
                    }
                    OutputConstruction::ReadModifyWrite { base_input, .. } => {
                        output.access == AccessMode::ReadWrite
                            && output.alias == (AliasPolicy::MayAlias { input: base_input })
                            && node_inputs(artifact, node.node)?
                                .get(base_input as usize)
                                .is_some()
                    }
                    _ => false,
                };
                if output_role != SlotRole::Derived || !construction_supported {
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
                        let base = node_inputs(artifact, node.node)?
                            .get(base_input as usize)
                            .copied();
                        let valid_destination = match output_role {
                            SlotRole::State => matches!(base,
                                Some(ArtifactSource::Slot(base)) if output_slot == base),
                            SlotRole::Derived => {
                                base.is_some() && base != Some(ArtifactSource::Slot(output_slot))
                            }
                            _ => false,
                        };
                        if !valid_destination
                            || output.access != AccessMode::ReadWrite
                            || output.alias != (AliasPolicy::MayAlias { input: base_input })
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

fn operation_requires_activation_fixed_range_shape(operation: &OperationReference) -> bool {
    operation.module_path.as_ref() == ["range"]
        && matches!(
            operation.operation_name.as_str(),
            "exclusive" | "exclusive-increment" | "inclusive" | "inclusive-increment"
        )
}

fn source_schema_and_shape(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    facts: &ActivationFacts,
) -> Result<(mech_core::SchemaId, ShapeInstance), ResidentActivationError> {
    let (schema_id, shape) = match source {
        ArtifactSource::Constant(constant) => {
            let value = artifact
                .constants()
                .get(constant)
                .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            (value.schema(), value.shape().clone())
        }
        ArtifactSource::Slot(slot) => {
            let declaration = &artifact.slots()[slot.get() as usize];
            (declaration.schema, slot_shape(artifact, slot, facts)?)
        }
    };
    Ok((schema_id, shape))
}

fn source_extents(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    facts: &ActivationFacts,
) -> Result<Box<[u64]>, ResidentActivationError> {
    let (schema_id, shape) = source_schema_and_shape(artifact, source, facts)?;
    let schema = artifact
        .schemas()
        .entry(schema_id)
        .ok_or(ResidentActivationError::RegionSizeOverflow)?
        .schema();
    match schema.body() {
        SchemaBody::Matrix { dimensions, .. } => dimensions
            .iter()
            .map(|dimension| evaluate_dimension(dimension, shape.parameter_values()))
            .collect::<Result<Vec<_>, _>>()
            .map(Vec::into_boxed_slice),
        _ => Ok(Box::new([])),
    }
}

fn node_output_requires_runtime_control_shape(artifact: &ProgramArtifact, node: NodeId) -> bool {
    let Some(producer) = artifact.nodes().get(node.get() as usize) else {
        return true;
    };
    let Some(operation) = producer.as_operation() else {
        return true;
    };
    operation.operation.resolved_range_mode().is_some()
        && node_inputs(artifact, node).is_ok_and(|inputs| {
            inputs
                .iter()
                .any(|source| matches!(source, ArtifactSource::Slot(_)))
        })
}

fn source_has_activation_shape_fact(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    facts: &ActivationFacts,
) -> bool {
    let ArtifactSource::Slot(slot) = source else {
        return true;
    };
    let declaration = &artifact.slots()[slot.get() as usize];
    if declaration.role == SlotRole::State {
        return match declaration.initializer {
            Some(InitializerReference::Constant(_)) => true,
            Some(InitializerReference::Activation(source)) => {
                source_has_activation_shape_fact(artifact, ArtifactSource::Slot(source), facts)
            }
            None => false,
        };
    }
    if facts.slot_shapes.contains_key(&slot) {
        return true;
    }
    match declaration.initializer {
        Some(InitializerReference::Constant(_)) => return true,
        Some(InitializerReference::Activation(source)) => {
            return source_has_activation_shape_fact(artifact, ArtifactSource::Slot(source), facts);
        }
        None => {}
    }
    if let ProducerReference::Output { source, .. } = declaration.producer {
        return source_has_activation_shape_fact(artifact, source, facts);
    }
    if let ProducerReference::NodeOutput { node, .. } = declaration.producer
        && node_output_requires_runtime_control_shape(artifact, node)
    {
        // Control outputs acquire their parameterized axes only when the
        // selected block executes. Ordinary operation outputs may still use
        // compiler-proven hints (or a closed schema) as activation facts.
        return false;
    }
    if artifact.slot_shape_hint(slot).is_some() {
        return true;
    }
    artifact
        .schemas()
        .get(declaration.schema)
        .is_some_and(|schema| schema.dimension_parameters().is_empty())
}

fn matrix_shape_for_extents(
    schema: &mech_core::Schema,
    extents: &[u64],
) -> Result<ShapeInstance, ResidentActivationError> {
    mech_core::shape_for_resolved_extents(schema, extents)
        .map_err(|_| ResidentActivationError::RegionSizeOverflow)
}

fn logical_selector_population(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    known: &BTreeMap<ArtifactSource, u64>,
) -> Option<u64> {
    if let Some(population) = known.get(&source) {
        return Some(*population);
    }
    let ArtifactSource::Constant(id) = source else {
        return None;
    };
    match artifact.constants().get(id)?.data() {
        mech_core::ValueData::Bool(value) => Some(u64::from(*value)),
        mech_core::ValueData::Matrix(matrix) => match matrix.elements() {
            mech_core::snapshot::SequenceView::Bool(values) => {
                u64::try_from(values.iter().filter(|value| **value).count()).ok()
            }
            _ => None,
        },
        _ => None,
    }
}

struct ConstantComparisonOperand<'a> {
    value: std::borrow::Cow<'a, Value>,
    schema: SchemaBody,
    element: SchemaBody,
    rows: usize,
    columns: usize,
}

fn scalar_comparison_supported(element: &SchemaBody, ordering: bool) -> bool {
    if ordering {
        matches!(
            element,
            SchemaBody::Index
                | SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(_)
                | SchemaBody::Rational64
                | SchemaBody::Complex(_)
        )
    } else {
        matches!(
            element,
            SchemaBody::Bool
                | SchemaBody::Index
                | SchemaBody::String
                | SchemaBody::UnsignedInteger(_)
                | SchemaBody::SignedInteger(_)
                | SchemaBody::FloatingPoint(_)
                | SchemaBody::Rational64
                | SchemaBody::Complex(_)
        )
    }
}

fn constant_comparison_operand<'a>(
    artifact: &'a ProgramArtifact,
    node: NodeId,
    source: ArtifactSource,
    facts: &ActivationFacts,
) -> Result<Option<ConstantComparisonOperand<'a>>, ResidentActivationError> {
    let value = match source {
        ArtifactSource::Constant(id) => std::borrow::Cow::Borrowed(
            artifact
                .constants()
                .get(id)
                .ok_or(ResidentActivationError::InvalidDependency { node })?,
        ),
        ArtifactSource::Slot(slot) => {
            let ProducerReference::NodeOutput { node: producer, .. } =
                artifact.slots()[slot.get() as usize].producer
            else {
                return Ok(None);
            };
            let Some(operation) = artifact.nodes()[producer.get() as usize].as_operation() else {
                return Ok(None);
            };
            if operation.operation.module_path.as_ref() != ["convert"]
                || operation.operation.operation_name != "kind"
            {
                return Ok(None);
            }
            let inputs = node_inputs(artifact, producer)?;
            let [input] = inputs.as_slice() else {
                return Ok(None);
            };
            let Some(source) = constant_comparison_operand(artifact, node, *input, facts)? else {
                return Ok(None);
            };
            let target_schema_id = artifact.slots()[slot.get() as usize].schema;
            let target_schema = artifact
                .schemas()
                .get(target_schema_id)
                .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            let target_element = match target_schema.body() {
                SchemaBody::Matrix { element, .. } => element.as_ref(),
                body => body,
            };
            let Ok(source_type) = ResolvedType::from_schema_body(&source.element, &[]) else {
                return Ok(None);
            };
            let Ok(target_type) = ResolvedType::from_schema_body(target_element, &[]) else {
                return Ok(None);
            };
            let Ok(conversion) = plan_explicit_cast(&source_type, &target_type) else {
                return Ok(None);
            };
            let Ok(draft) = source.value.canonical_data_draft() else {
                return Ok(None);
            };
            let converted = match draft {
                ValueDataDraft::Matrix(elements) => ValueDataDraft::Matrix({
                    let Ok(elements) = elements
                        .into_vec()
                        .into_iter()
                        .map(|element| execute_conversion_draft(element, &conversion.step))
                        .collect::<Result<Vec<_>, _>>()
                    else {
                        return Ok(None);
                    };
                    elements.into_boxed_slice()
                }),
                scalar => {
                    let Ok(converted) = execute_conversion_draft(scalar, &conversion.step) else {
                        return Ok(None);
                    };
                    converted
                }
            };
            let Ok(converted) = (ValueDraft {
                schema: target_schema_id,
                shape_values: source.value.shape().parameter_values().into(),
                data: converted,
            })
            .finalize(&SnapshotValidationContext::new(artifact.schemas())) else {
                return Ok(None);
            };
            std::borrow::Cow::Owned(converted)
        }
    };
    let schema = artifact
        .schemas()
        .entry(value.schema())
        .ok_or(ResidentActivationError::RegionSizeOverflow)?
        .schema();
    let (element, rows, columns) = match (schema.body(), value.data()) {
        (SchemaBody::Matrix { element, .. }, mech_core::ValueData::Matrix(matrix)) => {
            let extents = source_extents(artifact, source, facts)?;
            let [rows, columns] = extents.as_ref() else {
                return Ok(None);
            };
            let rows =
                usize::try_from(*rows).map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
            let columns = usize::try_from(*columns)
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
            let Some(count) = rows.checked_mul(columns) else {
                return Err(ResidentActivationError::RegionSizeOverflow);
            };
            if count > MAX_STATIC_SELECTOR_SOURCE_STEPS || count != matrix.elements().len() {
                return Ok(None);
            }
            (element.as_ref().clone(), rows, columns)
        }
        (body, _) => (body.clone(), 1, 1),
    };
    Ok(Some(ConstantComparisonOperand {
        value,
        schema: schema.body().clone(),
        element,
        rows,
        columns,
    }))
}

fn closed_aggregate_equality_admitted(
    artifact: &ProgramArtifact,
    left: &Value,
    right: &Value,
    materializes_canonical_bytes: bool,
) -> bool {
    let mut meter = super::budget::ResidentBudgetMeter::default();
    if super::budget::measure_canonical_data_comparison_work(
        &mut meter,
        artifact
            .schemas()
            .get(left.schema())
            .map(|schema| schema.body())
            .unwrap_or(&SchemaBody::Dynamic),
        left.data(),
    )
    .is_err()
        || super::budget::measure_canonical_data_comparison_work(
            &mut meter,
            artifact
                .schemas()
                .get(right.schema())
                .map(|schema| schema.body())
                .unwrap_or(&SchemaBody::Dynamic),
            right.data(),
        )
        .is_err()
    {
        return false;
    }
    let Ok(left_footprint) = left.retained_footprint(artifact.schemas()) else {
        return false;
    };
    let Ok(right_footprint) = right.retained_footprint(artifact.schemas()) else {
        return false;
    };
    let schema_work = if left.schema_key() == right.schema_key() {
        let Some(left_schema) = artifact.schemas().entry(left.schema()) else {
            return false;
        };
        let Some(right_schema) = artifact.schemas().entry(right.schema()) else {
            return false;
        };
        let Ok(work) = u64::try_from(
            left_schema
                .canonical_bytes()
                .len()
                .max(right_schema.canonical_bytes().len()),
        ) else {
            return false;
        };
        work
    } else {
        0
    };
    let Some(encoded_bytes) = left_footprint
        .encoded_bytes
        .checked_add(right_footprint.encoded_bytes)
    else {
        return false;
    };
    let additional_work = if materializes_canonical_bytes {
        left_footprint
            .encoded_bytes
            .min(right_footprint.encoded_bytes)
    } else {
        0
    };
    if materializes_canonical_bytes
        && (meter.charge_temporary_bytes(encoded_bytes).is_err()
            || meter.charge_cloned_bytes(encoded_bytes).is_err())
    {
        return false;
    }
    schema_work
        .checked_add(additional_work)
        .is_some_and(|work| meter.charge_comparison_work(work).is_ok())
}

fn closed_comparison_population(
    artifact: &ProgramArtifact,
    node: NodeId,
    facts: &ActivationFacts,
) -> Result<Option<u64>, ResidentActivationError> {
    // Activation planning may inspect closed constant comparisons, but it must
    // never execute arbitrary turn-dependent nodes to guess a live population.
    let operation = artifact
        .nodes()
        .get(node.get() as usize)
        .and_then(|node| node.as_operation())
        .ok_or(ResidentActivationError::InvalidDependency { node })?;
    if operation.operation.module_path.as_ref() != ["compare"] {
        return Ok(None);
    }
    let name = operation.operation.operation_name.as_str();
    if !matches!(
        name,
        "eq" | "neq" | "seq" | "sneq" | "lt" | "lte" | "gt" | "gte"
    ) {
        return Ok(None);
    }
    let output = node_output_slot(artifact, node)?;
    let output_schema = artifact
        .schemas()
        .get(artifact.slots()[output.get() as usize].schema)
        .ok_or(ResidentActivationError::RegionSizeOverflow)?;
    let scalar_output = matches!(output_schema.body(), SchemaBody::Bool);
    let inputs = node_inputs(artifact, node)?;
    let [left, right] = inputs.as_slice() else {
        return Ok(None);
    };
    let Some(left) = constant_comparison_operand(artifact, node, *left, facts)? else {
        return Ok(None);
    };
    let Some(right) = constant_comparison_operand(artifact, node, *right, facts)? else {
        return Ok(None);
    };
    if scalar_output {
        let compatible_matrix_identity = matches!(
            (&left.schema, &right.schema),
            (
                SchemaBody::Matrix { element: left_element, .. },
                SchemaBody::Matrix { element: right_element, .. },
            ) if left_element == right_element
                && left.rows == right.rows
                && left.columns == right.columns
        );
        if left.schema != right.schema && !compatible_matrix_identity {
            return Ok(match name {
                "seq" => Some(0),
                "sneq" => Some(1),
                _ => None,
            });
        }
        let same_shape = left.value.shape() == right.value.shape();
        let budgeted_equality = !scalar_comparison_supported(&left.schema, false)
            || matches!(left.schema, SchemaBody::String);
        if budgeted_equality
            && matches!(name, "eq" | "neq" | "seq" | "sneq")
            && !closed_aggregate_equality_admitted(
                artifact,
                &left.value,
                &right.value,
                matches!(name, "eq" | "neq"),
            )
        {
            return Ok(None);
        }
        let language_equal = || {
            same_shape
                && schema_data_language_eq(&left.schema, left.value.data(), right.value.data())
        };
        let ordinary_equal = || {
            if scalar_comparison_supported(&left.schema, false) {
                language_equal()
            } else {
                same_shape
                    && schema_data_snapshot_eq(&left.schema, left.value.data(), right.value.data())
            }
        };
        let order = || {
            same_shape
                .then(|| {
                    schema_data_partial_cmp(&left.schema, left.value.data(), right.value.data())
                })
                .flatten()
        };
        let matches = match name {
            "eq" => ordinary_equal(),
            "neq" => !ordinary_equal(),
            // Strict equality routes through strict_value_equal, whose dense
            // and snapshot paths both use language equality.
            "seq" => language_equal(),
            "sneq" => !language_equal(),
            "lt" if scalar_comparison_supported(&left.schema, true) => {
                order() == Some(std::cmp::Ordering::Less)
            }
            "lte" if scalar_comparison_supported(&left.schema, true) => matches!(
                order(),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ),
            "gt" if scalar_comparison_supported(&left.schema, true) => {
                order() == Some(std::cmp::Ordering::Greater)
            }
            "gte" if scalar_comparison_supported(&left.schema, true) => matches!(
                order(),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ),
            _ => return Ok(None),
        };
        return Ok(Some(u64::from(matches)));
    }
    if matches!(name, "seq" | "sneq")
        || !matches!(output_schema.body(), SchemaBody::Matrix { element, .. } if element.as_ref() == &SchemaBody::Bool)
        || left.element != right.element
    {
        return Ok(None);
    }
    let broadcast_axis = |left, right| {
        if left == right {
            Some(left)
        } else if left == 1 {
            Some(right)
        } else if right == 1 {
            Some(left)
        } else {
            None
        }
    };
    let Some(rows) = broadcast_axis(left.rows, right.rows) else {
        return Ok(None);
    };
    let Some(columns) = broadcast_axis(left.columns, right.columns) else {
        return Ok(None);
    };
    let output_len = rows
        .checked_mul(columns)
        .ok_or(ResidentActivationError::RegionSizeOverflow)?;
    if output_len > MAX_STATIC_SELECTOR_SOURCE_STEPS {
        return Ok(None);
    }
    let left_bytes = left
        .value
        .retained_footprint(artifact.schemas())
        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?
        .retained_bytes;
    let right_bytes = right
        .value
        .retained_footprint(artifact.schemas())
        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?
        .retained_bytes;
    let cloned_bytes = left_bytes
        .checked_add(right_bytes)
        .ok_or(ResidentActivationError::RegionSizeOverflow)?;
    if cloned_bytes > mech_core::RESIDENT_MAX_BYTES {
        return Ok(None);
    }
    let values = |operand: &ConstantComparisonOperand<'_>| match operand.value.data() {
        mech_core::ValueData::Matrix(matrix) => matrix.elements().to_values(),
        data => vec![data.clone()],
    };
    let left_values = values(&left);
    let right_values = values(&right);
    let element = &left.element;
    let compare = |left: &mech_core::ValueData, right: &mech_core::ValueData| {
        let order = || schema_data_partial_cmp(element, left, right);
        match name {
            "eq" => schema_data_language_eq(element, left, right),
            "neq" => !schema_data_language_eq(element, left, right),
            "seq" => schema_data_snapshot_eq(element, left, right),
            "sneq" => !schema_data_snapshot_eq(element, left, right),
            "lt" => order() == Some(std::cmp::Ordering::Less),
            "lte" => matches!(
                order(),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ),
            "gt" => order() == Some(std::cmp::Ordering::Greater),
            "gte" => matches!(
                order(),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ),
            _ => unreachable!("comparison operation checked above"),
        }
    };
    let mut population = 0_u64;
    for row in 0..rows {
        for column in 0..columns {
            let left_index = (row % left.rows) * left.columns + column % left.columns;
            let right_index = (row % right.rows) * right.columns + column % right.columns;
            if compare(&left_values[left_index], &right_values[right_index]) {
                population = population
                    .checked_add(1)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            }
        }
    }
    Ok(Some(population))
}

fn complete_activation_shape_facts(
    artifact: &ProgramArtifact,
    supplied: &ActivationFacts,
    classes: &[NodeClass],
    schedule: &ActivationSchedule,
) -> Result<ActivationFacts, ResidentActivationError> {
    let mut facts = supplied.clone();
    let mut required_logical_populations = BTreeSet::new();
    let mut pending_logical_sources = artifact
        .nodes()
        .iter()
        .filter_map(|node| {
            let operation = node.as_operation()?;
            let inputs = node_inputs(artifact, node.node).ok()?;
            (operation.operation.module_path.as_ref() == ["access"]
                && operation
                    .operation
                    .resolved_selection_mode(inputs.len().saturating_sub(1))
                    .is_some_and(|mode| !matches!(mode, ResolvedSelectionMode::LinearScalar)))
            .then(|| inputs[1..].to_vec())
        })
        .flatten()
        .collect::<Vec<_>>();
    while let Some(source) = pending_logical_sources.pop() {
        if !required_logical_populations.insert(source) {
            continue;
        }
        let ArtifactSource::Slot(slot) = source else {
            continue;
        };
        let ProducerReference::NodeOutput { node, .. } =
            artifact.slots()[slot.get() as usize].producer
        else {
            continue;
        };
        let Some(operation) = artifact.nodes()[node.get() as usize].as_operation() else {
            continue;
        };
        if operation.operation.module_path.as_ref() == ["matrix"]
            && matches!(
                operation.operation.operation_name.as_str(),
                "horzcat" | "vertcat"
            )
        {
            pending_logical_sources.extend(node_inputs(artifact, node)?);
        }
    }
    // Concatenation preserves the population of a logical selector. Track only
    // statically known populations; live masks require an explicit output shape
    // and are revalidated by the selection kernel on every turn.
    let mut logical_populations = BTreeMap::<ArtifactSource, u64>::new();
    for node_id in &schedule.nodes {
        let Some(node) = artifact.nodes()[node_id.get() as usize].as_operation() else {
            continue;
        };
        let class = classes[node.node.get() as usize];
        if matches!(class, NodeClass::Observation | NodeClass::External) {
            continue;
        }
        let output = node_output_slot(artifact, node.node)?;
        if class == NodeClass::Activation
            && required_logical_populations.contains(&ArtifactSource::Slot(output))
            && let Some(population) = closed_comparison_population(artifact, node.node, &facts)?
        {
            logical_populations.insert(ArtifactSource::Slot(output), population);
        }
        if node.operation.module_path.as_ref() == ["matrix"]
            && required_logical_populations.contains(&ArtifactSource::Slot(output))
            && matches!(
                node.operation.operation_name.as_str(),
                "horzcat" | "vertcat"
            )
        {
            let population = node_inputs(artifact, node.node)?.iter().try_fold(
                Some(0_u64),
                |total, source| match (
                    total,
                    logical_selector_population(artifact, *source, &logical_populations),
                ) {
                    (Some(total), Some(count)) => total
                        .checked_add(count)
                        .map(Some)
                        .ok_or(ResidentActivationError::RegionSizeOverflow),
                    _ => Ok(None),
                },
            )?;
            if let Some(population) = population {
                logical_populations.insert(ArtifactSource::Slot(output), population);
            }
        }
        if facts.slot_shapes.contains_key(&output) {
            continue;
        }
        let output_schema = artifact
            .schemas()
            .get(artifact.slots()[output.get() as usize].schema)
            .expect("validated output schema");
        // A declared shape relationship is authoritative for custom kernels,
        // not just operations recognized by the built-in shape rules below.
        if matches!(output_schema.body(), SchemaBody::Matrix { .. })
            && let Some(mech_core::ResolvedOperationContract::Declared(contract)) =
                artifact.contracts().get(node.contract)
            && let [port] = contract.outputs.as_ref()
            && let OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input },
            } = port.construction
        {
            let inputs = node_inputs(artifact, node.node)?;
            let source = inputs
                .get(input as usize)
                .copied()
                .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
            if !source_has_activation_shape_fact(artifact, source, &facts) {
                continue;
            }
            let extents = source_extents(artifact, source, &facts)?;
            let shape = matrix_shape_for_extents(output_schema, &extents)
                .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
            facts.slot_shapes.insert(output, shape);
            continue;
        }
        if node.operation.module_path.as_ref() == ["access"]
            && node.operation.operation_name == "index"
        {
            let inputs = node_inputs(artifact, node.node)?;
            let [source] = inputs.as_slice() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            if !source_has_activation_shape_fact(artifact, *source, &facts) {
                continue;
            }
            let extents = source_extents(artifact, *source, &facts)?;
            let declaration = &artifact.slots()[output.get() as usize];
            let output_schema = artifact
                .schemas()
                .entry(declaration.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                .schema();
            // Canonical matrix index conversion flattens selector values into
            // a column. The input may be a row range or any rectangular matrix;
            // copying its axes loses the conversion's declared singleton axis.
            let output_extents = match output_schema.body() {
                SchemaBody::Matrix { .. } => {
                    let cardinality = extents.iter().try_fold(1_u64, |count, extent| {
                        count
                            .checked_mul(*extent)
                            .ok_or(ResidentActivationError::RegionSizeOverflow)
                    })?;
                    vec![cardinality, 1]
                }
                _ => Vec::new(),
            };
            let output_shape = matrix_shape_for_extents(output_schema, &output_extents)
                .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
            facts.slot_shapes.insert(output, output_shape);
            continue;
        }
        let inputs = node_inputs(artifact, node.node)?;
        if node.operation.module_path.as_ref() == ["core"]
            && node.operation.operation_name == "composite-pack"
            && !matches!(output_schema.body(), SchemaBody::Matrix { .. })
        {
            let children = inputs
                .iter()
                .map(|source| source_schema_and_shape(artifact, *source, &facts))
                .collect::<Result<Vec<_>, _>>()?;
            let shape = mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                artifact.slots()[output.get() as usize].schema,
                &children,
                artifact.schemas(),
            )
            .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
            facts.slot_shapes.insert(output, shape);
            continue;
        }
        if node.operation.module_path.as_ref() == ["access"]
            && node.operation.operation_name == "column"
        {
            let [source, _selector] = inputs.as_slice() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            let source_schema_id = match source {
                ArtifactSource::Constant(constant) => artifact
                    .constants()
                    .get(*constant)
                    .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?
                    .schema(),
                ArtifactSource::Slot(slot) => {
                    artifact
                        .slots()
                        .get(slot.get() as usize)
                        .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?
                        .schema
                }
            };
            let source_schema = artifact
                .schemas()
                .entry(source_schema_id)
                .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?
                .schema();
            if let SchemaBody::Table { rows, .. } = source_schema.body() {
                // Bytecode intentionally omits non-wire slot-shape hints, so a
                // dynamic matrix output starts from its valid [0, 0] lower-bound
                // placeholder. An exact table declaration provides durable
                // authority for the selected column's [rows, 1] shape before
                // resident layout and binding. Dynamic or parameter-dependent
                // row counts need live activation facts and stay closed here.
                let CardinalitySpec::Exact(DimensionExpr::Constant(rows)) = rows else {
                    return Err(ResidentActivationError::UnresolvedShape { slot: output });
                };
                let shape = matrix_shape_for_extents(output_schema, &[*rows, 1])
                    .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
                facts.slot_shapes.insert(output, shape);
                continue;
            }
        }
        if node.operation.module_path.as_ref() == ["matrix"]
            && matches!(
                node.operation.operation_name.as_str(),
                "comprehension" | "horzcat" | "vertcat"
            )
        {
            if inputs.is_empty() {
                if node.operation.operation_name == "comprehension" {
                    let declaration = &artifact.slots()[output.get() as usize];
                    let schema = artifact
                        .schemas()
                        .entry(declaration.schema)
                        .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                        .schema();
                    let shape = matrix_shape_for_extents(schema, &[0, 0])
                        .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
                    facts.slot_shapes.insert(output, shape);
                }
                continue;
            }
            if inputs
                .iter()
                .copied()
                .any(|input| !source_has_activation_shape_fact(artifact, input, &facts))
            {
                continue;
            }
            let vertical = node.operation.operation_name == "vertcat";
            let mut common = None;
            let mut varying = 0_u64;
            for input in inputs.iter().copied() {
                let extents = source_extents(artifact, input, &facts)?;
                let (rows, columns) = match extents.as_ref() {
                    [] => (1, 1),
                    [rows, columns] => (*rows, *columns),
                    _ => {
                        return Err(ResidentActivationError::InvalidDependency { node: node.node });
                    }
                };
                let (candidate_common, candidate_varying) = if vertical {
                    (columns, rows)
                } else {
                    (rows, columns)
                };
                if common.is_some_and(|common| common != candidate_common) {
                    return Err(ResidentActivationError::InvalidDependency { node: node.node });
                }
                common = Some(candidate_common);
                varying = varying
                    .checked_add(candidate_varying)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            }
            let common = common.expect("non-empty concatenation has a common extent");
            let extents = if vertical {
                [varying, common]
            } else {
                [common, varying]
            };
            let declaration = &artifact.slots()[output.get() as usize];
            let schema = artifact
                .schemas()
                .entry(declaration.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                .schema();
            let output_extents = if matches!(schema.body(), SchemaBody::Matrix { .. }) {
                extents.as_slice()
            } else {
                &[]
            };
            let shape = matrix_shape_for_extents(schema, output_extents)
                .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
            facts.slot_shapes.insert(output, shape);
            continue;
        }
        if node.operation.module_path.as_ref() == ["access"]
            && let Some(mode) = node
                .operation
                .resolved_selection_mode(inputs.len().saturating_sub(1))
            && !matches!(mode, ResolvedSelectionMode::LinearScalar)
        {
            let Some(source) = inputs.first().copied() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            // A lower-bound snapshot placeholder is not an activation shape
            // fact. Keep the selected result snapshot-backed unless the
            // source axes are actually fixed for this activation.
            if !source_has_activation_shape_fact(artifact, source, &facts) {
                continue;
            }
            let selectors_have_shape_facts = inputs[1..].iter().copied().try_fold(
                true,
                |all, selector| -> Result<bool, ResidentActivationError> {
                    let schema = match selector {
                        ArtifactSource::Constant(id) => artifact
                            .constants()
                            .get(id)
                            .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?
                            .schema(),
                        ArtifactSource::Slot(id) => {
                            artifact
                                .slots()
                                .get(id.get() as usize)
                                .ok_or(ResidentActivationError::InvalidDependency {
                                    node: node.node,
                                })?
                                .schema
                        }
                    };
                    let is_matrix = artifact
                        .schemas()
                        .get(schema)
                        .is_some_and(|schema| matches!(schema.body(), SchemaBody::Matrix { .. }));
                    Ok(all
                        && (!is_matrix
                            || source_has_activation_shape_fact(artifact, selector, &facts)))
                },
            )?;
            if !selectors_have_shape_facts {
                continue;
            }
            let source_dimensions = source_extents(artifact, source, &facts)?;
            let [source_rows, source_columns] = source_dimensions.as_ref() else {
                continue;
            };
            let selector_count = |source: ArtifactSource| {
                let schema = match source {
                    ArtifactSource::Constant(id) => {
                        artifact.constants().get(id).map(|value| value.schema())
                    }
                    ArtifactSource::Slot(id) => artifact
                        .slots()
                        .get(id.get() as usize)
                        .map(|slot| slot.schema),
                }
                .and_then(|schema| artifact.schemas().get(schema))
                .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
                if matches!(schema.body(), SchemaBody::Bool)
                    || matches!(schema.body(), SchemaBody::Matrix { element, .. } if element.as_ref() == &SchemaBody::Bool)
                {
                    return logical_selector_population(artifact, source, &logical_populations)
                        .ok_or(ResidentActivationError::UnresolvedShape { slot: output });
                }
                source_extents(artifact, source, &facts).and_then(|extents| {
                    if extents.is_empty() {
                        Ok(1)
                    } else {
                        extents.iter().try_fold(1_u64, |total, extent| {
                            total
                                .checked_mul(*extent)
                                .ok_or(ResidentActivationError::RegionSizeOverflow)
                        })
                    }
                })
            };
            let extents = match mode {
                ResolvedSelectionMode::Whole => vec![*source_rows, *source_columns],
                ResolvedSelectionMode::LinearGather => {
                    let count = match &inputs[1..] {
                        [] => source_rows
                            .checked_mul(*source_columns)
                            .ok_or(ResidentActivationError::RegionSizeOverflow)?,
                        [selector] => selector_count(*selector)?,
                        _ => {
                            return Err(ResidentActivationError::InvalidDependency {
                                node: node.node,
                            });
                        }
                    };
                    vec![count, 1]
                }
                ResolvedSelectionMode::Rows => {
                    let [selector] = &inputs[1..] else {
                        return Err(ResidentActivationError::InvalidDependency { node: node.node });
                    };
                    vec![selector_count(*selector)?, *source_columns]
                }
                ResolvedSelectionMode::Columns => {
                    let [selector] = &inputs[1..] else {
                        return Err(ResidentActivationError::InvalidDependency { node: node.node });
                    };
                    vec![*source_rows, selector_count(*selector)?]
                }
                ResolvedSelectionMode::Rectangle => {
                    let [rows, columns] = &inputs[1..] else {
                        return Err(ResidentActivationError::InvalidDependency { node: node.node });
                    };
                    vec![selector_count(*rows)?, selector_count(*columns)?]
                }
                ResolvedSelectionMode::LinearScalar
                | ResolvedSelectionMode::Field { .. }
                | ResolvedSelectionMode::TableColumn { .. }
                | ResolvedSelectionMode::MapKey => continue,
            };
            let declaration = &artifact.slots()[output.get() as usize];
            let schema = artifact
                .schemas()
                .entry(declaration.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                .schema();
            let output_extents = if matches!(schema.body(), SchemaBody::Matrix { .. }) {
                extents.as_slice()
            } else {
                &[]
            };
            let shape = matrix_shape_for_extents(schema, output_extents)
                .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
            facts.slot_shapes.insert(output, shape);
            continue;
        }
        if node.operation.module_path.as_ref() == ["matrix"]
            && node.operation.operation_name == "transpose"
        {
            let [source] = inputs.as_slice() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            if !source_has_activation_shape_fact(artifact, *source, &facts) {
                continue;
            }
            let source_dimensions = source_extents(artifact, *source, &facts)?;
            let [rows, columns] = source_dimensions.as_ref() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            let declaration = &artifact.slots()[output.get() as usize];
            let schema = artifact
                .schemas()
                .entry(declaration.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                .schema();
            let shape = matrix_shape_for_extents(schema, &[*columns, *rows])
                .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
            facts.slot_shapes.insert(output, shape);
            continue;
        }
        let is_n_choose_k = node.operation.module_path.as_ref() == ["combinatorics"]
            && node.operation.operation_name == "n-choose-k";
        let n_choose_k_declares_its_output = is_n_choose_k
            && artifact
                .contracts()
                .get(node.contract)
                .is_some_and(|contract| {
                    matches!(
                        contract,
                        mech_core::ResolvedOperationContract::Declared(contract)
                            if matches!(
                                contract.outputs.as_ref(),
                                [output]
                                    if matches!(
                                        output.construction,
                                        OutputConstruction::FullWrite {
                                            shape: ShapeRule::Declared,
                                        }
                                    )
                            )
                    )
                });
        if matches!(
            node.operation.module_path.as_ref(),
            [module] if matches!(module.as_str(), "math" | "compare" | "logic")
        ) || n_choose_k_declares_its_output
        {
            let matrix_inputs_have_facts = inputs.iter().copied().try_fold(
                true,
                |all, source| -> Result<bool, ResidentActivationError> {
                    let schema = match source {
                        ArtifactSource::Constant(constant) => artifact
                            .constants()
                            .get(constant)
                            .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?
                            .schema(),
                        ArtifactSource::Slot(slot) => {
                            artifact
                                .slots()
                                .get(slot.get() as usize)
                                .ok_or(ResidentActivationError::InvalidDependency {
                                    node: node.node,
                                })?
                                .schema
                        }
                    };
                    let is_matrix = artifact
                        .schemas()
                        .get(schema)
                        .is_some_and(|schema| matches!(schema.body(), SchemaBody::Matrix { .. }));
                    Ok(all
                        && (!is_matrix
                            || source_has_activation_shape_fact(artifact, source, &facts)))
                },
            )?;
            // A parameterized matrix declaration carries only its lower-bound
            // placeholder until a live producer establishes the turn's axes.
            // Do not turn that placeholder into a downstream fixed-layout fact.
            if !matrix_inputs_have_facts {
                continue;
            }
            let matrix_inputs = inputs
                .iter()
                .copied()
                .map(|source| source_extents(artifact, source, &facts))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .filter(|extents| !extents.is_empty())
                .collect::<Vec<_>>();
            if !matrix_inputs.is_empty() {
                let mut rows = 1_u64;
                let mut columns = 1_u64;
                for extents in &matrix_inputs {
                    let [candidate_rows, candidate_columns] = extents.as_ref() else {
                        return Err(ResidentActivationError::InvalidDependency { node: node.node });
                    };
                    rows = rows.max(*candidate_rows);
                    columns = columns.max(*candidate_columns);
                }
                let declaration = &artifact.slots()[output.get() as usize];
                let schema = artifact
                    .schemas()
                    .entry(declaration.schema)
                    .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                    .schema();
                if matches!(schema.body(), SchemaBody::Matrix { .. }) {
                    let shape = matrix_shape_for_extents(schema, &[rows, columns])
                        .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
                    facts.slot_shapes.insert(output, shape);
                }
            }
            continue;
        }
        if let Some(mode) = node.operation.resolved_reduction_mode() {
            let [source] = inputs.as_slice() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            if !source_has_activation_shape_fact(artifact, *source, &facts) {
                continue;
            }
            let input_extents = source_extents(artifact, *source, &facts)?;
            let [rows, columns] = input_extents.as_ref() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            let extents = match mode {
                mech_core::ResolvedReductionMode::Columns => [*rows, 1],
                mech_core::ResolvedReductionMode::Rows => [1, *columns],
            };
            let declaration = &artifact.slots()[output.get() as usize];
            let schema = artifact
                .schemas()
                .entry(declaration.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                .schema();
            let shape = matrix_shape_for_extents(schema, &extents)
                .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
            facts.slot_shapes.insert(output, shape);
            continue;
        }
        if is_n_choose_k {
            if class != NodeClass::Activation {
                continue;
            }
            let declaration = &artifact.slots()[output.get() as usize];
            let schema = artifact
                .schemas()
                .entry(declaration.schema)
                .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
                .schema();
            if !matches!(schema.body(), SchemaBody::Matrix { .. }) {
                continue;
            }
            let inputs = node_inputs(artifact, node.node)?;
            let [values_source, selection_source] = inputs.as_slice() else {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            };
            // A control-produced matrix acquires its cardinality only when
            // activation executes. Keep n-choose-k snapshot-backed and let
            // its executor resolve both result axes from that live value.
            if !source_has_activation_shape_fact(artifact, *values_source, &facts) {
                continue;
            }
            let available = match values_source {
                ArtifactSource::Constant(constant) => {
                    let value = artifact
                        .constants()
                        .get(*constant)
                        .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
                    match value.data() {
                        mech_core::ValueData::Matrix(matrix) => matrix.elements().len(),
                        _ => 1,
                    }
                }
                ArtifactSource::Slot(slot) => {
                    let declaration = &artifact.slots()[slot.get() as usize];
                    let schema = artifact
                        .schemas()
                        .entry(declaration.schema)
                        .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?
                        .schema();
                    let shape = slot_shape(artifact, *slot, &facts)?;
                    match schema.body() {
                        SchemaBody::Matrix { dimensions, .. } if dimensions.len() == 2 => {
                            let rows =
                                evaluate_dimension(&dimensions[0], shape.parameter_values())?;
                            let columns =
                                evaluate_dimension(&dimensions[1], shape.parameter_values())?;
                            usize::try_from(rows)
                                .ok()
                                .and_then(|rows| {
                                    usize::try_from(columns)
                                        .ok()
                                        .and_then(|columns| rows.checked_mul(columns))
                                })
                                .ok_or(ResidentActivationError::RegionSizeOverflow)?
                        }
                        _ => 1,
                    }
                }
            };
            let ArtifactSource::Constant(selection) = selection_source else {
                // An activation-produced selection resolves k only when its
                // control executes. Preserve the parameterized result shape;
                // the resident mixed-layout executor validates k and resolves
                // both output axes from the committed selection value.
                continue;
            };
            let selection = artifact
                .constants()
                .get(*selection)
                .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
            let requested = crate::resident::numeric::canonical_n_choose_k_cardinality(selection)
                .filter(|requested| *requested != 0 && *requested <= available)
                .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
            let combinations =
                crate::resident::numeric::checked_combination_count(available, requested)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            if schema.dimension_parameters().len() == 2 {
                let shape = schema
                    .instantiate_shape(
                        vec![requested as u64, combinations as u64].into_boxed_slice(),
                    )
                    .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
                facts.slot_shapes.insert(output, shape);
            }
            continue;
        }
        let Some(mode) = node.operation.resolved_range_mode() else {
            continue;
        };
        if class != NodeClass::Activation {
            continue;
        }
        let inputs = node_inputs(artifact, node.node)?;
        if inputs
            .iter()
            .any(|source| matches!(source, ArtifactSource::Slot(_)))
        {
            // Activation-produced scalar endpoints acquire their values only
            // when the preceding closed control executes. Keep this range in
            // one snapshot lane and let its resident executor publish the
            // resolved row extent instead of treating a compiler shape hint as
            // durable cardinality authority.
            continue;
        }
        let values = inputs
            .iter()
            .map(|source| match source {
                ArtifactSource::Constant(constant) => artifact
                    .constants()
                    .get(*constant)
                    .ok_or(ResidentActivationError::InvalidDependency { node: node.node }),
                ArtifactSource::Slot(_) => {
                    Err(ResidentActivationError::InvalidDependency { node: node.node })
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (inclusive, incremented) = match mode {
            ResolvedRangeMode::Exclusive => (false, false),
            ResolvedRangeMode::ExclusiveIncrement => (false, true),
            ResolvedRangeMode::Inclusive => (true, false),
            ResolvedRangeMode::InclusiveIncrement => (true, true),
        };
        let count =
            crate::resident::numeric::canonical_range_cardinality(&values, inclusive, incremented)
                .filter(|count| *count != 0)
                .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
        let declaration = &artifact.slots()[output.get() as usize];
        let schema = artifact
            .schemas()
            .entry(declaration.schema)
            .ok_or(ResidentActivationError::InvalidNodeOutput { node: node.node })?
            .schema();
        let shape = matrix_shape_for_extents(schema, &[1, count as u64])
            .map_err(|_| ResidentActivationError::UnresolvedShape { slot: output })?;
        facts.slot_shapes.insert(output, shape);
    }
    Ok(facts)
}

struct LayoutBuild {
    control_locals: BTreeMap<(NodeId, u32, u32), (CellSlotId, NodeId)>,
    match_bindings: BTreeMap<(NodeId, u32, u32), (CellSlotId, NodeId)>,
    slots: Box<[ResolvedSlot]>,
    constant_regions: Box<[ResidentRegion]>,
    memory_plan: ProgramMemoryPlan,
}

fn build_layout(
    artifact: &ProgramArtifact,
    facts: &ActivationFacts,
    classes: &[NodeClass],
    positions: &BTreeMap<NodeId, u32>,
) -> Result<LayoutBuild, ResidentActivationError> {
    let mut last_consumers = BTreeMap::<CellSlotId, NodeId>::new();
    for binding in artifact.bindings() {
        if let BindingDeclaration::Input {
            node,
            source: ArtifactSource::Slot(slot),
            ..
        } = binding
        {
            last_consumers
                .entry(*slot)
                .and_modify(|last| {
                    if positions[last] < positions[node] {
                        *last = *node;
                    }
                })
                .or_insert(*node);
        }
    }
    let mut planned = Vec::with_capacity(artifact.constants().len() + artifact.slots().len());
    let mut constant_layouts = Vec::with_capacity(artifact.constants().len());
    for raw in 0..artifact.constants().len() {
        let constant = ConstantId::new(raw as u32);
        let value = artifact.constants().get(constant).unwrap();
        let (kind, shape) = value_layout(artifact, value)?;
        let len = shape
            .len()
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        let schema = artifact
            .schemas()
            .entry(value.schema())
            .unwrap()
            .schema()
            .clone();
        let descriptor =
            mech_core::ResolvedValueDescriptor::from_schema(schema, value.shape().clone())
                .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)?;
        let footprint = resident_value_footprint(artifact, value, kind, len)?;
        planned.push(ResidentValuePlanInput {
            owner: mech_core::MemoryObjectOwner::Constant(constant),
            slot: None,
            descriptor,
            class: PlannedValueClass::Constant,
            kind,
            elements: u64::try_from(len)
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
            footprint,
            lifetime: mech_core::MemoryLifetime::Program,
            producer: None,
        });
        constant_layouts.push((constant, kind, shape, len));
    }
    let mut slot_layouts = Vec::with_capacity(artifact.slots().len());
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
        let activation_fixed = slot_has_activation_fixed_shape(artifact, declaration.slot, facts);
        let (kind, resident_shape) = schema_layout(
            artifact,
            declaration.schema,
            &shape,
            activation_fixed,
            Some(declaration.slot),
        )?;
        let len = resident_shape
            .len()
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        let schema = artifact.schemas().entry(declaration.schema).unwrap();
        let descriptor =
            mech_core::ResolvedValueDescriptor::from_schema(schema.schema().clone(), shape.clone())
                .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)?;
        let footprint = declaration
            .initializer
            .and_then(|initializer| match initializer {
                InitializerReference::Constant(constant) => artifact.constants().get(constant),
                InitializerReference::Activation(_) => None,
            })
            .map(|value| resident_value_footprint(artifact, value, kind, len))
            .transpose()?
            .unwrap_or(CurrentMemoryFootprint {
                logical_elements: u64::try_from(len)
                    .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
                ..CurrentMemoryFootprint::default()
            });
        planned.push(ResidentValuePlanInput {
            owner: mech_core::MemoryObjectOwner::Slot(declaration.slot),
            slot: Some(declaration.slot),
            descriptor,
            class: match storage {
                ResidentStorageClass::Constant => PlannedValueClass::Constant,
                ResidentStorageClass::Input => PlannedValueClass::Input,
                ResidentStorageClass::State => PlannedValueClass::State,
                ResidentStorageClass::Scratch => PlannedValueClass::Scratch,
            },
            kind,
            elements: u64::try_from(len)
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
            footprint,
            lifetime: resident_slot_lifetime(
                declaration,
                storage,
                last_consumers.get(&declaration.slot).copied(),
                positions,
            )?,
            producer: match declaration.producer {
                ProducerReference::NodeOutput { node, .. } => Some(node),
                _ => None,
            },
        });
        slot_layouts.push((
            declaration.clone(),
            schema.key(),
            shape,
            activation_fixed,
            storage,
            kind,
            resident_shape,
            len,
        ));
    }
    let mut control_locals = BTreeMap::new();
    let mut match_bindings = BTreeMap::new();
    let mut next_local = 0u32;
    for node in artifact.nodes() {
        let local_definitions = match &node.body {
            crate::ExecutableNodeBody::Match(control) => {
                comprehension::all_match_local_definitions(control)
            }
            crate::ExecutableNodeBody::Activation(control) => {
                comprehension::all_match_local_definitions(control)
            }
            crate::ExecutableNodeBody::Comprehension(control) => {
                comprehension::all_local_definitions(control)
            }
            _ => continue,
        };
        let mut ordinal = 0usize;
        for (pattern_binding, turn_shaped_binding, block, local, schema_id) in local_definitions {
            let slot = CellSlotId(
                u32::try_from(slot_layouts.len())
                    .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
            );
            let physical_node = NodeId(
                u32::try_from(artifact.nodes().len())
                    .ok()
                    .and_then(|count| count.checked_add(next_local))
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?,
            );
            next_local = next_local
                .checked_add(1)
                .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            let schema = artifact.schemas().entry(schema_id).unwrap();
            let shape =
                mech_core::shape_for_declared_lower_bounds(schema.schema()).map_err(|_| {
                    ResidentActivationError::UnsupportedControlLayout { node: node.node }
                })?;
            let has_fixed_shape = schema.schema().dimension_parameters().is_empty();
            let variable_dense_matrix = matches!(
                schema.schema().body(),
                SchemaBody::Matrix { element, dimensions }
                    if dimensions.len() == 2 && dense_resident_kind(element).is_some()
            ) && !has_fixed_shape;
            let turn_shaped_schema = schema
                .schema()
                .dimension_parameters()
                .iter()
                .any(|parameter| parameter.lifetime() == DimensionLifetime::Turn);
            let (kind, resident_shape) =
                if variable_dense_matrix && (turn_shaped_binding || turn_shaped_schema) {
                    // A turn-shaped binding or operation local carries its live
                    // extent in the canonical snapshot; the scratch arena itself
                    // remains one fixed snapshot lane.
                    (ResidentValueKind::Snapshot, ResidentShape::SCALAR)
                } else {
                    schema_layout(artifact, schema_id, &shape, has_fixed_shape, None)?
                };
            let len = resident_shape
                .len()
                .ok_or(ResidentActivationError::RegionSizeOverflow)?;
            let (first, last) = schedule_points(positions[&node.node])
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
            planned.push(ResidentValuePlanInput {
                owner: mech_core::MemoryObjectOwner::NodeScratch {
                    node: node.node,
                    ordinal: u16::try_from(ordinal)
                        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
                },
                slot: Some(slot),
                descriptor: mech_core::ResolvedValueDescriptor::from_schema(
                    schema.schema().clone(),
                    shape.clone(),
                )
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
                class: PlannedValueClass::Scratch,
                kind,
                elements: len as u64,
                footprint: CurrentMemoryFootprint {
                    logical_elements: len as u64,
                    ..CurrentMemoryFootprint::default()
                },
                lifetime: mech_core::MemoryLifetime::Turn { first, last },
                producer: Some(node.node),
            });
            if pattern_binding {
                match_bindings.insert((node.node, block, local), (slot, physical_node));
            } else {
                control_locals.insert((node.node, block, local), (slot, physical_node));
            }
            slot_layouts.push((
                crate::SlotDeclaration {
                    slot,
                    schema: schema_id,
                    role: SlotRole::Derived,
                    producer: ProducerReference::NodeOutput {
                        node: node.node,
                        output_ordinal: 0,
                    },
                    initializer: None,
                },
                schema.key(),
                shape,
                true,
                ResidentStorageClass::Scratch,
                kind,
                resident_shape,
                len,
            ));
            // The structural maximum permits ordinals 0..=65535.
            ordinal += 1;
        }
    }
    let slot_owners = planned
        .iter()
        .filter_map(|value| value.slot.map(|slot| (slot, value.owner.clone())))
        .collect::<BTreeMap<_, _>>();
    let projection = plan_resident_arenas(&planned)
        .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
    ensure_resident_plan_admitted(&projection.plan)?;
    let constant_regions = constant_layouts
        .into_iter()
        .map(|(constant, kind, shape, len)| {
            Ok(ResidentRegion {
                kind,
                offset: *projection
                    .element_offsets
                    .get(&mech_core::MemoryObjectOwner::Constant(constant))
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?,
                len,
                shape,
            })
        })
        .collect::<Result<Vec<_>, ResidentActivationError>>()?;
    let slots = slot_layouts
        .into_iter()
        .map(
            |(
                declaration,
                schema_key,
                shape,
                activation_fixed_shape,
                storage,
                kind,
                resident_shape,
                len,
            )| {
                Ok(ResolvedSlot {
                    artifact_id: declaration.slot,
                    role: declaration.role,
                    physical_index: SlotIndex::new(declaration.slot.get()),
                    schema: declaration.schema,
                    schema_key,
                    shape,
                    activation_fixed_shape,
                    storage,
                    region: ResidentRegion {
                        kind,
                        offset: *projection
                            .element_offsets
                            .get(&slot_owners[&declaration.slot])
                            .ok_or(ResidentActivationError::RegionSizeOverflow)?,
                        len,
                        shape: resident_shape,
                    },
                })
            },
        )
        .collect::<Result<Vec<_>, ResidentActivationError>>()?;
    Ok(LayoutBuild {
        control_locals,
        match_bindings,
        slots: slots.into_boxed_slice(),
        constant_regions: constant_regions.into_boxed_slice(),
        memory_plan: projection.plan,
    })
}

fn resident_slot_lifetime(
    declaration: &crate::SlotDeclaration,
    storage: ResidentStorageClass,
    last_consumer: Option<NodeId>,
    positions: &BTreeMap<NodeId, u32>,
) -> Result<mech_core::MemoryLifetime, ResidentActivationError> {
    match storage {
        ResidentStorageClass::Input | ResidentStorageClass::State => {
            Ok(mech_core::MemoryLifetime::Activation)
        }
        ResidentStorageClass::Constant => Ok(match declaration.producer {
            ProducerReference::NodeOutput { .. } => mech_core::MemoryLifetime::Activation,
            _ => mech_core::MemoryLifetime::Program,
        }),
        ResidentStorageClass::Scratch => {
            let ProducerReference::NodeOutput { node, .. } = declaration.producer else {
                return Err(ResidentActivationError::RegionSizeOverflow);
            };
            let (first, producer_after) = schedule_points(positions[&node])
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
            let last = match last_consumer {
                Some(consumer) => {
                    schedule_points(positions[&consumer])
                        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?
                        .1
                }
                None => producer_after,
            };
            if last < first {
                return Err(ResidentActivationError::RegionSizeOverflow);
            }
            Ok(mech_core::MemoryLifetime::Turn { first, last })
        }
    }
}

fn slot_shape(
    artifact: &ProgramArtifact,
    slot: CellSlotId,
    facts: &ActivationFacts,
) -> Result<ShapeInstance, ResidentActivationError> {
    let declaration = &artifact.slots()[slot.get() as usize];
    match declaration.initializer {
        Some(InitializerReference::Constant(constant)) => {
            return Ok(artifact.constants().get(constant).unwrap().shape().clone());
        }
        Some(InitializerReference::Activation(source)) => {
            return slot_shape(artifact, source, facts);
        }
        None => {}
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
    if let Some(shape) = artifact.slot_shape_hint(slot) {
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
    // Snapshot-backed aggregate outputs carry their canonical shape inside
    // the published value. Their arena is one scalar snapshot cell, so a
    // durable artifact may start turn-varying parameters at their declared
    // lower bounds without serializing compilation-instance shape hints.
    let requires_dense_matrix_shape = matches!(
        schema.body(),
        SchemaBody::Matrix { element, dimensions }
            if dimensions.len() == 2 && dense_resident_kind(element).is_some()
    );
    if !requires_dense_matrix_shape
        || comprehension::owns_output(artifact, slot)
        || matches!(declaration.producer, ProducerReference::NodeOutput { .. })
    {
        return mech_core::shape_for_declared_lower_bounds(schema)
            .map_err(|_| ResidentActivationError::UnresolvedShape { slot });
    }
    Err(ResidentActivationError::UnresolvedShape { slot })
}

fn slot_has_activation_fixed_shape(
    artifact: &ProgramArtifact,
    slot: CellSlotId,
    facts: &ActivationFacts,
) -> bool {
    let declaration = &artifact.slots()[slot.get() as usize];
    // A state initializer fixes the resident arena only when its own shape is
    // known at planning time. Closed control initializers can determine their
    // aggregate shape only while activation executes, so those states retain
    // the canonical value (and its shape) in snapshot storage.
    if declaration.role == SlotRole::State {
        return match declaration.initializer {
            Some(InitializerReference::Constant(_)) => true,
            Some(InitializerReference::Activation(source)) => {
                slot_has_activation_fixed_shape(artifact, source, facts)
            }
            None => false,
        };
    }
    if facts.slot_shapes.contains_key(&slot) {
        return true;
    }
    match declaration.producer {
        ProducerReference::NodeOutput { node, .. }
            if node_output_requires_runtime_control_shape(artifact, node) =>
        {
            false
        }
        ProducerReference::NodeOutput { .. } => artifact.slot_shape_hint(slot).is_some(),
        ProducerReference::Output {
            source: ArtifactSource::Slot(source),
            ..
        } => slot_has_activation_fixed_shape(artifact, source, facts),
        _ => artifact.slot_shape_hint(slot).is_some(),
    }
}

fn value_layout(
    artifact: &ProgramArtifact,
    value: &Value,
) -> Result<(ResidentValueKind, ResidentShape), ResidentActivationError> {
    // A constant's concrete shape is immutable even when its semantic schema
    // originated from a turn-lifetime dynamic backing.
    schema_layout(artifact, value.schema(), value.shape(), true, None)
}

fn resident_value_footprint(
    artifact: &ProgramArtifact,
    value: &Value,
    kind: ResidentValueKind,
    logical_elements: usize,
) -> Result<CurrentMemoryFootprint, ResidentActivationError> {
    if !matches!(
        kind,
        ResidentValueKind::String | ResidentValueKind::Snapshot
    ) {
        return Ok(CurrentMemoryFootprint {
            logical_elements: u64::try_from(logical_elements)
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
            ..CurrentMemoryFootprint::default()
        });
    }
    let retained = value
        .retained_footprint(artifact.schemas())
        .map_err(|_| ResidentActivationError::InvalidSnapshotRepresentation)?;
    Ok(CurrentMemoryFootprint {
        logical_elements: u64::try_from(logical_elements)
            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
        payload_bytes: retained.retained_bytes,
        encoded_bytes: retained.encoded_bytes,
        retained_nodes: retained.node_count,
        shape_parameter_count: u64::try_from(value.shape().parameter_values().len())
            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
        ..CurrentMemoryFootprint::default()
    })
}

fn dense_resident_kind(body: &SchemaBody) -> Option<ResidentValueKind> {
    match body {
        SchemaBody::Bool => Some(ResidentValueKind::Bool),
        SchemaBody::Index => Some(ResidentValueKind::Index),
        SchemaBody::FloatingPoint(mech_core::FloatWidth::W64) => Some(ResidentValueKind::F64),
        SchemaBody::String => Some(ResidentValueKind::String),
        _ => None,
    }
}

fn schema_layout(
    artifact: &ProgramArtifact,
    schema: SchemaId,
    shape: &ShapeInstance,
    has_activation_shape_fact: bool,
    slot: Option<CellSlotId>,
) -> Result<(ResidentValueKind, ResidentShape), ResidentActivationError> {
    let schema_entry = artifact.schemas().entry(schema).unwrap();
    if slot.is_some_and(|slot| comprehension::owns_output(artifact, slot)) {
        return Ok((ResidentValueKind::Snapshot, ResidentShape::SCALAR));
    }
    // Only dense storage needs turn-invariant geometry in the arena. A
    // snapshot occupies one scalar slot and carries its own per-turn shape.
    let needs_dense_shape = matches!(schema_entry.schema().body(),
        SchemaBody::Matrix { element, dimensions }
            if dimensions.len() == 2 && dense_resident_kind(element).is_some());
    let produced_without_shape_fact = slot.is_some_and(|slot| {
        matches!(
            artifact.slots()[slot.get() as usize].producer,
            ProducerReference::NodeOutput { .. } | ProducerReference::Output { .. }
        )
    });
    if needs_dense_shape
        && !schema_entry.schema().dimension_parameters().is_empty()
        && !has_activation_shape_fact
        && produced_without_shape_fact
    {
        return Ok((ResidentValueKind::Snapshot, ResidentShape::SCALAR));
    }
    if needs_dense_shape
        && schema_entry
            .schema()
            .dimension_parameters()
            .iter()
            .any(|parameter| parameter.lifetime() == DimensionLifetime::Turn)
        && !has_activation_shape_fact
    {
        return Err(ResidentActivationError::TurnDimension { schema, slot });
    }
    match schema_entry.schema().body() {
        body @ (SchemaBody::Bool
        | SchemaBody::Index
        | SchemaBody::String
        | SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)) => {
            Ok((dense_resident_kind(body).unwrap(), ResidentShape::SCALAR))
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } if dimensions.len() == 2 && dense_resident_kind(element).is_some() => {
            let kind = dense_resident_kind(element).expect("guarded resident matrix element kind");
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

fn bind_resident_operation(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    node_id: NodeId,
    operation: &crate::OperationReference,
    contract_id: mech_core::OperationContractId,
    input_layouts: &[ResidentPortLayout],
    output_layout: ResidentPortLayout,
) -> Result<(BoundResidentKernel, mech_core::CallMemoryPlan), ResidentActivationError> {
    let canonical_operation = operation.canonical_name();
    let mech_core::ResolvedOperationContract::Declared(contract) = artifact
        .contracts()
        .get(contract_id)
        .ok_or(ResidentActivationError::LegacyOpaque { node: node_id })?
    else {
        return Err(ResidentActivationError::LegacyOpaque { node: node_id });
    };
    let output_contract = &contract.outputs[0];
    let input_descriptors = input_layouts
        .iter()
        .map(|layout| {
            artifact
                .schemas()
                .get(layout.schema_id)
                .cloned()
                .ok_or(ResidentActivationError::KernelBind {
                    node: node_id,
                    error: ResidentKernelBindError::InvalidParameters,
                })
                .and_then(|schema| {
                    mech_core::ResolvedValueDescriptor::from_schema(
                        schema,
                        layout.shape_instance.clone(),
                    )
                    .map_err(|_| ResidentActivationError::KernelBind {
                        node: node_id,
                        error: ResidentKernelBindError::InvalidParameters,
                    })
                })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let output_descriptor = artifact
        .schemas()
        .get(output_layout.schema_id)
        .cloned()
        .ok_or(ResidentActivationError::KernelBind {
            node: node_id,
            error: ResidentKernelBindError::InvalidParameters,
        })
        .and_then(|schema| {
            mech_core::ResolvedValueDescriptor::from_schema(
                schema,
                output_layout.shape_instance.clone(),
            )
            .map_err(|_| ResidentActivationError::KernelBind {
                node: node_id,
                error: ResidentKernelBindError::InvalidParameters,
            })
        })?;
    let resident_entry =
        catalog.resident_factory(&operation.module_path, &operation.operation_name);
    let resident_operation = resident_entry.map_or_else(
        || {
            ResidentOperationKey::new(
                operation.module_path.clone(),
                operation.operation_name.clone(),
            )
        },
        |entry| Some(entry.key.clone()),
    );
    let resident_operation = resident_operation.ok_or(ResidentActivationError::KernelBind {
        node: node_id,
        error: ResidentKernelBindError::InvalidParameters,
    })?;
    let semantic_operation = mech_core::ResolvedOperationDescriptor::from_resolved_contract(
        canonical_operation,
        artifact
            .contracts()
            .get(contract_id)
            .ok_or(ResidentActivationError::KernelBind {
                node: node_id,
                error: ResidentKernelBindError::InvalidParameters,
            })?,
    )
    .map_err(|_| ResidentActivationError::KernelBind {
        node: node_id,
        error: ResidentKernelBindError::InvalidParameters,
    })?;
    let resident_context = ResidentBuildContext {
        bound_call: BoundCall::artifact_operation(
            semantic_operation,
            input_descriptors,
            vec![output_descriptor].into_boxed_slice(),
            resident_operation,
        )
        .map_err(|_| ResidentActivationError::KernelBind {
            node: node_id,
            error: ResidentKernelBindError::InvalidParameters,
        })?,
    };
    let implementation_memory = resident_entry
        .map_or(ImplementationMemoryClass::NoAdditionalScratch, |entry| {
            entry.implementation_memory
        });
    let memory_plan = resident_call_memory_plan(
        &resident_context.bound_call,
        &input_layouts,
        &output_layout,
        output_contract,
        implementation_memory,
    )?;
    let bind_request = ResidentKernelBindRequest {
        contract: artifact.contracts().get(contract_id).unwrap(),
        schemas: artifact.schemas(),
        inputs: &input_layouts,
        output: output_layout,
    };
    let kernel = if let Some(factory) = resident_entry {
        (factory.factory)(&bind_request)
    } else {
        #[cfg(feature = "dynamic-modules")]
        {
            crate::function::bind_dynamic_resident_operation(
                &operation.module_path,
                &operation.operation_name,
                &bind_request,
            )
            .ok_or(ResidentActivationError::MissingResidentFactory { node: node_id })?
        }
        #[cfg(not(feature = "dynamic-modules"))]
        {
            return Err(ResidentActivationError::MissingResidentFactory { node: node_id });
        }
    }
    .map_err(|error| ResidentActivationError::KernelBind {
        node: node_id,
        error,
    })?
    .with_bound_call(resident_context.bound_call);
    Ok((kernel, memory_plan))
}

fn continuation_dependency_node(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    visiting: &mut BTreeSet<NodeId>,
) -> Result<Option<NodeId>, ResidentActivationError> {
    let ArtifactSource::Slot(slot) = source else {
        return Ok(None);
    };
    let declaration = &artifact.slots()[slot.get() as usize];
    // Mutable state is a retained turn boundary. Its current published value
    // does not depend on whether the node that produced an older version is
    // presently suspended.
    if declaration.role == SlotRole::State {
        return Ok(None);
    }
    let node = match declaration.producer {
        ProducerReference::Output { source, .. } => {
            return continuation_dependency_node(artifact, source, visiting);
        }
        ProducerReference::NodeOutput { node, .. } => node,
        ProducerReference::Input(_) => return Ok(None),
    };
    if !visiting.insert(node) {
        return Ok(None);
    }
    let declaration = &artifact.nodes()[node.get() as usize];
    if matches!(&declaration.body, crate::ExecutableNodeBody::Fsm(_))
        || matches!(&declaration.body, crate::ExecutableNodeBody::Match(control) if control.contains_suspend())
        || matches!(&declaration.body, crate::ExecutableNodeBody::Comprehension(control) if control.contains_suspend())
    {
        return Ok(Some(node));
    }
    for input in node_inputs(artifact, node)? {
        if let Some(node) = continuation_dependency_node(artifact, input, visiting)? {
            return Ok(Some(node));
        }
    }
    Ok(None)
}

fn build_plan(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    classes: Box<[NodeClass]>,
    schedule: ActivationSchedule,
    mut layout: LayoutBuild,
    activation_facts_fingerprint: [u8; 32],
    options: ResidentActivationOptions,
    static_selectors: &mut ArtifactStaticSelectorResolver,
) -> Result<ActivatedPlan, ResidentActivationError> {
    let mut reads = Vec::new();
    let mut steps = Vec::new();
    let mut activation_steps = Vec::new();
    let mut call_memory_plan_nodes = Vec::new();
    let mut call_memory_plans = Vec::new();
    let ActivationSchedule {
        nodes: scheduled_nodes,
        positions,
        mut artifact_to_activated,
        mut topology,
    } = schedule;
    let mut effect_ordinal = 0_u32;
    let activation_control = |node: &&crate::NodeDeclaration| {
        classes[node.node.get() as usize] == NodeClass::Activation && node.as_operation().is_none()
    };
    // Keep turn indexes stable; activation controls share the executor but are
    // appended outside its turn topology and never scheduled by a turn.
    for node in artifact
        .nodes()
        .iter()
        .filter(|node| !activation_control(node))
        .chain(artifact.nodes().iter().filter(activation_control))
    {
        if node.as_operation().is_none() {
            let index = ActivatedNodeIndex(steps.len() as u32);
            artifact_to_activated[node.node.get() as usize] = Some(index);
            if classes[node.node.get() as usize] == NodeClass::Activation {
                let output = &layout.slots[node_output_slot(artifact, node.node)?.get() as usize];
                activation_steps.push(ActivatedOnceNode {
                    artifact_node: node.node,
                    sources: Box::new([]),
                    base_input: None,
                    storage: output.storage,
                    write: output.region,
                    body: ActivatedOnceBody::Control(index),
                });
            }
        }
        if let crate::ExecutableNodeBody::Comprehension(control) = &node.body {
            let output_slot = node_output_slot(artifact, node.node)?;
            let output = &layout.slots[output_slot.get() as usize];
            let read_start = reads.len() as u32;
            for source in node_inputs(artifact, node.node)? {
                reads.push(resolve_read(&layout, source)?);
            }
            steps.push(ActivatedTurnStep::Comprehension(std::sync::Arc::new(
                ActivatedComprehensionNode {
                    artifact_node: node.node,
                    memory_node: node.node,
                    reads: read_start..reads.len() as u32,
                    write: ResidentWriteLocation {
                        slot: output_slot,
                        storage: output.storage,
                        region: output.region,
                    },
                    kind: control.kind,
                    output_schema: output.schema,
                    steps: Box::new([]),
                    locals: Box::new([]),
                    schema_reads: Box::new([]),
                    yield_value: ResidentReadLocation::Scratch(output.region),
                    yield_schema: output.schema,
                },
            )));
            continue;
        }
        let Some(node) = node.as_operation() else {
            let control = match &node.body {
                crate::ExecutableNodeBody::Match(control)
                | crate::ExecutableNodeBody::Activation(control) => control,
                _ => unreachable!(),
            };
            let input_sources = control_input_sources(artifact, node.node)?;
            let input_reads = input_sources
                .iter()
                .copied()
                .map(|source| resolve_read(&layout, source))
                .collect::<Result<Vec<_>, _>>()?;
            let output_slot = node_output_slot(artifact, node.node)?;
            steps.push(ActivatedTurnStep::Match(prepare_match_node(
                artifact,
                node.node,
                node.node,
                control,
                &input_sources,
                &input_reads,
                output_slot,
                &layout,
            )?));
            continue;
        };
        let class = classes[node.node.get() as usize];
        if class == NodeClass::Observation {
            continue;
        }
        let input_sources = control_input_sources(artifact, node.node)?;
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
            let payload_layout = source_port_layout(artifact, &layout, *source, static_selectors)?;
            let payload_shape = match source {
                ArtifactSource::Slot(slot) => layout.slots[slot.get() as usize].shape.clone(),
                ArtifactSource::Constant(constant) => {
                    artifact.constants().get(*constant).unwrap().shape().clone()
                }
            };
            let payload_region = payload.region();
            let captured_payload = ResidentRegion {
                kind: payload_region.kind,
                offset: plan_resident_effect_payload(
                    &mut layout.memory_plan,
                    node.node,
                    positions[&node.node],
                    payload_region.kind,
                    u64::try_from(payload_region.len)
                        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
                )
                .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?,
                len: payload_region.len,
                shape: payload_region.shape,
            };
            ensure_resident_plan_admitted(&layout.memory_plan)?;
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
        let output = layout.slots[output_slot.get() as usize].clone();
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
        // Resident storage has a fixed physical extent for the lifetime of an
        // activated plan. A range whose endpoints depend on turn inputs or
        // mutable state can legitimately change cardinality in the source
        // runtime, so reject that target capability before a bytecode-backed
        // instance is emitted instead of failing on a later execution turn.
        if class != NodeClass::Activation
            && operation_requires_activation_fixed_range_shape(&node.operation)
        {
            return Err(ResidentActivationError::KernelBind {
                node: node.node,
                error: ResidentKernelBindError::UnsupportedLayout,
            });
        }
        let input_layouts = input_sources
            .iter()
            .map(|source| source_port_layout(artifact, &layout, *source, static_selectors))
            .collect::<Result<Vec<_>, _>>()?;
        let output_layout = slot_port_layout(&output);
        let (kernel, memory_plan) = bind_resident_operation(
            artifact,
            catalog,
            node.node,
            node.operation,
            node.contract,
            &input_layouts,
            output_layout,
        )?;
        call_memory_plan_nodes.push(node.node);
        call_memory_plans.push(memory_plan);
        if class == NodeClass::Activation {
            activation_steps.push(ActivatedOnceNode {
                artifact_node: node.node,
                sources: input_sources.into_boxed_slice(),
                base_input: base,
                storage: output.storage,
                write: output.region,
                body: ActivatedOnceBody::Kernel(kernel),
            });
            continue;
        }
        let read_start = reads.len() as u32;
        // Derived RMW updates seed fresh scratch from their explicit base;
        // state RMW continues to seed its own candidate buffer.
        let rmw_base = if output.storage == ResidentStorageClass::Scratch {
            base.map(|index| resolve_read(&layout, input_sources[index]))
                .transpose()?
        } else {
            None
        };
        let rmw_previous = if rmw_base.is_some()
            && output_contract.change_detection == ChangeDetectionPolicy::KernelReported
        {
            Some(ResidentRegion {
                offset: crate::memory_planner::plan_resident_rmw_previous(
                    &mut layout.memory_plan,
                    node.node,
                    positions[&node.node],
                    output.region.kind,
                    output.region.len as u64,
                )
                .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?,
                ..output.region
            })
        } else {
            None
        };
        ensure_resident_plan_admitted(&layout.memory_plan)?;
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
            memory_node: node.node,
            reads: read_start..reads.len() as u32,
            write: ResidentWriteLocation {
                slot: output_slot,
                storage: output.storage,
                region: output.region,
            },
            construction: output_contract.construction.clone(),
            rmw_base,
            rmw_previous,
            change_detection: output_contract.change_detection,
            reads_state,
            scratch_prefix_reads,
            kernel,
        }));
    }
    let mut control_calls = Vec::new();
    for node in artifact.nodes() {
        if let crate::ExecutableNodeBody::Comprehension(control) = &node.body {
            let index = artifact_to_activated[node.node.get() as usize]
                .unwrap()
                .get() as usize;
            let (instructions, local_regions, schema_reads, yielded, yield_schema, call) =
                comprehension::bind(
                    artifact,
                    catalog,
                    node.node,
                    control,
                    &layout,
                    &mut steps,
                    &mut reads,
                    &mut control_calls,
                )?;
            let ActivatedTurnStep::Comprehension(prepared) = &mut steps[index] else {
                unreachable!()
            };
            let prepared = std::sync::Arc::get_mut(prepared).expect("unpublished collection plan");
            prepared.steps = instructions;
            prepared.locals = local_regions;
            prepared.schema_reads = schema_reads;
            prepared.yield_value = yielded;
            prepared.yield_schema = yield_schema;
            control_calls.push((
                node.node,
                crate::memory_planner::CallSiteMemoryTemplate {
                    node: node.node,
                    input_sources: node_inputs(artifact, node.node)?.into_boxed_slice(),
                    output_slots: vec![prepared.write.slot].into_boxed_slice(),
                },
                call,
            ));
            continue;
        }
        let control = match &node.body {
            crate::ExecutableNodeBody::Match(control)
            | crate::ExecutableNodeBody::Activation(control) => control,
            _ => continue,
        };
        let input_sources = node_inputs(artifact, node.node)?;
        let input_reads = input_sources
            .iter()
            .copied()
            .map(|source| resolve_read(&layout, source))
            .collect::<Result<Vec<_>, _>>()?;
        let root = artifact_to_activated[node.node.get() as usize].unwrap();
        let arms = bind_match_arms(
            artifact,
            catalog,
            node.node,
            control,
            &input_sources,
            &input_reads,
            &layout,
            &mut steps,
            &mut reads,
            &mut control_calls,
            &[root],
        )?;
        let ActivatedTurnStep::Match(matched) = &mut steps[artifact_to_activated
            [node.node.get() as usize]
            .unwrap()
            .get() as usize]
        else {
            unreachable!()
        };
        matched.arms = arms;
    }
    let mut pending = activation_steps
        .into_iter()
        .map(|step| (step.artifact_node, step))
        .collect::<BTreeMap<_, _>>();
    let activation_steps = scheduled_nodes
        .iter()
        .filter_map(|node| pending.remove(node))
        .collect::<Vec<_>>()
        .into_boxed_slice();
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
    let input_slots = inputs
        .iter()
        .map(|input| input.artifact_slot)
        .collect::<BTreeSet<_>>();
    let mut consumers = BTreeMap::<CellSlotId, Vec<(NodeId, usize)>>::new();
    for node in artifact.nodes() {
        for (ordinal, source) in node_inputs(artifact, node.node)?.into_iter().enumerate() {
            if let ArtifactSource::Slot(slot) = source {
                consumers
                    .entry(slot)
                    .or_default()
                    .push((node.node, ordinal));
            }
        }
    }
    let mut published = artifact
        .outputs()
        .iter()
        .map(|output| output.source)
        .collect::<BTreeSet<_>>();
    published.extend(artifact.outputs().iter().filter_map(|output| {
        match artifact.slots()[output.source.get() as usize].producer {
            ProducerReference::Output {
                source: ArtifactSource::Slot(source),
                ..
            } => Some(source),
            ProducerReference::Input(_)
            | ProducerReference::NodeOutput { .. }
            | ProducerReference::Output {
                source: ArtifactSource::Constant(_),
                ..
            } => None,
        }
    }));
    published.extend(
        artifact
            .constraints()
            .iter()
            .flat_map(|constraint| constraint.inputs.iter().copied())
            .map(|source| forwarded_output_source(artifact, source))
            .filter_map(|source| match source {
                Ok(ArtifactSource::Slot(slot)) => Some(Ok(slot)),
                Ok(ArtifactSource::Constant(_)) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<BTreeSet<_>, ResidentActivationError>>()?,
    );
    let output_materialization_source =
        |output_slot: CellSlotId| -> Result<ArtifactSource, ResidentActivationError> {
            let mut source = ArtifactSource::Slot(output_slot);
            let mut remaining = artifact.slots().len();
            while let ArtifactSource::Slot(slot) = source {
                let declaration = &artifact.slots()[slot.get() as usize];
                let ProducerReference::Output { source: next, .. } = declaration.producer else {
                    break;
                };
                if remaining == 0 {
                    return Err(ResidentActivationError::RegionSizeOverflow);
                }
                remaining -= 1;
                source = next;
            }
            for state in artifact
                .slots()
                .iter()
                .filter(|slot| slot.role == SlotRole::State)
            {
                let ProducerReference::NodeOutput { node, .. } = state.producer else {
                    continue;
                };
                let Some(operation) = artifact.nodes()[node.get() as usize].as_operation() else {
                    continue;
                };
                if operation.operation.module_path.as_ref() != ["core"]
                    || operation.operation.operation_name != "assign"
                {
                    continue;
                }
                if node_inputs(artifact, node)?.first().copied() == Some(source) {
                    return Ok(ArtifactSource::Slot(state.slot));
                }
            }
            Ok(source)
        };
    published.extend(
        artifact
            .outputs()
            .iter()
            .map(|output| output_materialization_source(output.source))
            .filter_map(|source| match source {
                Ok(ArtifactSource::Slot(slot)) => Some(Ok(slot)),
                Ok(ArtifactSource::Constant(_)) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<BTreeSet<_>, ResidentActivationError>>()?,
    );
    let output_sources = artifact
        .outputs()
        .iter()
        .map(|output| output_materialization_source(output.source))
        .filter_map(|source| match source {
            Ok(ArtifactSource::Slot(slot)) => Some(Ok(slot)),
            Ok(ArtifactSource::Constant(_)) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<BTreeSet<_>, ResidentActivationError>>()?;
    let activation_sample_edge = |node: NodeId, ordinal: usize| {
        matches!(
            &artifact.nodes()[node.get() as usize].body,
            crate::ExecutableNodeBody::Activation(control)
                if ordinal != control.scrutinee as usize
        )
    };
    let node_is_pure = |node: &crate::artifact::NodeDeclaration| match &node.body {
        crate::ExecutableNodeBody::Operation(operation) => matches!(
            artifact.contracts().get(operation.contract),
            Some(mech_core::ResolvedOperationContract::Declared(contract))
                if contract.interaction == ExternalInteraction::Pure
        ),
        crate::ExecutableNodeBody::Match(_) | crate::ExecutableNodeBody::Comprehension(_) => true,
        crate::ExecutableNodeBody::Activation(_) | crate::ExecutableNodeBody::Fsm(_) => false,
    };
    // Pure nodes used only to compute an activation capture belong to that
    // capture's sampled dependency cone. Host updates may refresh their input
    // snapshots, but only the activation scrutinee schedules their execution.
    let mut sampled_nodes = BTreeMap::<NodeId, BTreeSet<NodeId>>::new();
    loop {
        let before = sampled_nodes.len();
        for node in artifact.nodes().iter().rev() {
            if sampled_nodes.contains_key(&node.node) {
                continue;
            }
            if !node_is_pure(node) {
                continue;
            }
            let output = node_output_slot(artifact, node.node)?;
            if published.contains(&output)
                || artifact.slots()[output.get() as usize].role != SlotRole::Derived
            {
                continue;
            }
            let uses = consumers.get(&output).map(Vec::as_slice).unwrap_or(&[]);
            if !uses.is_empty()
                && uses.iter().all(|(consumer, ordinal)| {
                    activation_sample_edge(*consumer, *ordinal)
                        || sampled_nodes.contains_key(consumer)
                })
            {
                let owners = uses
                    .iter()
                    .flat_map(|(consumer, ordinal)| {
                        if activation_sample_edge(*consumer, *ordinal) {
                            vec![*consumer]
                        } else {
                            sampled_nodes
                                .get(consumer)
                                .into_iter()
                                .flat_map(|owners| owners.iter().copied())
                                .collect()
                        }
                    })
                    .collect();
                sampled_nodes.insert(node.node, owners);
            }
        }
        if sampled_nodes.len() == before {
            break;
        }
    }
    for node in artifact.nodes() {
        if node_is_pure(node) || sampled_nodes.contains_key(&node.node) {
            continue;
        }
        // Outputless effects are scheduling roots, not value producers. They
        // cannot form a capture-only derived-value cone.
        if node.output_bindings.is_empty() {
            continue;
        }
        let output = node_output_slot(artifact, node.node)?;
        if published.contains(&output)
            || artifact.slots()[output.get() as usize].role != SlotRole::Derived
        {
            continue;
        }
        let uses = consumers.get(&output).map(Vec::as_slice).unwrap_or(&[]);
        if !uses.is_empty()
            && uses.iter().all(|(consumer, ordinal)| {
                activation_sample_edge(*consumer, *ordinal) || sampled_nodes.contains_key(consumer)
            })
        {
            return Err(ResidentActivationError::InvalidDependency { node: node.node });
        }
    }
    let turn_trigger_inputs = inputs
        .iter()
        .filter(|input| {
            if published.contains(&input.artifact_slot) {
                return true;
            }
            let uses = consumers
                .get(&input.artifact_slot)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            uses.is_empty()
                || !uses.iter().all(|(consumer, ordinal)| {
                    activation_sample_edge(*consumer, *ordinal)
                        || sampled_nodes.contains_key(consumer)
                })
        })
        .map(|input| input.artifact_slot)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let mut activation_turn_inputs = Vec::new();
    let mut activation_update_owners = BTreeMap::<u32, NodeId>::new();
    for node in artifact.nodes() {
        let crate::ExecutableNodeBody::Activation(control) = &node.body else {
            continue;
        };
        let activation_inputs = node_inputs(artifact, node.node)?
            .into_iter()
            .map(|source| forwarded_output_source(artifact, source))
            .collect::<Result<Vec<_>, _>>()?;
        for source in activation_inputs.iter().copied() {
            if let Some(node) =
                continuation_dependency_node(artifact, source, &mut BTreeSet::new())?
            {
                return Err(ResidentActivationError::InvalidDependency { node });
            }
        }
        let source = *activation_inputs
            .get(control.scrutinee as usize)
            .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
        let mut dependencies = BTreeSet::new();
        collect_resident_input_dependencies(
            artifact,
            source,
            &input_slots,
            &mut BTreeSet::new(),
            &mut dependencies,
        )?;
        let activated = artifact_to_activated[node.node.get() as usize]
            .ok_or(ResidentActivationError::InvalidDependency { node: node.node })?;
        let is_state_writer = |candidate: ActivatedNodeIndex| {
            let artifact_node = steps[candidate.get() as usize].artifact_node();
            node_output_slot(artifact, artifact_node)
                .is_ok_and(|slot| artifact.slots()[slot.get() as usize].role == SlotRole::State)
        };
        // Follow every direct activation path, but stop expanding at retained
        // state. A descendant can also be reachable along a second direct
        // path; tracking reachability rather than writer ancestry preserves
        // that path when the graph converges again at an effect or output.
        let mut pre_state = vec![false; steps.len()];
        pre_state[activated.get() as usize] = true;
        let mut pending = VecDeque::from([activated]);
        while let Some(parent) = pending.pop_front() {
            for child in topology.same_turn_downstream(parent).iter().copied() {
                let index = child.get() as usize;
                if pre_state[index] {
                    continue;
                }
                pre_state[index] = true;
                if !is_state_writer(child) {
                    pending.push_back(child);
                }
            }
        }
        // State publication, integrity checks, effects, and output
        // materialization are suppression boundaries. Walk backward only
        // through the pre-state subgraph so ordinary consumers of published
        // state remain eligible on unrelated turns.
        let mut protected = vec![false; steps.len()];
        for candidate in topology.linear_node_order.iter().copied() {
            let index = candidate.get() as usize;
            if !pre_state[index] {
                continue;
            }
            let artifact_node = steps[index].artifact_node();
            let constrained = topology
                .mandatory_candidate_mask
                .get(index / 64)
                .is_some_and(|word| word & (1_u64 << (index % 64)) != 0);
            let effect = matches!(steps[index], ActivatedTurnStep::External(_));
            let output = node_output_slot(artifact, artifact_node)
                .is_ok_and(|slot| output_sources.contains(&slot));
            protected[index] = is_state_writer(candidate) || constrained || effect || output;
        }
        for candidate in topology.linear_node_order.iter().rev().copied() {
            let index = candidate.get() as usize;
            if !pre_state[index] || protected[index] {
                continue;
            }
            protected[index] = topology
                .same_turn_downstream(candidate)
                .iter()
                .any(|child| protected[child.get() as usize]);
        }
        let update_nodes = topology
            .linear_node_order
            .iter()
            .copied()
            .filter(|candidate| protected[candidate.get() as usize])
            .collect::<Vec<_>>();
        for update in &update_nodes {
            if activation_update_owners
                .insert(update.get(), node.node)
                .is_some_and(|owner| owner != node.node)
            {
                return Err(ResidentActivationError::InvalidDependency { node: node.node });
            }
        }
        activation_turn_inputs.push((
            activated,
            dependencies
                .into_iter()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            sampled_nodes
                .iter()
                .filter(|(_, owners)| owners.contains(&node.node))
                .filter_map(|(sampled, _)| artifact_to_activated[sampled.get() as usize])
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            update_nodes.into_boxed_slice(),
        ));
    }
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
        .filter_map(|slot| match slot.role {
            SlotRole::Output => Some(output_materialization_source(slot.slot).and_then(|source| {
                let producer = match source {
                    ArtifactSource::Slot(source)
                        if artifact.slots()[source.get() as usize].role != SlotRole::State =>
                    {
                        match artifact.slots()[source.get() as usize].producer {
                            ProducerReference::NodeOutput { node, .. } => {
                                artifact_to_activated[node.get() as usize]
                            }
                            ProducerReference::Input(_) | ProducerReference::Output { .. } => None,
                        }
                    }
                    ArtifactSource::Slot(_) | ArtifactSource::Constant(_) => None,
                };
                resolve_read(&layout, source).map(|source| ActivatedOutputMaterialization {
                    target: slot.slot,
                    source,
                    producer,
                })
            })),
            SlotRole::Input | SlotRole::State | SlotRole::Derived => None,
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
            let predicate_source = match constraint.inputs[0] {
                ArtifactSource::Slot(slot)
                    if matches!(
                        artifact.slots()[slot.get() as usize].producer,
                        ProducerReference::Output { .. }
                    ) =>
                {
                    output_materialization_source(slot)?
                }
                source => source,
            };
            let predicate = resolve_read(&layout, predicate_source)?;
            let predicate_layout =
                source_port_layout(artifact, &layout, predicate_source, static_selectors)?;
            if predicate_layout.kind != ResidentValueKind::Bool
                || predicate_layout.shape != ResidentShape::SCALAR
            {
                return Err(ResidentActivationError::InvalidConstraint {
                    constraint: constraint.constraint,
                });
            }
            Ok(ActivatedConstraint {
                artifact_id: constraint.constraint,
                predicate,
                producer: match predicate_source {
                    ArtifactSource::Slot(slot)
                        if artifact.slots()[slot.get() as usize].role != SlotRole::State =>
                    {
                        match artifact.slots()[slot.get() as usize].producer {
                            ProducerReference::NodeOutput { node, .. } => {
                                artifact_to_activated[node.get() as usize]
                            }
                            ProducerReference::Input(_) | ProducerReference::Output { .. } => None,
                        }
                    }
                    ArtifactSource::Slot(_) | ArtifactSource::Constant(_) => None,
                },
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
    let f64_read_tape = build_f64_read_tape(&reads);
    let external_step_count = steps
        .iter()
        .filter(|step| matches!(step, ActivatedTurnStep::External(_)))
        .count();
    let pure_kernel_steps = steps
        .iter()
        .all(|step| matches!(step, ActivatedTurnStep::Kernel(_)))
        .then(|| {
            steps
                .iter()
                .map(|step| match step {
                    ActivatedTurnStep::Kernel(node) => node.clone(),
                    ActivatedTurnStep::External(_)
                    | ActivatedTurnStep::Match(_)
                    | ActivatedTurnStep::Recur(_)
                    | ActivatedTurnStep::Suspend(_)
                    | ActivatedTurnStep::Publish(_)
                    | ActivatedTurnStep::Comprehension(_) => {
                        unreachable!("pure resident plan contains a non-kernel step")
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice()
        });
    let mut plans_by_node = call_memory_plan_nodes
        .into_iter()
        .zip(call_memory_plans)
        .collect::<BTreeMap<_, _>>();
    let call_plans = scheduled_nodes
        .iter()
        .map(|node| plans_by_node.remove(node))
        .collect::<Vec<_>>();
    let call_bindings = call_plans
        .iter()
        .map(|plan| plan.as_ref().map(|plan| plan.bound_call.clone()))
        .collect::<Vec<_>>();
    let mut call_template =
        plan_program_memory_template(artifact, &scheduled_nodes, &call_bindings, &call_plans)
            .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
    let mut local_nodes = call_template.call_nodes.into_vec();
    let mut local_sites = call_template.call_sites.into_vec();
    let mut local_calls = call_template.calls.into_vec();
    for (owner, site, call) in control_calls {
        call_template
            .node_positions
            .insert(site.node, positions[&owner]);
        local_nodes.push(site.node);
        local_sites.push(site);
        local_calls.push(call);
    }
    // Call lookup uses binary search over physical node identities. Control
    // materialization can precede its lexical kernels numerically even when
    // it is bound after them, so retain the shared plan's canonical order.
    let mut entries = local_nodes
        .into_iter()
        .zip(local_sites)
        .zip(local_calls)
        .collect::<Vec<_>>();
    entries.sort_by_key(|((node, _), _)| *node);
    let (sites, calls): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
    let (nodes, sites): (Vec<_>, Vec<_>) = sites.into_iter().unzip();
    call_template.call_nodes = nodes.into_boxed_slice();
    call_template.call_sites = sites.into_boxed_slice();
    call_template.calls = calls.into_boxed_slice();
    attach_resident_call_memory_template(&mut layout.memory_plan, &call_template)
        .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
    ensure_resident_plan_admitted(&layout.memory_plan)?;
    let structural_match = steps.iter().find_map(|step| match step {
        ActivatedTurnStep::Match(control) => control
            .arms
            .iter()
            .any(|arm| matches!(arm.pattern, ActivatedMatchPattern::Structural { .. }))
            .then_some(control.artifact_node),
        ActivatedTurnStep::Comprehension(control)
            if comprehension::uses_structural_patterns(control, artifact.schemas()) =>
        {
            Some(control.artifact_node)
        }
        ActivatedTurnStep::Comprehension(_)
        | ActivatedTurnStep::Kernel(_)
        | ActivatedTurnStep::Recur(_)
        | ActivatedTurnStep::Suspend(_)
        | ActivatedTurnStep::Publish(_)
        | ActivatedTurnStep::External(_) => None,
    });
    let (schemas, structural_projections) = if let Some(node) = structural_match {
        execution::structural_projection_schema_context(artifact.schemas())
            .map_err(|_| ResidentActivationError::UnsupportedControlLayout { node })?
    } else {
        (
            artifact.schemas().clone(),
            execution::StructuralProjectionTable::default(),
        )
    };
    let plan = ActivatedPlan {
        program_revision: artifact.revision(),
        activation_facts_fingerprint,
        plan_generation: PlanGeneration::ZERO,
        layout_generation: LayoutGeneration::ZERO,
        memory_plan: layout.memory_plan,
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
        turn_trigger_inputs,
        activation_turn_inputs: activation_turn_inputs.into_boxed_slice(),
        outputs,
        output_materializations,
        constraints,
        activation_nodes,
        activation_steps,
        schemas: std::sync::Arc::new(schemas),
        structural_projections,
        constant_regions: layout.constant_regions,
        state_slots,
        rmw_state_slots,
        state_hash_seed,
    };
    Ok(plan)
}

fn resident_call_memory_plan(
    bound_call: &BoundCall,
    inputs: &[ResidentPortLayout],
    output: &ResidentPortLayout,
    output_contract: &mech_core::ResolvedOutputPort,
    implementation_memory: ImplementationMemoryClass,
) -> Result<mech_core::CallMemoryPlan, ResidentActivationError> {
    let target = TargetMemoryProfile::current_resident_cpu()
        .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
    let lifetime = MemoryLifetime::Turn {
        first: MemoryPlanPoint::new(0),
        last: MemoryPlanPoint::new(1),
    };
    let input_storage = inputs
        .iter()
        .zip(bound_call.inputs())
        .map(|(layout, descriptor)| resident_storage_descriptor(descriptor, layout.kind, lifetime))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
    let output_storage = vec![
        resident_storage_descriptor(&bound_call.outputs()[0], output.kind, lifetime)
            .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?,
    ];
    let input_witnesses = inputs
        .iter()
        .map(resident_port_memory_witness)
        .collect::<Result<Vec<_>, _>>()?;
    let output_witnesses = vec![resident_port_memory_witness(output)?];
    let regions = vec![match &output_contract.construction {
        OutputConstruction::ReadModifyWrite { regions, .. } => RegionAccessPlan::Deferred(*regions),
        _ => RegionAccessPlan::WholeValue,
    }];
    plan_call_memory(CallMemoryPlanningRequest {
        bound_call,
        input_storage: &input_storage,
        output_storage: &output_storage,
        input_witnesses: &input_witnesses,
        output_witnesses: &output_witnesses,
        published_output_witnesses: &output_witnesses,
        implementation_memory,
        target: &target,
        regions: &regions,
    })
    .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })
}

fn resident_port_memory_witness(
    layout: &ResidentPortLayout,
) -> Result<MemoryFootprintWitness, ResidentActivationError> {
    if matches!(
        layout.kind,
        ResidentValueKind::String | ResidentValueKind::Snapshot
    ) {
        return Ok(MemoryFootprintWitness::Deferred(
            mech_core::MemoryWitnessStage::Turn,
        ));
    }
    let logical_elements = u64::try_from(
        layout
            .shape
            .len()
            .ok_or(ResidentActivationError::RegionSizeOverflow)?,
    )
    .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
    Ok(MemoryFootprintWitness::Known(CurrentMemoryFootprint {
        logical_elements,
        shape_parameter_count: u64::try_from(layout.shape_instance.parameter_values().len())
            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?,
        ..CurrentMemoryFootprint::default()
    }))
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
                ResidentReadLocation::LexicalInput(_) => return None,
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

fn execute_activation_kernel(
    step: &ActivatedOnceNode,
    plan: &ActivatedPlan,
    arena: &mut TypedResidentArena,
    transient_budget: Option<&std::rc::Rc<super::budget::payload::ResidentPayloadOwner>>,
) -> Result<(), ResidentActivationError> {
    let ActivatedOnceBody::Kernel(kernel) = &step.body else {
        unreachable!("control activation uses the shared resident dispatcher")
    };
    if step.storage != ResidentStorageClass::Constant {
        return Err(ResidentActivationError::InvalidDependency {
            node: step.artifact_node,
        });
    }
    let call = plan.memory_plan.call_for_node(step.artifact_node).ok_or(
        ResidentActivationError::ActivationKernel {
            node: step.artifact_node,
        },
    )?;
    let scope = arena
        .prepare_payload_write(step.write)
        .and_then(|scope| {
            if scope.is_some() {
                Ok(scope)
            } else {
                transient_budget
                    .map(|owner| owner.begin(step.write))
                    .transpose()
            }
        })
        .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    if let Some(scope) = &scope {
        let inputs = u64::try_from(step.sources.len())
            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
        let borrowed = inputs
            .checked_mul(core::mem::size_of::<ResidentValueRef<'_>>() as u64)
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        let owned = inputs
            .checked_mul(core::mem::size_of::<OwnedResidentValue>() as u64)
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        scope
            .admit_auxiliary(
                borrowed
                    .checked_add(owned)
                    .ok_or(ResidentActivationError::RegionSizeOverflow)?,
            )
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    }
    let borrowed = step
        .sources
        .iter()
        .map(|source| {
            let region = match source {
                ArtifactSource::Constant(constant) => {
                    plan.constant_regions[constant.get() as usize]
                }
                ArtifactSource::Slot(slot) => plan.slots[slot.get() as usize].region,
            };
            arena.read(region)
        })
        .collect::<Vec<_>>();
    let facts = live::facts(
        call,
        step.artifact_node,
        &borrowed,
        arena.read(step.write),
        &plan.schemas,
    )
    .map_err(|error| ResidentActivationError::ResidentMemoryPlanRejected { error })?;
    let turn_plan = crate::memory_planner::plan_current_resident_turn(
        &plan.memory_plan,
        step.artifact_node,
        &facts,
    )
    .map_err(|_| ResidentActivationError::ActivationKernel {
        node: step.artifact_node,
    })?;
    if !turn_plan.budget_violations.is_empty() {
        return Err(ResidentActivationError::ActivationKernel {
            node: step.artifact_node,
        });
    }
    if let Some(scope) = &scope {
        // The activation ABI owns its input vectors. Their copies precede
        // the kernel's own concrete admission, so reserve them explicitly.
        let mut auxiliary = 0_u64;
        if step.write.kind == ResidentValueKind::Snapshot
            || borrowed
                .iter()
                .any(|input| input.kind() == ResidentValueKind::Snapshot)
        {
            auxiliary = plan
                .schemas
                .clone_allocation_bound_bytes()
                .and_then(|bytes| bytes.checked_mul(2))
                .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        }
        for input in &borrowed {
            let bytes = super::budget::payload::clone_bytes(*input)
                .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
            auxiliary = auxiliary
                .checked_add(bytes)
                .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        }
        scope
            .admit_auxiliary(auxiliary)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
        if let Some(base) = step.base_input {
            scope
                .admit_copy(borrowed[base], 0)
                .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
        }
        scope.start();
    }
    let admission = scope.as_ref().map(|scope| scope.admission());
    let result = super::budget::with_payload_admission(admission, || {
        super::budget::with_resident_turn_plan(turn_plan, || {
            let inputs = step
                .sources
                .iter()
                .map(|source| owned_activation_input(plan, arena, *source))
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(base_input) = step.base_input {
                let source =
                    inputs
                        .get(base_input)
                        .ok_or(ResidentActivationError::InvalidDependency {
                            node: step.artifact_node,
                        })?;
                copy_owned_activation_value(source, arena.write(step.write)).map_err(|_| {
                    ResidentActivationError::ActivationKernel {
                        node: step.artifact_node,
                    }
                })?;
            }
            kernel
                .execute(
                    &OwnedActivationInputs {
                        values: &inputs,
                        omitted: step.base_input,
                    },
                    arena.write(step.write),
                )
                .map_err(|_| ResidentActivationError::ActivationKernel {
                    node: step.artifact_node,
                })
        })
    });
    let admission_error = scope.as_ref().and_then(|scope| scope.last_error());
    if result.is_err() || admission_error.is_some() {
        arena
            .abort_payload_write(step.write, scope)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    } else {
        arena
            .finish_payload_write(step.write, scope)
            .map_err(|error| ResidentActivationError::MemoryRuntime { error })?;
    }
    if let Some(error) = admission_error {
        return Err(ResidentActivationError::MemoryRuntime { error });
    }
    result?;
    Ok(())
}

fn order_activation_nodes(
    artifact: &ProgramArtifact,
    classes: &[NodeClass],
) -> Result<Box<[NodeId]>, ResidentActivationError> {
    let nodes = artifact
        .nodes()
        .iter()
        .filter(|node| classes[node.node.get() as usize] == NodeClass::Activation)
        .map(|node| node.node)
        .collect::<Vec<_>>();
    let by_node = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (*node, index))
        .collect::<BTreeMap<_, _>>();
    let mut downstream = vec![Vec::<usize>::new(); nodes.len()];
    for (current, &node) in nodes.iter().enumerate() {
        for source in node_inputs(artifact, node)? {
            let ArtifactSource::Slot(slot) = source else {
                continue;
            };
            let ProducerReference::NodeOutput { node: parent, .. } =
                artifact.slots()[slot.get() as usize].producer
            else {
                continue;
            };
            let parent = *by_node
                .get(&parent)
                .ok_or(ResidentActivationError::InvalidDependency { node })?;
            if !downstream[parent].contains(&current) {
                downstream[parent].push(current);
            }
        }
    }
    let keys = nodes.iter().map(|node| node.get()).collect::<Vec<_>>();
    let order = stable_topological_order(&downstream, &keys).ok_or_else(|| {
        ResidentActivationError::InvalidDependency {
            node: nodes.first().copied().unwrap_or(NodeId::new(0)),
        }
    })?;
    Ok(order
        .into_iter()
        .map(|index| nodes[index])
        .collect::<Vec<_>>()
        .into_boxed_slice())
}

struct ActivationSchedule {
    nodes: Box<[NodeId]>,
    positions: BTreeMap<NodeId, u32>,
    artifact_to_activated: Box<[Option<ActivatedNodeIndex>]>,
    topology: DependencyTopology,
}

fn build_activation_schedule(
    artifact: &ProgramArtifact,
    classes: &[NodeClass],
) -> Result<ActivationSchedule, ResidentActivationError> {
    let activation = order_activation_nodes(artifact, classes)?;
    let turn = artifact
        .nodes()
        .iter()
        .filter(|node| {
            matches!(
                classes[node.node.get() as usize],
                NodeClass::Turn | NodeClass::External
            )
        })
        .map(|node| node.node)
        .collect::<Vec<_>>();
    let mut artifact_to_activated = vec![None; artifact.nodes().len()].into_boxed_slice();
    for (index, node) in turn.iter().enumerate() {
        artifact_to_activated[node.get() as usize] = Some(ActivatedNodeIndex(index as u32));
    }
    let topology = build_topology(artifact, classes, &turn, &artifact_to_activated)?;
    // Published observations precede activation; turn order is the executor's
    // existing graph order, including its state-writer and external edges.
    let nodes = artifact
        .nodes()
        .iter()
        .filter(|node| classes[node.node.get() as usize] == NodeClass::Observation)
        .map(|node| node.node)
        .chain(activation.iter().copied())
        .chain(
            topology
                .linear_node_order
                .iter()
                .map(|index| turn[index.get() as usize]),
        )
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let positions = nodes
        .iter()
        .copied()
        .enumerate()
        .map(|(position, node)| {
            u32::try_from(position)
                .map(|position| (node, position))
                .map_err(|_| ResidentActivationError::RegionSizeOverflow)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(ActivationSchedule {
        nodes,
        positions,
        artifact_to_activated,
        topology,
    })
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

#[derive(Clone, Debug)]
struct ResidentContinuation {
    state: OwnedResidentValue,
    captures: Box<[(ResidentReadLocation, OwnedResidentValue)]>,
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

struct OwnedActivationInputs<'a> {
    values: &'a [OwnedResidentValue],
    omitted: Option<usize>,
}

impl ResidentKernelInputs for OwnedActivationInputs<'_> {
    fn len(&self) -> usize {
        self.values.len() - usize::from(self.omitted.is_some())
    }

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        let index = self
            .omitted
            .filter(|omitted| index >= *omitted)
            .map_or(index, |_| index + 1);
        self.values.get(index).map(OwnedResidentValue::as_ref)
    }
}

fn copy_owned_activation_value(
    source: &OwnedResidentValue,
    output: ResidentValueMut<'_>,
) -> Result<(), ()> {
    match (source, output) {
        (OwnedResidentValue::Bool(source), ResidentValueMut::Bool(output))
            if source.len() == output.len() =>
        {
            output.copy_from_slice(source);
        }
        (OwnedResidentValue::Index(source), ResidentValueMut::Index(output))
            if source.len() == output.len() =>
        {
            output.copy_from_slice(source);
        }
        (OwnedResidentValue::F64(source), ResidentValueMut::F64(output))
            if source.len() == output.len() =>
        {
            output.copy_from_slice(source);
        }
        (OwnedResidentValue::String(source), ResidentValueMut::String(output))
            if source.len() == output.len() =>
        {
            for (output, source) in output.iter_mut().zip(source) {
                *output = source.clone();
            }
        }
        (OwnedResidentValue::Snapshot(source), ResidentValueMut::Snapshot(output))
            if source.len() == output.len() =>
        {
            output.clone_from_slice(source);
        }
        _ => return Err(()),
    }
    Ok(())
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
    nodes: &[NodeId],
    artifact_to_activated: &[Option<ActivatedNodeIndex>],
) -> Result<DependencyTopology, ResidentActivationError> {
    let mut downstream = vec![Vec::<ActivatedNodeIndex>::new(); nodes.len()];
    let mut sample_ordering = vec![Vec::<ActivatedNodeIndex>::new(); nodes.len()];
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
        let activation_scrutinee = match &node.body {
            crate::ExecutableNodeBody::Activation(control) => Some(control.scrutinee as usize),
            _ => None,
        };
        for (ordinal, source) in node_inputs(artifact, node.node)?.into_iter().enumerate() {
            if activation_scrutinee.is_some_and(|scrutinee| ordinal != scrutinee) {
                // Sample captures are read by the activation body, but they do
                // not schedule it or form dirty-propagation edges into it.
                // A computed sample must nevertheless run before its owner
                // when both are selected for the same turn.
                if let ArtifactSource::Slot(slot_id) = source {
                    let slot = &artifact.slots()[slot_id.get() as usize];
                    if slot.role != SlotRole::State {
                        if let ProducerReference::NodeOutput { node: parent, .. } = slot.producer {
                            if let Some(parent) = artifact_to_activated[parent.get() as usize] {
                                if !sample_ordering[parent.get() as usize].contains(&current) {
                                    sample_ordering[parent.get() as usize].push(current);
                                }
                            }
                        }
                    }
                }
                continue;
            }
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
    let mut indegree = vec![0_usize; nodes.len()];
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
        .zip(&sample_ordering)
        .map(|(children, samples)| {
            children
                .iter()
                .chain(samples)
                .map(|child| child.get() as usize)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let keys = nodes.iter().map(|node| node.get()).collect::<Vec<_>>();
    let linear_node_order = stable_topological_order(&order_source, &keys)
        .ok_or_else(|| ResidentActivationError::InvalidDependency {
            node: nodes.first().copied().unwrap_or(NodeId::new(0)),
        })?
        .into_iter()
        .map(|index| ActivatedNodeIndex(index as u32))
        .collect::<Vec<_>>();
    let words = nodes.len().div_ceil(64);
    let mut masks = Vec::with_capacity(nodes.len());
    for children in &downstream {
        let mut mask = vec![0_u64; words].into_boxed_slice();
        for child in children {
            set_bit(&mut mask, child.get() as usize);
        }
        masks.push(mask);
    }
    let mut dependency_masks = masks.clone();
    for node in linear_node_order.iter().rev() {
        let index = node.get() as usize;
        for child in &downstream[index] {
            let child = child.get() as usize;
            let (target, source) = if index < child {
                let (left, right) = dependency_masks.split_at_mut(child);
                (&mut left[index], &right[0])
            } else {
                let (left, right) = dependency_masks.split_at_mut(index);
                (&mut right[0], &left[child])
            };
            for (target, source) in target.iter_mut().zip(source.iter()) {
                *target |= *source;
            }
        }
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
    let mut offsets = Vec::with_capacity(nodes.len() + 1);
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
        same_turn_dependency_masks: dependency_masks.into_boxed_slice(),
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
    static_selectors: &mut ArtifactStaticSelectorResolver,
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
                activation_fixed_shape: true,
                resolved_selector: resident_resolved_selector(value),
            })
        }
        ArtifactSource::Slot(slot) => {
            let mut port = slot_port_layout(&layout.slots[slot.get() as usize]);
            port.resolved_selector = static_selectors.resolve(artifact, source)?;
            Ok(port)
        }
    }
}

fn slot_port_layout(slot: &ResolvedSlot) -> ResidentPortLayout {
    ResidentPortLayout {
        schema_id: slot.schema,
        schema_key: slot.schema_key,
        kind: slot.region.kind,
        shape: slot.region.shape,
        shape_instance: slot.shape.clone(),
        activation_fixed_shape: slot.activation_fixed_shape,
        resolved_selector: None,
    }
}

fn resident_resolved_selector(
    value: &mech_core::Value,
) -> Option<mech_core::ResidentResolvedSelector> {
    use mech_core::ResidentResolvedSelector;

    let one_based = match value.data() {
        mech_core::ValueData::Id(value) => return Some(ResidentResolvedSelector::Id(*value)),
        value => mech_core::canonical_positional_ordinal(value).ok()?,
    };
    let ordinal = usize::try_from(one_based.checked_sub(1)?).ok()?;
    Some(ResidentResolvedSelector::Ordinal(ordinal))
}

/// Resolves immutable scalar selector conversions embedded in the artifact.
/// Only canonical `access/index` conversion nodes are followed. Resolution is
/// iterative, bounded across the complete activation plan, and path-cached so
/// a long producer chain cannot consume quadratic work or the process stack.
struct ArtifactStaticSelectorResolver {
    slots: Vec<Option<Option<mech_core::ResidentResolvedSelector>>>,
    remaining_steps: usize,
}

impl ArtifactStaticSelectorResolver {
    fn new(artifact: &ProgramArtifact) -> Self {
        Self {
            slots: vec![None; artifact.slots().len()],
            remaining_steps: MAX_STATIC_SELECTOR_SOURCE_STEPS,
        }
    }

    fn resolve(
        &mut self,
        artifact: &ProgramArtifact,
        source: ArtifactSource,
    ) -> Result<Option<mech_core::ResidentResolvedSelector>, ResidentActivationError> {
        let mut source = source;
        let mut path = Vec::new();
        let mut visited = BTreeSet::new();
        let resolved = loop {
            match source {
                ArtifactSource::Constant(constant) => {
                    break artifact
                        .constants()
                        .get(constant)
                        .and_then(resident_resolved_selector);
                }
                ArtifactSource::Slot(slot) => {
                    let index = slot.get() as usize;
                    let Some(cached) = self.slots.get(index) else {
                        return Err(ResidentActivationError::InvalidDependency {
                            node: NodeId::new(slot.get()),
                        });
                    };
                    if let Some(cached) = cached {
                        break *cached;
                    }
                    if !visited.insert(slot) {
                        let node = artifact
                            .slots()
                            .get(index)
                            .and_then(|slot| match slot.producer {
                                ProducerReference::NodeOutput { node, .. } => Some(node),
                                _ => None,
                            })
                            .unwrap_or_else(|| NodeId::new(slot.get()));
                        return Err(ResidentActivationError::InvalidDependency { node });
                    }
                    let Some(remaining) = self.remaining_steps.checked_sub(1) else {
                        return Err(ResidentActivationError::StaticSelectorResolutionLimit {
                            slot,
                        });
                    };
                    self.remaining_steps = remaining;
                    path.push(slot);
                    let declaration = artifact.slots().get(index).ok_or(
                        ResidentActivationError::InvalidDependency {
                            node: NodeId::new(slot.get()),
                        },
                    )?;
                    let ProducerReference::NodeOutput { node, .. } = declaration.producer else {
                        break None;
                    };
                    let declaration = artifact
                        .nodes()
                        .get(node.get() as usize)
                        .ok_or(ResidentActivationError::InvalidDependency { node })?;
                    let Some(declaration) = declaration.as_operation() else {
                        break None;
                    };
                    if declaration.operation.module_path.as_ref() != ["access"]
                        || declaration.operation.operation_name != "index"
                    {
                        break None;
                    }
                    let inputs = node_inputs(artifact, declaration.node)?;
                    let [input] = inputs.as_slice() else {
                        return Err(ResidentActivationError::InvalidDependency { node });
                    };
                    source = *input;
                }
            }
        };
        for slot in path {
            self.slots[slot.get() as usize] = Some(resolved);
        }
        Ok(resolved)
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

fn control_input_sources(
    artifact: &ProgramArtifact,
    node: NodeId,
) -> Result<Vec<ArtifactSource>, ResidentActivationError> {
    let normalize_forwarded = matches!(
        artifact
            .nodes()
            .get(node.get() as usize)
            .map(|node| &node.body),
        Some(crate::ExecutableNodeBody::Activation(_))
    );
    node_inputs(artifact, node)?
        .into_iter()
        .map(|source| {
            if normalize_forwarded {
                forwarded_output_source(artifact, source)
            } else {
                Ok(source)
            }
        })
        .collect()
}

fn forwarded_output_source(
    artifact: &ProgramArtifact,
    mut source: ArtifactSource,
) -> Result<ArtifactSource, ResidentActivationError> {
    let mut remaining = artifact.slots().len();
    while let ArtifactSource::Slot(slot) = source {
        let declaration = artifact.slots().get(slot.get() as usize).ok_or(
            ResidentActivationError::InvalidDependency {
                node: NodeId::new(slot.get()),
            },
        )?;
        let ProducerReference::Output { source: next, .. } = declaration.producer else {
            break;
        };
        let Some(next_remaining) = remaining.checked_sub(1) else {
            return Err(ResidentActivationError::InvalidDependency {
                node: NodeId::new(slot.get()),
            });
        };
        remaining = next_remaining;
        source = next;
    }
    Ok(source)
}

fn collect_resident_input_dependencies(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
    input_slots: &BTreeSet<CellSlotId>,
    visited_nodes: &mut BTreeSet<NodeId>,
    dependencies: &mut BTreeSet<CellSlotId>,
) -> Result<(), ResidentActivationError> {
    let ArtifactSource::Slot(slot) = source else {
        return Ok(());
    };
    if input_slots.contains(&slot) {
        dependencies.insert(slot);
        return Ok(());
    }
    let declaration = artifact.slots().get(slot.get() as usize).ok_or(
        ResidentActivationError::InvalidDependency {
            node: NodeId::new(slot.get()),
        },
    )?;
    if declaration.role == SlotRole::State {
        // Retained state is sampled from the previous publication. Its
        // historical writer inputs do not trigger this activation.
        return Ok(());
    }
    let node = match declaration.producer {
        ProducerReference::Output { source, .. } => {
            return collect_resident_input_dependencies(
                artifact,
                source,
                input_slots,
                visited_nodes,
                dependencies,
            );
        }
        ProducerReference::NodeOutput { node, .. } => node,
        ProducerReference::Input(_) => return Ok(()),
    };
    if !visited_nodes.insert(node) {
        return Ok(());
    }
    for input in node_inputs(artifact, node)? {
        collect_resident_input_dependencies(
            artifact,
            input,
            input_slots,
            visited_nodes,
            dependencies,
        )?;
    }
    Ok(())
}

fn activated_input_source(artifact: &ProgramArtifact, slot: CellSlotId) -> ActivatedInputSource {
    match artifact.slots()[slot.get() as usize].producer {
        ProducerReference::Input(input) => ActivatedInputSource::DeclaredInput { input },
        ProducerReference::NodeOutput { node, .. } => {
            let requirement = artifact.nodes()[node.get() as usize]
                .as_operation()
                .expect("observation is an ordinary operation")
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

#[cfg(test)]
mod shape_fact_tests {
    use super::*;
    use crate::{NodeDeclaration, ProgramArtifactDraft, SlotDeclaration};
    use mech_core::{
        BindingId, ConstantStoreBuilder, DeclaredOperationContract, DimensionParameterDeclaration,
        DimensionParameterId, DimensionParameterOrigin, FloatWidth, ManagedMemoryBudget,
        OperationContractTableBuilder, ResolvedInputPort, ResolvedOperationContract,
        ResolvedOutputPort, SchemaDraft, SchemaTableBuilder, ValueDataDraft, ValueDraft,
        snapshot::{F64Bits, SnapshotValidationContext},
    };

    #[test]
    fn structural_finalization_count_follows_activated_snapshot_storage() {
        let region = |kind| ResidentRegion {
            kind,
            offset: 0,
            len: 1,
            shape: ResidentShape::SCALAR,
        };
        let pattern = crate::CollectionPattern::Tuple(
            vec![
                crate::CollectionPattern::Bind {
                    local: 0,
                    schema: ActivatedPatternBinding {
                        region: region(ResidentValueKind::F64),
                        schema: SchemaId::new(0),
                    },
                },
                crate::CollectionPattern::Bind {
                    local: 1,
                    schema: ActivatedPatternBinding {
                        region: region(ResidentValueKind::Snapshot),
                        schema: SchemaId::new(1),
                    },
                },
                crate::CollectionPattern::Equal(ActivatedPatternValue {
                    location: ResidentReadLocation::Constant(region(ResidentValueKind::Snapshot)),
                    schema: SchemaId::new(2),
                }),
            ]
            .into_boxed_slice(),
        );
        assert_eq!(snapshot_pattern_finalization_count(&pattern), Some(2));
    }

    #[test]
    fn aborted_state_payload_is_discarded_before_epoch_evidence() {
        fn string_arena(budget: &ManagedMemoryBudget) -> TypedResidentArena {
            let mut arena = TypedResidentArena::allocate_projected_sizes(ResidentArenaSizes {
                strings: 1,
                ..ResidentArenaSizes::default()
            });
            let string_prepaid = budget.reserve_capacity(0).unwrap();
            let snapshot_prepaid = budget.reserve_capacity(0).unwrap();
            arena.payload_budget = Some(
                super::super::budget::payload::ResidentPayloadOwner::new(
                    budget.clone(),
                    string_prepaid,
                    snapshot_prepaid,
                    1,
                    0,
                )
                .unwrap(),
            );
            arena
        }

        let budget = ManagedMemoryBudget::new(1_000_000);
        let region = ResidentRegion {
            kind: ResidentValueKind::String,
            offset: 0,
            len: 1,
            shape: ResidentShape::SCALAR,
        };
        let mut state = StateArena {
            buffers: [string_arena(&budget), string_arena(&budget)],
            versions: vec![StateVersion {
                slot: CellSlotId::new(0),
                region,
                epochs: [Some(InstanceEpoch::ZERO), None],
            }]
            .into_boxed_slice(),
            version_by_slot: vec![Some(0)].into_boxed_slice(),
        };
        let baseline = budget.used_bytes();
        let working = InstanceEpoch::new(1);
        let first_value = ["failed candidate".repeat(1024)];
        let scope = state.buffers[1].prepare_payload_write(region).unwrap();
        scope
            .as_ref()
            .unwrap()
            .admit_copy(ResidentValueRef::String(&first_value), 0)
            .unwrap();
        scope.as_ref().unwrap().start();
        let ResidentValueMut::String(candidate) = state.buffers[1].write(region) else {
            unreachable!()
        };
        candidate[0] = first_value[0].clone();
        state.buffers[1]
            .finish_payload_write(region, scope)
            .unwrap();
        state.versions[0].epochs[1] = Some(working);
        assert!(budget.used_bytes() > baseline);

        state.abort_payloads(working);
        state.versions[0].epochs[1] = None;
        assert_eq!(budget.used_bytes(), baseline);
        assert!(state.buffers[1].string_storage()[0].is_empty());
        assert_eq!(
            state.epochs(CellSlotId::new(0)),
            [Some(InstanceEpoch::ZERO), None]
        );

        let retry_value = ["retry candidate".repeat(1024)];
        let retry = state.buffers[1].prepare_payload_write(region).unwrap();
        retry
            .as_ref()
            .unwrap()
            .admit_copy(ResidentValueRef::String(&retry_value), 0)
            .unwrap();
        retry.as_ref().unwrap().start();
        let ResidentValueMut::String(candidate) = state.buffers[1].write(region) else {
            unreachable!()
        };
        candidate[0] = retry_value[0].clone();
        state.buffers[1]
            .finish_payload_write(region, retry)
            .unwrap();
    }

    fn copy_chain(output_upper_bound: u64) -> ProgramArtifact {
        let mut schemas = SchemaTableBuilder::new();
        let mut matrix = |upper_bound| {
            schemas
                .insert(
                    SchemaDraft {
                        dimension_parameters: vec![DimensionParameterDeclaration {
                            id: DimensionParameterId::new(0),
                            origin: DimensionParameterOrigin::Explicit,
                            lifetime: DimensionLifetime::Turn,
                            lower_bound: DimensionExpr::Constant(1),
                            upper_bound: Some(DimensionExpr::Constant(upper_bound)),
                        }]
                        .into_boxed_slice(),
                        body: SchemaBody::Matrix {
                            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                            dimensions: vec![
                                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                                DimensionExpr::Constant(1),
                            ]
                            .into_boxed_slice(),
                        },
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap()
        };
        let input = matrix(8);
        let output = matrix(output_upper_bound);
        let build = schemas.finish().unwrap();
        let input = build.resolve(input).unwrap();
        let output = build.resolve(output).unwrap();
        let (schemas, _) = build.into_parts();
        let value = ValueDraft {
            schema: input,
            shape_values: vec![3].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::F64(F64Bits::from_f64(7.0)); 3].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let mut constants = ConstantStoreBuilder::new(&schemas);
        let value = constants.insert(value).unwrap();
        let build = constants.finish().unwrap();
        let value = build.resolve(value).unwrap();
        let (constants, _) = build.into_parts();
        let mut contracts = OperationContractTableBuilder::new();
        let mut contract = |input_schema| {
            contracts
                .insert(ResolvedOperationContract::Declared(
                    DeclaredOperationContract {
                        inputs: vec![ResolvedInputPort {
                            schema: input_schema,
                            access: AccessMode::Read,
                            delivery: DeliveryMode::Signal,
                        }]
                        .into_boxed_slice(),
                        outputs: vec![ResolvedOutputPort {
                            schema: output,
                            access: AccessMode::Write,
                            delivery: DeliveryMode::Signal,
                            construction: OutputConstruction::FullWrite {
                                shape: ShapeRule::SameAsInput { input: 0 },
                            },
                            alias: AliasPolicy::NoAlias,
                            change_detection: ChangeDetectionPolicy::KernelReported,
                        }]
                        .into_boxed_slice(),
                        interaction: ExternalInteraction::Pure,
                    },
                ))
                .unwrap()
        };
        let consumer = contract(output);
        let producer = contract(input);
        let build = contracts.finish().unwrap();
        let ids = [
            build.resolve(consumer).unwrap(),
            build.resolve(producer).unwrap(),
        ];
        let (contracts, _) = build.into_parts();
        let mut nodes = Vec::new();
        let mut slots = Vec::new();
        let mut bindings = Vec::new();
        for raw in 0..2 {
            let node = NodeId::new(raw);
            nodes.push(NodeDeclaration {
                node,
                body: crate::ExecutableNodeBody::Operation(crate::OperationNodeBody {
                    operation: OperationReference {
                        module_path: vec!["custom".to_owned()].into_boxed_slice(),
                        operation_name: "copy".to_owned(),
                    },
                    contract: ids[raw as usize],
                    requirement: None,
                }),
                input_bindings: raw * 2..raw * 2 + 1,
                output_bindings: raw * 2 + 1..raw * 2 + 2,
            });
            slots.push(SlotDeclaration {
                slot: CellSlotId::new(raw),
                schema: output,
                role: SlotRole::Derived,
                producer: ProducerReference::NodeOutput {
                    node,
                    output_ordinal: 0,
                },
                initializer: None,
            });
            bindings.push(BindingDeclaration::Input {
                id: BindingId::new(raw * 2),
                node,
                port_ordinal: 0,
                source: if raw == 0 {
                    ArtifactSource::Slot(CellSlotId::new(1))
                } else {
                    ArtifactSource::Constant(value)
                },
            });
            bindings.push(BindingDeclaration::Output {
                id: BindingId::new(raw * 2 + 1),
                node,
                port_ordinal: 0,
                target: CellSlotId::new(raw),
            });
        }
        ProgramArtifactDraft {
            schemas,
            constants,
            contracts,
            requirements: Default::default(),
            inputs: Box::new([]),
            slots: slots.into_boxed_slice(),
            nodes: nodes.into_boxed_slice(),
            bindings: bindings.into_boxed_slice(),
            outputs: Box::new([]),
            constraints: Box::new([]),
            compute_regions: Box::new([]),
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn custom_same_input_shape_follows_dependency_order_without_hints() {
        let artifact = copy_chain(8);
        let classes = [NodeClass::Activation, NodeClass::Activation];
        let schedule = build_activation_schedule(&artifact, &classes).unwrap();
        assert_eq!(schedule.nodes.as_ref(), &[NodeId::new(1), NodeId::new(0)]);
        let facts = complete_activation_shape_facts(
            &artifact,
            &ActivationFacts::default(),
            &classes,
            &schedule,
        )
        .unwrap();
        for slot in [CellSlotId::new(0), CellSlotId::new(1)] {
            assert_eq!(
                slot_shape(&artifact, slot, &facts)
                    .unwrap()
                    .parameter_values(),
                &[3]
            );
        }
    }

    #[test]
    fn custom_same_input_shape_still_rejects_output_bound_violations() {
        let artifact = copy_chain(2);
        let classes = [NodeClass::Activation, NodeClass::Activation];
        let schedule = build_activation_schedule(&artifact, &classes).unwrap();
        assert!(matches!(
            complete_activation_shape_facts(
                &artifact, &ActivationFacts::default(), &classes, &schedule,
            ),
            Err(ResidentActivationError::UnresolvedShape { slot }) if slot == CellSlotId::new(1)
        ));
    }

    fn canonical_index_artifact(input_extents: Option<[u64; 2]>) -> ProgramArtifact {
        let mut schemas = SchemaTableBuilder::new();
        let input = schemas
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: input_extents.map_or(
                        SchemaBody::FloatingPoint(FloatWidth::W64),
                        |extents| SchemaBody::Matrix {
                            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                            dimensions: extents.map(DimensionExpr::Constant).into(),
                        },
                    ),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let output = schemas
            .insert(
                SchemaDraft {
                    dimension_parameters: if input_extents.is_some() {
                        vec![DimensionParameterDeclaration {
                            id: DimensionParameterId::new(0),
                            origin: DimensionParameterOrigin::Explicit,
                            lifetime: DimensionLifetime::Turn,
                            lower_bound: DimensionExpr::Constant(0),
                            upper_bound: None,
                        }]
                        .into_boxed_slice()
                    } else {
                        Box::new([])
                    },
                    body: input_extents.map_or(SchemaBody::Index, |_| SchemaBody::Matrix {
                        element: Box::new(SchemaBody::Index),
                        dimensions: vec![
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                            DimensionExpr::Constant(1),
                        ]
                        .into_boxed_slice(),
                    }),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let build = schemas.finish().unwrap();
        let input = build.resolve(input).unwrap();
        let output = build.resolve(output).unwrap();
        let (schemas, _) = build.into_parts();
        let element = ValueDataDraft::F64(F64Bits::from_f64(1.0));
        let value = ValueDraft {
            schema: input,
            shape_values: Box::new([]),
            data: input_extents.map_or(element.clone(), |[rows, columns]| {
                ValueDataDraft::Matrix(vec![element; (rows * columns) as usize].into_boxed_slice())
            }),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap();
        let mut constants = ConstantStoreBuilder::new(&schemas);
        let value = constants.insert(value).unwrap();
        let build = constants.finish().unwrap();
        let value = build.resolve(value).unwrap();
        let (constants, _) = build.into_parts();
        let mut contracts = OperationContractTableBuilder::new();
        let contract = contracts
            .insert(ResolvedOperationContract::Declared(
                DeclaredOperationContract {
                    inputs: vec![ResolvedInputPort {
                        schema: input,
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    }]
                    .into_boxed_slice(),
                    outputs: vec![ResolvedOutputPort {
                        schema: output,
                        access: AccessMode::Write,
                        delivery: DeliveryMode::Signal,
                        construction: OutputConstruction::FullWrite {
                            shape: ShapeRule::Declared,
                        },
                        alias: AliasPolicy::NoAlias,
                        change_detection: ChangeDetectionPolicy::KernelReported,
                    }]
                    .into_boxed_slice(),
                    interaction: ExternalInteraction::Pure,
                },
            ))
            .unwrap();
        let build = contracts.finish().unwrap();
        let contract = build.resolve(contract).unwrap();
        let (contracts, _) = build.into_parts();
        let node = NodeId::new(0);
        let slot = CellSlotId::new(0);
        ProgramArtifactDraft {
            schemas,
            constants,
            contracts,
            requirements: Default::default(),
            inputs: Box::new([]),
            slots: vec![SlotDeclaration {
                slot,
                schema: output,
                role: SlotRole::Derived,
                producer: ProducerReference::NodeOutput {
                    node,
                    output_ordinal: 0,
                },
                initializer: None,
            }]
            .into_boxed_slice(),
            nodes: vec![NodeDeclaration {
                node,
                body: crate::ExecutableNodeBody::Operation(crate::OperationNodeBody {
                    operation: OperationReference {
                        module_path: vec!["access".to_owned()].into_boxed_slice(),
                        operation_name: "index".to_owned(),
                    },
                    contract,
                    requirement: None,
                }),
                input_bindings: 0..1,
                output_bindings: 1..2,
            }]
            .into_boxed_slice(),
            bindings: vec![
                BindingDeclaration::Input {
                    id: BindingId::new(0),
                    node,
                    port_ordinal: 0,
                    source: ArtifactSource::Constant(value),
                },
                BindingDeclaration::Output {
                    id: BindingId::new(1),
                    node,
                    port_ordinal: 0,
                    target: slot,
                },
            ]
            .into_boxed_slice(),
            outputs: Box::new([]),
            constraints: Box::new([]),
            compute_regions: Box::new([]),
        }
        .finalize()
        .unwrap()
    }

    #[test]
    fn canonical_index_activation_flattens_matrix_axes_without_shape_hints() {
        for input_extents in [
            None,
            Some([1, 3]),
            Some([3, 1]),
            Some([2, 3]),
            Some([1, 0]),
            Some([0, 3]),
        ] {
            let artifact = canonical_index_artifact(input_extents);
            let classes = [NodeClass::Activation];
            let schedule = build_activation_schedule(&artifact, &classes).unwrap();
            let facts = complete_activation_shape_facts(
                &artifact,
                &ActivationFacts::default(),
                &classes,
                &schedule,
            )
            .unwrap();
            let shape = slot_shape(&artifact, CellSlotId::new(0), &facts).unwrap();
            let expected =
                input_extents.map_or_else(Vec::new, |[rows, columns]| vec![rows * columns]);
            assert_eq!(
                shape.parameter_values(),
                expected,
                "input extents {input_extents:?}"
            );
            let extents =
                source_extents(&artifact, ArtifactSource::Slot(CellSlotId::new(0)), &facts)
                    .unwrap();
            let expected =
                input_extents.map_or_else(Vec::new, |[rows, columns]| vec![rows * columns, 1]);
            assert_eq!(
                extents.as_ref(),
                expected,
                "input extents {input_extents:?}"
            );
        }
    }
}

fn prepare_match_node(
    artifact: &ProgramArtifact,
    owner: NodeId,
    budget_node: NodeId,
    control: &crate::MatchDeclaration,
    inputs: &[ArtifactSource],
    input_reads: &[ResidentReadLocation],
    output_slot: CellSlotId,
    layout: &LayoutBuild,
) -> Result<ActivatedMatchNode, ResidentActivationError> {
    let scrutinee_source = inputs[control.scrutinee as usize];
    let scrutinee = input_reads[control.scrutinee as usize];
    let (scrutinee_schema, scrutinee_shape_values) = match scrutinee_source {
        ArtifactSource::Constant(constant) => {
            let value = artifact
                .constants()
                .get(constant)
                .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?;
            (
                value.schema(),
                value.shape().parameter_values().to_vec().into_boxed_slice(),
            )
        }
        ArtifactSource::Slot(slot) => {
            let resolved = &layout.slots[slot.get() as usize];
            (
                resolved.schema,
                resolved
                    .shape
                    .parameter_values()
                    .to_vec()
                    .into_boxed_slice(),
            )
        }
    };
    if control
        .arms
        .iter()
        .any(|arm| matches!(&arm.pattern, crate::MatchPattern::Literal(_)))
    {
        let region = scrutinee.region();
        let source = inputs[control.scrutinee as usize];
        let schema = match source {
            ArtifactSource::Constant(constant) => {
                artifact.constants().get(constant).unwrap().schema()
            }
            ArtifactSource::Slot(slot) => layout.slots[slot.get() as usize].schema,
        };
        let schema = artifact.schemas().get(schema).unwrap();
        let supported = matches!(
            region.kind,
            ResidentValueKind::Bool | ResidentValueKind::Index | ResidentValueKind::F64
        ) || (region.kind == ResidentValueKind::Snapshot
            && crate::is_control_scalar_schema(schema));
        if !supported || region.len != 1 {
            return Err(ResidentActivationError::UnsupportedControlLayout { node: owner });
        }
    }
    let output = &layout.slots[output_slot.get() as usize];
    Ok(ActivatedMatchNode {
        artifact_node: owner,
        budget_node,
        scrutinee,
        scrutinee_schema,
        scrutinee_shape_values,
        write: ResidentWriteLocation {
            slot: output_slot,
            storage: output.storage,
            region: output.region,
        },
        arms: Box::new([]),
        locals: comprehension::all_match_local_definitions(control)
            .into_iter()
            .map(|(pattern_binding, _, block, local, _)| {
                let slot = if pattern_binding {
                    layout.match_bindings[&(owner, block, local)].0
                } else {
                    layout.control_locals[&(owner, block, local)].0
                };
                layout.slots[slot.get() as usize].region
            })
            .collect(),
        continuation: control.contains_suspend(),
        capture_sources: control
            .captures
            .iter()
            .map(|capture| {
                let source = input_reads[capture.input as usize];
                match (capture.freeze_on_suspend, source) {
                    (true, ResidentReadLocation::Input(region)) => {
                        Ok(ResidentReadLocation::LexicalInput(region))
                    }
                    (false, ResidentReadLocation::Input(_)) | (true, _) => Ok(source),
                    // A live capture has meaning only for an external input.
                    // Scratch and state values are necessarily snapshots at
                    // suspension, so accepting a false flag would silently
                    // change the artifact's declared semantics.
                    (false, _) => {
                        Err(ResidentActivationError::UnsupportedControlLayout { node: owner })
                    }
                }
            })
            .collect::<Result<Box<[_]>, _>>()?,
    })
}

fn snapshot_pattern_finalization_count(
    pattern: &crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
) -> Option<u64> {
    let mut pending = vec![pattern];
    let mut count = 0_u64;
    while let Some(pattern) = pending.pop() {
        match pattern {
            crate::CollectionPattern::Wildcard => {}
            crate::CollectionPattern::Bind { schema, .. }
                if schema.region.kind == ResidentValueKind::Snapshot =>
            {
                count = count.checked_add(1)?;
            }
            crate::CollectionPattern::Equal(value)
                if value.location.region().kind == ResidentValueKind::Snapshot =>
            {
                count = count.checked_add(1)?;
            }
            crate::CollectionPattern::Bind { .. } | crate::CollectionPattern::Equal(_) => {}
            crate::CollectionPattern::Enum { payload, .. } => {
                pending.extend(payload.iter().map(Box::as_ref));
            }
            crate::CollectionPattern::Tuple(items) => pending.extend(items.iter()),
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => {
                pending.extend(prefix.iter());
                pending.extend(rest.iter().map(Box::as_ref));
                pending.extend(suffix.iter());
            }
        }
    }
    Some(count)
}

fn activate_match_pattern(
    artifact: &ProgramArtifact,
    owner: NodeId,
    owner_block: u32,
    pattern: &crate::MatchPattern,
    inputs: &[ArtifactSource],
    layout: &LayoutBuild,
) -> Result<ActivatedMatchPattern, ResidentActivationError> {
    fn structural(
        artifact: &ProgramArtifact,
        owner: NodeId,
        owner_block: u32,
        pattern: &crate::CollectionPattern<SchemaId, crate::MatchPatternValue>,
        inputs: &[ArtifactSource],
        layout: &LayoutBuild,
    ) -> Result<
        crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
        ResidentActivationError,
    > {
        Ok(match pattern {
            crate::CollectionPattern::Wildcard => crate::CollectionPattern::Wildcard,
            crate::CollectionPattern::Bind { local, schema } => {
                let (slot, _) = layout.match_bindings[&(owner, owner_block, *local)];
                crate::CollectionPattern::Bind {
                    local: *local,
                    schema: ActivatedPatternBinding {
                        region: layout.slots[slot.get() as usize].region,
                        schema: *schema,
                    },
                }
            }
            crate::CollectionPattern::Equal(crate::MatchPatternValue::Literal(constant)) => {
                crate::CollectionPattern::Equal(ActivatedPatternValue {
                    location: ResidentReadLocation::Constant(
                        layout.constant_regions[constant.get() as usize],
                    ),
                    schema: artifact.constants().get(*constant).unwrap().schema(),
                })
            }
            crate::CollectionPattern::Equal(crate::MatchPatternValue::Binding(local)) => {
                let (slot, _) = layout.match_bindings[&(owner, owner_block, *local)];
                let slot = &layout.slots[slot.get() as usize];
                crate::CollectionPattern::Equal(ActivatedPatternValue {
                    location: ResidentReadLocation::Scratch(slot.region),
                    schema: slot.schema,
                })
            }
            crate::CollectionPattern::Equal(crate::MatchPatternValue::Input(input)) => {
                let source = *inputs
                    .get(*input as usize)
                    .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?;
                let schema = match source {
                    ArtifactSource::Constant(constant) => artifact
                        .constants()
                        .get(constant)
                        .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?
                        .schema(),
                    ArtifactSource::Slot(slot) => layout.slots[slot.get() as usize].schema,
                };
                crate::CollectionPattern::Equal(ActivatedPatternValue {
                    location: resolve_read(layout, source)?,
                    schema,
                })
            }
            crate::CollectionPattern::Enum { ordinal, payload } => crate::CollectionPattern::Enum {
                ordinal: *ordinal,
                payload: payload
                    .as_deref()
                    .map(|item| {
                        structural(artifact, owner, owner_block, item, inputs, layout).map(Box::new)
                    })
                    .transpose()?,
            },
            crate::CollectionPattern::Tuple(items) => crate::CollectionPattern::Tuple(
                items
                    .iter()
                    .map(|item| structural(artifact, owner, owner_block, item, inputs, layout))
                    .collect::<Result<_, _>>()?,
            ),
            crate::CollectionPattern::Array {
                prefix,
                rest,
                suffix,
            } => crate::CollectionPattern::Array {
                prefix: prefix
                    .iter()
                    .map(|item| structural(artifact, owner, owner_block, item, inputs, layout))
                    .collect::<Result<_, _>>()?,
                rest: rest
                    .as_deref()
                    .map(|item| {
                        structural(artifact, owner, owner_block, item, inputs, layout).map(Box::new)
                    })
                    .transpose()?,
                suffix: suffix
                    .iter()
                    .map(|item| structural(artifact, owner, owner_block, item, inputs, layout))
                    .collect::<Result<_, _>>()?,
            },
        })
    }
    Ok(match pattern {
        crate::MatchPattern::Literal(id) => ActivatedMatchPattern::Literal(
            ResidentReadLocation::Constant(layout.constant_regions[id.get() as usize]),
        ),
        crate::MatchPattern::Wildcard => ActivatedMatchPattern::Wildcard,
        crate::MatchPattern::Bind => ActivatedMatchPattern::Bind,
        crate::MatchPattern::Structural(pattern) => {
            let metrics = crate::pattern_metrics(pattern)
                .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?;
            let pattern = structural(artifact, owner, owner_block, pattern, inputs, layout)?;
            let snapshot_finalization_count = snapshot_pattern_finalization_count(&pattern)
                .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?;
            ActivatedMatchPattern::Structural {
                pattern,
                work: u64::try_from(metrics.nodes).map_err(|_| {
                    ResidentActivationError::UnsupportedControlLayout { node: owner }
                })?,
                binding_count: u64::try_from(metrics.bindings).map_err(|_| {
                    ResidentActivationError::UnsupportedControlLayout { node: owner }
                })?,
                equality_count: u64::try_from(metrics.equalities).map_err(|_| {
                    ResidentActivationError::UnsupportedControlLayout { node: owner }
                })?,
                snapshot_finalization_count,
                clone_depth: u64::try_from(metrics.depth).map_err(|_| {
                    ResidentActivationError::UnsupportedControlLayout { node: owner }
                })?,
            }
        }
    })
}

fn bind_match_arms(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    owner: NodeId,
    control: &crate::MatchDeclaration,
    captures: &[ArtifactSource],
    capture_reads: &[ResidentReadLocation],
    layout: &LayoutBuild,
    steps: &mut Vec<ActivatedTurnStep>,
    reads: &mut Vec<ResidentReadLocation>,
    calls: &mut Vec<(
        NodeId,
        crate::memory_planner::CallSiteMemoryTemplate,
        mech_core::CallMemoryPlan,
    )>,
    recursive_roots: &[ActivatedNodeIndex],
) -> Result<Box<[ActivatedMatchArm]>, ResidentActivationError> {
    control
        .arms
        .iter()
        .map(|arm| {
            let owner_block = arm.body.id.0;
            let pattern = activate_match_pattern(
                artifact,
                owner,
                owner_block,
                &arm.pattern,
                captures,
                layout,
            )?;
            fn collect_binding_regions(
                pattern: &crate::CollectionPattern<ActivatedPatternBinding, ActivatedPatternValue>,
                bindings: &mut Vec<(u32, ResidentRegion)>,
            ) {
                match pattern {
                    crate::CollectionPattern::Wildcard | crate::CollectionPattern::Equal(_) => {}
                    crate::CollectionPattern::Bind { local, schema } => {
                        bindings.push((*local, schema.region));
                    }
                    crate::CollectionPattern::Enum { payload, .. } => {
                        if let Some(payload) = payload {
                            collect_binding_regions(payload, bindings);
                        }
                    }
                    crate::CollectionPattern::Tuple(items) => {
                        for item in items {
                            collect_binding_regions(item, bindings);
                        }
                    }
                    crate::CollectionPattern::Array {
                        prefix,
                        rest,
                        suffix,
                    } => {
                        for item in prefix {
                            collect_binding_regions(item, bindings);
                        }
                        if let Some(rest) = rest {
                            collect_binding_regions(rest, bindings);
                        }
                        for item in suffix {
                            collect_binding_regions(item, bindings);
                        }
                    }
                }
            }
            let mut bindings = Vec::new();
            if let ActivatedMatchPattern::Structural { pattern, .. } = &pattern {
                collect_binding_regions(pattern, &mut bindings);
            }
            let binding_regions = bindings
                .iter()
                .map(|(_, region)| *region)
                .collect::<Vec<_>>();
            let binding_local_indices = bindings
                .iter()
                .enumerate()
                .map(|(index, (local, _))| {
                    u32::try_from(index)
                        .map(|index| (*local, index))
                        .map_err(|_| ResidentActivationError::RegionSizeOverflow)
                })
                .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
            let mut bind = |block: &crate::ControlBlock| {
                bind_control_block(
                    artifact,
                    catalog,
                    owner,
                    owner_block,
                    control,
                    block,
                    captures,
                    capture_reads,
                    &binding_regions,
                    &binding_local_indices,
                    layout,
                    steps,
                    reads,
                    calls,
                    recursive_roots,
                )
            };
            let guard = arm.guard.as_ref().map(&mut bind).transpose()?;
            let body = bind(&arm.body)?;
            let guard_regions = arm
                .guard
                .iter()
                .flat_map(|block| {
                    block
                        .operations
                        .iter()
                        .map(move |operation| (block.id.0, operation.node))
                })
                .map(|(block, node)| {
                    let (slot, _) = layout.control_locals[&(owner, block, node)];
                    layout.slots[slot.get() as usize].region
                })
                .collect();
            Ok(ActivatedMatchArm {
                pattern,
                binding_regions: binding_regions.into_boxed_slice(),
                guard_regions,
                guard,
                body,
            })
        })
        .collect()
}

fn bind_control_block(
    artifact: &ProgramArtifact,
    catalog: &FunctionCatalog,
    owner: NodeId,
    pattern_owner: u32,
    control: &crate::MatchDeclaration,
    block: &crate::ControlBlock,
    captures: &[ArtifactSource],
    capture_reads: &[ResidentReadLocation],
    binding_regions: &[ResidentRegion],
    binding_local_indices: &std::collections::BTreeMap<u32, u32>,
    layout: &LayoutBuild,
    steps: &mut Vec<ActivatedTurnStep>,
    reads: &mut Vec<ResidentReadLocation>,
    calls: &mut Vec<(
        NodeId,
        crate::memory_planner::CallSiteMemoryTemplate,
        mech_core::CallMemoryPlan,
    )>,
    recursive_roots: &[ActivatedNodeIndex],
) -> Result<ActivatedControlBlock, ResidentActivationError> {
    let source = |value: crate::ControlValue| -> ArtifactSource {
        match value {
            crate::ControlValue::Constant(id) => ArtifactSource::Constant(id),
            crate::ControlValue::Parameter { ordinal, .. } => {
                let input = match block.parameters[ordinal as usize].source {
                    crate::ControlParameterSource::Scrutinee => control.scrutinee,
                    crate::ControlParameterSource::PatternBinding(local) => {
                        return ArtifactSource::Slot(
                            layout.match_bindings[&(owner, pattern_owner, local)].0,
                        );
                    }
                    crate::ControlParameterSource::Capture(index) => {
                        control.captures[index as usize].input
                    }
                };
                captures[input as usize]
            }
            crate::ControlValue::Local { node, .. } => {
                ArtifactSource::Slot(layout.control_locals[&(owner, block.id.0, node)].0)
            }
        }
    };
    let read =
        |value: crate::ControlValue| -> Result<ResidentReadLocation, ResidentActivationError> {
            let crate::ControlValue::Parameter { ordinal, .. } = value else {
                return resolve_read(layout, source(value));
            };
            let selected = match block.parameters[ordinal as usize].source {
                crate::ControlParameterSource::Scrutinee => {
                    capture_reads[control.scrutinee as usize]
                }
                crate::ControlParameterSource::Capture(index) => {
                    let capture = &control.captures[index as usize];
                    let selected = capture_reads[capture.input as usize];
                    match (capture.freeze_on_suspend, selected) {
                        (true, ResidentReadLocation::Input(region)) => {
                            ResidentReadLocation::LexicalInput(region)
                        }
                        _ => selected,
                    }
                }
                crate::ControlParameterSource::PatternBinding(_) => {
                    resolve_read(layout, source(value))?
                }
            };
            Ok(selected)
        };
    let port = |source: ArtifactSource| -> Result<ResidentPortLayout, ResidentActivationError> {
        match source {
            ArtifactSource::Slot(slot) => Ok(slot_port_layout(&layout.slots[slot.get() as usize])),
            ArtifactSource::Constant(id) => {
                let value = artifact.constants().get(id).unwrap();
                let region = layout.constant_regions[id.get() as usize];
                Ok(ResidentPortLayout {
                    kind: region.kind,
                    shape: region.shape,
                    schema_id: value.schema(),
                    schema_key: value.schema_key(),
                    shape_instance: value.shape().clone(),
                    activation_fixed_shape: true,
                    resolved_selector: None,
                })
            }
        }
    };
    let mut direct_steps = Vec::new();
    let mut local_regions = binding_regions.to_vec();
    let binding_count = u32::try_from(binding_regions.len())
        .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
    let mut operation_locals = std::collections::BTreeMap::new();
    for operation in &block.operations {
        let (output_slot, physical_node) =
            layout.control_locals[&(owner, block.id.0, operation.node)];
        let output = &layout.slots[output_slot.get() as usize];
        let inputs = operation
            .inputs
            .iter()
            .copied()
            .map(source)
            .collect::<Vec<_>>();
        let input_reads = operation
            .inputs
            .iter()
            .copied()
            .map(read)
            .collect::<Result<Vec<_>, _>>()?;
        let index =
            u32::try_from(steps.len()).map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
        let nested_match = matches!(operation.body, crate::ControlOperationBody::Match(_));
        let prior_local_count = u32::try_from(local_regions.len())
            .map_err(|_| ResidentActivationError::RegionSizeOverflow)?;
        let current_local = prior_local_count;
        local_regions.push(output.region);
        operation_locals.insert(operation.node, current_local);
        let retained_local_count = prior_local_count
            .checked_add(u32::from(nested_match))
            .ok_or(ResidentActivationError::RegionSizeOverflow)?;
        let excluded_locals = if nested_match {
            Box::new([])
        } else {
            let mut excluded = operation
                .inputs
                .iter()
                .filter_map(|value| match value {
                    crate::ControlValue::Parameter { ordinal, .. } => {
                        let parameter = &block.parameters[*ordinal as usize];
                        let crate::ControlParameterSource::PatternBinding(local) = parameter.source
                        else {
                            return None;
                        };
                        binding_local_indices.get(&local).copied()
                    }
                    crate::ControlValue::Local { node, .. } => operation_locals.get(node).copied(),
                    crate::ControlValue::Constant(_) => None,
                })
                .filter(|local| *local < retained_local_count)
                .collect::<Vec<_>>();
            excluded.sort_unstable();
            excluded.dedup();
            excluded.into_boxed_slice()
        };
        debug_assert!(retained_local_count >= binding_count);
        direct_steps.push(ActivatedControlStep {
            node: ActivatedNodeIndex(index),
            retained_local_count,
            excluded_locals,
        });
        let crate::ControlOperationBody::Operation {
            operation: reference,
            contract: contract_id,
        } = &operation.body
        else {
            match &operation.body {
                crate::ControlOperationBody::Match(nested) => {
                    let prepared = prepare_match_node(
                        artifact,
                        owner,
                        physical_node,
                        nested,
                        &inputs,
                        &input_reads,
                        output_slot,
                        layout,
                    )?;
                    steps.push(ActivatedTurnStep::Match(prepared));
                    let mut nested_roots = recursive_roots.to_vec();
                    nested_roots.push(ActivatedNodeIndex(index));
                    let arms = bind_match_arms(
                        artifact,
                        catalog,
                        owner,
                        nested,
                        &inputs,
                        &input_reads,
                        layout,
                        steps,
                        reads,
                        calls,
                        &nested_roots,
                    )?;
                    let ActivatedTurnStep::Match(prepared) = &mut steps[index as usize] else {
                        unreachable!()
                    };
                    prepared.arms = arms;
                }
                crate::ControlOperationBody::Comprehension(nested) => {
                    let start = reads.len() as u32;
                    reads.extend(input_reads.iter().copied());
                    steps.push(ActivatedTurnStep::Comprehension(std::sync::Arc::new(
                        ActivatedComprehensionNode {
                            artifact_node: owner,
                            memory_node: physical_node,
                            reads: start..reads.len() as u32,
                            write: ResidentWriteLocation {
                                slot: output_slot,
                                storage: ResidentStorageClass::Scratch,
                                region: output.region,
                            },
                            kind: nested.kind,
                            output_schema: output.schema,
                            steps: Box::new([]),
                            locals: Box::new([]),
                            schema_reads: Box::new([]),
                            yield_value: ResidentReadLocation::Scratch(output.region),
                            yield_schema: output.schema,
                        },
                    )));
                    let (
                        nested_steps,
                        nested_locals,
                        nested_schema_reads,
                        yielded,
                        yield_schema,
                        memory,
                    ) = comprehension::bind_inner(
                        artifact,
                        catalog,
                        owner,
                        nested,
                        &inputs,
                        output_slot,
                        layout,
                        steps,
                        reads,
                        calls,
                    )?;
                    let ActivatedTurnStep::Comprehension(prepared) = &mut steps[index as usize]
                    else {
                        unreachable!()
                    };
                    let prepared = std::sync::Arc::get_mut(prepared)
                        .expect("unpublished nested collection plan");
                    prepared.steps = nested_steps;
                    prepared.locals = nested_locals;
                    prepared.schema_reads = nested_schema_reads;
                    prepared.yield_value = yielded;
                    prepared.yield_schema = yield_schema;
                    calls.push((
                        owner,
                        crate::memory_planner::CallSiteMemoryTemplate {
                            node: physical_node,
                            input_sources: inputs.into_boxed_slice(),
                            output_slots: vec![output_slot].into_boxed_slice(),
                        },
                        memory,
                    ));
                }
                crate::ControlOperationBody::Recur(ancestor) => {
                    let target = recursive_roots
                        .len()
                        .checked_sub(usize::from(*ancestor) + 1)
                        .and_then(|index| recursive_roots.get(index))
                        .copied()
                        .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?;
                    let [_argument] = inputs.as_slice() else {
                        return Err(ResidentActivationError::UnsupportedControlLayout {
                            node: owner,
                        });
                    };
                    steps.push(ActivatedTurnStep::Recur(ActivatedRecursiveCall {
                        artifact_node: owner,
                        target,
                        argument: input_reads[0],
                        write: ResidentWriteLocation {
                            slot: output_slot,
                            storage: output.storage,
                            region: output.region,
                        },
                    }));
                }
                crate::ControlOperationBody::Suspend => {
                    let target = recursive_roots
                        .last()
                        .copied()
                        .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?;
                    let [_argument] = inputs.as_slice() else {
                        return Err(ResidentActivationError::UnsupportedControlLayout {
                            node: owner,
                        });
                    };
                    steps.push(ActivatedTurnStep::Suspend(ActivatedSuspension {
                        artifact_node: owner,
                        target,
                        argument: input_reads[0],
                    }));
                }
                crate::ControlOperationBody::Publish => {
                    let target = recursive_roots
                        .last()
                        .copied()
                        .ok_or(ResidentActivationError::UnsupportedControlLayout { node: owner })?;
                    let [_value] = inputs.as_slice() else {
                        return Err(ResidentActivationError::UnsupportedControlLayout {
                            node: owner,
                        });
                    };
                    steps.push(ActivatedTurnStep::Publish(ActivatedPublication {
                        artifact_node: owner,
                        target,
                        value: input_reads[0],
                    }));
                }
                crate::ControlOperationBody::Operation { .. } => unreachable!(),
            }
            continue;
        };
        let input_layouts = inputs
            .iter()
            .copied()
            .map(port)
            .collect::<Result<Vec<_>, _>>()?;
        let (kernel, memory_plan) = bind_resident_operation(
            artifact,
            catalog,
            owner,
            reference,
            *contract_id,
            &input_layouts,
            slot_port_layout(output),
        )?;
        calls.push((
            owner,
            crate::memory_planner::CallSiteMemoryTemplate {
                node: physical_node,
                input_sources: inputs.clone().into_boxed_slice(),
                output_slots: vec![output_slot].into_boxed_slice(),
            },
            memory_plan,
        ));
        let read_start = reads.len() as u32;
        reads.extend(input_reads);
        let mech_core::ResolvedOperationContract::Declared(contract) =
            artifact.contracts().get(*contract_id).unwrap()
        else {
            unreachable!()
        };
        let policy = &contract.outputs[0];
        if policy.change_detection == ChangeDetectionPolicy::SemanticHash {
            return Err(ResidentActivationError::UnsupportedChangeDetection { node: owner });
        }
        steps.push(ActivatedTurnStep::Kernel(ActivatedKernelNode {
            artifact_node: owner,
            memory_node: physical_node,
            reads: read_start..reads.len() as u32,
            write: ResidentWriteLocation {
                slot: output_slot,
                storage: ResidentStorageClass::Scratch,
                region: output.region,
            },
            construction: policy.construction.clone(),
            rmw_base: None,
            rmw_previous: None,
            change_detection: policy.change_detection,
            reads_state: reads[read_start as usize..]
                .iter()
                .any(|read| matches!(read, ResidentReadLocation::State { .. })),
            scratch_prefix_reads: false,
            kernel,
        }));
    }
    let yielded = source(block.yield_value);
    Ok(ActivatedControlBlock {
        steps: direct_steps.into(),
        locals: local_regions.into(),
        yield_value: read(block.yield_value)?,
        yield_layout: port(yielded)?,
    })
}
