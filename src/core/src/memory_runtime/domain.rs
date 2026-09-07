use crate::{
    AllocationPlan, ArenaBackingKind, ArenaPlan, MemoryArenaId, MemoryBudgetLimits,
    MemoryBudgetViolation, MemoryLifetime, MemoryObjectId, MemoryPlanPoint, MemorySpace,
    ResourceDemand,
};

#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
    vec::Vec,
};

use core::cell::RefCell;

use super::{
    AllocationHandle, HostBlock, MemoryDomainId, MemoryPlanRevision, MemoryRuntimeError,
    MemoryRuntimeResult, PlanObjectKey, PublishedValueVersion, RegionIncarnation,
};

/// Borrowed adapter over one existing R5 plan.
#[derive(Clone, Copy, Debug)]
pub struct RuntimePlanView<'a> {
    revision: MemoryPlanRevision,
    allocations: &'a [AllocationPlan],
    arenas: &'a [ArenaPlan],
    admitted_demand: ResourceDemand,
    limits: MemoryBudgetLimits,
    violations: &'a [MemoryBudgetViolation],
}

impl<'a> RuntimePlanView<'a> {
    pub fn new(
        revision: MemoryPlanRevision,
        allocations: &'a [AllocationPlan],
        arenas: &'a [ArenaPlan],
        admitted_demand: ResourceDemand,
        limits: MemoryBudgetLimits,
        violations: &'a [MemoryBudgetViolation],
    ) -> Self {
        Self {
            revision,
            allocations,
            arenas,
            admitted_demand,
            limits,
            violations,
        }
    }

    pub const fn revision(self) -> MemoryPlanRevision {
        self.revision
    }

