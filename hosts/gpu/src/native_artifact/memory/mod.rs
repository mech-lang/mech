//! Actual typed native transport arenas, distinct from semantic schemas.
use crate::native_artifact::runtime::NativeExecutionError;
use mech_core::{
    FixedHostArenaPlan, IntegerWidth, ManagedMemoryBudget, PlannedArenaElement,
    PlannedArenaProjection, PlannedSlotKind, ScalarMemoryKind,
};

pub(in crate::native_artifact) fn transport_bytes(
    words: usize,
) -> Result<u64, NativeExecutionError> {
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

/// The closed typed projection retains the sole physical realization. No
/// generated pointer survives it. The opaque identity keeps stale/foreign
/// handles distinct without constructing a mutable general-purpose domain.
pub(in crate::native_artifact) struct NativeTypedStorage<T: PlannedArenaElement> {
    values: PlannedArenaProjection<T>,
    identity: mech_core::MemoryDomainId,
}
pub(in crate::native_artifact) type NativeWordStorage = NativeTypedStorage<u64>;
impl NativeTypedStorage<u64> {
    pub(in crate::native_artifact) fn new_compact(
        funded: usize,
        physical: usize,
        limit: u64,
        budget: &ManagedMemoryBudget,
    ) -> Result<Self, NativeExecutionError> {
        Self::for_slot(
            funded,
            physical,
            PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64)),
            limit,
            Some(budget),
        )
    }
    /// The caller must charge setup work before this initializes every word.
    pub(in crate::native_artifact) fn new(
        words: usize,
        limit: u64,
        shared_budget: Option<&ManagedMemoryBudget>,
    ) -> Result<Self, NativeExecutionError> {
        Self::for_slot(
            words,
            words,
            PlannedSlotKind::FixedScalar(ScalarMemoryKind::Unsigned(IntegerWidth::W64)),
            limit,
            shared_budget,
        )
    }
}
impl NativeTypedStorage<Option<mech_core::Value>> {
    pub(in crate::native_artifact) fn new_values(
        count: usize,
        limit: u64,
        budget: &ManagedMemoryBudget,
    ) -> Result<Self, NativeExecutionError> {
        Self::for_slot(
            count,
            count,
            PlannedSlotKind::CanonicalValueHandle,
            limit,
            Some(budget),
        )
    }

    /// Physical canonical-owner storage need not include raw numeric argument
    /// words. Keep the same complete logical admission and transport ceiling.
    pub(in crate::native_artifact) fn new_values_compact(
        funded: usize,
        physical: usize,
        limit: u64,
        budget: &ManagedMemoryBudget,
    ) -> Result<Self, NativeExecutionError> {
        Self::for_slot(
            funded,
            physical,
            PlannedSlotKind::CanonicalValueHandle,
            limit,
            Some(budget),
        )
    }
}
impl<T: PlannedArenaElement> NativeTypedStorage<T> {
    #[cfg(any(test, feature = "canonical-native-program-runtime"))]
    pub(in crate::native_artifact) fn admission_bytes(
        count: usize,
    ) -> Result<u64, NativeExecutionError> {
        let bytes = count
            .checked_mul(core::mem::size_of::<T>())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(NativeExecutionError::Descriptor {
                reason: "native typed transport size overflows",
            })?;
        mech_core::MemoryDomain::host_arena_admission_bytes(bytes)
            .map_err(NativeExecutionError::from)
    }

    fn for_slot(
        words: usize,
        physical: usize,
        slot: PlannedSlotKind,
        limit: u64,
        shared_budget: Option<&ManagedMemoryBudget>,
    ) -> Result<Self, NativeExecutionError> {
        if physical > words {
            return Err(NativeExecutionError::Descriptor {
                reason: "native physical prefix exceeds admitted transport",
            });
        }
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
        // The compiler's native lane fixes T, slot and element count. Lower
        // that closed one-object plan directly; generic region validation,
        // transaction maps and mutable publication registries are unnecessary.
        // Core still checks T/slot compatibility, exact size/alignment, full
        // shared-account admission, initialization and exclusive ownership.
        let plan = FixedHostArenaPlan::<T>::new(words, slot)?;
        debug_assert_eq!(plan.capacity_bytes(), bytes);
        let (values, identity) = plan.materialize_prefix(physical, &budget)?;
        Ok(Self { values, identity })
    }

    pub(in crate::native_artifact) fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }
    pub(in crate::native_artifact) fn values(&self) -> &[T] {
        &self.values
    }
    pub(in crate::native_artifact) fn domain_id(&self) -> mech_core::MemoryDomainId {
        self.identity
    }
}

#[cfg(test)]
mod admission_tests;
