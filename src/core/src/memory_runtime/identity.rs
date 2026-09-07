use crate::MemoryObjectId;
use core::sync::atomic::{AtomicU64, Ordering};

use super::MemoryRuntimeError;

static NEXT_MEMORY_DOMAIN_ID: AtomicU64 = AtomicU64::new(1);

fn next_global_identity(
    counter: &AtomicU64,
    identity: &'static str,
) -> Result<u64, MemoryRuntimeError> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| MemoryRuntimeError::IdentityExhausted { identity })
}

macro_rules! runtime_identity {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

runtime_identity!(MemoryDomainId);
runtime_identity!(MemoryPlanRevision);
runtime_identity!(PublishedValueVersion);
runtime_identity!(RegionIncarnation);

impl MemoryDomainId {
    pub(crate) fn issue() -> Result<Self, MemoryRuntimeError> {
        next_global_identity(&NEXT_MEMORY_DOMAIN_ID, "memory domain").map(Self)
    }
}

impl MemoryPlanRevision {
    pub(crate) const fn initial() -> Self {
        Self(1)
    }

    pub(crate) fn checked_successor(
        self,
        identity: &'static str,
    ) -> Result<Self, MemoryRuntimeError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(MemoryRuntimeError::IdentityExhausted { identity })
    }
}

impl PublishedValueVersion {
    pub(crate) const fn initial() -> Self {
        Self(1)
    }

    pub(crate) fn checked_successor(
        self,
        identity: &'static str,
    ) -> Result<Self, MemoryRuntimeError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(MemoryRuntimeError::IdentityExhausted { identity })
    }
}

impl RegionIncarnation {
    pub(crate) const fn initial() -> Self {
        Self(1)
    }

    pub(crate) fn checked_successor(self) -> Result<Self, MemoryRuntimeError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(MemoryRuntimeError::IdentityExhausted {
                identity: "region incarnation",
            })
    }
}

/// Domain-local physical ownership identity.
///
/// Fields are intentionally private; only a [`MemoryDomain`](super::MemoryDomain)
/// can construct a handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AllocationHandle {
    domain: MemoryDomainId,
    slot: u32,
    generation: u64,
}

impl AllocationHandle {
    pub const fn domain(self) -> MemoryDomainId {
        self.domain
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub(crate) const fn new(domain: MemoryDomainId, slot: u32, generation: u64) -> Self {
        Self {
            domain,
            slot,
            generation,
        }
    }
}

/// Stable R5 object coordinate scoped to one realized plan revision.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlanObjectKey {
    domain: MemoryDomainId,
    revision: MemoryPlanRevision,
    object: MemoryObjectId,
}

impl PlanObjectKey {
    pub const fn domain(self) -> MemoryDomainId {
        self.domain
    }

    pub const fn revision(self) -> MemoryPlanRevision {
        self.revision
    }

    pub const fn object(self) -> MemoryObjectId {
        self.object
    }

    pub(crate) const fn new(
        domain: MemoryDomainId,
        revision: MemoryPlanRevision,
        object: MemoryObjectId,
    ) -> Self {
        Self {
            domain,
            revision,
            object,
        }
    }
}