    pub const fn admitted_demand(self) -> ResourceDemand {
        self.admitted_demand
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnedAllocationState {
    Reserved,
    Initialized,
    Live,
    Retired,
    Free,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeBinding {
    Empty {
        space: MemorySpace,
        alignment: u32,
        incarnation: RegionIncarnation,
    },
    ManagedHostRegion {
        handle: AllocationHandle,
        offset_bytes: u64,
        capacity_bytes: u64,
        required_initialization_bytes: u64,
        initialized_bytes: u64,
        incarnation: RegionIncarnation,
    },
    ManagedCanonicalPayload {
        handle: AllocationHandle,
        capacity_bytes: u64,
        required_initialization_bytes: u64,
        initialized_bytes: u64,
        incarnation: RegionIncarnation,
    },
    Device {
        handle: AllocationHandle,
        capacity_bytes: u64,
        required_initialization_bytes: u64,
        initialized_bytes: u64,
        incarnation: RegionIncarnation,
    },
    PinnedExternal {
        capacity_bytes: u64,
        incarnation: RegionIncarnation,
    },
}

impl RuntimeBinding {
    pub const fn handle(&self) -> Option<AllocationHandle> {
        match self {
            Self::ManagedHostRegion { handle, .. }
            | Self::ManagedCanonicalPayload { handle, .. }
            | Self::Device { handle, .. } => Some(*handle),
            Self::Empty { .. } | Self::PinnedExternal { .. } => None,
        }
    }

    pub const fn capacity_bytes(&self) -> u64 {
        match self {
            Self::Empty { .. } => 0,
            Self::ManagedHostRegion { capacity_bytes, .. }
            | Self::ManagedCanonicalPayload { capacity_bytes, .. }
            | Self::Device { capacity_bytes, .. }
            | Self::PinnedExternal { capacity_bytes, .. } => *capacity_bytes,
        }
    }

    pub const fn initialized_bytes(&self) -> u64 {
        match self {
            Self::Empty { .. } => 0,
            Self::ManagedHostRegion {
                initialized_bytes, ..
            }
            | Self::ManagedCanonicalPayload {
                initialized_bytes, ..
            }
            | Self::Device {
                initialized_bytes, ..
            } => *initialized_bytes,
            Self::PinnedExternal { capacity_bytes, .. } => *capacity_bytes,
        }
    }

    pub const fn required_initialization_bytes(&self) -> u64 {
        match self {
            Self::Empty { .. } => 0,
            Self::ManagedHostRegion {
                required_initialization_bytes,
                ..
            }
            | Self::ManagedCanonicalPayload {
                required_initialization_bytes,
                ..
            }
            | Self::Device {
                required_initialization_bytes,
                ..
            } => *required_initialization_bytes,
            Self::PinnedExternal { capacity_bytes, .. } => *capacity_bytes,
        }
    }

    pub const fn incarnation(&self) -> RegionIncarnation {
        match self {
            Self::Empty { incarnation, .. }
            | Self::ManagedHostRegion { incarnation, .. }
            | Self::ManagedCanonicalPayload { incarnation, .. }
            | Self::Device { incarnation, .. }
            | Self::PinnedExternal { incarnation, .. } => *incarnation,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryLedgerSnapshot {
    pub reserved_bytes: u64,
    pub committed_bytes: u64,
    pub retired_pinned_bytes: u64,
    pub exported_snapshot_bytes: u64,
    pub in_flight_device_bytes: u64,
    pub in_flight_transfer_bytes: u64,
    pub active_reservations: u32,
    pub live_allocations: u32,
    pub retired_allocations: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManagedAllocationObservation {
    pub handle: AllocationHandle,
    pub state: OwnedAllocationState,
    pub capacity_bytes: u64,
    pub actual_block_bytes: u64,
    pub alignment: u32,
    pub actual_block_alignment: u32,
    pub space: MemorySpace,
    pub active_leases: u32,
    pub snapshot_pins: u32,
    pub submission_pins: u32,
}

#[derive(Clone, Debug)]
struct ObjectAuthorization {
    key: PlanObjectKey,
    arena: MemoryArenaId,
    space: MemorySpace,
    current_bytes: u64,
    capacity_bytes: u64,
    alignment: u32,
    offset: u64,
}

#[derive(Clone, Debug)]
struct ArenaAuthorization {
    id: MemoryArenaId,
    space: MemorySpace,
    backing: ArenaBackingKind,
    alignment: u32,
    capacity_bytes: u64,
}

/// Non-forgeable authority to realize exactly one validated R5 plan view.
pub struct MemoryReservation {
    domain: Weak<RefCell<DomainState>>,
    domain_id: MemoryDomainId,
    revision: MemoryPlanRevision,
    bytes: u64,
    active: bool,
    arenas: Box<[ArenaAuthorization]>,
    objects: Box<[ObjectAuthorization]>,
}

impl MemoryReservation {
    pub const fn revision(&self) -> MemoryPlanRevision {
        self.revision
    }

    pub const fn reserved_bytes(&self) -> u64 {
        self.bytes
    }

    fn release(&mut self) {
        if !self.active {
            return;
        }
        if let Some(domain) = self.domain.upgrade()
            && let Ok(mut state) = domain.try_borrow_mut()
        {
            let _ = state.release_reservation(self.bytes);
        }
        self.active = false;
        self.bytes = 0;
    }
}

impl Drop for MemoryReservation {
    fn drop(&mut self) {
        self.release();
    }
}

/// Receipt mapping every R5 plan object to its current live runtime binding.
#[derive(Clone)]
pub struct RealizedMemoryPlan {
    domain: MemoryDomainId,
    revision: MemoryPlanRevision,
    bindings: Rc<RefCell<Box<[(PlanObjectKey, RuntimeBinding)]>>>,
}

impl RealizedMemoryPlan {
    pub const fn domain(&self) -> MemoryDomainId {
        self.domain
    }

    pub const fn revision(&self) -> MemoryPlanRevision {
        self.revision
    }

    pub fn binding(&self, key: PlanObjectKey) -> MemoryRuntimeResult<RuntimeBinding> {
        if key.revision() != self.revision {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: self.revision,
                actual: key.revision(),
            });
        }
        let bindings = self.bindings.borrow();
        bindings
            .binary_search_by_key(&key, |(candidate, _)| *candidate)
            .ok()
            .map(|index| bindings[index].1.clone())
            .ok_or(MemoryRuntimeError::UnknownPlanObject { key })
    }

    pub fn bindings(&self) -> Box<[(PlanObjectKey, RuntimeBinding)]> {
        self.bindings.borrow().to_vec().into_boxed_slice()
    }

    pub(crate) fn record_initialized(
        &self,
        key: PlanObjectKey,
        initialized_bytes: u64,
    ) -> MemoryRuntimeResult<()> {
        let mut bindings = self.bindings.borrow_mut();
        let index = bindings
            .binary_search_by_key(&key, |(candidate, _)| *candidate)
            .map_err(|_| MemoryRuntimeError::UnknownPlanObject { key })?;
        let binding = &mut bindings[index].1;
        if initialized_bytes > binding.capacity_bytes() {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: key.object(),
                requested: initialized_bytes,
                capacity: binding.capacity_bytes(),
            });
        }
        match binding {
            RuntimeBinding::ManagedHostRegion {
                initialized_bytes: current,
                ..
            }
            | RuntimeBinding::ManagedCanonicalPayload {
                initialized_bytes: current,
                ..
            }
            | RuntimeBinding::Device {
                initialized_bytes: current,
                ..
            } => *current = (*current).max(initialized_bytes),
            RuntimeBinding::Empty { .. } if initialized_bytes == 0 => {}
            RuntimeBinding::Empty { .. } | RuntimeBinding::PinnedExternal { .. } => {
                return Err(MemoryRuntimeError::UnplannedAllocation {
                    object: Some(key.object()),
                    requested: initialized_bytes,
                });
            }
        }
        Ok(())
    }
}

pub(crate) struct ActiveLeaseRecord {
    pub token: u64,
    pub start: u64,
    pub end: u64,
    pub write: bool,
}

pub(crate) struct AllocationRecord {
    pub state: OwnedAllocationState,
    pub block: Option<HostBlock>,
    pub capacity_bytes: u64,
    pub alignment: u32,
    pub space: MemorySpace,
    pub leases: Vec<ActiveLeaseRecord>,
    pub snapshot_pins: u32,
    pub submission_pins: u32,
}

pub(crate) struct AllocationSlot {
    pub generation: u64,
    pub permanently_retired: bool,
    pub record: Option<AllocationRecord>,
}

pub(crate) struct DomainState {
    pub id: MemoryDomainId,
    pub closed: bool,
    pub next_revision: MemoryPlanRevision,
    pub latest_issued_revision: Option<MemoryPlanRevision>,
    pub next_publication: PublishedValueVersion,
    pub next_lease_token: u64,
    pub ledger: MemoryLedgerSnapshot,
    pub allocations: Vec<AllocationSlot>,
}

impl DomainState {
    fn release_reservation(&mut self, bytes: u64) -> MemoryRuntimeResult<()> {
        self.ledger.reserved_bytes = self.ledger.reserved_bytes.checked_sub(bytes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "reserved bytes",
                current: self.ledger.reserved_bytes,
                change: bytes,
            },
        )?;
        self.ledger.active_reservations = self.ledger.active_reservations.checked_sub(1).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "active reservations",
                current: u64::from(self.ledger.active_reservations),
                change: 1,
            },
        )?;
        Ok(())
    }

