use super::MemoryObjectOwner;

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

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

pub fn evaluate_memory_budget(
    owner: MemoryObjectOwner,
    demand: ResourceDemand,
    output_bytes: u64,
    storage_buffer_bytes: u64,
    limits: MemoryBudgetLimits,
) -> Box<[MemoryBudgetViolation]> {
    let mut violations = Vec::new();
    macro_rules! check {
        ($dimension:ident, $required:expr, $limit:expr) => {
            if let Some(limit) = $limit
                && $required > limit
            {
                violations.push(MemoryBudgetViolation {
                    owner: owner.clone(),
                    dimension: MemoryBudgetDimension::$dimension,
                    required: $required,
                    limit,
                });
            }
        };
    }
    check!(
        OutputElements,
        demand.output_elements,
        limits.max_output_elements
    );
    check!(OutputBytes, output_bytes, limits.max_output_bytes);
    check!(
        TemporaryBytes,
        demand.turn_peak_bytes,
        limits.max_temporary_bytes
    );
    check!(ClonedBytes, demand.cloned_bytes, limits.max_cloned_bytes);
    check!(
        RetainedNodes,
        demand.retained_nodes,
        limits.max_retained_nodes
    );
    check!(
        ComparisonWork,
        demand.work.comparison,
        limits.max_comparison_work
    );
    check!(ComputeWork, demand.work.compute, limits.max_compute_work);
    check!(
        ScalarInstructions,
        demand.work.scalar_instructions,
        limits.max_scalar_instructions
    );
    check!(
        TransferBytes,
        demand.transfer_bytes,
        limits.max_transfer_bytes
    );
    check!(
        StorageBufferBytes,
        storage_buffer_bytes,
        limits.max_storage_buffer_bytes
    );
    check!(
        StorageBindings,
        u64::from(demand.storage_bindings),
        limits.max_storage_bindings.map(u64::from)
    );
    violations.sort();
    violations.into_boxed_slice()
}
