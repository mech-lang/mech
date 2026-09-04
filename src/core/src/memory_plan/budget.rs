use super::MemoryObjectOwner;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkDemand {
    pub comparison: u64,
    pub compute: u64,
    pub canonicalization: u64,
    pub scalar_instructions: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceDemand {
    pub persistent_bytes: u64,
    pub activation_bytes: u64,
    pub turn_peak_bytes: u64,
    pub transaction_peak_bytes: u64,
    pub cloned_bytes: u64,
    pub transfer_bytes: u64,
    pub retained_nodes: u64,
    pub output_elements: u64,
    pub storage_bindings: u32,
    pub work: WorkDemand,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryBudgetLimits {
    pub max_output_elements: Option<u64>,
    pub max_output_bytes: Option<u64>,
    pub max_temporary_bytes: Option<u64>,
    pub max_cloned_bytes: Option<u64>,
    pub max_retained_nodes: Option<u64>,
    pub max_comparison_work: Option<u64>,
    pub max_compute_work: Option<u64>,
    pub max_scalar_instructions: Option<u64>,
    pub max_transfer_bytes: Option<u64>,
    pub max_storage_buffer_bytes: Option<u64>,
    pub max_storage_bindings: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryBudgetDimension {
    OutputElements,
    OutputBytes,
    TemporaryBytes,
    ClonedBytes,
    RetainedNodes,
    ComparisonWork,
    ComputeWork,
    ScalarInstructions,
    TransferBytes,
    StorageBufferBytes,
    StorageBindings,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemoryBudgetViolation {
    pub owner: MemoryObjectOwner,
    pub dimension: MemoryBudgetDimension,
    pub required: u64,
    pub limit: u64,
}
