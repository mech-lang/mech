use crate::{MemoryObjectId, MemorySpace};

#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(not(feature = "no_std"))]
use std::string::{String, ToString};

use super::{
    AllocationHandle, MemoryDomainId, MemoryPlanRevision, PlanObjectKey, RegionIncarnation,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryRuntimeError {
    InvalidPlanRevision {
        expected: MemoryPlanRevision,
        actual: MemoryPlanRevision,
    },
    UnknownPlanObject {
        key: PlanObjectKey,
    },
    InvalidAllocationHandle {
        handle: AllocationHandle,
    },
    StaleAllocationGeneration {
        handle: AllocationHandle,
        current: u64,
    },
    StaleRegionIncarnation {
        key: PlanObjectKey,
        expected: RegionIncarnation,
        actual: RegionIncarnation,
    },
    WrongMemoryDomain {
        expected: MemoryDomainId,
        actual: MemoryDomainId,
    },
    IdentityExhausted {
        identity: &'static str,
    },
    UnplannedAllocation {
        object: Option<MemoryObjectId>,
        requested: u64,
    },
    InvalidLayout {
        object: Option<MemoryObjectId>,
        size: u64,
        alignment: u32,
        reason: &'static str,
    },
    CapacityExceeded {
        object: MemoryObjectId,
        requested: u64,
        capacity: u64,
    },
    BudgetExceeded {
        operation: Option<String>,
        requested: u64,
        limit: u64,
    },
    AllocationFailed {
        object: Option<MemoryObjectId>,
        requested: u64,
        alignment: u32,
        space: MemorySpace,
    },
    BorrowConflict {
        object: MemoryObjectId,
    },
    UninitializedAccess {
        object: MemoryObjectId,
        requested: u64,
        initialized: u64,
    },
    InvalidLifetimeTransition {
        object: Option<MemoryObjectId>,
        from: &'static str,
        to: &'static str,
    },
    InvalidReuse {
        object: MemoryObjectId,
        reason: &'static str,
    },
    OutstandingLease {
        handle: AllocationHandle,
    },
    SnapshotRetained {
        handle: AllocationHandle,
    },
    ExternalCapacityExceeded {
        requested: u64,
        capacity: u64,
    },
    CandidateValidationFailed {
        object: Option<MemoryObjectId>,
        reason: String,
    },
    PublicationInProgress,
    PublicationAlreadyCompleted,
    TurnInFlight,
    DeviceLost {
        object: Option<MemoryObjectId>,
    },
    DomainClosed,
    AccountingInvariantViolation {
        dimension: &'static str,
        current: u64,
        change: u64,
    },
}

impl core::fmt::Display for MemoryRuntimeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl crate::MechErrorKind for MemoryRuntimeError {
    fn name(&self) -> &str {
        "MemoryRuntimeError"
    }

    fn message(&self) -> String {
        self.to_string()
    }
}

impl From<MemoryRuntimeError> for crate::MechError {
    fn from(error: MemoryRuntimeError) -> Self {
        crate::MechError::new(error, None).with_compiler_loc()
    }
}

#[cfg(any(not(feature = "no_std"), feature = "std"))]
impl std::error::Error for MemoryRuntimeError {}

pub type MemoryRuntimeResult<T> = Result<T, MemoryRuntimeError>;