    pub(crate) fn record(
        &self,
        handle: AllocationHandle,
    ) -> MemoryRuntimeResult<&AllocationRecord> {
        self.validate_domain(handle)?;
        let slot = self
            .allocations
            .get(handle.slot() as usize)
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
        if slot.generation != handle.generation() {
            return Err(MemoryRuntimeError::StaleAllocationGeneration {
                handle,
                current: slot.generation,
            });
        }
        slot.record
            .as_ref()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })
    }

    pub(crate) fn record_mut(
        &mut self,
        handle: AllocationHandle,
    ) -> MemoryRuntimeResult<&mut AllocationRecord> {
        self.validate_domain(handle)?;
        let slot = self
            .allocations
            .get_mut(handle.slot() as usize)
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })?;
        if slot.generation != handle.generation() {
            return Err(MemoryRuntimeError::StaleAllocationGeneration {
                handle,
                current: slot.generation,
            });
        }
        slot.record
            .as_mut()
            .ok_or(MemoryRuntimeError::InvalidAllocationHandle { handle })
    }

    fn validate_domain(&self, handle: AllocationHandle) -> MemoryRuntimeResult<()> {
        if handle.domain() != self.id {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id,
                actual: handle.domain(),
            });
        }
        Ok(())
    }

    fn insert_record(&mut self, record: AllocationRecord) -> MemoryRuntimeResult<AllocationHandle> {
        let candidate = self
            .allocations
            .iter()
            .position(|slot| slot.record.is_none() && !slot.permanently_retired);
        let slot = if let Some(slot) = candidate {
            slot
        } else {
            self.allocations.push(AllocationSlot {
                generation: 1,
                permanently_retired: false,
                record: None,
            });
            self.allocations.len() - 1
        };
        let raw_slot = u32::try_from(slot).map_err(|_| MemoryRuntimeError::IdentityExhausted {
            identity: "allocation slot",
        })?;
        let generation = self.allocations[slot].generation;
        self.allocations[slot].record = Some(record);
        self.ledger.live_allocations = self.ledger.live_allocations.checked_add(1).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "live allocation count",
            },
        )?;
        Ok(AllocationHandle::new(self.id, raw_slot, generation))
    }
}

