//! Explicit resource-budget selection for canonical native programs.
use crate::native_artifact::{NativeExecutionLimits, NativeProgramExecutionLimits};
use mech_core::{ManagedMemoryBudget, MemoryBudgetLimits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeProgramLimitMode {
    /// Set every configurable compilation, execution, library and managed
    /// memory budget to its representation's maximum.
    NoLimits,
}

impl NativeProgramLimitMode {
    #[cfg(feature = "canonical-native-program")]
    pub const fn compilation(self) -> crate::native_artifact::NativeCompileLimits {
        crate::native_artifact::NativeCompileLimits {
            max_analysis_bytes: u64::MAX,
            max_nodes: u64::MAX,
            max_inputs: u64::MAX,
            max_outputs: u64::MAX,
            max_code_bytes: u64::MAX,
            max_stack_bytes: u64::MAX,
        }
    }

    pub const fn execution(self) -> NativeProgramExecutionLimits {
        NativeProgramExecutionLimits {
            execution: NativeExecutionLimits {
                max_work: u64::MAX,
                max_transport_bytes: u64::MAX,
                max_native_stack_bytes: u64::MAX,
            },
            library: MemoryBudgetLimits {
                max_output_elements: Some(u64::MAX),
                max_output_bytes: Some(u64::MAX),
                max_temporary_bytes: Some(u64::MAX),
                max_cloned_bytes: Some(u64::MAX),
                max_retained_nodes: Some(u64::MAX),
                max_comparison_work: Some(u64::MAX),
                max_compute_work: Some(u64::MAX),
                max_scalar_instructions: Some(u64::MAX),
                max_transfer_bytes: Some(u64::MAX),
                max_storage_buffer_bytes: Some(u64::MAX),
                max_storage_bindings: Some(u32::MAX),
            },
        }
    }

    pub fn memory_budget(self) -> ManagedMemoryBudget {
        ManagedMemoryBudget::new(u64::MAX)
    }
}
