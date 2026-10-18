//! Actual typed native transport arenas, distinct from semantic schemas.
use super::runtime::NativeExecutionError;
use mech_core::{
    AllocationPlan, AllocationRole, ArenaBackingKind, ArenaPlacement, ArenaPlan, IntegerWidth,
    ManagedMemoryBudget, MemoryArenaId, MemoryBudgetLimits, MemoryDomain, MemoryLifetime,
    MemoryObjectId, MemoryObjectOwner, MemorySpace, PlannedArenaElement, PlannedArenaProjection,
    PlannedSlotKind, PortDirection, ResourceDemand, RuntimePlanView, ScalarMemoryKind,
};

pub(super) fn transport_bytes(words: usize) -> Result<u64, NativeExecutionError> {
    let bytes =
        words
            .checked_mul(core::mem::size_of::<u64>())
            .ok_or(NativeExecutionError::Descriptor {
                reason: "native transport exceeds the host address range",
            })?;
    u64::try_from(bytes).map_err(|_| NativeExecutionError::Descriptor {
        reason: "native transport byte count exceeds u64",
    })
}

/// The projection retains the sole physical realization. Field order drops it
/// before the domain handle; no generated pointer survives this owner.
pub(super) struct NativeTypedStorage<T: PlannedArenaElement> {
    values: PlannedArenaProjection<T>,
    _domain: MemoryDomain,
}
pub(super) type NativeWordStorage = NativeTypedStorage<u64>;
impl NativeTypedStorage<u64> {
    /// The caller must charge setup work before this initializes every word.
    pub(super) fn new(
        words: usize,
        limit: u64,
        shared_budget: Option<&ManagedMemoryBudget>,
    ) -> Result<Self, NativeExecutionError> {
        Self::for_slot(
            words,
            PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64)),
            limit,
            shared_budget,
        )
    }
}
impl NativeTypedStorage<Option<mech_core::Value>> {
    pub(super) fn new_values(
        count: usize,
        limit: u64,
        budget: &ManagedMemoryBudget,
    ) -> Result<Self, NativeExecutionError> {
        Self::for_slot(
            count,
            PlannedSlotKind::CanonicalValueHandle,
            limit,
            Some(budget),
        )
    }
}
impl<T: PlannedArenaElement> NativeTypedStorage<T> {
    fn for_slot(
        words: usize,
        slot: PlannedSlotKind,
        limit: u64,
        shared_budget: Option<&ManagedMemoryBudget>,
    ) -> Result<Self, NativeExecutionError> {
        let bytes = words
            .checked_mul(core::mem::size_of::<T>())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(NativeExecutionError::Descriptor {
                reason: "native typed transport size overflows",
            })?;
        if bytes > limit {
            return Err(NativeExecutionError::TransportLimit {
                required: bytes,
                limit,
            });
        }
        let budget = shared_budget
            .cloned()
            .unwrap_or_else(|| ManagedMemoryBudget::new(limit));
        let domain = MemoryDomain::with_memory_budget(budget)?;
        if words == 0 {
            return Ok(Self {
                values: PlannedArenaProjection::empty()?,
                _domain: domain,
            });
        }
        let revision = domain.issue_plan_revision()?;
        let arena = MemoryArenaId::new(0);
        let object = MemoryObjectId::new(0);
        // This is actual typed transport, not a relabelled semantic value.
        // Core projection independently verifies T against the planned slot.
        let allocations = [AllocationPlan {
            id: object,
            owner: MemoryObjectOwner::DirectCallPort {
                call: 0,
                direction: PortDirection::Output,
                port: 0,
            },
            role: AllocationRole::FixedStorage,
            slot: Some(slot),
            space: MemorySpace::Host,
            current_bytes: bytes,
            capacity_bytes: bytes,
            payload_block_capacity: 0,
            alignment: core::mem::align_of::<T>() as u32,
            // The owner exists for one evaluate call, including validation
            // and atomic copy-out. This is not canonical Activation scope.
            lifetime: MemoryLifetime::Program,
            placement: ArenaPlacement { arena, offset: 0 },
            reuse_group: None,
        }];
        let arenas = [ArenaPlan {
            id: arena,
            space: MemorySpace::Host,
            backing: ArenaBackingKind::ContiguousBytes,
            alignment: core::mem::align_of::<T>() as u32,
            capacity_bytes: bytes,
            members: Box::new([object]),
        }];
        let reservation = domain.prepare_realization(RuntimePlanView::for_aggregate(
            revision,
            &allocations,
            &arenas,
            ResourceDemand {
                persistent_bytes: bytes,
                turn_peak_bytes: bytes,
                ..ResourceDemand::default()
            },
            MemoryBudgetLimits {
                max_temporary_bytes: Some(limit),
                max_storage_buffer_bytes: Some(limit),
                ..MemoryBudgetLimits::default()
            },
            &[],
            1,
            &[],
        ))?;
        let realized = domain.materialize(reservation)?;
        let values = domain.project_host_arena::<T>(&realized, arena, words)?;
        Ok(Self {
            values,
            _domain: domain,
        })
    }

    pub(super) fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }
    pub(super) fn values(&self) -> &[T] {
        &self.values
    }
    pub(super) fn domain_id(&self) -> mech_core::MemoryDomainId {
        self._domain.id()
    }
}