/// Owner-thread-confined managed memory domain.
#[derive(Clone)]
pub struct MemoryDomain {
    pub(crate) state: Rc<RefCell<DomainState>>,
}

impl MemoryDomain {
    pub fn new() -> MemoryRuntimeResult<Self> {
        let id = MemoryDomainId::issue()?;
        Ok(Self {
            state: Rc::new(RefCell::new(DomainState {
                id,
                closed: false,
                next_revision: MemoryPlanRevision::initial(),
                latest_issued_revision: None,
                next_publication: PublishedValueVersion::initial(),
                next_lease_token: 1,
                ledger: MemoryLedgerSnapshot::default(),
                allocations: Vec::new(),
            })),
        })
    }

    pub fn id(&self) -> MemoryDomainId {
        self.state.borrow().id
    }

    pub fn issue_plan_revision(&self) -> MemoryRuntimeResult<MemoryPlanRevision> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let revision = state.next_revision;
        state.next_revision = revision.checked_successor("memory plan revision")?;
        state.latest_issued_revision = Some(revision);
        Ok(revision)
    }

    pub fn plan_object_key(
        &self,
        revision: MemoryPlanRevision,
        object: MemoryObjectId,
    ) -> MemoryRuntimeResult<PlanObjectKey> {
        let state = self.state.borrow();
        if state.latest_issued_revision != Some(revision) {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.latest_issued_revision.unwrap_or(state.next_revision),
                actual: revision,
            });
        }
        Ok(PlanObjectKey::new(revision, object))
    }

    pub fn prepare_realization(
        &self,
        view: RuntimePlanView<'_>,
    ) -> MemoryRuntimeResult<MemoryReservation> {
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        if state.latest_issued_revision != Some(view.revision) {
            return Err(MemoryRuntimeError::InvalidPlanRevision {
                expected: state.latest_issued_revision.unwrap_or(state.next_revision),
                actual: view.revision,
            });
        }
        if let Some(violation) = view.violations.first() {
            return Err(MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: violation.required,
                limit: violation.limit,
            });
        }
        let (arenas, objects, bytes) = validate_plan_view(view)?;
        let record_count = arenas
            .iter()
            .filter(|arena| arena.backing == ArenaBackingKind::ContiguousBytes)
            .count()
            + objects
                .iter()
                .filter(|object| {
                    arenas.iter().any(|arena| {
                        arena.id == object.arena
                            && arena.backing == ArenaBackingKind::IndirectOwnedPayloads
                    })
                })
                .count();
        state.allocations.try_reserve(record_count).map_err(|_| {
            MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: u64::try_from(record_count).unwrap_or(u64::MAX),
                alignment: 1,
                space: MemorySpace::Host,
            }
        })?;
        state.ledger.reserved_bytes = state.ledger.reserved_bytes.checked_add(bytes).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "reserved bytes",
                current: state.ledger.reserved_bytes,
                change: bytes,
            },
        )?;
        state.ledger.active_reservations = state.ledger.active_reservations.checked_add(1).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "active reservation count",
            },
        )?;
        Ok(MemoryReservation {
            domain: Rc::downgrade(&self.state),
            domain_id: state.id,
            revision: view.revision,
            bytes,
            active: true,
            arenas,
            objects,
        })
    }

    pub fn materialize(
        &self,
        mut reservation: MemoryReservation,
    ) -> MemoryRuntimeResult<RealizedMemoryPlan> {
        if !reservation.active {
            return Err(MemoryRuntimeError::UnplannedAllocation {
                object: None,
                requested: 0,
            });
        }
        if reservation.domain_id != self.id() {
            return Err(MemoryRuntimeError::WrongMemoryDomain {
                expected: self.id(),
                actual: reservation.domain_id,
            });
        }

        struct PendingArena {
            authorization: ArenaAuthorization,
            block: HostBlock,
        }
        struct PendingObject {
            authorization: ObjectAuthorization,
            block: HostBlock,
        }

        let mut contiguous = Vec::new();
        let mut indirect = Vec::new();
        contiguous
            .try_reserve_exact(reservation.arenas.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: reservation.bytes,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        indirect
            .try_reserve_exact(reservation.objects.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: reservation.bytes,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        for arena in reservation.arenas.iter().cloned() {
            match arena.backing {
                ArenaBackingKind::ContiguousBytes => {
                    let block = HostBlock::allocate(
                        None,
                        arena.capacity_bytes,
                        arena.alignment,
                        arena.space,
                    )?;
                    contiguous.push(PendingArena {
                        authorization: arena,
                        block,
                    });
                }
                ArenaBackingKind::IndirectOwnedPayloads => {}
            }
        }
        for object in reservation.objects.iter().cloned() {
            let backing = reservation
                .arenas
                .iter()
                .find(|arena| arena.id == object.arena)
                .map(|arena| arena.backing)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?;
            if backing == ArenaBackingKind::IndirectOwnedPayloads {
                let block = HostBlock::allocate(
                    Some(object.key.object()),
                    object.capacity_bytes,
                    object.alignment,
                    object.space,
                )?;
                indirect.push(PendingObject {
                    authorization: object,
                    block,
                });
            }
        }

        let mut bindings = Vec::new();
        bindings
            .try_reserve_exact(reservation.objects.len())
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: reservation.bytes,
                alignment: 1,
                space: MemorySpace::Host,
            })?;
        let mut state = self.state.borrow_mut();
        if state.closed {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let mut arena_handles = BTreeMap::new();
        for pending in contiguous {
            let capacity = pending.authorization.capacity_bytes;
            let handle = state.insert_record(AllocationRecord {
                state: OwnedAllocationState::Live,
                block: Some(pending.block),
                capacity_bytes: capacity,
                alignment: pending.authorization.alignment,
                space: pending.authorization.space,
                leases: Vec::new(),
                snapshot_pins: 0,
                submission_pins: 0,
            })?;
            arena_handles.insert(pending.authorization.id, handle);
        }
        let mut indirect_handles = BTreeMap::new();
        for pending in indirect {
            let capacity = pending.authorization.capacity_bytes;
            let handle = state.insert_record(AllocationRecord {
                state: OwnedAllocationState::Live,
                block: Some(pending.block),
                capacity_bytes: capacity,
                alignment: pending.authorization.alignment,
                space: pending.authorization.space,
                leases: Vec::new(),
                snapshot_pins: 0,
                submission_pins: 0,
            })?;
            indirect_handles.insert(pending.authorization.key, handle);
        }
        for object in &reservation.objects {
            let arena = reservation
                .arenas
                .iter()
                .find(|arena| arena.id == object.arena)
                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?;
            let incarnation = RegionIncarnation::initial();
            let binding = if object.capacity_bytes == 0 {
                RuntimeBinding::Empty {
                    space: object.space,
                    alignment: object.alignment,
                    incarnation,
                }
            } else {
                match arena.backing {
                    ArenaBackingKind::ContiguousBytes => RuntimeBinding::ManagedHostRegion {
                        handle: *arena_handles
                            .get(&object.arena)
                            .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?,
                        offset_bytes: object.offset,
                        capacity_bytes: object.capacity_bytes,
                        required_initialization_bytes: object.current_bytes,
                        initialized_bytes: 0,
                        incarnation,
                    },
                    ArenaBackingKind::IndirectOwnedPayloads => {
                        RuntimeBinding::ManagedCanonicalPayload {
                            handle: *indirect_handles
                                .get(&object.key)
                                .ok_or(MemoryRuntimeError::UnknownPlanObject { key: object.key })?,
                            capacity_bytes: object.capacity_bytes,
                            required_initialization_bytes: object.current_bytes,
                            initialized_bytes: 0,
                            incarnation,
                        }
                    }
                }
            };
            bindings.push((object.key, binding));
        }
        bindings.sort_by_key(|(key, _)| *key);
        state.ledger.committed_bytes = state
            .ledger
            .committed_bytes
            .checked_add(reservation.bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "committed bytes",
                current: state.ledger.committed_bytes,
                change: reservation.bytes,
            })?;
        state.release_reservation(reservation.bytes)?;
        reservation.active = false;
        reservation.bytes = 0;
        Ok(RealizedMemoryPlan {
            domain: state.id,
            revision: reservation.revision,
            bindings: Rc::new(RefCell::new(bindings.into_boxed_slice())),
        })
    }

    pub fn retire(&self, handle: AllocationHandle) -> MemoryRuntimeResult<()> {
        let mut state = self.state.borrow_mut();
        let capacity = {
            let record = state.record_mut(handle)?;
            if record.state == OwnedAllocationState::Retired {
                return Ok(());
            }
            if !record.leases.is_empty() {
                return Err(MemoryRuntimeError::OutstandingLease { handle });
            }
            record.state = OwnedAllocationState::Retired;
            record.capacity_bytes
        };
        state.ledger.live_allocations = state.ledger.live_allocations.checked_sub(1).ok_or(
            MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "live allocations",
                current: u64::from(state.ledger.live_allocations),
                change: 1,
            },
        )?;
        state.ledger.retired_allocations = state.ledger.retired_allocations.checked_add(1).ok_or(
            MemoryRuntimeError::IdentityExhausted {
                identity: "retired allocation count",
            },
        )?;
        state.ledger.retired_pinned_bytes = state
            .ledger
            .retired_pinned_bytes
            .checked_add(capacity)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "retired pinned bytes",
                current: state.ledger.retired_pinned_bytes,
                change: capacity,
            })?;
        Ok(())
    }

    pub fn collect_retired(&self) -> MemoryRuntimeResult<u32> {
        let mut state = self.state.borrow_mut();
        let mut reclaimed = 0_u32;
        let mut reclaimed_bytes = 0_u64;
        for slot in &mut state.allocations {
            let reclaimable = slot.record.as_ref().is_some_and(|record| {
                record.state == OwnedAllocationState::Retired
                    && record.leases.is_empty()
                    && record.snapshot_pins == 0
                    && record.submission_pins == 0
            });
            if !reclaimable {
                continue;
            }
            let record = slot.record.take().expect("reclaimable record exists");
            reclaimed_bytes = reclaimed_bytes.checked_add(record.capacity_bytes).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "reclaimed bytes",
                    current: reclaimed_bytes,
                    change: record.capacity_bytes,
                },
            )?;
            if let Some(next) = slot.generation.checked_add(1) {
                slot.generation = next;
            } else {
                slot.permanently_retired = true;
            }
            reclaimed = reclaimed
                .checked_add(1)
                .ok_or(MemoryRuntimeError::IdentityExhausted {
                    identity: "reclaimed allocation count",
                })?;
        }
        state.ledger.committed_bytes = state
            .ledger
            .committed_bytes
            .checked_sub(reclaimed_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "committed bytes",
                current: state.ledger.committed_bytes,
                change: reclaimed_bytes,
            })?;
        state.ledger.retired_pinned_bytes = state
            .ledger
            .retired_pinned_bytes
            .checked_sub(reclaimed_bytes)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "retired pinned bytes",
                current: state.ledger.retired_pinned_bytes,
                change: reclaimed_bytes,
            })?;
        state.ledger.retired_allocations = state
            .ledger
            .retired_allocations
            .checked_sub(reclaimed)
            .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
                dimension: "retired allocations",
                current: u64::from(state.ledger.retired_allocations),
                change: u64::from(reclaimed),
            })?;
        Ok(reclaimed)
    }

    pub fn close(&self) -> MemoryRuntimeResult<()> {
        let handles = {
            let mut state = self.state.borrow_mut();
            if state.closed {
                return Ok(());
            }
            if state.ledger.active_reservations != 0 {
                return Err(MemoryRuntimeError::TurnInFlight);
            }
            state.closed = true;
            state
                .allocations
                .iter()
                .enumerate()
                .filter_map(|(slot, entry)| {
                    entry.record.as_ref().and_then(|record| {
                        (record.state == OwnedAllocationState::Live)
                            .then(|| AllocationHandle::new(state.id, slot as u32, entry.generation))
                    })
                })
                .collect::<Vec<_>>()
        };
        for handle in handles {
            self.retire(handle)?;
        }
        self.collect_retired()?;
        Ok(())
    }

    pub fn ledger(&self) -> MemoryLedgerSnapshot {
        self.state.borrow().ledger
    }

    pub fn allocation_observations(&self) -> Box<[ManagedAllocationObservation]> {
        let state = self.state.borrow();
        state
            .allocations
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| {
                entry.record.as_ref().map(|record| {
                    let (actual_block_bytes, actual_block_alignment) = record
                        .block
                        .as_ref()
                        .map(|block| {
                            (
                                u64::try_from(block.bytes()).unwrap_or(u64::MAX),
                                u32::try_from(block.alignment()).unwrap_or(u32::MAX),
                            )
                        })
                        .unwrap_or((0, 1));
                    ManagedAllocationObservation {
                        handle: AllocationHandle::new(state.id, slot as u32, entry.generation),
                        state: record.state,
                        capacity_bytes: record.capacity_bytes,
                        actual_block_bytes,
                        alignment: record.alignment,
                        actual_block_alignment,
                        space: record.space,
                        active_leases: u32::try_from(record.leases.len()).unwrap_or(u32::MAX),
                        snapshot_pins: record.snapshot_pins,
                        submission_pins: record.submission_pins,
                    }
                })
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}

fn validate_plan_view(
    view: RuntimePlanView<'_>,
) -> MemoryRuntimeResult<(Box<[ArenaAuthorization]>, Box<[ObjectAuthorization]>, u64)> {
    let mut arena_ids = BTreeSet::new();
    let mut arena_members = BTreeMap::<MemoryArenaId, BTreeSet<MemoryObjectId>>::new();
    let mut arenas = Vec::new();
    arenas.try_reserve_exact(view.arenas.len()).map_err(|_| {
        MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: view.arenas.len() as u64,
            alignment: 1,
            space: MemorySpace::Host,
        }
    })?;
    for arena in view.arenas {
        if !arena_ids.insert(arena.id) {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "duplicate arena id",
            });
        }
        if arena.alignment == 0 || !arena.alignment.is_power_of_two() {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "arena alignment is not a nonzero power of two",
            });
        }
        if let Some(limit) = view.limits.max_storage_buffer_bytes
            && arena.backing == ArenaBackingKind::ContiguousBytes
            && arena.capacity_bytes > limit
        {
            return Err(MemoryRuntimeError::BudgetExceeded {
                operation: None,
                requested: arena.capacity_bytes,
                limit,
            });
        }
        let members = arena.members.iter().copied().collect::<BTreeSet<_>>();
        if members.len() != arena.members.len() {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "arena repeats a member object",
            });
        }
        arena_members.insert(arena.id, members);
        arenas.push(ArenaAuthorization {
            id: arena.id,
            space: arena.space,
            backing: arena.backing,
            alignment: arena.alignment,
            capacity_bytes: arena.capacity_bytes,
        });
    }

    let mut object_ids = BTreeSet::new();
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(view.allocations.len())
        .map_err(|_| MemoryRuntimeError::AllocationFailed {
            object: None,
            requested: view.allocations.len() as u64,
            alignment: 1,
            space: MemorySpace::Host,
        })?;
    for allocation in view.allocations {
        if !object_ids.insert(allocation.id) {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "duplicate memory object id",
            });
        }
        let arena = view
            .arenas
            .iter()
            .find(|arena| arena.id == allocation.placement.arena)
            .ok_or(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation references an unknown arena",
            })?;
        if !arena_members
            .get(&arena.id)
            .is_some_and(|members| members.contains(&allocation.id))
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation is absent from its arena member list",
            });
        }
        if arena.space != allocation.space {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation and arena use different memory spaces",
            });
        }
        if allocation.current_bytes > allocation.capacity_bytes {
            return Err(MemoryRuntimeError::CapacityExceeded {
                object: allocation.id,
                requested: allocation.current_bytes,
                capacity: allocation.capacity_bytes,
            });
        }
        if allocation.alignment == 0 || !allocation.alignment.is_power_of_two() {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: Some(allocation.id),
                size: allocation.capacity_bytes,
                alignment: allocation.alignment,
                reason: "allocation alignment is not a nonzero power of two",
            });
        }
        if arena.backing == ArenaBackingKind::ContiguousBytes {
            if allocation.placement.offset % u64::from(allocation.alignment) != 0 {
                return Err(MemoryRuntimeError::InvalidLayout {
                    object: Some(allocation.id),
                    size: allocation.capacity_bytes,
                    alignment: allocation.alignment,
                    reason: "allocation offset violates its alignment",
                });
            }
            let end = allocation
                .placement
                .offset
                .checked_add(allocation.capacity_bytes)
                .ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(allocation.id),
                    size: allocation.capacity_bytes,
                    alignment: allocation.alignment,
                    reason: "allocation end overflows",
                })?;
            if end > arena.capacity_bytes {
                return Err(MemoryRuntimeError::CapacityExceeded {
                    object: allocation.id,
                    requested: end,
                    capacity: arena.capacity_bytes,
                });
            }
        }
        objects.push(ObjectAuthorization {
            key: PlanObjectKey::new(view.revision, allocation.id),
            arena: arena.id,
            space: allocation.space,
            current_bytes: allocation.current_bytes,
            capacity_bytes: allocation.capacity_bytes,
            alignment: allocation.alignment,
            offset: allocation.placement.offset,
        });
    }
    for arena in view.arenas {
        if arena
            .members
            .iter()
            .any(|member| !object_ids.contains(member))
        {
            return Err(MemoryRuntimeError::InvalidLayout {
                object: None,
                size: arena.capacity_bytes,
                alignment: arena.alignment,
                reason: "arena member references an unknown allocation",
            });
        }
    }
    validate_overlaps(view.allocations, view.arenas)?;

    let mut bytes = 0_u64;
    for arena in view.arenas {
        if arena.backing == ArenaBackingKind::ContiguousBytes {
            bytes = bytes.checked_add(arena.capacity_bytes).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "reservation bytes",
                    current: bytes,
                    change: arena.capacity_bytes,
                },
            )?;
        }
    }
    for object in &objects {
        if arenas.iter().any(|arena| {
            arena.id == object.arena && arena.backing == ArenaBackingKind::IndirectOwnedPayloads
        }) {
            bytes = bytes.checked_add(object.capacity_bytes).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "reservation bytes",
                    current: bytes,
                    change: object.capacity_bytes,
                },
            )?;
        }
    }
    Ok((arenas.into_boxed_slice(), objects.into_boxed_slice(), bytes))
}

