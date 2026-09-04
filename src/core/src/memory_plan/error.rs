use super::{MemoryBudgetViolation, MemoryObjectId, MemoryPlanAuditMismatch, MemoryWitnessStage};

#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(not(feature = "no_std"))]
use std::string::String;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryPlanError {
    DescriptorArityMismatch,
    DescriptorMismatch,
    MissingFootprintWitness { stage: MemoryWitnessStage },
    UnsupportedStorageLayout,
    UnsupportedDenseRank { rank: u64 },
    InvalidAlignment { alignment: u32 },
    ArithmeticOverflow { field: &'static str },
    TargetAddressOverflow,
    CyclicDimensionUpperBound,
    CapacityBelowCurrent { current: u64, maximum: u64 },
    DynamicCardinalityExceedsBound { current: u64, maximum: u64 },
    IncompatibleAlias { input: u16, reason: String },
    RequiredInPlaceAliasUnavailable { input: u16 },
    LifetimeOrderInvalid,
    ReuseOverlap,
    MissingImplementationMemoryClass,
    MatrixSolveLayoutInvalid,
    ZeroSizedGpuBinding,
    TargetLimitExceeded { violation: MemoryBudgetViolation },
    ObservationMissing { object: MemoryObjectId },
    ObservationUnexpected { object: MemoryObjectId },
    ObservationExceeded { mismatch: MemoryPlanAuditMismatch },
}

impl core::fmt::Display for MemoryPlanError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

#[cfg(feature = "functions")]
impl crate::MechErrorKind for MemoryPlanError {
    fn name(&self) -> &str {
        "MemoryPlanError"
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
impl std::error::Error for MemoryPlanError {}
