#[cfg(feature = "functions")]
use crate::BoundCall;
use crate::{
    CellSlotId, ConstantId, ExtentEvolution, MemoryTopology, NodeId, PortDirection, RegionPolicy,
    ResolvedValueDescriptor, ScalarMemoryKind, StorageCapabilityDescriptor,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String};

macro_rules! plan_id {
    ($name:ident) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

plan_id!(MemoryObjectId);
plan_id!(MemoryArenaId);
plan_id!(AliasGroupId);
plan_id!(ReuseGroupId);
plan_id!(MemoryPlanPoint);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryTargetKind {
    DirectHost,
    ResidentCpu,
    NativeHost,
    WasmHost,
    Gpu,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemorySpace {
    Host,
    ResidentCpu,
    Device { region: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannedSlotKind {
    FixedScalar(ScalarMemoryKind),
    StringHeader,
    CanonicalValueHandle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageLayoutClass {
    Scalar { slot: PlannedSlotKind },
    DenseColumnMajor { slot: PlannedSlotKind },
    CanonicalSnapshot { topology: MemoryTopology },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SlotLayout {
    pub bytes: u64,
    pub alignment: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapacityAuthority {
    ExactSemantic,
    ActivationSemantic,
    SemanticUpperBound,
    CurrentValueWitness,
    TargetPolicyLimit,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GrowthPolicy {
    Fixed,
    ReservedToBound,
    ReplanBeforeGrowth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapacityRequirement {
    pub current: u64,
    pub required: u64,
    pub maximum: Option<u64>,
    pub authority: CapacityAuthority,
    pub growth: GrowthPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AxisCapacityPlan {
    pub current: u64,
    pub capacity: CapacityRequirement,
    pub evolution: ExtentEvolution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PayloadCapacityPlan {
    pub current_bytes: u64,
    pub required_bytes: u64,
    pub maximum_bytes: Option<u64>,
    pub current_nodes: u64,
    pub required_nodes: u64,
    pub maximum_nodes: Option<u64>,
    pub authority: CapacityAuthority,
    pub growth: GrowthPolicy,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CurrentMemoryFootprint {
    pub logical_elements: u64,
    pub fixed_bytes: u64,
    pub payload_bytes: u64,
    pub encoded_bytes: u64,
    pub retained_nodes: u64,
    pub schema_bytes: u64,
    pub shape_parameter_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryWitnessStage {
    Activation,
    Turn,
    ExternalIngress,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryFootprintWitness {
    Known(CurrentMemoryFootprint),
    Deferred(MemoryWitnessStage),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueLayoutPlan {
    pub storage: StorageLayoutClass,
    pub axes: Box<[AxisCapacityPlan]>,
    pub current_elements: u64,
    pub capacity_elements: CapacityRequirement,
    pub slot: SlotLayout,
    pub strides_bytes: Box<[u64]>,
    pub current_address_span_bytes: u64,
    pub capacity_bytes: u64,
    pub payload: PayloadCapacityPlan,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryObjectOwner {
    Constant(ConstantId),
    Slot(CellSlotId),
    NodeInput {
        node: NodeId,
        port: u16,
    },
    NodeOutput {
        node: NodeId,
        port: u16,
    },
    NodeScratch {
        node: NodeId,
        ordinal: u16,
    },
    TransactionStage {
        node: NodeId,
        output: u16,
    },
    Transfer {
        ordinal: u32,
    },
    DirectCallPort {
        call: u32,
        direction: PortDirection,
        port: u16,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AllocationRole {
    FixedStorage,
    VariablePayload,
    OrderedIndex,
    SelectorPlan,
    Scratch,
    TransactionStage,
    TransferStage,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArenaPlacement {
    pub arena: MemoryArenaId,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryLifetime {
    Program,
    Activation,
    Turn {
        first: MemoryPlanPoint,
        last: MemoryPlanPoint,
    },
    Transaction {
        first: MemoryPlanPoint,
        last: MemoryPlanPoint,
    },
    Transfer {
        first: MemoryPlanPoint,
        last: MemoryPlanPoint,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllocationPlan {
    pub id: MemoryObjectId,
    pub owner: MemoryObjectOwner,
    pub role: AllocationRole,
    pub space: MemorySpace,
    pub current_bytes: u64,
    pub capacity_bytes: u64,
    pub alignment: u32,
    pub lifetime: MemoryLifetime,
    pub placement: ArenaPlacement,
    pub reuse_group: Option<ReuseGroupId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArenaPlan {
    pub id: MemoryArenaId,
    pub space: MemorySpace,
    pub alignment: u32,
    pub capacity_bytes: u64,
    pub members: Box<[MemoryObjectId]>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AliasDecision {
    Disjoint,
    BorrowInput { input: u16 },
    ReuseInput { input: u16 },
    InPlaceRequired { input: u16 },
    StageThenPublish { input: Option<u16> },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TransactionRequirement {
    None,
    StageAndSwap {
        current: MemoryObjectId,
        staged: MemoryObjectId,
    },
    UndoSnapshot {
        target: MemoryObjectId,
        undo: MemoryObjectId,
    },
    DoubleBuffer {
        current: MemoryObjectId,
        next: MemoryObjectId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegionAccessPlan {
    WholeValue,
    Contiguous {
        offset_bytes: u64,
        length_bytes: u64,
    },
    Strided {
        offset_bytes: u64,
        count: u64,
        stride_bytes: u64,
        element_bytes: u64,
    },
    Rectangle {
        base_offset_bytes: u64,
        rows: u64,
        columns: u64,
        row_stride_bytes: u64,
        column_stride_bytes: u64,
    },
    Gather {
        selected_elements: u64,
        index_bytes: u64,
    },
    CollectionEntry {
        key_bytes: u64,
    },
    Deferred(RegionPolicy),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TransferDirection {
    Upload,
    Readback,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferPlan {
    pub slot: CellSlotId,
    pub direction: TransferDirection,
    pub source: MemorySpace,
    pub destination: MemorySpace,
    pub current_bytes: u64,
    pub capacity_bytes: u64,
    pub lifetime: MemoryLifetime,
    pub consumer: Option<NodeId>,
    pub interface_name: Option<String>,
}

/// Pointer-free description of one backing selected by R4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalStorageDescriptor {
    pub capabilities: StorageCapabilityDescriptor,
    pub slot: PlannedSlotKind,
    pub space: MemorySpace,
    pub lifetime: MemoryLifetime,
    pub reusable_turn_temporary: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortMemoryPlan {
    pub descriptor: ResolvedValueDescriptor,
    pub value: ValueLayoutPlan,
    pub region: RegionAccessPlan,
    pub object: MemoryObjectId,
}

#[cfg(feature = "functions")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallMemoryPlan {
    pub bound_call: BoundCall,
    pub inputs: Box<[PortMemoryPlan]>,
    pub outputs: Box<[PortMemoryPlan]>,
    pub allocations: Box<[AllocationPlan]>,
    pub aliases: Box<[AliasDecision]>,
    pub transactions: Box<[TransactionRequirement]>,
    pub implementation_memory: super::ImplementationMemoryClass,
    pub demand: super::ResourceDemand,
    pub deferred_witnesses: Box<[MemoryWitnessStage]>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryPlanAuditStatus {
    Exact,
    WithinPlannedCapacity,
    CapacityDeferredToR6,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryPlanObservation {
    pub object: MemoryObjectId,
    pub current_bytes: u64,
    pub capacity_bytes: u64,
    pub payload_bytes: u64,
    pub retained_nodes: u64,
    pub logical_elements: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryPlanAuditMismatch {
    pub object: MemoryObjectId,
    pub field: &'static str,
    pub planned: u64,
    pub observed: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryPlanAuditReport {
    pub statuses: Box<[(MemoryObjectId, MemoryPlanAuditStatus)]>,
    pub mismatches: Box<[MemoryPlanAuditMismatch]>,
}

impl MemoryPlanAuditReport {
    pub fn assert_conformant(&self) -> Result<(), super::MemoryPlanError> {
        if let Some(mismatch) = self.mismatches.first() {
            return Err(super::MemoryPlanError::ObservationExceeded {
                mismatch: mismatch.clone(),
            });
        }
        Ok(())
    }
}