fn validate_overlaps(
    allocations: &[AllocationPlan],
    arenas: &[ArenaPlan],
) -> MemoryRuntimeResult<()> {
    for arena in arenas {
        if arena.backing == ArenaBackingKind::IndirectOwnedPayloads {
            continue;
        }
        let members = allocations
            .iter()
            .filter(|allocation| allocation.placement.arena == arena.id)
            .collect::<Vec<_>>();
        for (position, left) in members.iter().enumerate() {
            let left_end = left
                .placement
                .offset
                .checked_add(left.capacity_bytes)
                .ok_or(MemoryRuntimeError::InvalidLayout {
                    object: Some(left.id),
                    size: left.capacity_bytes,
                    alignment: left.alignment,
                    reason: "allocation end overflows",
                })?;
            for right in &members[position + 1..] {
                let right_end = right
                    .placement
                    .offset
                    .checked_add(right.capacity_bytes)
                    .ok_or(MemoryRuntimeError::InvalidLayout {
                        object: Some(right.id),
                        size: right.capacity_bytes,
                        alignment: right.alignment,
                        reason: "allocation end overflows",
                    })?;
                let overlaps = left.placement.offset < right_end
                    && right.placement.offset < left_end
                    && left.capacity_bytes != 0
                    && right.capacity_bytes != 0;
                if !overlaps {
                    continue;
                }
                let allowed = left.reuse_group.is_some()
                    && left.reuse_group == right.reuse_group
                    && lifetimes_disjoint(left.lifetime, right.lifetime);
                if !allowed {
                    return Err(MemoryRuntimeError::InvalidReuse {
                        object: right.id,
                        reason: "overlapping live regions are not one disjoint-lifetime reuse group",
                    });
                }
            }
        }
    }
    Ok(())
}

fn lifetimes_disjoint(left: MemoryLifetime, right: MemoryLifetime) -> bool {
    fn interval(lifetime: MemoryLifetime) -> Option<(MemoryPlanPoint, MemoryPlanPoint)> {
        match lifetime {
            MemoryLifetime::Turn { first, last }
            | MemoryLifetime::Transaction { first, last }
            | MemoryLifetime::Transfer { first, last } => Some((first, last)),
            MemoryLifetime::Program | MemoryLifetime::Activation => None,
        }
    }
    match (interval(left), interval(right)) {
        (Some((left_first, left_last)), Some((right_first, right_last))) => {
            left_last < right_first || right_last < left_first
        }
        _ => false,
    }
}
